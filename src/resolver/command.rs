use anyhow::{Result, bail};

use crate::model::Finding;
use crate::ownership;
use crate::system;

pub fn resolve(name: &str) -> Result<Finding> {
    let Some(path) = ownership::command_path(name) else {
        bail!("command '{name}' was not found in PATH");
    };

    let canonical = path.canonicalize().unwrap_or(path.clone());
    let mut finding = Finding::new(
        name,
        "command",
        format!("'{name}' resolves to {}", canonical.display()),
    )
    .fact("Path", canonical.display().to_string());

    if let Some(description) = system::command_description(name) {
        finding = finding.fact("Description", description);
    }

    if let Some(package) = ownership::nix_store_package(&canonical) {
        finding = finding
            .fact("Provided by", package)
            .note("This command is backed by a Nix store path.");
    } else if let Some(package) = ownership::pacman_owner(&canonical)? {
        finding = finding.fact("Provided by", package);
    } else {
        finding = finding.note("No supported package manager claimed ownership of this command.");
    }

    Ok(finding)
}
