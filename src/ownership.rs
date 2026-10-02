use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::system;

pub fn command_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

pub fn nix_store_root(path: &Path) -> Option<PathBuf> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let text = canonical.to_string_lossy();
    let rest = text.strip_prefix("/nix/store/")?;
    let store_item = rest.split('/').next()?;
    Some(PathBuf::from("/nix/store").join(store_item))
}

pub fn nix_store_package(path: &Path) -> Option<String> {
    let root = nix_store_root(path)?;
    let store_item = root.file_name()?.to_str()?;
    let (_, package) = store_item.split_once('-')?;
    Some(package.to_owned())
}

pub fn pacman_owner(path: &Path) -> Result<Option<String>> {
    if !system::command_exists("pacman") {
        return Ok(None);
    }

    let path = path.to_string_lossy();
    let out = system::run("pacman", &["-Qo", &path])?;
    if !out.success {
        return Ok(None);
    }

    let owner = out
        .stdout
        .split_once(" is owned by ")
        .and_then(|(_, rest)| rest.split_whitespace().next())
        .map(str::to_owned);

    Ok(owner)
}

#[cfg(test)]
mod tests {
    use super::{nix_store_package, nix_store_root};
    use std::path::Path;

    #[test]
    fn parses_nix_store_path() {
        let path = Path::new("/nix/store/abc123-git-2.55.0/bin/git");
        assert_eq!(
            nix_store_root(path).as_deref(),
            Some(Path::new("/nix/store/abc123-git-2.55.0"))
        );
        assert_eq!(nix_store_package(path).as_deref(), Some("git-2.55.0"));
    }
}
