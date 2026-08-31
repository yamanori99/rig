use crate::schema::{Host, Role, ShellKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyPlan {
    pub host: String,
    pub role: String,
    pub os: String,
    pub shell: String,
    pub user: String,
    pub package_sets: Vec<String>,
    pub steps: Vec<ApplyStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyStep {
    pub id: String,
    pub detail: String,
    pub skip: bool,
}

pub fn build_plan(host: &Host, role: &Role) -> ApplyPlan {
    let shell = host.shell.unwrap_or_else(|| {
        role.default_shell
            .unwrap_or_else(crate::schema::detect_shell)
    });
    let os = host.resolved_os();
    let user = host.resolved_user();

    let mut package_sets = role.packages.clone();
    for add in &host.packages.add {
        if !package_sets.iter().any(|s| s == add) {
            package_sets.push(add.clone());
        }
    }
    package_sets.retain(|s| !host.packages.remove.iter().any(|r| r == s));

    let mut steps = Vec::new();
    steps.push(ApplyStep {
        id: "validate".into(),
        detail: format!("{}  schema {}", host.role, host.schema_version),
        skip: false,
    });
    if matches!(shell, ShellKind::Zsh) {
        steps.push(ApplyStep {
            id: "omz".into(),
            detail: "ensure Oh My Zsh + powerlevel10k + plugins".into(),
            skip: false,
        });
    }
    steps.push(ApplyStep {
        id: "link-shell".into(),
        detail: format!(
            "templates/shell/{{common,{}}} -> ~/.{}rc{}",
            shell.as_str(),
            shell.as_str(),
            if matches!(shell, ShellKind::Zsh) {
                " + OMZ/p10k product rc"
            } else {
                ""
            }
        ),
        skip: false,
    });
    steps.push(ApplyStep {
        id: "link-tmux".into(),
        detail: "templates/tmux/tmux.conf -> ~/.tmux.conf (overlay preferred)".into(),
        skip: false,
    });
    steps.push(ApplyStep {
        id: "packages".into(),
        detail: format!(
            "{} sets via {}: {}",
            package_sets.len(),
            match os {
                crate::schema::OsKind::Macos => "brew",
                crate::schema::OsKind::Linux => "apt",
            },
            package_sets.join(", ")
        ),
        skip: package_sets.is_empty(),
    });
    steps.push(ApplyStep {
        id: "ssh-config".into(),
        detail: "generate ssh config from hosts/*.toml".into(),
        skip: false,
    });

    let features = role.features.with_host(&host.features);
    let f = &features;
    steps.push(feature_step(
        "gui",
        f.gui,
        "install / enable GUI apps",
        "off in role",
    ));
    steps.push(feature_step(
        "cursor",
        f.cursor,
        "link Cursor user settings",
        "off in role",
    ));
    steps.push(feature_step(
        "remote-login",
        f.remote_login,
        "enable remote login / sshd",
        "off in role",
    ));
    steps.push(feature_step(
        "screen-sharing",
        f.screen_sharing && matches!(os, crate::schema::OsKind::Macos),
        "listen :5900 (Screen Sharing.app: toggle once in System Settings)",
        match os {
            crate::schema::OsKind::Macos => "off in role",
            crate::schema::OsKind::Linux => "macOS only",
        },
    ));
    steps.push(feature_step(
        "tailscale",
        f.tailscale,
        "configure Tailscale (CLI or Tailscale.app)",
        "off in role",
    ));
    let tb_ip = host.thunderbolt_ip();
    let tb_on = f.thunderbolt && tb_ip.is_some();
    steps.push(feature_step(
        "thunderbolt",
        tb_on,
        match &tb_ip {
            Some(ip) => format!("configure thunderbolt IP {ip}"),
            None => "no thunderbolt [[ssh]] link on host".into(),
        },
        if f.thunderbolt {
            "no thunderbolt [[ssh]] link on host"
        } else {
            "off in role"
        },
    ));
    steps.push(feature_step(
        "stay-awake",
        f.stay_awake,
        match os {
            crate::schema::OsKind::Macos => "pmset -a: sleep/display/disk/powernap off",
            crate::schema::OsKind::Linux => "logind: ignore idle and lid (systemd)",
        },
        "off in role",
    ));

    ApplyPlan {
        host: host.name.clone(),
        role: host.role.clone(),
        os: os.as_str().into(),
        shell: shell.as_str().into(),
        user,
        package_sets,
        steps,
    }
}

fn feature_step(
    id: &str,
    enabled: bool,
    on: impl Into<String>,
    off: impl Into<String>,
) -> ApplyStep {
    ApplyStep {
        id: id.into(),
        detail: if enabled { on.into() } else { off.into() },
        skip: !enabled,
    }
}

#[allow(dead_code)]
pub fn shell_rc_name(shell: ShellKind) -> &'static str {
    match shell {
        ShellKind::Zsh => ".zshrc",
        ShellKind::Bash => ".bashrc",
    }
}
