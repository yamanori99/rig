use crate::error::{Result, RigError};
use rust_embed::Embed;
use std::fs;
use std::path::{Path, PathBuf};

/// Product roles shipped inside the binary.
#[derive(Embed)]
#[folder = "$CARGO_MANIFEST_DIR/../../roles"]
struct EmbeddedRoles;

/// Package lists / Brewfiles.
#[derive(Embed)]
#[folder = "$CARGO_MANIFEST_DIR/../../packages"]
struct EmbeddedPackages;

/// Shell / tmux / ssh / cursor templates.
#[derive(Embed)]
#[folder = "$CARGO_MANIFEST_DIR/../../templates"]
struct EmbeddedTemplates;

/// Host file examples (`hosts/examples`).
#[derive(Embed)]
#[folder = "$CARGO_MANIFEST_DIR/../../hosts/examples"]
struct EmbeddedHostExamples;

const PRODUCT_VERSION_FILE: &str = ".rig-product-version";

/// Writable product tree: roles, packages, templates, hosts/examples.
/// User inventory (`~/.rig-hosts/*.toml`) and `overlay/` are preserved
/// across upgrades.
pub fn product_data_root() -> PathBuf {
    directories::ProjectDirs::from("dev", "rig", "rig")
        .map(|d| d.data_local_dir().join("product"))
        .unwrap_or_else(|| PathBuf::from(".rig-product"))
}

/// Ensure embedded product assets are on disk; return that root.
pub fn ensure_embedded_root() -> Result<PathBuf> {
    let root = product_data_root();
    let stamp = root.join(PRODUCT_VERSION_FILE);
    let version = env!("CARGO_PKG_VERSION");
    let fresh = match fs::read_to_string(&stamp) {
        Ok(v) => v.trim() != version,
        Err(_) => true,
    };
    let incomplete = !(root.join("roles").is_dir() && root.join("packages").is_dir());

    if fresh || incomplete {
        extract_tree::<EmbeddedRoles>(&root.join("roles"))?;
        extract_tree::<EmbeddedPackages>(&root.join("packages"))?;
        extract_tree::<EmbeddedTemplates>(&root.join("templates"))?;
        extract_tree::<EmbeddedHostExamples>(&root.join("hosts").join("examples"))?;
        fs::create_dir_all(root.join("hosts")).map_err(RigError::Io)?;
        fs::create_dir_all(root.join("overlay")).map_err(RigError::Io)?;
        // Keep overlay/.gitkeep semantics for empty overlay
        let keep = root.join("overlay/.gitkeep");
        if !keep.exists() {
            let _ = fs::write(&keep, b"");
        }
        fs::write(&stamp, format!("{version}\n")).map_err(RigError::Io)?;
    } else {
        fs::create_dir_all(root.join("hosts")).map_err(RigError::Io)?;
        fs::create_dir_all(root.join("overlay")).map_err(RigError::Io)?;
    }

    Ok(root)
}

fn extract_tree<E: Embed>(dest: &Path) -> Result<()> {
    // Replace managed tree so upgrades pick up template changes.
    if dest.exists() {
        fs::remove_dir_all(dest).map_err(RigError::Io)?;
    }
    fs::create_dir_all(dest).map_err(RigError::Io)?;
    for name in E::iter() {
        let Some(file) = E::get(name.as_ref()) else {
            continue;
        };
        let path = dest.join(name.as_ref());
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(RigError::Io)?;
        }
        fs::write(&path, file.data.as_ref()).map_err(RigError::Io)?;
        #[cfg(unix)]
        if name.ends_with(".sh") {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o755));
        }
    }
    Ok(())
}
