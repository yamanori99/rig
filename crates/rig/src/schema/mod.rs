mod host;
mod role;

pub use host::{Host, LinkKind, ShellKind};
pub use role::{OsKind, Role};

use crate::error::{Result, RigError};
use crate::paths;
use std::path::Path;

pub fn load_hosts(root: &Path) -> Result<Vec<(std::path::PathBuf, Host)>> {
    let dir = paths::hosts_dir(root);
    let mut out = Vec::new();
    if !dir.is_dir() {
        return Ok(out);
    }
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("toml") {
            continue;
        }
        // skip examples/ subdirectory files when scanning top-level only
        let raw = std::fs::read_to_string(&path)?;
        let host: Host = toml::from_str(&raw).map_err(|source| RigError::Toml {
            path: path.display().to_string(),
            source,
        })?;
        host.validate(&path)?;
        out.push((path, host));
    }
    out.sort_by(|a, b| a.1.name.cmp(&b.1.name));
    Ok(out)
}

pub fn load_role(root: &Path, name: &str) -> Result<Role> {
    let path = paths::roles_dir(root).join(format!("{name}.toml"));
    let raw = std::fs::read_to_string(&path).map_err(|_| {
        RigError::Msg(format!("role not found: {} ({})", name, path.display()))
    })?;
    let role: Role = toml::from_str(&raw).map_err(|source| RigError::Toml {
        path: path.display().to_string(),
        source,
    })?;
    Ok(role)
}

pub fn list_roles(root: &Path) -> Result<Vec<String>> {
    let dir = paths::roles_dir(root);
    let mut names = Vec::new();
    if !dir.is_dir() {
        return Ok(names);
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                names.push(stem.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}

pub fn detect_current_host<'a>(
    hosts: &'a [(std::path::PathBuf, Host)],
) -> Option<&'a Host> {
    let hostname = current_hostname();
    let short = hostname.split('.').next().unwrap_or(&hostname);
    hosts.iter().find_map(|(_, h)| {
        if h.name == short || h.name == hostname || hostname.contains(&h.name) {
            Some(h)
        } else {
            None
        }
    })
}

pub fn current_hostname() -> String {
    whoami::fallible::hostname()
        .unwrap_or_else(|_| "unknown".into())
        .to_lowercase()
}

pub fn detect_shell() -> ShellKind {
    let shell = std::env::var("SHELL").unwrap_or_default();
    if shell.ends_with("/bash") || shell == "bash" {
        ShellKind::Bash
    } else {
        ShellKind::Zsh
    }
}

pub fn detect_os() -> OsKind {
    if cfg!(target_os = "macos") {
        OsKind::Macos
    } else if cfg!(target_os = "linux") {
        OsKind::Linux
    } else {
        OsKind::Macos
    }
}
