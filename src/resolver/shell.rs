use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::{Result, bail};

use crate::model::Finding;

#[derive(Debug)]
struct Definition {
    kind: &'static str,
    source: PathBuf,
    line: usize,
    text: String,
    underlying: Option<String>,
}

pub fn resolve(name: &str) -> Result<Finding> {
    let definitions = find_definitions(name);
    let Some(primary) = definitions.first() else {
        bail!("no shell alias or function named '{name}' was found in known shell config files");
    };

    let mut finding = Finding::new(
        name,
        "shell",
        format!("{name} is defined as a shell {}", primary.kind),
    )
    .fact("Kind", primary.kind)
    .fact(
        "Defined at",
        format!("{}:{}", primary.source.display(), primary.line),
    )
    .fact("Definition", primary.text.clone());

    if let Some(underlying) = &primary.underlying {
        finding = finding.inferred_fact("Underlying", underlying);
    }

    if definitions.len() > 1 {
        finding = finding.fact(
            "Also defined at",
            definitions[1..]
                .iter()
                .map(|item| format!("{}:{}", item.source.display(), item.line))
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }

    Ok(finding)
}

pub fn exists(name: &str) -> bool {
    !find_definitions(name).is_empty()
}

fn find_definitions(name: &str) -> Vec<Definition> {
    let mut matches = Vec::new();

    for file in shell_files().into_iter().filter(|path| path.is_file()) {
        let Ok(content) = fs::read_to_string(&file) else {
            continue;
        };

        for (index, line) in content.lines().enumerate() {
            if let Some((kind, underlying)) = classify_definition(line, name) {
                matches.push(Definition {
                    kind,
                    source: file.clone(),
                    line: index + 1,
                    text: line.trim().to_owned(),
                    underlying,
                });
            }
        }
    }

    matches
}

fn shell_files() -> Vec<PathBuf> {
    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        return Vec::new();
    };

    [
        ".zshrc",
        ".zshenv",
        ".zprofile",
        ".bashrc",
        ".bash_profile",
        ".profile",
        ".config/zsh/.zshrc",
        ".config/zsh/.zshenv",
        ".config/zsh/.zprofile",
        ".config/fish/config.fish",
    ]
    .into_iter()
    .map(|relative| home.join(relative))
    .collect()
}

fn classify_definition(line: &str, name: &str) -> Option<(&'static str, Option<String>)> {
    let trimmed = line.trim();

    if let Some(rest) = trimmed.strip_prefix("alias ")
        && let Some((alias, value)) = rest.split_once('=')
        && alias.trim() == name
    {
        let value = value.trim().trim_matches(['\'', '"']);
        let underlying = value.split_whitespace().next().map(str::to_owned);
        return Some(("alias", underlying));
    }

    if trimmed.starts_with(&format!("function {name} "))
        || trimmed == format!("function {name}")
        || trimmed.starts_with(&format!("{name}()"))
        || trimmed.starts_with(&format!("{name} ()"))
    {
        return Some(("function", None));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::classify_definition;

    #[test]
    fn recognizes_aliases() {
        let found = classify_definition("alias ll='ls -lah'", "ll");
        assert!(matches!(found, Some(("alias", Some(command))) if command == "ls"));
    }

    #[test]
    fn recognizes_functions() {
        assert!(matches!(
            classify_definition("mkcd() {", "mkcd"),
            Some(("function", None))
        ));
        assert!(matches!(
            classify_definition("function mkcd {", "mkcd"),
            Some(("function", None))
        ));
    }

    #[test]
    fn ignores_other_names() {
        assert!(classify_definition("alias git='git'", "ll").is_none());
    }
}
