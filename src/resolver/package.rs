use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use walkdir::WalkDir;

use crate::model::Finding;
use crate::ownership;
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
    let mut required_by = "None".to_owned();

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

    let impact = if required_by.eq_ignore_ascii_case("none") {
        "No installed package reports a direct dependency.".to_owned()
    } else {
        format!("Direct dependents: {required_by}")
    };

    let mut finding = Finding::new(
        name,
        "package",
        format!("{name} is installed through pacman"),
    )
    .fact("Version", version)
    .fact("Install reason", reason)
    .fact("Required by", required_by)
    .fact("Removal impact", impact);

    if let Some(history) = pacman_history(name) {
        finding = finding.fact("History", history);
    }

    Ok(Some(finding))
}

fn resolve_nix(name: &str) -> Result<Option<Finding>> {
    let Some(path) = ownership::command_path(name) else {
        return resolve_nix_profile(name);
    };

    let from_system_profile = path.starts_with("/run/current-system/sw/");
    let canonical = path.canonicalize().unwrap_or(path);
    let Some(store_root) = ownership::nix_store_root(&canonical) else {
        return resolve_nix_profile(name);
    };
    let package = ownership::nix_store_package(&canonical).unwrap_or_else(|| name.to_owned());
    let package_name = nix_package_name(&package);

    let mut finding = Finding::new(
        name,
        "package",
        format!("{name} is present through the Nix store"),
    )
    .fact("Store package", package)
    .fact("Store path", store_root.display().to_string())
    .fact("Executable", canonical.display().to_string());

    let mut declarations = find_nix_declarations(name);
    if package_name != name {
        for declaration in find_nix_declarations(&package_name) {
            if !declarations.contains(&declaration) {
                declarations.push(declaration);
            }
        }
    }

    if declarations.is_empty() {
        if from_system_profile {
            finding = finding.fact("Source", "NixOS system profile").fact(
                "Reason",
                "Included transitively by the active system closure",
            );
        } else {
            finding = finding
                .note("No explicit declaration was found in the known Nix configuration roots.");
        }
    } else {
        finding = finding
            .fact("Source", "Declarative Nix configuration")
            .fact("Declared at", declarations.join("\n"));

        if let Some(history) = nix_declaration_history(&package_name, &declarations) {
            finding = finding.fact("History", history);
        }
    }

    if let Some(referrers) = nix_referrers(&store_root) {
        finding = finding.fact("Referenced by", referrers);
    }

    Ok(Some(finding))
}

fn nix_package_name(store_package: &str) -> String {
    let mut parts = store_package.rsplitn(2, '-');
    let last = parts.next().unwrap_or(store_package);
    let rest = parts.next();

    if last.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        rest.unwrap_or(store_package).to_owned()
    } else {
        store_package.to_owned()
    }
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
        .fact("Source", "nix profile")
        .fact(
            "Removal impact",
            "Removing the profile entry will remove it from that profile on the next profile update.",
        ),
    ))
}

fn pacman_history(name: &str) -> Option<String> {
    let content = std::fs::read_to_string("/var/log/pacman.log").ok()?;
    let installed = format!(" installed {name} (");
    let upgraded = format!(" upgraded {name} (");
    let reinstalled = format!(" reinstalled {name} (");

    let mut entries: Vec<String> = content
        .lines()
        .filter(|line| {
            line.contains(&installed) || line.contains(&upgraded) || line.contains(&reinstalled)
        })
        .filter_map(|line| {
            let (timestamp, event) = line.split_once("] [ALPM] ")?;
            Some(format!("{} {}", timestamp.trim_start_matches('['), event))
        })
        .collect();

    entries.reverse();
    entries.truncate(3);

    if entries.is_empty() {
        None
    } else {
        Some(entries.join("\n"))
    }
}

fn nix_declaration_history(name: &str, declarations: &[String]) -> Option<String> {
    if !system::command_exists("git") {
        return None;
    }

    for declaration in declarations {
        let location = declaration.lines().next()?;
        let (path_text, _) = location.rsplit_once(':')?;
        let path = Path::new(path_text);
        let root = path
            .ancestors()
            .find(|ancestor| ancestor.join(".git").exists())?;
        let relative = path.strip_prefix(root).ok()?;

        let root = root.to_string_lossy();
        let relative = relative.to_string_lossy();
        let out = system::run(
            "git",
            &[
                "-C",
                &root,
                "log",
                "-1",
                "--format=%cs %h %s",
                "-S",
                name,
                "--",
                &relative,
            ],
        )
        .ok()?;

        if out.success && !out.stdout.is_empty() {
            return Some(out.stdout);
        }
    }

    None
}

fn nix_referrers(store_root: &Path) -> Option<String> {
    if !system::command_exists("nix-store") {
        return None;
    }

    let root = store_root.to_string_lossy();
    let out = system::run("nix-store", &["--query", "--referrers", &root]).ok()?;
    if !out.success {
        return None;
    }

    let mut refs: Vec<String> = out
        .stdout
        .lines()
        .filter(|line| *line != root)
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

    refs.sort();
    refs.dedup();

    if refs.is_empty() {
        return Some("No live store referrers found.".to_owned());
    }

    let extra = refs.len().saturating_sub(8);
    refs.truncate(8);
    let mut rendered = refs.join("\n");

    if extra > 0 {
        rendered.push_str(&format!("\n…and {extra} more"));
    }

    Some(rendered)
}

fn find_nix_declarations(name: &str) -> Vec<String> {
    let mut roots = vec![PathBuf::from("/etc/nixos")];

    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        roots.push(home.join(".config/home-manager"));
        roots.push(home.join(".config/nixpkgs"));
    }

    let mut matches = Vec::new();

    for root in roots.into_iter().filter(|root| root.exists()) {
        scan_nix_root(&root, name, &mut matches);
        if matches.len() >= 8 {
            break;
        }
    }

    matches.truncate(8);
    matches
}

fn scan_nix_root(root: &Path, name: &str, matches: &mut Vec<String>) {
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        if !path.extension().is_some_and(|ext| ext == "nix") {
            continue;
        }

        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };

        for (index, line) in content.lines().enumerate() {
            if line_mentions(line, name) {
                matches.push(format!(
                    "{}:{}\n  {}",
                    path.display(),
                    index + 1,
                    line.trim()
                ));

                if matches.len() >= 8 {
                    return;
                }
            }
        }
    }
}

fn line_mentions(line: &str, name: &str) -> bool {
    line.split(|c: char| !(c.is_alphanumeric() || matches!(c, '_' | '-' | '.')))
        .filter(|token| !token.is_empty())
        .any(|token| token == name || token.rsplit('.').next() == Some(name))
}

#[cfg(test)]
mod tests {
    use super::{line_mentions, nix_package_name};

    #[test]
    fn finds_nix_package_tokens() {
        assert!(line_mentions("  git", "git"));
        assert!(line_mentions("  pkgs.git", "git"));
        assert!(!line_mentions("  github-cli", "git"));
    }

    #[test]
    fn strips_store_package_version() {
        assert_eq!(nix_package_name("coreutils-9.11"), "coreutils");
        assert_eq!(nix_package_name("git-2.55.0"), "git");
        assert_eq!(nix_package_name("hello"), "hello");
    }
}
