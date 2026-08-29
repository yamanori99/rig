use super::role::OsKind;
use crate::error::{Result, RigError};
use serde::{Deserialize, Serialize};
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
