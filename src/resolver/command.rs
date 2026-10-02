use std::path::PathBuf;

use anyhow::{Result, bail};

use crate::model::Finding;
use crate::system;

pub fn resolve(name: &str) -> Result<Finding> {
    let Some(path) = find_in_path(name) else {
        bail!("command '{name}' was not found in PATH");
    };

    let canonical = path.canonicalize().unwrap_or(path.clone());
    let mut finding = Finding::new(
        name,
        "command",
        format!("'{name}' resolves to {}", canonical.display()),
    )
    .fact("Path", canonical.display().to_string());

    if let Some(package) = nix_store_owner(&canonical)? {
        finding = finding
            .fact("Provided by", package)
            .note("This command is backed by a Nix store path.");
    } else if let Some(package) = pacman_owner(&canonical)? {
        finding = finding.fact("Provided by", package);
    } else {
        finding = finding.note("No supported package manager claimed ownership of this command.");
    }

    Ok(finding)
}

fn find_in_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

fn nix_store_owner(path: &std::path::Path) -> Result<Option<String>> {
    let text = path.to_string_lossy();
    let Some(rest) = text.strip_prefix("/nix/store/") else {
        return Ok(None);
    };

    let Some((store_item, _)) = rest.split_once('/') else {
        return Ok(None);
    };

    let package = store_item
        .split_once('-')
        .map(|(_, name)| name)
        .unwrap_or(store_item);

    Ok(Some(package.to_owned()))
}

fn pacman_owner(path: &std::path::Path) -> Result<Option<String>> {
    if !system::command_exists("pacman") {
        return Ok(None);
    }

    let out = system::run("pacman", &["-Qo", &path.to_string_lossy()])?;
    if !out.success {
        return Ok(None);
    }

    let package = out
        .stdout
        .split_whitespace()
        .rev()
        .nth(1)
        .map(str::to_owned);

    Ok(package)
}
