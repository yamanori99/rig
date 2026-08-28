use crate::embed;
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

    if let Ok(dir) = std::env::current_dir() {
        let mut dir = dir;
        loop {
            if looks_like_root(&dir) {
                return Ok(dir);
            }
            if !dir.pop() {
                break;
            }
        }
    }

    // Dev: executable sitting inside a checkout (target/debug/rig → repo root).
    if let Ok(exe) = std::env::current_exe() {
        if let Some(root) = exe
            .ancestors()
            .find(|p| looks_like_root(p))
            .map(|p| p.to_path_buf())
        {
            return Ok(root);
        }
    }

    // Standalone binary: materialize embedded roles/packages/templates.
    Ok(embed::ensure_embedded_root()?)
}

fn looks_like_root(p: &Path) -> bool {
    let roles = p.join("roles").is_dir();
    let packages = p.join("packages").is_dir();
    let crates = p.join("crates").is_dir();
    roles && (packages || crates)
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
