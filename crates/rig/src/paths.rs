use crate::error::RigError;
use std::path::{Path, PathBuf};

pub fn discover_root(explicit: Option<PathBuf>) -> miette::Result<PathBuf> {
    if let Some(p) = explicit {
        if looks_like_root(&p) {
            return Ok(p);
        }
        return Err(RigError::Msg(format!(
            "path does not look like a rig root: {}",
            p.display()
        ))
        .into());
    }

    let mut dir = std::env::current_dir().map_err(RigError::Io)?;
    loop {
        if looks_like_root(&dir) {
            return Ok(dir);
        }
        if !dir.pop() {
            break;
        }
    }

    // Fallback: executable near repo (dev): ../../ from target/debug/rig
    if let Ok(exe) = std::env::current_exe() {
        if let Some(root) = exe
            .ancestors()
            .find(|p| looks_like_root(p))
            .map(|p| p.to_path_buf())
        {
            return Ok(root);
        }
    }

    Err(RigError::RootNotFound.into())
}

fn looks_like_root(p: &Path) -> bool {
    p.join("roles").is_dir() && p.join("crates").is_dir() || p.join("roles").is_dir() && p.join("packages").is_dir()
}

pub fn hosts_dir(root: &Path) -> PathBuf {
    root.join("hosts")
}

pub fn roles_dir(root: &Path) -> PathBuf {
    root.join("roles")
}

pub fn packages_dir(root: &Path) -> PathBuf {
    root.join("packages")
}

pub fn templates_dir(root: &Path) -> PathBuf {
    root.join("templates")
}

pub fn state_path() -> PathBuf {
    directories::ProjectDirs::from("dev", "rig", "rig")
        .map(|d| d.data_local_dir().join("state.json"))
        .unwrap_or_else(|| PathBuf::from(".rig-state.json"))
}
