use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::model::Finding;
use crate::ownership;

const MAX_ANCESTORS: usize = 16;

#[derive(Debug)]
struct ProcessInfo {
    pid: u32,
    parent: Option<u32>,
    name: String,
    command: String,
    executable: Option<PathBuf>,
    cwd: Option<PathBuf>,
}

pub fn resolve(pid: u32) -> Result<Finding> {
    let info = inspect(pid)?;

    let mut finding = Finding::new(
        pid.to_string(),
        "process",
        format!("{} is running as PID {pid}", info.name),
    )
    .fact("Name", info.name.clone());

    if !info.command.is_empty() {
        finding = finding.fact("Command", info.command.clone());
    }

    if let Some(exe) = &info.executable {
        finding = finding.fact("Executable", exe.display().to_string());

        if let Some(package) = ownership::nix_store_package(exe) {
            finding = finding.fact("Provided by", package);
        } else if let Some(package) = ownership::pacman_owner(exe)? {
            finding = finding.fact("Provided by", package);
        }
    }

    if let Some(cwd) = &info.cwd {
        finding = finding.fact("Working dir", cwd.display().to_string());

        if let Some(project) = project_root(cwd) {
            finding = finding.fact("Project", project.display().to_string());
        }
    }

    if let Some(unit) = systemd_unit(pid) {
        finding = finding.fact("Systemd unit", unit);
    }

    let ancestry = ancestry(pid);
    if ancestry.len() > 1 {
        finding = finding.fact("Why running", render_ancestry(&ancestry));
    } else if let Some(parent) = info.parent {
        finding = finding.fact("Parent PID", parent.to_string());
    }

    Ok(finding)
}

fn inspect(pid: u32) -> Result<ProcessInfo> {
    let root = PathBuf::from(format!("/proc/{pid}"));
    if !root.exists() {
        bail!("process {pid} does not exist");
    }

    let name = fs::read_to_string(root.join("comm"))
        .with_context(|| format!("failed to read process {pid} name"))?
        .trim()
        .to_owned();

    Ok(ProcessInfo {
        pid,
        parent: parent_pid(pid).ok().flatten(),
        name,
        command: command_line(&root),
        executable: fs::read_link(root.join("exe")).ok(),
        cwd: fs::read_link(root.join("cwd")).ok(),
    })
}

fn command_line(root: &Path) -> String {
    fs::read(root.join("cmdline"))
        .ok()
        .map(|bytes| {
            bytes
                .split(|byte| *byte == 0)
                .filter(|part| !part.is_empty())
                .map(|part| String::from_utf8_lossy(part))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

fn parent_pid(pid: u32) -> Result<Option<u32>> {
    let status = fs::read_to_string(format!("/proc/{pid}/status"))?;
    Ok(status.lines().find_map(|line| {
        let value = line.strip_prefix("PPid:")?.trim();
        value.parse().ok()
    }))
}

fn ancestry(pid: u32) -> Vec<ProcessInfo> {
    let mut chain = Vec::new();
    let mut current = Some(pid);

    for _ in 0..MAX_ANCESTORS {
        let Some(next) = current else {
            break;
        };

        let Ok(info) = inspect(next) else {
            break;
        };

        current = info.parent.filter(|parent| *parent > 0 && *parent != next);
        chain.push(info);

        if next == 1 {
            break;
        }
    }

    chain.reverse();
    chain
}

fn render_ancestry(chain: &[ProcessInfo]) -> String {
    chain
        .iter()
        .enumerate()
        .map(|(index, process)| {
            let prefix = if index == 0 {
                String::new()
            } else {
                format!("{}└─ ", "   ".repeat(index - 1))
            };
            format!("{prefix}{} ({})", display_name(process), process.pid)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn display_name(process: &ProcessInfo) -> String {
    if !matches!(process.name.as_str(), "MainThread" | "python" | "python3") {
        return process.name.clone();
    }

    if let Some(exe) = &process.executable
        && let Some(name) = exe.file_name().and_then(|name| name.to_str())
    {
        return name.to_owned();
    }

    process
        .command
        .split_whitespace()
        .next()
        .map(str::to_owned)
        .unwrap_or_else(|| process.name.clone())
}

fn project_root(cwd: &Path) -> Option<PathBuf> {
    cwd.ancestors()
        .find(|path| path.join(".git").exists())
        .map(Path::to_path_buf)
}

fn systemd_unit(pid: u32) -> Option<String> {
    let cgroup = fs::read_to_string(format!("/proc/{pid}/cgroup")).ok()?;

    cgroup
        .split(['/', ':', '\n'])
        .find(|part| part.ends_with(".service") || part.ends_with(".scope"))
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::{ProcessInfo, render_ancestry};
    use std::path::PathBuf;

    fn process(pid: u32, name: &str) -> ProcessInfo {
        ProcessInfo {
            pid,
            parent: None,
            name: name.to_owned(),
            command: String::new(),
            executable: None,
            cwd: Some(PathBuf::from("/tmp")),
        }
    }

    #[test]
    fn renders_process_tree() {
        let chain = vec![
            process(1, "systemd"),
            process(100, "kitty"),
            process(200, "zsh"),
            process(300, "node"),
        ];

        assert_eq!(
            render_ancestry(&chain),
            "systemd (1)\n└─ kitty (100)\n   └─ zsh (200)\n      └─ node (300)"
        );
    }
}
