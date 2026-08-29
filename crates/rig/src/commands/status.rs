use crate::apply;
use crate::embed;
use crate::error::Result;
use crate::paths;
use crate::schema;
use std::path::Path;

pub fn run(root: &Path) -> Result<()> {
    let hosts = schema::load_hosts(root)?;
    let hn = schema::current_hostname();
    let short = hn.split('.').next().unwrap_or(&hn);
    let detected = schema::detect_current_host(&hosts);

    println!("rig status");
    println!(
        "  version  {}  ({})",
        env!("CARGO_PKG_VERSION"),
        running_exe()
    );
    println!("  os       {}", schema::detect_os().as_str());
    println!("  root     {}", root.display());
    println!("           {}", root_kind(root));

    match detected {
        Some(h) => {
            println!("  host     {}  role={}  (matched {})", h.name, h.role, hn);
            if let Ok(role) = schema::load_role(root, &h.role) {
                let f = role.features.with_host(&h.features);
                println!(
                    "  features gui={} cursor={} remote_login={} tailscale={} thunderbolt={} stay_awake={}",
                    yn(f.gui),
                    yn(f.cursor),
                    yn(f.remote_login),
                    yn(f.tailscale),
                    yn(f.thunderbolt),
                    yn(f.stay_awake)
                );
            }
        }
        None => {
            println!("  host     {short}  (no hosts/{short}.toml — rig init)");
        }
    }

    let peer_paths: usize = hosts
        .iter()
        .filter(|(_, h)| detected.map(|d| h.name != d.name).unwrap_or(true))
        .map(|(_, h)| h.ssh_paths().len())
        .sum();
    println!(
        "  hosts    {} file(s), {peer_paths} peer path(s)  (rig check)",
        hosts.len()
    );

    match apply::load_state()? {
        Some(st) => {
            println!(
                "  apply    {} / {}  ({} files, {} sets)",
                st.host,
                st.role,
                st.managed_files.len(),
                st.package_sets.len()
            );
            println!("           {}", paths::state_path().display());
            if !st.steps.is_empty() {
                println!("  apply steps");
                for (id, detail) in &st.steps {
                    println!("    {id:<14} {detail}");
                }
            }
            if !st.managed_files.is_empty() {
                println!("  managed files");
                for p in &st.managed_files {
                    dump_managed(p);
                }
            }
        }
        None => println!("  apply    never  (rig apply --yes)"),
    }

    apply::print_live(schema::detect_os());

    let ssh = dirs_home().join(".ssh/config.d/rig.conf");
    if ssh.is_file() {
        println!("  ssh      {}", ssh.display());
        if let Ok(s) = std::fs::read_to_string(&ssh) {
            for line in s.lines() {
                println!("    {line}");
            }
        }
    } else {
        println!("  ssh      not generated  (rig ssh-config --yes)");
    }

    let overlay = root.join("overlay");
    let overlay_note = if overlay.is_dir() && overlay_has_files(&overlay) {
        "present"
    } else {
        "empty"
    };
    println!("  overlay  {overlay_note}  {}", overlay.display());
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

const MANAGED_DUMP_MAX: u64 = 8 * 1024;

fn dump_managed(path_s: &str) {
    let path = Path::new(path_s);
    println!("    {path_s}");
    let meta = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(_) => {
            println!("      (missing)");
            return;
        }
    };
    if meta.len() > MANAGED_DUMP_MAX {
        println!("      (skipped, {} bytes)", meta.len());
        return;
    }
    match std::fs::read_to_string(path) {
        Ok(s) if !s.trim().is_empty() => {
            for line in s.lines() {
                println!("      {line}");
            }
        }
        Ok(_) => println!("      (empty)"),
        Err(_) => println!("      (unreadable)"),
    }
}

fn yn(v: bool) -> &'static str {
    if v {
        "on"
    } else {
        "off"
    }
}
