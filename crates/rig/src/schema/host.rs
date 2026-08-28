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
    #[serde(default)]
    pub vpn: Option<String>,
    #[serde(default)]
    pub lan: Option<String>,
    #[serde(default)]
    pub thunderbolt: Option<String>,
    #[serde(default)]
    pub packages: HostPackages,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
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
        for field in [
            ("vpn", &self.vpn),
            ("lan", &self.lan),
            ("thunderbolt", &self.thunderbolt),
        ] {
            if let Some(ip) = field.1 {
                if ip.split('.').count() != 4 && !ip.contains(':') {
                    return Err(RigError::Msg(format!(
                        "{}: {} looks invalid: {ip}",
                        path.display(),
                        field.0
                    )));
                }
            }
        }
        Ok(())
    }

    pub fn resolved_shell(&self) -> ShellKind {
        self.shell.unwrap_or_else(super::detect_shell)
    }

    pub fn resolved_os(&self) -> OsKind {
        self.os.unwrap_or_else(super::detect_os)
    }

    pub fn resolved_user(&self) -> String {
        self.user
            .clone()
            .unwrap_or_else(|| whoami::username())
    }
}
