use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::model::Finding;
use crate::system;

#[derive(Debug)]
struct ProjectProcess {
    pid: u32,
    name: String,
}

pub fn resolve_current() -> Result<Finding> {
    let cwd = env::current_dir()?;
    let root = git_root(&cwd).unwrap_or_else(|| cwd.clone());

    if !root.exists() {
        bail!("current directory does not exist");
    }

    let name = root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("current directory");

    let mut finding = Finding::new(
        root.display().to_string(),
        "context",
        format!("{name} is the current working context"),
    )
    .fact("Directory", root.display().to_string());

    if root.join(".git").exists() {
        finding = finding.fact("Type", "Git repository");

        if let Some(branch) = git_output(&root, &["branch", "--show-current"]) {
            finding = finding.fact("Branch", branch);
        }

        if let Some(remote) = git_output(&root, &["remote", "get-url", "origin"]) {
            finding = finding.fact("Remote", remote);
        }

        if let Some(state) = git_state(&root) {
            finding = finding.fact("Git state", state);
        }
    } else {
        finding = finding.fact("Type", "directory");
    }

    let runtimes = detect_project_files(&root);
    if !runtimes.is_empty() {
        finding = finding.inferred_fact("Detected", runtimes.join(", "));
    }

    let processes = project_processes(&root);
    if !processes.is_empty() {
        finding = finding.fact(
            "Processes",
            processes
                .iter()
                .take(8)
                .map(|process| format!("{} ({})", process.name, process.pid))
                .collect::<Vec<_>>()
                .join("\n"),
        );

        let ports = project_ports(&processes);
        if !ports.is_empty() {
            finding = finding.fact("Listening", ports.join("\n"));
        }
    }

    Ok(finding)
}

fn git_root(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find(|ancestor| ancestor.join(".git").exists())
        .map(Path::to_path_buf)
}

fn git_output(repo: &Path, args: &[&str]) -> Option<String> {
    if !system::command_exists("git") {
        return None;
    }

    let repo = repo.to_string_lossy();
    let mut command = vec!["-C", repo.as_ref()];
    command.extend_from_slice(args);
    let out = system::run("git", &command).ok()?;

    if out.success && !out.stdout.is_empty() {
        Some(out.stdout)
    } else {
        None
    }
}

fn git_state(repo: &Path) -> Option<String> {
    let status = git_output(repo, &["status", "--short"]);
    match status {
        Some(output) if !output.is_empty() => {
            let count = output.lines().count();
            Some(format!("{count} changed path(s)"))
        }
        _ => Some("clean".to_owned()),
    }
}

fn detect_project_files(root: &Path) -> Vec<String> {
    let mut found = Vec::new();

    for (file, label) in [
        ("flake.nix", "Nix flake"),
        ("Cargo.toml", "Rust"),
        ("package.json", "Node.js"),
        ("pyproject.toml", "Python"),
        ("go.mod", "Go"),
    ] {
        if root.join(file).exists() {
            found.push(label.to_owned());
        }
    }

    found
}

fn project_processes(root: &Path) -> Vec<ProjectProcess> {
    let Ok(entries) = fs::read_dir("/proc") else {
        return Vec::new();
    };

    let own_pid = std::process::id();
    let mut processes = Vec::new();

    for entry in entries.filter_map(Result::ok) {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|value| value.parse::<u32>().ok())
        else {
            continue;
        };

        if pid == own_pid {
            continue;
        }

        let proc = entry.path();
        let Ok(cwd) = fs::read_link(proc.join("cwd")) else {
            continue;
        };

        if !cwd.starts_with(root) {
            continue;
        }

        let Ok(name) = fs::read_to_string(proc.join("comm")) else {
            continue;
        };
        let name = name.trim().to_owned();

        if is_shell_noise(&name) {
            continue;
        }

        processes.push(ProjectProcess { pid, name });
    }

    processes.sort_by_key(|process| process.pid);
    processes
}

fn is_shell_noise(name: &str) -> bool {
    matches!(
        name,
        "sh" | "bash" | "zsh" | "fish" | "why" | "cargo" | "rustc" | "nix" | "nix-shell"
    )
}

fn project_ports(processes: &[ProjectProcess]) -> Vec<String> {
    if !system::command_exists("ss") {
        return Vec::new();
    }

    let Ok(out) = system::run("ss", &["-ltnup"]) else {
        return Vec::new();
    };
    if !out.success {
        return Vec::new();
    }

    let mut ports = Vec::new();

    for line in out.stdout.lines().skip(1) {
        for process in processes {
            if !line.contains(&format!("pid={},", process.pid)) {
                continue;
            }

            let parts: Vec<_> = line.split_whitespace().collect();
            let Some(protocol) = parts.first() else {
                continue;
            };
            let Some(local) = parts.get(4) else {
                continue;
            };

            ports.push(format!("{protocol} {local} ({})", process.name));
        }
    }

    ports.sort();
    ports.dedup();
    ports
}

#[cfg(test)]
mod tests {
    use super::is_shell_noise;

    #[test]
    fn filters_project_shell_noise() {
        assert!(is_shell_noise("zsh"));
        assert!(is_shell_noise("cargo"));
        assert!(!is_shell_noise("vite"));
        assert!(!is_shell_noise("python3"));
    }
}
