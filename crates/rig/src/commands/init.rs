use crate::error::RigError;
use crate::paths;
use crate::schema;
use crate::ui;
use miette::Result;
use std::fs;

pub fn run(root: &std::path::Path, role: &str, name: Option<&str>) -> Result<()> {
    let roles = schema::list_roles(root)?;
    if !roles.iter().any(|r| r == role) {
        return Err(RigError::Msg(format!(
            "unknown role `{role}` (have: {})",
            roles.join(", ")
        ))
        .into());
    }

    let host_name = name
        .map(|s| s.to_string())
        .unwrap_or_else(schema::current_hostname);
    let short = host_name
        .split('.')
        .next()
        .unwrap_or(&host_name)
        .to_string();

    let dest = paths::hosts_dir(root).join(format!("{short}.toml"));
    if dest.symlink_metadata().is_ok() {
        return Err(RigError::Msg(format!(
            "already exists: {} (file or symlink). skip init; `rig apply` uses this path. \
             If apply cannot see the host, product/hosts is not your inventory — \
             symlink it to the clone (e.g. ~/rig-hosts)",
            dest.display()
        ))
        .into());
    }

    let example = paths::hosts_dir(root)
        .join("examples")
        .join(format!("{role}.toml"));
    let mut body = if example.exists() {
        fs::read_to_string(&example).map_err(RigError::Io)?
    } else {
        default_host_toml(role)
    };

    // rewrite name = "..."
    body = rewrite_name(&body, &short);
    // Never pin OS from examples — detect at apply unless the user sets it later.
    body = strip_os_assignment(&body);

    fs::create_dir_all(paths::hosts_dir(root)).map_err(RigError::Io)?;
    fs::write(&dest, body).map_err(RigError::Io)?;
    let overlay = root.join("overlay");
    ui::title("init", false);
    ui::kv("wrote", dest.display());
    ui::blank();
    ui::section("edit");
    ui::kv("host", dest.display());
    ui::kvc("role / [[ssh]] / [packages] / [features]");
    ui::kv("overlay", format!("{}/", overlay.display()));
    ui::kvc("personal shell / tmux / cursor overrides");
    ui::note("templates", "leave alone — use overlay/");
    ui::blank();
    ui::kv(
        "detect",
        "os/shell at apply (override in the toml if needed)",
    );
    ui::next("rig apply");
    Ok(())
}

fn rewrite_name(toml: &str, name: &str) -> String {
    let mut out = Vec::new();
    let mut done = false;
    for line in toml.lines() {
        if !done && line.trim_start().starts_with("name") {
            out.push(format!("name = \"{name}\""));
            done = true;
        } else {
            out.push(line.to_string());
        }
    }
    if !done {
        out.insert(0, format!("name = \"{name}\""));
    }
    out.join("\n") + "\n"
}

/// Drop active `os = "..."` so apply uses runtime detection.
fn strip_os_assignment(toml: &str) -> String {
    let mut out = Vec::new();
    let mut inserted_hint = false;
    for line in toml.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("os") && trimmed.contains('=') && !trimmed.starts_with('#') {
            if !inserted_hint {
                out.push("# os omitted → auto-detect at apply".to_string());
                inserted_hint = true;
            }
            continue;
        }
        out.push(line.to_string());
    }
    out.join("\n") + "\n"
}

fn default_host_toml(role: &str) -> String {
    format!(
        r#"# Local host file (gitignored). Edit name and [[ssh]] by hand.
# name = inventory id; OS hostname is not changed by rig.
# [[ssh]] alias = SSH Host name you choose; link = vpn|lan|thunderbolt
name = "change-me"
role = "{role}"
schema_version = 1
# os / shell omitted → auto-detect at apply
# user = "you"
# [[ssh]]
# alias = "change-me-lan"
# ip = "192.168.x.x"
# link = "lan"
"#
    )
}
