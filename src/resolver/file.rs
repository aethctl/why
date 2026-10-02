use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};

use crate::model::Finding;
use crate::ownership;

pub fn resolve(input: &str) -> Result<Finding> {
    let path = PathBuf::from(input);
    if !path.exists() && fs::symlink_metadata(&path).is_err() {
        bail!("file '{}' does not exist", path.display());
    }

    let meta = fs::symlink_metadata(&path)
        .with_context(|| format!("failed to inspect {}", path.display()))?;

    let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
    let kind = if meta.file_type().is_symlink() {
        "symlink"
    } else if meta.is_dir() {
        "directory"
    } else if meta.is_file() {
        "file"
    } else {
        "special"
    };

    let mut finding = Finding::new(
        path.display().to_string(),
        "file",
        format!("{} is a {kind}", path.display()),
    )
    .fact("Canonical", canonical.display().to_string())
    .fact("Type", kind)
    .fact("Size", meta.len().to_string());

    if meta.file_type().is_symlink()
        && let Ok(target) = fs::read_link(&path)
    {
        finding = finding.fact("Symlink target", target.display().to_string());
    }

    if let Some(package) = ownership::nix_store_package(&canonical) {
        finding = finding.fact("Provided by", package);
    } else if let Some(package) = ownership::pacman_owner(&canonical)? {
        finding = finding.fact("Provided by", package);
    } else {
        finding = finding.note("No supported package manager claimed ownership of this path.");
    }

    Ok(finding)
}
