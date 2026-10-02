use anyhow::Result;
use owo_colors::OwoColorize;

use crate::model::{Evidence, Finding};

pub fn print(finding: &Finding, json: bool, plain: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(finding)?);
        return Ok(());
    }

    if plain {
        println!("{}", finding.subject);
        println!("{}", finding.summary);
    } else {
        println!("{}", finding.subject.bold());
        println!("{}", finding.summary);
    }
    println!();

    for fact in &finding.facts {
        print_fact(&fact.label, &fact.value, fact.evidence, plain);
    }

    if !finding.notes.is_empty() {
        println!();
        for note in &finding.notes {
            if plain {
                println!("note {}", note);
            } else {
                println!("{} {}", "note".yellow().bold(), note);
            }
        }
    }

    Ok(())
}

fn print_fact(label: &str, value: &str, evidence: Evidence, plain: bool) {
    let mut lines = value.lines();
    let first = lines.next().unwrap_or_default();
    let marker = match evidence {
        Evidence::Confirmed => String::new(),
        Evidence::Inferred if plain => " (inferred)".to_owned(),
        Evidence::Inferred => format!(" {}", "(inferred)".dimmed()),
    };

    if plain {
        println!("{label:<16} {first}{marker}");
    } else {
        println!("{:<16} {}{}", label.dimmed(), first, marker);
    }

    for line in lines {
        println!("{:<16} {}", "", line);
    }
}
