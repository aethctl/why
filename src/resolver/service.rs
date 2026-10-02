use anyhow::{Result, bail};

use crate::model::Finding;
use crate::system;

pub fn resolve(name: &str) -> Result<Finding> {
    if !system::command_exists("systemctl") {
        bail!("systemctl is not available on this system");
    }

    let service = if name.contains('.') {
        name.to_owned()
    } else {
        format!("{name}.service")
    };

    let status = system::run(
        "systemctl",
        &[
            "show",
            &service,
            "--no-pager",
            "--property=Id,Description,LoadState,ActiveState,SubState,UnitFileState,FragmentPath",
        ],
    )?;

    if !status.success {
        bail!("systemd could not resolve '{service}': {}", status.stderr);
    }

    let props = parse_properties(&status.stdout);
    if property(&props, "LoadState") == "not-found" {
        bail!("systemd unit '{service}' does not exist");
    }

    let description = props
        .get("Description")
        .filter(|value| !value.is_empty())
        .cloned()
        .unwrap_or_else(|| "systemd service".to_owned());

    let mut finding = Finding::new(&service, "service", description)
        .fact("Load state", property(&props, "LoadState"))
        .fact("Active", property(&props, "ActiveState"))
        .fact("State", property(&props, "SubState"))
        .fact("Enabled", property(&props, "UnitFileState"))
        .fact("Unit file", property(&props, "FragmentPath"));

    if let Ok(deps) = system::run(
        "systemctl",
        &[
            "list-dependencies",
            "--reverse",
            "--plain",
            "--no-pager",
            &service,
        ],
    ) {
        let users: Vec<_> = deps
            .stdout
            .lines()
            .skip(1)
            .map(str::trim)
            .filter(|line| !line.is_empty() && *line != service)
            .take(8)
            .collect();

        if !users.is_empty() {
            finding = finding.fact("Referenced by", users.join(", "));
        }
    }

    Ok(finding)
}

fn parse_properties(text: &str) -> std::collections::BTreeMap<String, String> {
    text.lines()
        .filter_map(|line| line.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
}

fn property(props: &std::collections::BTreeMap<String, String>, key: &str) -> String {
    props
        .get(key)
        .filter(|value| !value.is_empty())
        .cloned()
        .unwrap_or_else(|| "unknown".to_owned())
}
