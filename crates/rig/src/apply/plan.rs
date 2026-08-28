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
        detail: format!("role={} schema={}", host.role, host.schema_version),
        skip: false,
    });
    steps.push(ApplyStep {
        id: "link-shell".into(),
        detail: format!(
            "templates/shell/{{common,{}}} -> ~/.{}rc",
            shell.as_str(),
            shell.as_str()
        ),
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
    steps.push(ApplyStep {
        id: "hostname".into(),
        detail: format!("set hostname to {}", host.name),
        skip: false,
    });

    let f = &role.features;
    steps.push(feature_step("gui", f.gui, "install / enable GUI apps"));
    steps.push(feature_step("cursor", f.cursor, "link Cursor user settings"));
    steps.push(feature_step(
        "remote-login",
        f.remote_login,
        "enable remote login / sshd",
    ));
    steps.push(feature_step("tailscale", f.tailscale, "configure Tailscale"));
    steps.push(feature_step(
        "thunderbolt",
        f.thunderbolt && host.thunderbolt.is_some(),
        match &host.thunderbolt {
            Some(ip) => format!("configure thunderbolt IP {ip}"),
            None => "no thunderbolt IP on host".into(),
        },
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

fn feature_step(id: &str, enabled: bool, detail: impl Into<String>) -> ApplyStep {
    ApplyStep {
        id: id.into(),
        detail: detail.into(),
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
