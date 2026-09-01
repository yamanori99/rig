use crate::apply;
use crate::embed;
use crate::error::Result;
use crate::paths;
use crate::schema;
use crate::ui;
use std::collections::HashSet;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn run(root: &Path) -> Result<()> {
    let hosts = schema::load_hosts(root)?;
    let hn = schema::current_hostname();
    let short = hn.split('.').next().unwrap_or(&hn);
    let detected = schema::detect_current_host(&hosts);

    ui::title("status", false);
    ui::kv(
        "version",
        format!("{}  ({})", env!("CARGO_PKG_VERSION"), running_exe()),
    );
    if !print_machine(root) {
        ui::kv("os", schema::detect_os().as_str());
    }
    ui::kv("root", root.display());
    ui::kvc(root_kind(root));

    let mut live = apply::LiveWanted::default();
    match detected {
        Some(h) => {
            ui::kv("host", format!("{}  matched {}", h.name, hn));
            ui::kv("role", &h.role);
            if let Some(detail) = h.user_write_needed() {
                ui::note("user", detail);
            }
            if let Ok(role) = schema::load_role(root, &h.role) {
                let f = role.features.with_host(&h.features);
                ui::kv("want", feature_summary(&f));
                let plan = apply::build_plan(h, &role);
                apply::print_package_extras(root, &plan.package_sets, schema::detect_os())?;
                live = apply::LiveWanted {
                    stay_awake: f.stay_awake,
                    remote_login: f.remote_login,
                    screen_sharing: f.screen_sharing,
                    thunderbolt: f.thunderbolt && h.thunderbolt_ip().is_some(),
                    tailscale: f.tailscale,
                    cursor: f.cursor,
                };
            }
        }
        None => {
            ui::kv(
                "host",
                format!("{short}  no ~/.rig-hosts/{short}.toml — rig init"),
            );
        }
    }

    let peer_paths: usize = hosts
        .iter()
        .filter(|(_, h)| detected.map(|d| h.name != d.name).unwrap_or(true))
        .map(|(_, h)| h.ssh_paths().len())
        .sum();
    ui::kv(
        "hosts",
        format!(
            "{} file(s), {peer_paths} peer path(s)  (rig host check)",
            hosts.len()
        ),
    );

    match apply::load_state()? {
        Some(st) => {
            ui::kv(
                "apply",
                format!(
                    "{} / {}  {} files, {} sets",
                    st.host,
                    st.role,
                    st.managed_files.len(),
                    st.package_sets.len()
                ),
            );
            ui::kvc(paths::state_path().display());
            if !st.steps.is_empty() {
                let ids: Vec<&str> = st.steps.keys().map(String::as_str).collect();
                ui::kv("steps", ids.join(" "));
            }
            // Paths are in state.json; listing them here just ellipsizes.
        }
        None => ui::kv("apply", "never  (rig apply --yes)"),
    }

    apply::print_live(schema::detect_os(), live);

    let ssh = dirs_home().join(".ssh/config.d/rig.conf");
    if ssh.is_file() {
        ui::kv("ssh", ssh.display());
        if let Ok(s) = std::fs::read_to_string(&ssh) {
            let skip = self_aliases(detected);
            let aliases: Vec<String> = apply::host_aliases(&s)
                .into_iter()
                .filter(|a| !skip.contains(a))
                .collect();
            ui::kvc(format!("{} peer alias(es)", aliases.len()));
            for a in aliases {
                ui::note("alias", a);
            }
        }
    } else {
        ui::kv("ssh", "not generated  (rig host ssh-config -y)");
    }

    let overlay = root.join("overlay");
    let overlay_note = if overlay.is_dir() && overlay_has_files(&overlay) {
        "present"
    } else {
        "empty"
    };
    ui::kv("overlay", format!("{overlay_note}  {}", overlay.display()));
    Ok(())
}

/// Banner fields via fastfetch, no ASCII logo. False if fastfetch is missing.
fn print_machine(root: &Path) -> bool {
    let cfg = root.join("templates/shell/fastfetch/config.jsonc");
    if !cfg.is_file() {
        return false;
    }
    let Ok(out) = Command::new("fastfetch")
        .arg("-c")
        .arg(&cfg)
        .args(["-l", "none", "--pipe", "true"])
        .stdin(Stdio::null())
        .output()
    else {
        return false;
    };
    if !out.status.success() {
        return false;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut any = false;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if !any {
            ui::section("machine");
            any = true;
        }
        if let Some((k, v)) = line.split_once(':') {
            let v = v.trim();
            if v.is_empty() || k.eq_ignore_ascii_case("shell") {
                // fastfetch sees this process (`rig`), not the login shell
                continue;
            }
            ui::kv(machine_key(k.trim()), v);
        } else {
            ui::kv("who", line);
        }
    }
    if any {
        ui::kv("shell", login_shell());
    }
    any
}

fn login_shell() -> String {
    let path = std::env::var("SHELL").unwrap_or_else(|_| "?".into());
    let name = Path::new(&path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(path.as_str());
    let Ok(out) = Command::new(&path)
        .arg("--version")
        .stdin(Stdio::null())
        .output()
    else {
        return name.to_string();
    };
    let first = String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    if first.is_empty() {
        return name.to_string();
    }
    // `zsh 5.9 (arm-apple-darwin…)` → `zsh 5.9`
    first
        .split_whitespace()
        .take(2)
        .collect::<Vec<_>>()
        .join(" ")
}

fn machine_key(k: &str) -> &'static str {
    match k.to_ascii_lowercase().as_str() {
        "os" => "system",
        "host" => "model",
        "kernel" => "kernel",
        "uptime" => "uptime",
        "memory" => "memory",
        "shell" => "shell",
        _ => "info",
    }
}

fn running_exe() -> String {
    std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "?".into())
}

fn root_kind(root: &Path) -> &'static str {
    if root == embed::product_data_root() {
        "product data (embedded unpack)"
    } else {
        "checkout / --root"
    }
}

fn dirs_home() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

fn overlay_has_files(dir: &Path) -> bool {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return false;
    };
    rd.flatten().any(|e| {
        let name = e.file_name();
        name != ".gitkeep" && name != ".DS_Store"
    })
}

fn feature_summary(f: &schema::RoleFeatures) -> String {
    let pairs = [
        ("gui", f.gui),
        ("cursor", f.cursor),
        ("remote", f.remote_login),
        ("screen", f.screen_sharing),
        ("tailscale", f.tailscale),
        ("thunderbolt", f.thunderbolt),
        ("awake", f.stay_awake),
    ];
    let on: Vec<&str> = pairs.iter().filter(|(_, v)| *v).map(|(k, _)| *k).collect();
    let off: Vec<&str> = pairs.iter().filter(|(_, v)| !*v).map(|(k, _)| *k).collect();
    if off.is_empty() {
        on.join(" ")
    } else if on.is_empty() {
        format!("off {}", off.join(" "))
    } else {
        format!("{}  off {}", on.join(" "), off.join(" "))
    }
}

fn self_aliases(detected: Option<&schema::Host>) -> HashSet<String> {
    let Some(h) = detected else {
        return HashSet::new();
    };
    let mut s: HashSet<String> = h.ssh_paths().into_iter().map(|p| p.alias).collect();
    s.insert(h.name.clone());
    s
}
