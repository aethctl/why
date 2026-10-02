use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::model::Finding;
use crate::ownership;
use crate::system;

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

    if let Some(repo) = git_root(&canonical) {
        finding = finding.fact("Git repo", repo.display().to_string());

        if let Some(state) = git_state(&repo, &canonical) {
            finding = finding.fact("Git state", state);
        }
    }

    Ok(finding)
}

fn git_root(path: &Path) -> Option<PathBuf> {
    let start = if path.is_dir() { path } else { path.parent()? };

    for ancestor in start.ancestors() {
        if ancestor.join(".git").exists() {
            return Some(ancestor.to_path_buf());
        }
    }

    None
}

fn git_state(repo: &Path, path: &Path) -> Option<String> {
    if !system::command_exists("git") {
        return None;
    }

    let relative = path.strip_prefix(repo).ok()?;
    let repo = repo.to_string_lossy();
    let relative = relative.to_string_lossy();

    let tracked = system::run(
        "git",
        &["-C", &repo, "ls-files", "--error-unmatch", &relative],
    )
    .ok()?;
    if !tracked.success {
        return Some("untracked".to_owned());
    }

    let status = system::run("git", &["-C", &repo, "status", "--short", "--", &relative]).ok()?;
    if !status.success || status.stdout.is_empty() {
        return Some("tracked, clean".to_owned());
    }

    Some(format!("tracked, modified ({})", status.stdout))
}

#[cfg(test)]
mod tests {
    use super::git_root;
    use std::fs;

    #[test]
    fn finds_git_root() {
        let root = std::env::temp_dir().join(format!("why-git-root-test-{}", std::process::id()));
        let nested = root.join("a/b");

        fs::create_dir_all(root.join(".git")).unwrap();
        fs::create_dir_all(&nested).unwrap();

        assert_eq!(git_root(&nested).as_deref(), Some(root.as_path()));

        fs::remove_dir_all(root).unwrap();
    }
}
