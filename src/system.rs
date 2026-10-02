use std::process::Command;

use anyhow::{Context, Result};

pub struct Output {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

pub fn run(program: &str, args: &[&str]) -> Result<Output> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("failed to run {program}"))?;

    Ok(Output {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    })
}

pub fn command_exists(program: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };

    std::env::split_paths(&path).any(|dir| dir.join(program).is_file())
}

pub fn command_description(name: &str) -> Option<String> {
    let friendly = match name {
        "chmod" => Some("Changes who can read, write, or run files and folders."),
        "chown" => Some("Changes which user or group owns a file or folder."),
        "cp" => Some("Copies files and folders."),
        "mv" => Some("Moves or renames files and folders."),
        "rm" => Some("Removes files and folders."),
        "mkdir" => Some("Creates a new folder."),
        "ls" => Some("Lists files and folders."),
        "cat" => Some("Prints the contents of a file."),
        "grep" => Some("Searches text for matching words or patterns."),
        "git" => Some("Tracks changes to files and manages Git repositories."),
        "ssh" => Some("Connects securely to another computer over a network."),
        "curl" => Some("Transfers data to or from URLs, commonly for web requests."),
        _ => None,
    };

    if let Some(description) = friendly {
        return Some(description.to_owned());
    }

    if !command_exists("whatis") {
        return None;
    }

    let out = run("whatis", &[name]).ok()?;
    if !out.success {
        return None;
    }

    let description = out
        .stdout
        .lines()
        .next()
        .and_then(|line| line.split_once(" - "))
        .map(|(_, description)| description.trim())?;

    let mut chars = description.chars();
    let first = chars.next()?;
    let mut sentence = first.to_uppercase().collect::<String>();
    sentence.push_str(chars.as_str());
    if !sentence.ends_with(['.', '!', '?']) {
        sentence.push('.');
    }

    Some(sentence)
}
