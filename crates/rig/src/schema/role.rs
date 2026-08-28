use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub description: String,
    #[serde(default)]
    pub packages: Vec<String>,
    #[serde(default)]
    pub features: RoleFeatures,
    #[serde(default)]
    pub default_shell: Option<super::ShellKind>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoleFeatures {
    #[serde(default)]
    pub gui: bool,
    #[serde(default)]
    pub cursor: bool,
    #[serde(default)]
    pub remote_login: bool,
    #[serde(default)]
    pub tailscale: bool,
    #[serde(default)]
    pub thunderbolt: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OsKind {
    Macos,
    Linux,
}

impl OsKind {
    pub fn as_str(self) -> &'static str {
        match self {
            OsKind::Macos => "macos",
            OsKind::Linux => "linux",
        }
    }
}
