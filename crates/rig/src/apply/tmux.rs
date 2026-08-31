use crate::error::{Result, RigError};
use crate::paths;
use std::fs;
use std::path::{Path, PathBuf};

pub struct TmuxReport {
    pub detail: String,
    pub linked: Option<PathBuf>,
    pub extra: Vec<PathBuf>,
}

/// Symlink ~/.tmux.conf and install status scripts under ~/.config/rig/tmux/scripts.
pub fn link_tmux(root: &Path) -> Result<TmuxReport> {
    let src = resolve_src(root).ok_or_else(|| {
        RigError::Msg(
            "missing tmux template (templates/tmux/tmux.conf or overlay/tmux.conf)".into(),
        )
    })?;

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| RigError::Msg("HOME is not set".into()))?;
    let dst = home.join(".tmux.conf");
    let mut extra = Vec::new();

    // Status scripts → ~/.config/rig/tmux/scripts
    let scripts_src = {
        let ov = root.join("overlay/tmux/scripts");
        let tpl = paths::templates_dir(root).join("tmux/scripts");
        if ov.is_dir() {
            Some(ov)
        } else if tpl.is_dir() {
            Some(tpl)
        } else {
            None
        }
    };
    if let Some(scripts_src) = scripts_src {
        let cfg = directories::ProjectDirs::from("dev", "rig", "rig")
            .map(|d| d.config_dir().join("tmux/scripts"))
            .unwrap_or_else(|| PathBuf::from(".rig-config/tmux/scripts"));
        fs::create_dir_all(&cfg).map_err(RigError::Io)?;
        for e in fs::read_dir(&scripts_src).map_err(RigError::Io)? {
            let e = e.map_err(RigError::Io)?;
            let src_f = e.path();
            if !src_f.is_file() {
                continue;
            }
            let dst_f = cfg.join(e.file_name());
            fs::copy(&src_f, &dst_f).map_err(RigError::Io)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&dst_f, fs::Permissions::from_mode(0o755));
            }
            extra.push(dst_f);
        }
    }

    if dst.is_symlink() {
        if let Ok(current) = fs::read_link(&dst) {
            if current == src {
                return Ok(TmuxReport {
                    detail: format!("already linked → {}", src.display()),
                    linked: Some(dst),
                    extra,
                });
            }
        }
        fs::remove_file(&dst).map_err(RigError::Io)?;
    } else if dst.is_file() {
        let bak = PathBuf::from(format!("{}.bak.{}", dst.display(), epoch_secs()));
        fs::rename(&dst, &bak).map_err(RigError::Io)?;
        std::os::unix::fs::symlink(&src, &dst).map_err(RigError::Io)?;
        return Ok(TmuxReport {
            detail: format!(
                "linked {} (backed up previous → {})",
                src.display(),
                bak.display()
            ),
            linked: Some(dst),
            extra,
        });
    } else if dst.exists() {
        return Err(RigError::Msg(format!(
            "refusing to replace non-file: {}",
            dst.display()
        )));
    }

    std::os::unix::fs::symlink(&src, &dst).map_err(RigError::Io)?;
    let kind = if src.starts_with(root.join("overlay")) {
        "overlay"
    } else {
        "templates"
    };
    Ok(TmuxReport {
        detail: format!("{kind} → {}  {} scripts", dst.display(), extra.len()),
        linked: Some(dst),
        extra,
    })
}

fn resolve_src(root: &Path) -> Option<PathBuf> {
    let overlay = root.join("overlay/tmux.conf");
    if overlay.is_file() {
        return Some(overlay);
    }
    let tpl = paths::templates_dir(root).join("tmux/tmux.conf");
    if tpl.is_file() {
        return Some(tpl);
    }
    None
}

fn epoch_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
