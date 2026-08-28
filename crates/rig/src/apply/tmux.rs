use crate::error::{Result, RigError};
use crate::paths;
use std::fs;
use std::path::{Path, PathBuf};

pub struct TmuxReport {
    pub detail: String,
    pub linked: Option<PathBuf>,
}

/// Symlink ~/.tmux.conf from overlay/tmux.conf (preferred) or templates/tmux/tmux.conf.
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

    if dst.is_symlink() {
        if let Ok(current) = fs::read_link(&dst) {
            if current == src {
                return Ok(TmuxReport {
                    detail: format!("already linked → {}", src.display()),
                    linked: Some(dst),
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
        detail: format!("{kind} → {}", dst.display()),
        linked: Some(dst),
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
