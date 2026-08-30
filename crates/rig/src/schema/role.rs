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
    /// macOS Screen Sharing (VNC, port 5900). No-op on Linux.
    #[serde(default)]
    pub screen_sharing: bool,
    #[serde(default)]
    pub tailscale: bool,
    #[serde(default)]
    pub thunderbolt: bool,
    /// Keep the machine from idling to sleep (compute nodes).
    #[serde(default)]
    pub stay_awake: bool,
}

impl RoleFeatures {
    /// Host `[features]` keys overlay the role; omitted keys keep the role value.
    pub fn with_host(&self, host: &super::host::HostFeatures) -> Self {
        Self {
            gui: host.gui.unwrap_or(self.gui),
            cursor: host.cursor.unwrap_or(self.cursor),
            remote_login: host.remote_login.unwrap_or(self.remote_login),
            screen_sharing: host.screen_sharing.unwrap_or(self.screen_sharing),
            tailscale: host.tailscale.unwrap_or(self.tailscale),
            thunderbolt: host.thunderbolt.unwrap_or(self.thunderbolt),
            stay_awake: host.stay_awake.unwrap_or(self.stay_awake),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stay_awake_defaults_off_when_omitted() {
        let r: RoleFeatures = toml::from_str("gui = true").unwrap();
        assert!(!r.stay_awake);
        assert!(r.gui);
    }

    #[test]
    fn host_features_overlay_only_set_keys() {
        let role = RoleFeatures {
            stay_awake: true,
            remote_login: true,
            ..RoleFeatures::default()
        };
        let host = super::super::host::HostFeatures {
            stay_awake: Some(false),
            ..super::super::host::HostFeatures::default()
        };
        let merged = role.with_host(&host);
        assert!(!merged.stay_awake);
        assert!(merged.remote_login);
    }
}
