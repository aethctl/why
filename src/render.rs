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
        println!("{:<16} {}", fact.label.dimmed(), fact.value);
    }

    if !finding.notes.is_empty() {
        println!();
        for note in &finding.notes {
            println!("{} {}", "note".yellow().bold(), note);
        }
    }

    Ok(())
}
