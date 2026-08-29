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
        }
        None => println!("  apply    never  (rig apply --yes)"),
    }

    let ssh = dirs_home().join(".ssh/config.d/rig.conf");
    if ssh.is_file() {
        println!("  ssh      {}", ssh.display());
    } else {
        println!("  ssh      not generated  (rig ssh-config --write)");
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
