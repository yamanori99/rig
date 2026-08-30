use crate::error::Result;
use crate::packages;
use crate::schema::OsKind;
use std::collections::HashSet;
use std::path::Path;

use super::features::StepReport;
use super::packages as pkg;

/// Install GUI casks from role package Brewfiles (macOS). Linux is a soft no-op.
///
/// Do not use `brew bundle --casks` here: current Homebrew treats `--cask(s)` as
/// invalid on `bundle install` (`install` does not accept `--cask`).
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

    let have: HashSet<String> = pkg::brew_installed_casks().into_iter().collect();
    let mut ok = true;
    let mut installed = 0usize;
    let mut already = 0usize;
    for cask in &casks {
        if have.contains(cask) {
            crate::ui::item(format!("already  {cask}"));
            already += 1;
            continue;
        }
        pkg::brew_banner(&format!("cask  {cask}"));
        if pkg::run_brew(&["install", "--cask", cask])? {
            installed += 1;
        } else {
            ok = false;
        }
    }

    Ok(StepReport {
        ok,
        detail: format!(
            "{} cask(s): {already} already, {installed} installed",
            casks.len()
        ),
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
