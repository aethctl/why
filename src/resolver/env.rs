use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::model::Finding;

pub fn resolve(name: &str) -> Result<Finding> {
    let key = name.trim_start_matches('$');
    let Some(value) = env::var_os(key) else {
        bail!("environment variable '{key}' is not set");
    };

    let sensitive = is_sensitive(key);
    let shown = if sensitive {
        "[redacted]".to_owned()
    } else {
        value.to_string_lossy().into_owned()
    };

    let mut finding = Finding::new(
        key,
        "environment",
        format!("{key} is set in the current process environment"),
    )
    .fact("Value", shown);

    let sources = find_sources(key);
    if !sources.is_empty() {
        finding = finding.inferred_fact("Possible source", sources.join("\n"));
    }

    if sensitive {
        finding = finding
            .note("The value was redacted because the variable name looks credential-sensitive.");
    }

    Ok(finding)
}

fn is_sensitive(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    [
        "TOKEN",
        "SECRET",
        "PASSWORD",
        "PASSWD",
        "API_KEY",
        "PRIVATE_KEY",
        "CREDENTIAL",
        "COOKIE",
        "AUTH",
    ]
    .iter()
    .any(|needle| upper.contains(needle))
}

fn find_sources(name: &str) -> Vec<String> {
    let mut files = vec![PathBuf::from("/etc/environment")];

    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        for relative in [
            ".profile",
            ".zshenv",
            ".zprofile",
            ".zshrc",
            ".bash_profile",
            ".bashrc",
            ".config/zsh/.zshenv",
            ".config/zsh/.zprofile",
            ".config/zsh/.zshrc",
        ] {
            files.push(home.join(relative));
        }

        append_conf_files(&home.join(".config/environment.d"), &mut files);
    }

    append_conf_files(Path::new("/etc/environment.d"), &mut files);

    let mut matches = Vec::new();

    for file in files.into_iter().filter(|path| path.is_file()) {
        let Ok(content) = fs::read_to_string(&file) else {
            continue;
        };

        for (index, line) in content.lines().enumerate() {
            if assigns_variable(line, name) {
                matches.push(format!("{}:{}", file.display(), index + 1));
                if matches.len() == 8 {
                    return matches;
                }
            }
        }
    }

    matches
}

fn append_conf_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    let mut confs: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "conf"))
        .collect();

    confs.sort();
    files.extend(confs);
}

fn assigns_variable(line: &str, name: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.starts_with('#') {
        return false;
    }

    let trimmed = trimmed.strip_prefix("export ").unwrap_or(trimmed);
    trimmed
        .split_once('=')
        .is_some_and(|(key, _)| key.trim() == name)
}

#[cfg(test)]
mod tests {
    use super::{assigns_variable, is_sensitive};

    #[test]
    fn detects_assignments() {
        assert!(assigns_variable("XCURSOR_THEME=Bibata", "XCURSOR_THEME"));
        assert!(assigns_variable("export EDITOR=nvim", "EDITOR"));
        assert!(!assigns_variable("# EDITOR=vim", "EDITOR"));
    }

    #[test]
    fn redacts_sensitive_names() {
        assert!(is_sensitive("GITHUB_TOKEN"));
        assert!(is_sensitive("api_key"));
        assert!(!is_sensitive("XDG_CURRENT_DESKTOP"));
    }
}
