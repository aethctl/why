use std::env;
use std::path::Path;

use anyhow::{Result, bail};

use crate::model::Finding;
use crate::system;

pub fn resolve(name: &str) -> Result<Finding> {
    if !exists(name) {
        bail!("'{name}' is not a recognized shell builtin");
    }

    let shell = current_shell_name();
    let description = description(name).unwrap_or("A command implemented directly by your shell.");

    let mut finding = Finding::new(name, "builtin", format!("{name} is built into your shell"))
        .fact("Description", description)
        .fact("Shell", shell)
        .fact("Source", "Shell builtin");

    if let Some(reason) = builtin_reason(name) {
        finding = finding.fact("Reason", reason);
    }

    Ok(finding)
}

pub fn exists(name: &str) -> bool {
    if builtin_description(name).is_some() {
        return true;
    }

    shell_reports_builtin(name)
}

fn shell_reports_builtin(name: &str) -> bool {
    let shell = env::var("SHELL").unwrap_or_default();
    let shell_name = Path::new(&shell)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    match shell_name {
        "zsh" if system::command_exists("zsh") => system::run(
            "zsh",
            &["-fc", &format!("whence -w -- {}", shell_quote(name))],
        )
        .is_ok_and(|out| out.success && out.stdout.contains(": builtin")),

        "bash" if system::command_exists("bash") => system::run(
            "bash",
            &["-lc", &format!("type -t -- {}", shell_quote(name))],
        )
        .is_ok_and(|out| out.success && out.stdout.trim() == "builtin"),

        _ => false,
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn current_shell_name() -> String {
    env::var("SHELL")
        .ok()
        .and_then(|shell| {
            Path::new(&shell)
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "current shell".to_owned())
}

fn description(name: &str) -> Option<&'static str> {
    builtin_description(name)
}

fn builtin_reason(name: &str) -> Option<&'static str> {
    match name {
        "cd" => Some(
            "It has to change the current shell's own working directory. A separate program could only change its own directory, then exit.",
        ),
        "export" | "unset" | "set" | "typeset" | "declare" | "local" => Some(
            "It changes variables or settings inside the current shell, so the shell must handle it directly.",
        ),
        "alias" | "unalias" => Some(
            "Aliases change how the current shell interprets later commands, so they live inside the shell.",
        ),
        "jobs" | "fg" | "bg" | "wait" => Some(
            "It controls jobs owned by the current shell, which a separate program cannot manage for it.",
        ),
        "source" | "." => Some(
            "It runs a file inside the current shell so changes to variables and directories remain after the file finishes.",
        ),
        "exit" | "return" => Some(
            "It changes the control flow of the current shell itself, so it must be implemented by the shell.",
        ),
        _ => None,
    }
}

fn builtin_description(name: &str) -> Option<&'static str> {
    match name {
        "cd" => Some("Changes the shell's current working directory."),
        "pwd" => Some("Prints the directory the shell is currently working in."),
        "export" => Some("Makes a shell variable available to programs started from this shell."),
        "unset" => Some("Removes a shell variable or function."),
        "alias" => Some("Creates or displays short names for longer shell commands."),
        "unalias" => Some("Removes a shell alias."),
        "source" | "." => Some("Runs commands from a file inside the current shell."),
        "jobs" => Some("Lists jobs started by the current shell."),
        "fg" => Some("Brings a background or stopped job into the foreground."),
        "bg" => Some("Continues a stopped job in the background."),
        "read" => Some("Reads input into one or more shell variables."),
        "eval" => Some("Treats text as a shell command and runs it."),
        "exec" => Some("Replaces the current shell process with another command."),
        "command" => {
            Some("Runs or identifies a command while bypassing shell functions and aliases.")
        }
        "builtin" => Some("Runs a command using the shell's builtin implementation."),
        "typeset" | "declare" => {
            Some("Creates shell variables and can assign special attributes to them.")
        }
        "local" => Some("Creates a variable that only exists inside the current shell function."),
        "set" => Some("Changes shell options or positional parameters."),
        "shift" => Some("Moves positional arguments down by one or more places."),
        "umask" => {
            Some("Controls the default permissions used when new files and folders are created.")
        }
        "ulimit" => Some("Shows or changes resource limits for programs started by the shell."),
        "history" => Some("Shows commands previously entered in the shell."),
        "hash" | "rehash" => Some("Refreshes or inspects the shell's cached command locations."),
        "wait" => Some("Waits for a background process or job to finish."),
        "trap" => Some("Runs shell commands when signals or shell events occur."),
        "return" => Some("Stops a shell function or sourced file and optionally returns a status."),
        "exit" => Some("Closes the current shell."),
        "true" => Some("Does nothing and returns a successful exit status."),
        "false" => Some("Does nothing and returns a failing exit status."),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::builtin_description;

    #[test]
    fn knows_common_builtins() {
        assert!(builtin_description("cd").is_some());
        assert!(builtin_description("export").is_some());
        assert!(builtin_description("jobs").is_some());
    }

    #[test]
    fn ignores_regular_commands() {
        assert!(builtin_description("git").is_none());
        assert!(builtin_description("chmod").is_none());
    }
}
