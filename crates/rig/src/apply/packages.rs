use crate::error::{Result, RigError};
use crate::packages;
use crate::paths;
use crate::schema::OsKind;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Homebrew often prints nothing until a formula finishes.
pub(crate) fn note_brew_may_look_stuck(doing: &str) {
    println!("  … {doing}");
    println!("    brew can sit with no output for minutes — still running");
    let _ = io::stdout().flush();
}

pub struct PackageReport {
    pub backend: &'static str,
    #[allow(dead_code)]
    pub sets: Vec<String>,
    pub ok: bool,
    pub detail: String,
}

pub fn apply_packages(root: &Path, sets: &[String], os: OsKind) -> Result<PackageReport> {
    match os {
        OsKind::Macos => brew_bundle(root, sets),
        OsKind::Linux => apt_install(root, sets),
    }
}

fn brew_bundle(root: &Path, sets: &[String]) -> Result<PackageReport> {
    if which("brew").is_none() {
        return Ok(PackageReport {
            backend: "brew",
            sets: sets.to_vec(),
            ok: false,
            detail: "brew not found on PATH".into(),
        });
    }

    let mut notes = Vec::new();
    let mut ok = true;
    for set in sets {
        let file = paths::packages_dir(root)
            .join("brew")
            .join(format!("{set}.Brewfile"));
        if !file.is_file() {
            notes.push(format!("missing {set}.Brewfile"));
            ok = false;
            continue;
        }
        note_brew_may_look_stuck(&format!("brew bundle {set}"));
        let status = Command::new("brew")
            .args([
                "bundle",
                "--file",
                &file.display().to_string(),
                "--no-upgrade",
                "--verbose",
            ])
            .env("HOMEBREW_NO_ENV_HINTS", "1")
            .env("HOMEBREW_NO_INSTALL_CLEANUP", "1")
            .status()
            .map_err(RigError::Io)?;
        if status.success() {
            notes.push(format!("{set}: ok"));
        } else {
            notes.push(format!("{set}: brew bundle failed ({status})"));
            ok = false;
        }
    }

    Ok(PackageReport {
        backend: "brew",
        sets: sets.to_vec(),
        ok,
        detail: notes.join("; "),
    })
}

fn apt_install(root: &Path, sets: &[String]) -> Result<PackageReport> {
    if which("apt-get").is_none() {
        return Ok(PackageReport {
            backend: "apt",
            sets: sets.to_vec(),
            ok: false,
            detail: "apt-get not found".into(),
        });
    }

    let mut pkgs = Vec::new();
    for set in sets {
        let contents = packages::load_package_set(root, set)?;
        pkgs.extend(contents.apt);
    }
    pkgs.sort();
    pkgs.dedup();
    if pkgs.is_empty() {
        return Ok(PackageReport {
            backend: "apt",
            sets: sets.to_vec(),
            ok: true,
            detail: "no packages".into(),
        });
    }

    let mut cmd = Command::new("sudo");
    cmd.args(["apt-get", "install", "-y"]);
    cmd.args(&pkgs);
    let status = cmd.status().map_err(RigError::Io)?;
    Ok(PackageReport {
        backend: "apt",
        sets: sets.to_vec(),
        ok: status.success(),
        detail: if status.success() {
            format!("installed {}", pkgs.join(" "))
        } else {
            format!("apt-get failed ({status})")
        },
    })
}

fn which(bin: &str) -> Option<PathBuf> {
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
