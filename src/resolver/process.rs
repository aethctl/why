use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};

use crate::model::Finding;
use crate::ownership;

pub fn resolve(pid: u32) -> Result<Finding> {
    let root = PathBuf::from(format!("/proc/{pid}"));
    if !root.exists() {
        bail!("process {pid} does not exist");
    }

    let comm = fs::read_to_string(root.join("comm"))
        .with_context(|| format!("failed to read process {pid} name"))?
        .trim()
        .to_owned();

    let exe = fs::read_link(root.join("exe")).ok();
    let cwd = fs::read_link(root.join("cwd")).ok();
    let parent = parent_pid(pid).ok().flatten();
    let cmdline = fs::read(root.join("cmdline"))
        .ok()
        .map(|bytes| {
            bytes
                .split(|byte| *byte == 0)
                .filter(|part| !part.is_empty())
                .map(|part| String::from_utf8_lossy(part))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();

    let mut finding = Finding::new(
        pid.to_string(),
        "process",
        format!("{comm} is running as PID {pid}"),
    )
    .fact("Name", comm);

    if !cmdline.is_empty() {
        finding = finding.fact("Command", cmdline);
    }

    if let Some(exe) = exe {
        finding = finding.fact("Executable", exe.display().to_string());

        if let Some(package) = ownership::nix_store_package(&exe) {
            finding = finding.fact("Provided by", package);
        } else if let Some(package) = ownership::pacman_owner(&exe)? {
            finding = finding.fact("Provided by", package);
        }
    }

    if let Some(cwd) = cwd {
        finding = finding.fact("Working dir", cwd.display().to_string());
    }

    if let Some(ppid) = parent {
        finding = finding.fact("Parent PID", ppid.to_string());
    }

    Ok(finding)
}

fn parent_pid(pid: u32) -> Result<Option<u32>> {
    let status = fs::read_to_string(format!("/proc/{pid}/status"))?;
    Ok(status.lines().find_map(|line| {
        let value = line.strip_prefix("PPid:")?.trim();
        value.parse().ok()
    }))
}
