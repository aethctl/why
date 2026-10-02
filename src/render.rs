use anyhow::Result;
use owo_colors::OwoColorize;

use crate::model::{Evidence, Fact, Finding};

const CARD_WIDTH: usize = 74;
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
    top_border(&finding.subject);
    card_styled_line(
        &subtitle(finding),
        &format!("{}", subtitle(finding).dimmed()),
    );

    card_blank();
    section_label("what");
    card_paragraph(&what_is_it(finding));

    card_blank();
    section_label("why");
    card_paragraph(&why_is_it_here(finding));

    let visible: Vec<&Fact> = finding
        .facts
        .iter()
        .filter(|fact| show_in_default(finding, &fact.label))
        .collect();

    if !visible.is_empty() {
        card_blank();
        section_label("details");
        for fact in visible {
            card_fact(display_label(&fact.label), &fact.value, fact.evidence);
        }
    }

    if !finding.notes.is_empty() {
        card_blank();
        section_label("note");
        for note in &finding.notes {
            card_paragraph(note);
        }
    }

    bottom_border();
    println!(
        "  {}   {}   {}   {}",
        "--deep".bold(),
        "more detail".dimmed(),
        "·".dimmed(),
        "--plain  script-friendly".dimmed()
    );
}

fn subtitle(finding: &Finding) -> String {
    match finding.kind.as_str() {
        "package" => finding
            .value("Store package")
            .map(|package| format!("command  ·  {package}"))
            .unwrap_or_else(|| "software package".to_owned()),
        "command" => finding
            .value("Provided by")
            .map(|package| format!("command  ·  {package}"))
            .unwrap_or_else(|| "terminal command".to_owned()),
        "builtin" => finding
            .value("Shell")
            .map(|shell| format!("shell builtin  ·  {shell}"))
            .unwrap_or_else(|| "shell builtin".to_owned()),
        "shell" => finding
            .value("Kind")
            .map(|kind| format!("shell {kind}"))
            .unwrap_or_else(|| "shell shortcut".to_owned()),
        "service" => "systemd service".to_owned(),
        "process" => "running process".to_owned(),
        "port" => "network listener".to_owned(),
        "file" => "filesystem path".to_owned(),
        "environment" => "environment variable".to_owned(),
        "context" => "current project".to_owned(),
        _ => friendly_kind(&finding.kind).to_owned(),
    }
}

fn friendly_kind(kind: &str) -> &'static str {
    match kind {
        "package" => "software package",
        "command" => "terminal command",
        "builtin" => "shell builtin",
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

fn what_is_it(finding: &Finding) -> String {
    if let Some(description) = finding.value("Description") {
        return description.to_owned();
    }

    match finding.kind.as_str() {
        "package" => format!("{} is software available on this system.", finding.subject),
        "command" => format!(
            "{} is a command you can run from the terminal.",
            finding.subject
        ),
        "builtin" => format!(
            "{} is a command implemented directly by your shell.",
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
    }
}

fn why_is_it_here(finding: &Finding) -> String {
    if finding.kind == "builtin"
        && let Some(reason) = finding.value("Reason")
    {
        return reason.to_owned();
    }

    match finding.value("Source") {
        Some("NixOS system profile") => {
            let package = finding.value("Store package").unwrap_or(&finding.subject);
            format!(
                "NixOS includes {package} as part of the active system because something else depends on it. You may never have installed it directly."
            )
        }
        Some("Declarative Nix configuration") => {
            "Your Nix configuration declares it directly, so every rebuild keeps it available."
                .to_owned()
        }
        Some("nix profile") => {
            "It was installed into your personal Nix profile, so it is available to your user account."
                .to_owned()
        }
        Some("Shell builtin") => {
            "Your shell provides this command itself instead of launching a separate program."
                .to_owned()
        }
        _ => match finding.kind.as_str() {
            "process" => finding
                .value("Why running")
                .map(|_| {
                    "Another process started it. The process ancestry below shows the chain."
                        .to_owned()
                })
                .unwrap_or_else(|| {
                    "It is currently running in this session or system environment.".to_owned()
                }),
            "port" => {
                "A running program opened this port so it can accept network connections.".to_owned()
            }
            "service" => {
                "systemd knows about this service and controls when it starts or stops.".to_owned()
            }
            "file" => finding
                .value("Provided by")
                .map(|owner| format!("The {owner} package provides this path."))
                .unwrap_or_else(|| {
                    "The path exists on your filesystem, but no supported package manager claimed it."
                        .to_owned()
                }),
            "environment" => finding
                .value("Possible source")
                .map(|source| format!("Your shell or environment appears to set it from {source}."))
                .unwrap_or_else(|| {
                    "The current process inherited it from its environment.".to_owned()
                }),
            "shell" => finding
                .value("Defined at")
                .map(|source| format!("Your shell configuration defines it at {source}."))
                .unwrap_or_else(|| "Your shell configuration defines this shortcut.".to_owned()),
            "builtin" => {
                "Your shell implements it directly because it needs to interact with shell state."
                    .to_owned()
            }
            "context" => {
                "You asked about this, so why inspected the current directory and its live activity."
                    .to_owned()
            }
            _ => finding.summary.clone(),
        },
    }
}

fn show_in_default(finding: &Finding, label: &str) -> bool {
    if matches!(
        label,
        "Description" | "Reason" | "Store path" | "Executable"
    ) {
        return false;
    }

    if finding.kind == "builtin" && label == "Source" {
        return false;
    }

    true
}

fn display_label(label: &str) -> &str {
    match label {
        "Store package" => "package",
        "Source" => "source",
        "Referenced by" => "used by",
        "Declared at" => "declared in",
        "Working dir" => "working in",
        "Systemd unit" => "service",
        "Local address" => "address",
        "Possible source" => "set from",
        "Defined at" => "defined in",
        "Shell" => "shell",
        "Kind" => "kind",
        "Also defined at" => "also in",
        "Parent PID" => "parent",
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
        Evidence::Inferred => "  ~ inferred",
    };

    let label_width = 13;
    let value_width = INNER_WIDTH.saturating_sub(label_width + 2);

    if label == "used by" {
        let lines: Vec<&str> = value.lines().collect();
        let shown: Vec<&str> = lines.iter().copied().take(2).collect();
        let mut summary = shown.join("  ·  ");
        if lines.len() > shown.len() {
            summary.push_str(&format!("  +{} more", lines.len() - shown.len()));
        }
        card_fact_line(label, &summary, marker, label_width, value_width);
        return;
    }

    if label == "declared in" {
        let first = value.lines().next().unwrap_or(value);
        card_fact_line(label, first, marker, label_width, value_width);
        return;
    }

    let mut first_row = true;
    for logical_line in value.lines() {
        let wrapped = wrap_text(logical_line, value_width);
        for line in wrapped {
            let row_label = if first_row { label } else { "" };
            let row_marker = if first_row { marker } else { "" };
            card_fact_line(row_label, &line, row_marker, label_width, value_width);
            first_row = false;
        }
    }
}

fn card_fact_line(label: &str, value: &str, marker: &str, label_width: usize, value_width: usize) {
    let wrapped = wrap_text(value, value_width);
    for (index, line) in wrapped.iter().enumerate() {
        let shown_label = if index == 0 { label } else { "" };
        let shown_marker = if index == 0 { marker } else { "" };
        card_raw_line(&format!(
            "{shown_label:<label_width$}  {line}{shown_marker}"
        ));
    }
}

fn top_border(subject: &str) {
    let title = format!(" why · {subject} ");
    let fill = CARD_WIDTH.saturating_sub(title.chars().count() + 2);
    println!("╭─{}{}╮", title, "─".repeat(fill));
}

fn bottom_border() {
    println!("╰{}╯", "─".repeat(CARD_WIDTH - 2));
}

fn section_label(label: &str) {
    let raw = format!("  {label}");
    let styled = format!("  {}", label.dimmed());
    card_styled_line(&raw, &styled);
}

fn card_blank() {
    card_raw_line("");
}

fn card_paragraph(text: &str) {
    for line in wrap_text(text, INNER_WIDTH - 2) {
        card_raw_line(&format!("  {line}"));
    }
}

fn card_styled_line(raw: &str, styled: &str) {
    let width = raw.chars().count();
    let padding = INNER_WIDTH.saturating_sub(width);
    println!("│ {styled}{} │", " ".repeat(padding));
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
