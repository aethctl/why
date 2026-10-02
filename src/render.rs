use anyhow::Result;
use owo_colors::OwoColorize;

use crate::model::Finding;

pub fn print(finding: &Finding, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(finding)?);
        return Ok(());
    }

    println!("{}", finding.subject.bold());
    println!("{}", finding.summary);
    println!();

    for fact in &finding.facts {
        print_fact(&fact.label, &fact.value);
    }

    if !finding.notes.is_empty() {
        println!();
        for note in &finding.notes {
            println!("{} {}", "note".yellow().bold(), note);
        }
    }

    Ok(())
}

fn print_fact(label: &str, value: &str) {
    let mut lines = value.lines();
    let first = lines.next().unwrap_or_default();

    println!("{:<16} {}", label.dimmed(), first);

    for line in lines {
        println!("{:<16} {}", "", line);
    }
}
