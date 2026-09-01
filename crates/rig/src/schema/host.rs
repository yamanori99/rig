use super::role::OsKind;
use crate::error::{Result, RigError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    pub name: String,
    pub role: String,
    #[serde(default)]
    pub os: Option<OsKind>,
    #[serde(default)]
    pub shell: Option<ShellKind>,
    #[serde(default)]
    pub user: Option<String>,
    /// User-chosen SSH Host aliases (`alias` + `ip` + `link`).
    #[serde(default)]
    pub ssh: Vec<SshPath>,
    /// Legacy flat address (implies alias `{name}-ts`). Prefer `[[ssh]]`.
    #[serde(default)]
    pub vpn: Option<String>,
    /// Legacy flat address (implies alias `{name}-lan`). Prefer `[[ssh]]`.
    #[serde(default)]
    pub lan: Option<String>,
    /// Legacy flat address (implies alias `{name}-tb`). Prefer `[[ssh]]`.
    #[serde(default)]
    pub thunderbolt: Option<String>,
    #[serde(default)]
    pub packages: HostPackages,
    /// Optional per-host feature overrides (unset keys keep the role).
    #[serde(default)]
    pub features: HostFeatures,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
}

/// One reachable path for this machine, with a user-chosen SSH `Host` alias.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshPath {
    /// SSH config `Host` name (fully user-chosen, e.g. `mini-ts` or `lab`).
    pub alias: String,
    pub ip: String,
    /// How this path is wired — drives TB apply + keys preference.
    pub link: LinkKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkKind {
    #[serde(alias = "ts", alias = "tailscale")]
    Vpn,
    Lan,
    #[serde(alias = "tb")]
    Thunderbolt,
}

impl LinkKind {
    pub fn as_str(self) -> &'static str {
        match self {
            LinkKind::Vpn => "vpn",
            LinkKind::Lan => "lan",
            LinkKind::Thunderbolt => "tb",
        }
    }

    pub fn comment(self) -> &'static str {
        match self {
            LinkKind::Vpn => "Tailscale / VPN",
            LinkKind::Lan => "LAN",
            LinkKind::Thunderbolt => "Thunderbolt",
        }
    }

    /// Prefer LAN/TB for authorized_keys (system sshd) over Tailscale.
    pub fn prefer_for_keys(self) -> bool {
        matches!(self, LinkKind::Lan | LinkKind::Thunderbolt)
    }
}

fn default_schema_version() -> u32 {
    1
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HostFeatures {
    pub gui: Option<bool>,
    pub cursor: Option<bool>,
    pub remote_login: Option<bool>,
    pub screen_sharing: Option<bool>,
    pub tailscale: Option<bool>,
    pub thunderbolt: Option<bool>,
    pub stay_awake: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HostPackages {
    #[serde(default)]
    pub add: Vec<String>,
    #[serde(default)]
    pub remove: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShellKind {
    Zsh,
    Bash,
}

impl ShellKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ShellKind::Zsh => "zsh",
            ShellKind::Bash => "bash",
        }
    }
}

impl Host {
    pub fn validate(&self, path: &Path) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(RigError::Msg(format!(
                "{}: name must not be empty",
                path.display()
            )));
        }
        if self.role.trim().is_empty() {
            return Err(RigError::Msg(format!(
                "{}: role must not be empty",
                path.display()
            )));
        }
        if self.schema_version != 1 {
            return Err(RigError::Msg(format!(
                "{}: unsupported schema_version {}",
                path.display(),
                self.schema_version
            )));
        }
        for p in self.ssh_paths() {
            if p.alias.trim().is_empty() {
                return Err(RigError::Msg(format!(
                    "{}: ssh.alias must not be empty",
                    path.display()
                )));
            }
            if p.alias.chars().any(char::is_whitespace) {
                return Err(RigError::Msg(format!(
                    "{}: ssh.alias must not contain whitespace: {}",
                    path.display(),
                    p.alias
                )));
            }
            validate_ip(path, &p.alias, &p.ip)?;
        }
        Ok(())
    }

    /// Resolved SSH paths: explicit `[[ssh]]` first, else legacy vpn/lan/thunderbolt.
    pub fn ssh_paths(&self) -> Vec<SshPath> {
        if !self.ssh.is_empty() {
            return self.ssh.clone();
        }
        let mut v = Vec::new();
        if let Some(ip) = &self.vpn {
            v.push(SshPath {
                alias: format!("{}-ts", self.name),
                ip: ip.clone(),
                link: LinkKind::Vpn,
            });
        }
        if let Some(ip) = &self.lan {
            v.push(SshPath {
                alias: format!("{}-lan", self.name),
                ip: ip.clone(),
                link: LinkKind::Lan,
            });
        }
        if let Some(ip) = &self.thunderbolt {
            v.push(SshPath {
                alias: format!("{}-tb", self.name),
                ip: ip.clone(),
                link: LinkKind::Thunderbolt,
            });
        }
        v
    }

    pub fn has_network(&self) -> bool {
        !self.ssh_paths().is_empty()
    }

    /// IPv4 used to configure this machine's Thunderbolt bridge0, if any.
    pub fn thunderbolt_ip(&self) -> Option<String> {
        self.ssh_paths()
            .into_iter()
            .find(|p| p.link == LinkKind::Thunderbolt)
            .map(|p| p.ip)
    }

    pub fn resolved_shell(&self) -> ShellKind {
        self.shell.unwrap_or_else(super::detect_shell)
    }

    pub fn resolved_os(&self) -> OsKind {
        self.os.unwrap_or_else(super::detect_os)
    }

    pub fn resolved_user(&self) -> String {
        self.user.clone().unwrap_or_else(|| whoami::username())
    }

    pub fn user_write_needed(&self) -> Option<String> {
        user_write_needed(self.user.as_deref(), &whoami::username())
    }
}

fn user_write_needed(toml_user: Option<&str>, local: &str) -> Option<String> {
    match toml_user.map(str::trim).filter(|s| !s.is_empty()) {
        Some(u) if u == local => None,
        Some(u) => Some(format!("{u} → {local}")),
        None => Some(format!("write {local}")),
    }
}

/// Set or insert `user = "..."` without dropping comments.
pub fn set_user_line(toml: &str, user: &str) -> String {
    let assign = format!("user = \"{user}\"");
    let mut out = Vec::new();
    let mut done = false;
    for line in toml.lines() {
        let t = line.trim_start();
        if !done && t.starts_with("user") && t.contains('=') && !t.starts_with('#') {
            out.push(assign.clone());
            done = true;
        } else {
            out.push(line.to_string());
        }
    }
    if !done {
        let mut with = Vec::new();
        let mut inserted = false;
        for line in &out {
            with.push(line.clone());
            if !inserted && line.trim_start().starts_with("role") {
                with.push(assign.clone());
                inserted = true;
            }
        }
        if !inserted {
            with.push(assign);
        }
        out = with;
    }
    out.join("\n") + "\n"
}

pub fn persist_user(path: &Path, user: &str) -> Result<bool> {
    let raw = fs::read_to_string(path).map_err(RigError::Io)?;
    let next = set_user_line(&raw, user);
    if next == raw {
        return Ok(false);
    }
    fs::write(path, next).map_err(RigError::Io)?;
    Ok(true)
}

fn validate_ip(path: &Path, label: &str, ip: &str) -> Result<()> {
    if ip.split('.').count() != 4 && !ip.contains(':') {
        return Err(RigError::Msg(format!(
            "{}: {label} looks invalid: {ip}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_write_skip_when_matches() {
        assert!(user_write_needed(Some("admin"), "admin").is_none());
    }

    #[test]
    fn user_write_when_differs() {
        assert_eq!(
            user_write_needed(Some("tak"), "admin").as_deref(),
            Some("tak → admin")
        );
    }

    #[test]
    fn user_write_when_omitted() {
        assert_eq!(
            user_write_needed(None, "admin").as_deref(),
            Some("write admin")
        );
    }

    #[test]
    fn set_user_replaces_line() {
        let t = "name = \"x\"\nrole = \"compute\"\nuser = \"tak\"\n";
        assert_eq!(
            set_user_line(t, "admin"),
            "name = \"x\"\nrole = \"compute\"\nuser = \"admin\"\n"
        );
    }

    #[test]
    fn set_user_inserts_after_role() {
        let t = "name = \"x\"\nrole = \"compute\"\nschema_version = 1\n";
        let out = set_user_line(t, "admin");
        assert!(out.contains("role = \"compute\"\nuser = \"admin\"\n"));
    }
}
