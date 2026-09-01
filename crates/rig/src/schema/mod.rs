mod host;
mod role;

pub use host::{Host, LinkKind, ShellKind, SshPath};
pub use role::{OsKind, Role, RoleFeatures};

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
    let raw = std::fs::read_to_string(&path)
        .map_err(|_| RigError::Msg(format!("role not found: {} ({})", name, path.display())))?;
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

pub fn detect_current_host<'a>(hosts: &'a [(std::path::PathBuf, Host)]) -> Option<&'a Host> {
    let ids = machine_ids();
    hosts.iter().find_map(|(path, h)| {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        if identity_hit(&ids, &h.name, stem) {
            Some(h)
        } else {
            None
        }
    })
}

/// Why `rig apply` / `keys` could not bind this machine to ~/.rig-hosts.
pub fn unregistered_hint(root: &Path, hosts: &[(std::path::PathBuf, Host)]) -> String {
    let hn = current_hostname();
    let short = short_host(&hn);
    let dir = paths::hosts_dir(root);
    if hosts.is_empty() {
        return format!(
            "no host toml in {} (this OS hostname is {hn}, short {short}). \
             clone the inventory git into ~/.rig-hosts or run `rig init`. \
             product hosts/ is examples only",
            dir.display()
        );
    }
    let names: Vec<&str> = hosts.iter().map(|(_, h)| h.name.as_str()).collect();
    format!(
        "this machine is not in ~/.rig-hosts (hostname={hn}, short={short}). \
         have: {}. expected {}/{short}.toml with name = \"{short}\"",
        names.join(", "),
        dir.display()
    )
}

pub fn current_hostname() -> String {
    whoami::fallible::hostname()
        .unwrap_or_else(|_| "unknown".into())
        .to_lowercase()
}

fn short_host(name: &str) -> &str {
    name.split('.').next().unwrap_or(name)
}

fn machine_ids() -> Vec<String> {
    let mut ids = Vec::new();
    let hn = current_hostname();
    ids.push(hn.clone());
    ids.push(short_host(&hn).to_string());
    #[cfg(target_os = "macos")]
    if let Some(local) = scutil_get("LocalHostName") {
        let local = local.to_lowercase();
        ids.push(local.clone());
        ids.push(short_host(&local).to_string());
    }
    ids.sort();
    ids.dedup();
    ids
}

fn identity_hit(machine_ids: &[String], inventory_name: &str, file_stem: &str) -> bool {
    let inv = inventory_name.to_ascii_lowercase();
    let stem = file_stem.to_ascii_lowercase();
    for raw in [&inv, &stem] {
        if raw.is_empty() {
            continue;
        }
        let short = short_host(raw);
        for id in machine_ids {
            if raw == id || short == short_host(id) || raw == short_host(id) {
                return true;
            }
        }
    }
    false
}

#[cfg(target_os = "macos")]
fn scutil_get(key: &str) -> Option<String> {
    let out = std::process::Command::new("scutil")
        .args(["--get", key])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_matches_bonjour_suffix() {
        let ids = vec!["m4-mini-tak.local".into(), "m4-mini-tak".into()];
        assert!(identity_hit(&ids, "m4-mini-tak", "m4-mini-tak"));
        assert!(identity_hit(&ids, "M4-Mini-Tak", "other"));
        assert!(identity_hit(&ids, "nope", "m4-mini-tak"));
        assert!(!identity_hit(&ids, "m4-mba-neva", "m4-mba-neva"));
    }
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
