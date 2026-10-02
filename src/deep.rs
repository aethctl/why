use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use anyhow::Result;

use crate::model::Finding;
use crate::system;

pub fn enrich(finding: &mut Finding) -> Result<()> {
    match finding.kind.as_str() {
        "process" => enrich_process(finding),
        "file" => enrich_file(finding),
        "package" => enrich_package(finding),
        "service" => enrich_service(finding),
        _ => {}
    }

    Ok(())
}

fn enrich_process(finding: &mut Finding) {
    let Ok(pid) = finding.subject.parse::<u32>() else {
        return;
    };

    let fd_path = format!("/proc/{pid}/fd");
    if let Ok(entries) = fs::read_dir(fd_path) {
        finding.push_fact(
            "Open FDs",
            entries.filter_map(Result::ok).count().to_string(),
        );
    }

    let children = child_processes(pid);
    if !children.is_empty() {
        finding.push_fact("Children", children.join("\n"));
    }
}

fn child_processes(parent: u32) -> Vec<String> {
    let Ok(entries) = fs::read_dir("/proc") else {
        return Vec::new();
    };

    let mut children = Vec::new();

    for entry in entries.filter_map(Result::ok) {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|value| value.parse::<u32>().ok())
        else {
            continue;
        };

        let Ok(status) = fs::read_to_string(entry.path().join("status")) else {
            continue;
        };

        let ppid = status
            .lines()
            .find_map(|line| line.strip_prefix("PPid:")?.trim().parse::<u32>().ok());

        if ppid != Some(parent) {
            continue;
        }

        let name = fs::read_to_string(entry.path().join("comm"))
            .unwrap_or_else(|_| "unknown".to_owned())
            .trim()
            .to_owned();

        children.push(format!("{name} ({pid})"));
        if children.len() == 12 {
            break;
        }
    }

    children
}

fn enrich_file(finding: &mut Finding) {
    let path = Path::new(&finding.subject);
    let Ok(metadata) = fs::metadata(path) else {
        return;
    };

    finding.push_fact("Mode", format!("{:04o}", metadata.mode() & 0o7777));
    finding.push_fact(
        "Owner",
        format!("uid {} · gid {}", metadata.uid(), metadata.gid()),
    );
    finding.push_fact("Modified", metadata.mtime().to_string());
}

fn enrich_package(finding: &mut Finding) {
    let Some(store_path) = finding.value("Store path").map(str::to_owned) else {
        return;
    };

    if !system::command_exists("nix-store") {
        return;
    }

    let Ok(out) = system::run("nix-store", &["--query", "--references", &store_path]) else {
        return;
    };

    if !out.success {
        return;
    }

    let mut deps: Vec<String> = out
        .stdout
        .lines()
        .filter(|line| *line != store_path)
        .filter_map(|line| {
            let file = Path::new(line).file_name()?.to_str()?;
            Some(
                file.split_once('-')
                    .map(|(_, name)| name)
                    .unwrap_or(file)
                    .to_owned(),
            )
        })
        .collect();

    deps.sort();
    deps.dedup();

    if deps.is_empty() {
        return;
    }

    let total = deps.len();
    deps.truncate(12);
    let mut rendered = deps.join("\n");

    if total > deps.len() {
        rendered.push_str(&format!("\n…and {} more", total - deps.len()));
    }

    finding.push_fact("Dependencies", rendered);
    finding.push_fact("Dependency count", total.to_string());
}

fn enrich_service(finding: &mut Finding) {
    if !system::command_exists("systemctl") {
        return;
    }

    let service = if finding.subject.contains('.') {
        finding.subject.clone()
    } else {
        format!("{}.service", finding.subject)
    };

    let Ok(out) = system::run(
        "systemctl",
        &[
            "show",
            &service,
            "--no-pager",
            "--property=MainPID,TasksCurrent,MemoryCurrent,ActiveEnterTimestamp",
        ],
    ) else {
        return;
    };

    if !out.success {
        return;
    }

    for line in out.stdout.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if value.is_empty() || value == "[not set]" || value == "0" {
            continue;
        }

        let label = match key {
            "MainPID" => "Main PID",
            "TasksCurrent" => "Tasks",
            "MemoryCurrent" => "Memory bytes",
            "ActiveEnterTimestamp" => "Active since",
            _ => continue,
        };

        finding.push_fact(label, value);
    }
}
