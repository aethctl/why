use anyhow::{Result, bail};

use crate::model::Finding;
use crate::system;

pub fn resolve(port: u16) -> Result<Finding> {
    if !system::command_exists("ss") {
        bail!("the 'ss' command is required to inspect ports");
    }

    let out = system::run("ss", &["-ltnup", &format!("sport = :{port}")])?;
    if !out.success {
        bail!("failed to inspect port {port}: {}", out.stderr);
    }

    let line = out
        .stdout
        .lines()
        .skip(1)
        .find(|line| !line.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("nothing is listening on port {port}"))?;

    let parts: Vec<_> = line.split_whitespace().collect();
    let mut finding = Finding::new(
        port.to_string(),
        "port",
        format!("port {port} is currently listening"),
    );

    if let Some(protocol) = parts.first() {
        finding = finding.fact("Protocol", (*protocol).to_owned());
    }

    if let Some(local) = parts.get(4) {
        finding = finding.fact("Local address", (*local).to_owned());
    }

    if let Some((process, pid)) = parse_process(line) {
        finding = finding
            .fact("Process", process)
            .fact("PID", pid.to_string());

        if let Ok(process_finding) = crate::resolver::process::resolve(pid) {
            for fact in process_finding.facts {
                if matches!(
                    fact.label.as_str(),
                    "Executable" | "Provided by" | "Working dir"
                ) {
                    finding = finding.fact(fact.label, fact.value);
                }
            }
        }
    } else {
        finding = finding
            .note("The listening socket was visible, but process ownership was unavailable.");
    }

    Ok(finding)
}

fn parse_process(line: &str) -> Option<(String, u32)> {
    let users = line.split("users:").nth(1)?;
    let process = users.split('"').nth(1)?.to_owned();
    let pid_text = users.split("pid=").nth(1)?.split(',').next()?;
    let pid = pid_text.parse().ok()?;
    Some((process, pid))
}

#[cfg(test)]
mod tests {
    use super::parse_process;

    #[test]
    fn parses_ss_process() {
        let line = r#"tcp LISTEN 0 5 127.0.0.1:8123 0.0.0.0:* users:(("python3",pid=860017,fd=3))"#;
        let parsed = parse_process(line);
        assert_eq!(parsed, Some(("python3".to_owned(), 860017)));
    }
}
