use crate::error::{Result, RigError};
use crate::paths;
use crate::schema::OsKind;
use std::fs;
use std::path::{Path, PathBuf};

use super::features::StepReport;

const FILES: &[&str] = &["settings.json", "keybindings.json"];

/// Symlink Cursor User settings from overlay (preferred) or templates.
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
        match link_one(&src, &dst)? {
            LinkOutcome::Already => notes.push(format!("{name} already linked")),
            LinkOutcome::Linked { backed_up } => {
                if backed_up {
                    notes.push(format!("{name} linked (backed up previous)"));
                } else {
                    notes.push(format!("{name} linked"));
                }
                linked.push(dst);
            }
        }
    }

    if !any_src {
        return Ok((
            StepReport {
                ok: true,
                detail: "no cursor templates — add templates/cursor/User or overlay/cursor/User"
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
    let tpl = paths::templates_dir(root).join("cursor/User").join(name);
    if tpl.is_file() {
        return Some((tpl, false));
    }
    None
}

fn cursor_user_dir(os: OsKind) -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| RigError::Msg("HOME is not set".into()))?;
    Ok(match os {
        OsKind::Macos => home.join("Library/Application Support/Cursor/User"),
        OsKind::Linux => home.join(".config/Cursor/User"),
    })
}

enum LinkOutcome {
    Already,
    Linked { backed_up: bool },
}

fn link_one(src: &Path, dst: &Path) -> Result<LinkOutcome> {
    if dst.is_symlink() {
        if let Ok(current) = fs::read_link(dst) {
            if current == src {
                return Ok(LinkOutcome::Already);
            }
        }
        fs::remove_file(dst).map_err(RigError::Io)?;
        std::os::unix::fs::symlink(src, dst).map_err(RigError::Io)?;
        return Ok(LinkOutcome::Linked { backed_up: false });
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
    Ok(LinkOutcome::Linked { backed_up })
}

fn epoch_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
