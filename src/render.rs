use anyhow::Result;
use owo_colors::OwoColorize;

use crate::model::{Evidence, Finding};

const CARD_WIDTH: usize = 76;
const INNER_WIDTH: usize = CARD_WIDTH - 4;

pub fn print(finding: &Finding, json: bool, plain: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(finding)?);
        return Ok(());
    }

    if plain {
        print_plain(finding);
    } else {
        print_card(finding);
    }

    Ok(())
}

fn print_plain(finding: &Finding) {
    println!("{}", finding.subject);
    println!("{}", finding.summary);
    println!();

    for fact in &finding.facts {
        print_plain_fact(&fact.label, &fact.value, fact.evidence);
    }

    if !finding.notes.is_empty() {
        println!();
        for note in &finding.notes {
            println!("note {note}");
        }
    }
}

fn print_card(finding: &Finding) {
    top_border("WHY");
    card_line(&finding.subject);
    card_line(&format!("Type: {}", friendly_kind(&finding.kind)));

    section_border("WHAT IS THIS?");
    for line in what_is_it(finding) {
        card_line(&line);
    }

    section_border("WHY IS IT HERE?");
    for line in why_is_it_here(finding) {
        card_line(&line);
    }

    section_border("DETAILS");
    for fact in &finding.facts {
        if matches!(
            fact.label.as_str(),
            "Description" | "Reason" | "Store path" | "Executable"
        ) {
            continue;
        }
        card_fact(display_label(&fact.label), &fact.value, fact.evidence);
    }

    if !finding.notes.is_empty() {
        section_border("NOTES");
        for note in &finding.notes {
            card_line(&format!("• {note}"));
        }
    }

    bottom_border();
    println!(
        "{}",
        "Tip: use --deep for more detail, --plain for scripts.".dimmed()
    );
}

fn friendly_kind(kind: &str) -> &'static str {
    match kind {
        "package" => "software package / command",
        "command" => "terminal command",
        "service" => "background service",
        "process" => "running program",
        "port" => "network port",
        "file" => "file or folder",
        "environment" => "environment variable",
        "shell" => "shell shortcut",
        "context" => "current project",
        _ => "system item",
    }
}

fn what_is_it(finding: &Finding) -> Vec<String> {
    if let Some(description) = finding.value("Description") {
        let package = finding
            .value("Store package")
            .or_else(|| finding.value("Provided by"));

        let text = match package {
            Some(package) => {
                format!("{description} It is provided by the {package} package.")
            }
            None => description.to_owned(),
        };

        return wrap_text(&text, INNER_WIDTH);
    }

    let text = match finding.kind.as_str() {
        "package" => format!("{} is software available on this system.", finding.subject),
        "command" => format!(
            "{} is a command you can run from the terminal.",
            finding.subject
        ),
        "service" => format!(
            "{} is a background service managed by systemd.",
            finding.subject
        ),
        "process" => format!(
            "{} identifies a program that is currently running.",
            finding.subject
        ),
        "port" => format!(
            "{} is a network port currently being used by a program.",
            finding.subject
        ),
        "file" => format!("{} is a path on your filesystem.", finding.subject),
        "environment" => format!(
            "{} is an environment variable that programs can read.",
            finding.subject
        ),
        "shell" => format!(
            "{} is a shortcut or function defined by your shell configuration.",
            finding.subject
        ),
        "context" => "This is the project or directory you are currently working in.".to_owned(),
        _ => finding.summary.clone(),
    };

    wrap_text(&text, INNER_WIDTH)
}

fn why_is_it_here(finding: &Finding) -> Vec<String> {
    let text = match finding.value("Source") {
        Some("NixOS system profile") => {
            let package = finding.value("Store package").unwrap_or(&finding.subject);
            format!(
                "Your current NixOS system includes {package} because another part of the active system depends on it. You did not necessarily install it yourself."
            )
        }
        Some("Declarative Nix configuration") => {
            "It is present because your Nix configuration declares it directly. Rebuilding the system keeps it available.".to_owned()
        }
        Some("nix profile") => {
            "It was installed into your personal Nix profile, so it is available to your user account.".to_owned()
        }
        _ => match finding.kind.as_str() {
            "process" => finding
                .value("Why running")
                .map(|_| "It is running because another process started it. The process chain is shown below.".to_owned())
                .unwrap_or_else(|| "It is currently running in this session or system environment.".to_owned()),
            "port" => "A running program opened this port so it can accept network connections.".to_owned(),
            "service" => "systemd knows about this service and controls when it starts or stops.".to_owned(),
            "file" => finding
                .value("Provided by")
                .map(|owner| format!("This path exists because the {owner} package provides it."))
                .unwrap_or_else(|| "This path exists in your filesystem, but no supported package manager claimed ownership of it.".to_owned()),
            "environment" => finding
                .value("Possible source")
                .map(|source| format!("Your shell or environment configuration appears to set it from {source}."))
                .unwrap_or_else(|| "It is set in the environment inherited by the current process.".to_owned()),
            "shell" => finding
                .value("Defined at")
                .map(|source| format!("Your shell configuration defines it at {source}."))
                .unwrap_or_else(|| "Your shell configuration defines this shortcut.".to_owned()),
            "context" => "You asked about this, so why inspected the current working directory and the activity connected to it.".to_owned(),
            _ => finding.summary.clone(),
        },
    };

    wrap_text(&text, INNER_WIDTH)
}

fn display_label(label: &str) -> &str {
    match label {
        "Store package" => "Package",
        "Source" => "Comes from",
        "Referenced by" => "Used by",
        "Declared at" => "Declared in",
        "Working dir" => "Working in",
        "Systemd unit" => "Service",
        "Local address" => "Address",
        "Possible source" => "Set from",
        _ => label,
    }
}

fn print_plain_fact(label: &str, value: &str, evidence: Evidence) {
    let mut lines = value.lines();
    let first = lines.next().unwrap_or_default();
    let marker = match evidence {
        Evidence::Confirmed => String::new(),
        Evidence::Inferred => " (inferred)".to_owned(),
    };

    println!("{label:<16} {first}{marker}");

    for line in lines {
        println!("{:<16} {}", "", line);
    }
}

fn card_fact(label: &str, value: &str, evidence: Evidence) {
    let marker = match evidence {
        Evidence::Confirmed => "",
        Evidence::Inferred => " (inferred)",
    };

    let label_width = 15;
    let value_width = INNER_WIDTH.saturating_sub(label_width + 1);
    let logical_lines: Vec<&str> = value.lines().collect();
    let mut first = true;

    for (index, logical_line) in logical_lines.iter().enumerate() {
        let mut wrapped = wrap_text(logical_line, value_width);

        if label == "Used by" && index >= 3 && logical_lines.len() > 4 {
            continue;
        }

        if label == "Used by" && index == 3 && logical_lines.len() > 4 {
            wrapped = vec![format!("…and {} more", logical_lines.len() - 3)];
        }

        for line in wrapped {
            let prefix = if first {
                format!("{label:<label_width$}")
            } else {
                " ".repeat(label_width)
            };
            let suffix = if first { marker } else { "" };
            card_raw_line(&format!("{prefix} {line}{suffix}"));
            first = false;
        }
    }

    if first {
        card_raw_line(&format!("{label:<label_width$} {marker}"));
    }
}

fn top_border(title: &str) {
    let prefix = format!("╭─ {title} ");
    let fill = CARD_WIDTH.saturating_sub(prefix.chars().count() + 1);
    println!("{prefix}{}╮", "─".repeat(fill));
}

fn section_border(title: &str) {
    let prefix = format!("├─ {title} ");
    let fill = CARD_WIDTH.saturating_sub(prefix.chars().count() + 1);
    println!("{prefix}{}┤", "─".repeat(fill));
}

fn bottom_border() {
    println!("╰{}╯", "─".repeat(CARD_WIDTH - 2));
}

fn card_line(text: &str) {
    for line in wrap_text(text, INNER_WIDTH) {
        let width = line.chars().count();
        let padding = INNER_WIDTH.saturating_sub(width);
        println!("│ {line}{} │", " ".repeat(padding));
    }
}

fn card_raw_line(text: &str) {
    let width = text.chars().count();
    let padding = INNER_WIDTH.saturating_sub(width);
    println!("│ {text}{} │", " ".repeat(padding));
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let separator = usize::from(!current.is_empty());
        if current.chars().count() + separator + word.chars().count() <= width {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
            continue;
        }

        if !current.is_empty() {
            lines.push(current);
            current = String::new();
        }

        if word.chars().count() <= width {
            current.push_str(word);
            continue;
        }

        let chars: Vec<char> = word.chars().collect();
        for chunk in chars.chunks(width) {
            let piece: String = chunk.iter().collect();
            if chunk.len() == width {
                lines.push(piece);
            } else {
                current = piece;
            }
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::wrap_text;

    #[test]
    fn wraps_text_to_requested_width() {
        let lines = wrap_text("one two three four", 9);
        assert_eq!(lines, vec!["one two", "three", "four"]);
        assert!(lines.iter().all(|line| line.chars().count() <= 9));
    }
}
