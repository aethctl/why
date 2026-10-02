use std::path::Path;

use anyhow::{Result, bail};

use crate::model::Finding;
use crate::ownership;
use crate::resolver;

pub fn resolve(subject: &str) -> Result<Finding> {
    if subject == "this" {
        return resolver::project::resolve_current();
    }

    if let Some(port) = subject
        .strip_prefix(':')
        .and_then(|value| value.parse::<u16>().ok())
    {
        return resolver::port::resolve(port);
    }

    if looks_like_path(subject) {
        return resolver::file::resolve(subject);
    }

    if let Ok(pid) = subject.parse::<u32>()
        && Path::new(&format!("/proc/{pid}")).exists()
    {
        return resolver::process::resolve(pid);
    }

    if let Ok(finding) = resolver::package::resolve(subject) {
        return Ok(finding);
    }

    if let Ok(finding) = resolver::service::resolve(subject) {
        return Ok(finding);
    }

    if ownership::command_path(subject).is_some() {
        return resolver::command::resolve(subject);
    }

    bail!("nothing matched '{subject}'")
}

fn looks_like_path(subject: &str) -> bool {
    subject.starts_with('/')
        || subject.starts_with("./")
        || subject.starts_with("../")
        || subject.starts_with("~/")
        || Path::new(subject).exists()
}

#[cfg(test)]
mod tests {
    use super::looks_like_path;

    #[test]
    fn recognizes_paths() {
        assert!(looks_like_path("/etc/hosts"));
        assert!(looks_like_path("./Cargo.toml"));
        assert!(looks_like_path("../thing"));
        assert!(looks_like_path("~/thing"));
    }
}
