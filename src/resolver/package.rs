use std::path::PathBuf;

use anyhow::{Result, bail};
use walkdir::WalkDir;

use crate::model::Finding;
use crate::system;

pub fn resolve(name: &str) -> Result<Finding> {
    if system::command_exists("pacman")
        && let Some(finding) = resolve_pacman(name)?
    {
        return Ok(finding);
    }

    if let Some(finding) = resolve_nix(name)? {
        return Ok(finding);
    }

    bail!("package '{name}' was not found by a supported package manager")
}

fn resolve_pacman(name: &str) -> Result<Option<Finding>> {
    let info = system::run("pacman", &["-Qi", name])?;
    if !info.success {
        return Ok(None);
    }

    let mut version = "unknown".to_owned();
    let mut reason = "unknown".to_owned();
    let mut required_by = "none".to_owned();

    for line in info.stdout.lines() {
        if let Some((key, value)) = line.split_once(':') {
            match key.trim() {
                "Version" => version = value.trim().to_owned(),
                "Install Reason" => reason = value.trim().to_owned(),
                "Required By" => required_by = value.trim().to_owned(),
                _ => {}
            }
        }
    }

    Ok(Some(
        Finding::new(
            name,
            "package",
            format!("{name} is installed through pacman"),
        )
        .fact("Version", version)
        .fact("Install reason", reason)
        .fact("Required by", required_by),
    ))
}

fn resolve_nix(name: &str) -> Result<Option<Finding>> {
    let Some(path) = find_in_path(name) else {
        return resolve_nix_profile(name);
    };

    let canonical = path.canonicalize().unwrap_or(path);
    let text = canonical.to_string_lossy();
    let Some(rest) = text.strip_prefix("/nix/store/") else {
        return resolve_nix_profile(name);
    };

    let store_item = rest.split('/').next().unwrap_or_default();
    let package = store_item
        .split_once('-')
        .map(|(_, value)| value)
        .unwrap_or(store_item);

    let mut finding = Finding::new(
        name,
        "package",
        format!("{name} is present through the Nix store"),
    )
    .fact("Store package", package)
    .fact("Executable", canonical.display().to_string());

    let declarations = find_nix_declarations(name);
    if !declarations.is_empty() {
        finding = finding.fact("Declared in", declarations.join(", "));
    } else {
        finding = finding.note(
            "The package is reachable from PATH, but its declarative source was not found under /etc/nixos.",
        );
    }

    Ok(Some(finding))
}

fn resolve_nix_profile(name: &str) -> Result<Option<Finding>> {
    if !system::command_exists("nix") {
        return Ok(None);
    }

    let profile = system::run("nix", &["profile", "list", "--json"])?;
    if !profile.success || !profile.stdout.contains(name) {
        return Ok(None);
    }

    Ok(Some(
        Finding::new(
            name,
            "package",
            format!("{name} appears in the active Nix profile"),
        )
        .fact("Source", "nix profile"),
    ))
}

fn find_in_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

fn find_nix_declarations(name: &str) -> Vec<String> {
    let root = std::path::Path::new("/etc/nixos");
    if !root.exists() {
        return Vec::new();
    }

    let needle = name.to_lowercase();
    let mut matches = Vec::new();

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        let is_nix = path.extension().is_some_and(|ext| ext == "nix");
        if !is_nix {
            continue;
        }

        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };

        if content.to_lowercase().contains(&needle) {
            matches.push(path.display().to_string());
            if matches.len() == 5 {
                break;
            }
        }
    }

    matches
}
