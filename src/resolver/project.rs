use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::model::Finding;
use crate::system;

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
    } else {
        finding = finding.fact("Type", "directory");
    }

    let runtimes = detect_project_files(&root);
    if !runtimes.is_empty() {
        finding = finding.inferred_fact("Detected", runtimes.join(", "));
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
