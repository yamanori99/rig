use crate::apply;
use crate::embed;
use crate::error::Result;
use crate::paths;
use crate::schema;
use crate::ui;
use std::path::Path;

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
    ui::kv("os", schema::detect_os().as_str());
    ui::kv("root", root.display());
    ui::kvc(root_kind(root));

    let mut live = apply::LiveWanted::default();
    match detected {
        Some(h) => {
            ui::kv("host", format!("{}  matched {}", h.name, hn));
            ui::kv("role", &h.role);
            if let Ok(role) = schema::load_role(root, &h.role) {
                let f = role.features.with_host(&h.features);
                ui::kv(
                    "features",
                    format!(
                        "gui={}  cursor={}  remote_login={}  tailscale={}  thunderbolt={}  stay_awake={}",
                        yn(f.gui),
                        yn(f.cursor),
                        yn(f.remote_login),
                        yn(f.tailscale),
                        yn(f.thunderbolt),
                        yn(f.stay_awake)
                    ),
                );
                let plan = apply::build_plan(h, &role);
                apply::print_package_extras(root, &plan.package_sets, schema::detect_os())?;
                live = apply::LiveWanted {
                    stay_awake: f.stay_awake,
                    remote_login: f.remote_login,
                    thunderbolt: f.thunderbolt && h.thunderbolt_ip().is_some(),
                    tailscale: f.tailscale,
                    cursor: f.cursor,
                };
            }
        }
        None => {
            ui::kv("host", format!("{short}  no hosts/{short}.toml — rig init"));
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
            "{} file(s), {peer_paths} peer path(s)  (rig check)",
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
                ui::section("steps");
                for (id, detail) in &st.steps {
                    ui::item(format!("{id:<14} {detail}"));
                }
            }
            if !st.managed_files.is_empty() {
                ui::section("files");
                for p in &st.managed_files {
                    ui::item(p);
                }
            }
        }
        None => ui::kv("apply", "never  (rig apply --yes)"),
    }

    apply::print_live(schema::detect_os(), live);

    let ssh = dirs_home().join(".ssh/config.d/rig.conf");
    if ssh.is_file() {
        ui::kv("ssh", ssh.display());
        if let Ok(s) = std::fs::read_to_string(&ssh) {
            for line in s.lines() {
                ui::item(line);
            }
        }
    } else {
        ui::kv("ssh", "not generated  (rig ssh-config --yes)");
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

fn yn(v: bool) -> &'static str {
    if v {
        "on"
    } else {
        "off"
    }
}
