use crate::error::{Result, RigError};
use crate::paths;
use crate::schema::OsKind;
use std::fs;
use std::path::{Path, PathBuf};

use super::features::StepReport;

const FILES: &[&str] = &["settings.json", "keybindings.json"];

/// Install Cursor User settings from overlay (symlink) or templates (copy from seed).
pub fn apply_cursor(root: &Path, os: OsKind) -> Result<(StepReport, Vec<PathBuf>)> {
    let dest_dir = cursor_user_dir(os)?;
    fs::create_dir_all(&dest_dir).map_err(RigError::Io)?;

    let mut notes = Vec::new();
    let mut linked = Vec::new();
    let mut any_src = false;
    let mut used_overlay = false;
    let mut used_templates = false;

    for name in FILES {
        let (src, from_overlay) = match resolve_src(root, name) {
            Some(v) => v,
            None => {
                notes.push(format!("skip {name} (no template/overlay)"));
                continue;
            }
        };
        any_src = true;
        if from_overlay {
            used_overlay = true;
        } else {
            used_templates = true;
        }
        let dst = dest_dir.join(name);
        match install_one(&src, &dst, from_overlay)? {
            InstallOutcome::Already => notes.push(format!("{name} already in place")),
            InstallOutcome::Installed { backed_up, mode } => {
                if backed_up {
                    notes.push(format!("{name} {mode} (backed up previous)"));
                } else {
                    notes.push(format!("{name} {mode}"));
                }
                linked.push(dst);
            }
        }
    }

    if !any_src {
        return Ok((
            StepReport {
                ok: true,
                detail: "no cursor templates — add *.example under templates/cursor/User or overlay/cursor/User"
                    .into(),
            },
            linked,
        ));
    }

    let src_kind = match (used_overlay, used_templates) {
        (true, true) => "overlay+templates",
        (true, false) => "overlay",
        _ => "templates",
    };
    notes.insert(0, format!("{src_kind} → {}", dest_dir.display()));
    Ok((
        StepReport {
            ok: true,
            detail: notes.join("; "),
        },
        linked,
    ))
}

fn resolve_src(root: &Path, name: &str) -> Option<(PathBuf, bool)> {
    let overlay = root.join("overlay/cursor/User").join(name);
    if overlay.is_file() {
        return Some((overlay, true));
    }
    let tpl_dir = paths::templates_dir(root).join("cursor/User");
    let live = tpl_dir.join(name);
    if live.is_file() {
        return Some((live, false));
    }
    let example = tpl_dir.join(format!("{name}.example"));
    if example.is_file() {
        return Some((example, false));
    }
    None
}

pub(crate) fn cursor_user_dir(os: OsKind) -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| RigError::Msg("HOME is not set".into()))?;
    Ok(match os {
        OsKind::Macos => home.join("Library/Application Support/Cursor/User"),
        OsKind::Linux => home.join(".config/Cursor/User"),
    })
}

enum InstallOutcome {
    Already,
    Installed { backed_up: bool, mode: &'static str },
}

fn install_one(src: &Path, dst: &Path, symlink: bool) -> Result<InstallOutcome> {
    if symlink {
        return link_one(src, dst);
    }
    // Product templates: copy so Cursor edits stay out of the repo tree.
    if dst.is_file() && !dst.is_symlink() {
        return Ok(InstallOutcome::Already);
    }
    let mut backed_up = false;
    if dst.is_symlink() || dst.is_file() {
        let bak = PathBuf::from(format!("{}.bak.{}", dst.display(), epoch_secs()));
        fs::rename(dst, &bak).map_err(RigError::Io)?;
        backed_up = true;
    } else if dst.exists() {
        return Err(RigError::Msg(format!(
            "refusing to replace non-file Cursor path: {}",
            dst.display()
        )));
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(RigError::Io)?;
    }
    fs::copy(src, dst).map_err(RigError::Io)?;
    Ok(InstallOutcome::Installed {
        backed_up,
        mode: "copied",
    })
}

fn link_one(src: &Path, dst: &Path) -> Result<InstallOutcome> {
    if dst.is_symlink() {
        if let Ok(current) = fs::read_link(dst) {
            if current == src {
                return Ok(InstallOutcome::Already);
            }
        }
        fs::remove_file(dst).map_err(RigError::Io)?;
        std::os::unix::fs::symlink(src, dst).map_err(RigError::Io)?;
        return Ok(InstallOutcome::Installed {
            backed_up: false,
            mode: "linked",
        });
    }

    let mut backed_up = false;
    if dst.is_file() {
        let bak = PathBuf::from(format!("{}.bak.{}", dst.display(), epoch_secs()));
        fs::rename(dst, &bak).map_err(RigError::Io)?;
        backed_up = true;
    } else if dst.exists() {
        return Err(RigError::Msg(format!(
            "refusing to replace non-file Cursor path: {}",
            dst.display()
        )));
    }

    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(RigError::Io)?;
    }
    std::os::unix::fs::symlink(src, dst).map_err(RigError::Io)?;
    Ok(InstallOutcome::Installed {
        backed_up,
        mode: "linked",
    })
}

fn epoch_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
