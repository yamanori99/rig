use crate::error::{Result, RigError};
use crate::packages;
use crate::paths;
use crate::schema::OsKind;
use std::path::Path;
use std::process::{Command, Stdio};

use super::features::StepReport;

/// Install GUI casks from role package Brewfiles (macOS). Linux is a soft no-op.
pub fn apply_gui(root: &Path, package_sets: &[String], os: OsKind) -> Result<StepReport> {
    match os {
        OsKind::Linux => Ok(StepReport {
            ok: true,
            detail: "skipped on linux (no brew casks)".into(),
        }),
        OsKind::Macos => install_casks(root, package_sets),
    }
}

fn install_casks(root: &Path, package_sets: &[String]) -> Result<StepReport> {
    if which("brew").is_none() {
        return Ok(StepReport {
            ok: true,
            detail: "brew not found — skip GUI casks".into(),
        });
    }

    let mut casks = Vec::new();
    for set in package_sets {
        let contents = packages::load_package_set(root, set)?;
        for entry in contents.brew {
            if let Some(name) = entry.strip_prefix("cask:") {
                if !name.is_empty() {
                    casks.push(name.to_string());
                }
            }
        }
    }
    casks.sort();
    casks.dedup();
    if casks.is_empty() {
        return Ok(StepReport {
            ok: true,
            detail: "no casks in package sets".into(),
        });
    }

    // Prefer brew bundle --casks when a Brewfile exists for the set.
    let mut notes = Vec::new();
    let mut ok = true;
    for set in package_sets {
        let file = paths::packages_dir(root)
            .join("brew")
            .join(format!("{set}.Brewfile"));
        if !file.is_file() {
            continue;
        }
        let status = Command::new("brew")
            .args([
                "bundle",
                "--file",
                &file.display().to_string(),
                "--casks",
                "--no-upgrade",
            ])
            .env("HOMEBREW_NO_ENV_HINTS", "1")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(RigError::Io)?;
        if status.success() {
            notes.push(format!("{set}: casks ok"));
        } else {
            // Fallback: install each cask individually (partial Brewfiles / old brew).
            let mut set_ok = true;
            for cask in &casks {
                let st = Command::new("brew")
                    .args(["install", "--cask", cask])
                    .env("HOMEBREW_NO_ENV_HINTS", "1")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .map_err(RigError::Io)?;
                if !st.success() {
                    set_ok = false;
                }
            }
            if set_ok {
                notes.push(format!("{set}: casks ok (install)"));
            } else {
                notes.push(format!("{set}: some casks failed"));
                ok = false;
            }
        }
    }

    if notes.is_empty() {
        notes.push(format!("casks: {}", casks.join(" ")));
    }

    Ok(StepReport {
        ok,
        detail: notes.join("; "),
    })
}

fn which(bin: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        for dir in std::env::split_paths(&paths) {
            let p = dir.join(bin);
            if p.is_file() {
                return Some(p);
            }
        }
        None
    })
}
