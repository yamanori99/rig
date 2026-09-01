use crate::embed;
use crate::error::{Result, RigError};
use std::fs;
use std::path::{Path, PathBuf};

pub fn discover_root(explicit: Option<PathBuf>) -> miette::Result<PathBuf> {
    if let Some(p) = explicit {
        if looks_like_root(&p) {
            return ready(p);
        }
        return Err(RigError::Msg(format!(
            "path does not look like a rig root: {}",
            p.display()
        ))
        .into());
    }

    if let Ok(dir) = std::env::current_dir() {
        let mut dir = dir;
        loop {
            if looks_like_root(&dir) {
                return ready(dir);
            }
            if !dir.pop() {
                break;
            }
        }
    }

    // Dev: executable sitting inside a checkout (target/debug/rig → repo root).
    if let Ok(exe) = std::env::current_exe() {
        if let Some(root) = exe
            .ancestors()
            .find(|p| looks_like_root(p))
            .map(|p| p.to_path_buf())
        {
            return ready(root);
        }
    }

    // Standalone binary: materialize embedded roles/packages/templates.
    ready(embed::ensure_embedded_root()?)
}

fn ready(root: PathBuf) -> miette::Result<PathBuf> {
    let root = refresh_if_embedded(root);
    ensure_hosts_inventory(&root)?;
    Ok(root)
}

/// Application Support unpack is pinned as RIG_ROOT after apply. Refresh it
/// when the binary version is newer so `rig update` actually ships templates.
fn refresh_if_embedded(root: PathBuf) -> PathBuf {
    if root == embed::product_data_root() {
        if let Ok(fresh) = embed::ensure_embedded_root() {
            return fresh;
        }
    }
    root
}

fn looks_like_root(p: &Path) -> bool {
    let roles = p.join("roles").is_dir();
    let packages = p.join("packages").is_dir();
    let crates = p.join("crates").is_dir();
    roles && (packages || crates)
}

/// Remind where product data lives. stderr so piped stdout (e.g. ssh-config) stays clean.
pub fn eprint_data_hint(root: &Path) {
    let os = crate::schema::detect_os().as_str();
    crate::ui::data_hint(root, os);
}

/// Machine inventory (`~/.rig-hosts/*.toml`). Samples stay in `hosts/examples/`.
pub fn hosts_dir(_root: &Path) -> PathBuf {
    home_dir().join(".rig-hosts")
}

pub fn home_dir() -> PathBuf {
    directories::BaseDirs::new()
        .map(|b| b.home_dir().to_path_buf())
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn hosts_examples_dir(root: &Path) -> PathBuf {
    root.join("hosts").join("examples")
}

/// Move a legacy product `hosts/` symlink or loose tomls into `~/.rig-hosts`.
pub fn ensure_hosts_inventory(root: &Path) -> Result<PathBuf> {
    let dest = hosts_dir(root);
    migrate_legacy_hosts(root, &dest)?;
    Ok(dest)
}

fn migrate_legacy_hosts(root: &Path, dest: &Path) -> Result<()> {
    if dest.exists() {
        return Ok(());
    }
    let legacy = root.join("hosts");
    let meta = match fs::symlink_metadata(&legacy) {
        Ok(m) => m,
        Err(_) => return Ok(()),
    };
    if meta.file_type().is_symlink() {
        fs::rename(&legacy, dest).map_err(RigError::Io)?;
        fs::create_dir_all(&legacy).map_err(RigError::Io)?;
        return Ok(());
    }
    if !meta.is_dir() {
        return Ok(());
    }
    let mut moved = false;
    for entry in fs::read_dir(&legacy).map_err(RigError::Io)? {
        let entry = entry.map_err(RigError::Io)?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("toml") {
            continue;
        }
        if !moved {
            fs::create_dir_all(dest).map_err(RigError::Io)?;
            moved = true;
        }
        fs::rename(&path, dest.join(entry.file_name())).map_err(RigError::Io)?;
    }
    Ok(())
}

pub fn roles_dir(root: &Path) -> PathBuf {
    root.join("roles")
}

pub fn packages_dir(root: &Path) -> PathBuf {
    root.join("packages")
}

pub fn templates_dir(root: &Path) -> PathBuf {
    root.join("templates")
}

pub fn state_path() -> PathBuf {
    directories::ProjectDirs::from("dev", "rig", "rig")
        .map(|d| d.data_local_dir().join("state.json"))
        .unwrap_or_else(|| PathBuf::from(".rig-state.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static N: AtomicU64 = AtomicU64::new(0);

    fn scratch() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "rig-hosts-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn hosts_dir_is_home_dot_rig_hosts() {
        let p = hosts_dir(Path::new("/ignored-product-root"));
        assert_eq!(p.file_name().unwrap(), ".rig-hosts");
        assert_eq!(p.parent().unwrap(), home_dir().as_path());
    }

    #[test]
    fn moves_loose_tomls_out_of_hosts() {
        let root = scratch();
        let dest = root.join("inv");
        let hosts = root.join("hosts");
        fs::create_dir_all(hosts.join("examples")).unwrap();
        fs::write(hosts.join("mini.toml"), "x").unwrap();
        fs::write(hosts.join("examples/keep.toml"), "ex").unwrap();
        migrate_legacy_hosts(&root, &dest).unwrap();
        assert!(dest.join("mini.toml").is_file());
        assert!(hosts.join("examples/keep.toml").is_file());
        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn moves_hosts_symlink_to_home_inventory() {
        let root = scratch();
        let dest = root.join("inv");
        let real = root.join("inventory");
        fs::create_dir_all(&real).unwrap();
        fs::write(real.join("mini.toml"), "x").unwrap();
        std::os::unix::fs::symlink(&real, root.join("hosts")).unwrap();
        migrate_legacy_hosts(&root, &dest).unwrap();
        assert!(dest.symlink_metadata().unwrap().is_symlink());
        assert!(root.join("hosts").is_dir());
        assert!(!root.join("hosts").symlink_metadata().unwrap().is_symlink());
        assert!(dest.join("mini.toml").is_file());
        let _ = fs::remove_dir_all(&root);
    }
}
