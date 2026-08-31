use crate::apply::state::{self};
use crate::error::{Result, RigError};
use crate::packages;
use crate::paths;
use crate::schema::OsKind;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const BEGIN: &str = "# >>> begin rig >>>";
const END: &str = "# <<< end rig <<<";

pub struct CleanReport {
    pub lines: Vec<String>,
    pub errors: Vec<String>,
}

/// Reverse apply using the local state manifest.
pub fn execute(root: &Path, yes: bool, packages: bool) -> Result<CleanReport> {
    let preview = !yes;
    let Some(st) = state::load()? else {
        return Ok(CleanReport {
            lines: vec!["no state — nothing to clean".into()],
            errors: Vec::new(),
        });
    };

    let mut actions = Vec::new();
    let mut errors = Vec::new();

    for path_s in &st.managed_files {
        let path = PathBuf::from(path_s);
        match clean_managed_path(&path, preview) {
            Ok(Some(msg)) => actions.push(msg),
            Ok(None) => {}
            Err(e) => errors.push(format!("{}: {e}", path.display())),
        }
    }

    // Also strip rig blocks from common rc files even if not listed (older state).
    for name in [".bashrc", ".bash_profile", ".zshrc", ".zprofile"] {
        if let Some(home) = std::env::var_os("HOME") {
            let path = PathBuf::from(home).join(name);
            if path.is_file() {
                match strip_rig_block(&path, preview) {
                    Ok(true) => actions.push(format!("stripped block in {}", path.display())),
                    Ok(false) => {}
                    Err(e) => errors.push(format!("{}: {e}", path.display())),
                }
            }
        }
    }

    // Config dir written by link-shell
    if let Some(cfg) = dirs_config() {
        let shell_dir = cfg.join("shell");
        if shell_dir.is_dir() {
            if preview {
                actions.push(format!("would remove {}", shell_dir.display()));
            } else {
                match fs::remove_dir_all(&shell_dir) {
                    Ok(()) => actions.push(format!("removed {}", shell_dir.display())),
                    Err(e) => errors.push(format!("{}: {e}", shell_dir.display())),
                }
            }
        }
    }

    if packages && !st.package_sets.is_empty() {
        let os = detect_os_for_clean();
        match uninstall_packages(root, &st.package_sets, os, preview) {
            Ok(msgs) => actions.extend(msgs),
            Err(e) => errors.push(e.to_string()),
        }
    } else if packages {
        actions.push("no package sets recorded".into());
    }

    match clean_stay_awake(&st, preview) {
        Ok(Some(msg)) => actions.push(msg),
        Ok(None) => {}
        Err(e) => errors.push(format!("stay-awake: {e}")),
    }

    let state_path = paths::state_path();
    if preview {
        actions.push(format!("would remove state {}", state_path.display()));
    } else if state_path.is_file() {
        match fs::remove_file(&state_path) {
            Ok(()) => actions.push(format!("removed state {}", state_path.display())),
            Err(e) => errors.push(format!("state: {e}")),
        }
    }

    Ok(CleanReport {
        lines: actions,
        errors,
    })
}

fn stay_awake_was_applied(st: &state::RigState) -> bool {
    match st.steps.get("stay-awake") {
        Some(d) if d != "skipped" => true,
        _ => false,
    }
}

fn clean_stay_awake(st: &state::RigState, preview: bool) -> Result<Option<String>> {
    if !stay_awake_was_applied(st) {
        return Ok(None);
    }
    match detect_os_for_clean() {
        OsKind::Macos => Ok(Some("pmset left as-is (no recorded prior values)".into())),
        OsKind::Linux => {
            let path = super::features::LOGIND_DROPIN_PATH;
            if !std::path::Path::new(path).is_file() {
                return Ok(None);
            }
            if preview {
                return Ok(Some(format!("would remove {path}")));
            }
            let status = crate::ui::sudo_command()
                .args(["rm", "-f", path])
                .status()
                .map_err(RigError::Io)?;
            if !status.success() {
                return Err(RigError::Msg(format!("rm {path} failed ({status})")));
            }
            let _ = crate::ui::sudo_command()
                .args(["systemctl", "restart", "systemd-logind"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            Ok(Some(format!("removed {path}")))
        }
    }
}

fn clean_managed_path(path: &Path, preview: bool) -> Result<Option<String>> {
    if !path.exists() && !path.is_symlink() {
        return Ok(None);
    }

    // Shell rc: strip block instead of deleting the whole file.
    if is_shell_rc(path) {
        return match strip_rig_block(path, preview)? {
            true => Ok(Some(format!("stripped block in {}", path.display()))),
            false => Ok(None),
        };
    }

    if path.is_symlink() || path.is_file() {
        if preview {
            return Ok(Some(format!("would remove {}", path.display())));
        }
        fs::remove_file(path).map_err(RigError::Io)?;
        return Ok(Some(format!("removed {}", path.display())));
    }

    Ok(Some(format!("skip non-file {}", path.display())))
}

fn is_shell_rc(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|s| s.to_str()),
        Some(".bashrc" | ".bash_profile" | ".zshrc" | ".zprofile")
    )
}

fn strip_rig_block(path: &Path, preview: bool) -> Result<bool> {
    let existing = fs::read_to_string(path).map_err(RigError::Io)?;
    let Some(start) = existing.find(BEGIN) else {
        return Ok(false);
    };
    let Some(end) = existing.find(END) else {
        return Ok(false);
    };
    if end < start {
        return Ok(false);
    }
    let end_at = end + END.len();
    let mut out = String::new();
    out.push_str(&existing[..start]);
    let rest = existing[end_at..].trim_start_matches('\n');
    if !rest.is_empty() {
        out.push_str(rest);
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    // collapse triple blank lines
    while out.contains("\n\n\n") {
        out = out.replace("\n\n\n", "\n\n");
    }
    if preview {
        return Ok(true);
    }
    fs::write(path, out).map_err(RigError::Io)?;
    Ok(true)
}

fn uninstall_packages(
    root: &Path,
    sets: &[String],
    os: OsKind,
    preview: bool,
) -> Result<Vec<String>> {
    let mut msgs = Vec::new();
    match os {
        OsKind::Macos => {
            let mut formulas = Vec::new();
            for set in sets {
                let contents = packages::load_package_set(root, set)?;
                formulas.extend(contents.brew);
            }
            let (names, casks) = brew_formula_names(&formulas);
            if names.is_empty() && casks.is_empty() {
                msgs.push("brew: no formulas in recorded sets".into());
                return Ok(msgs);
            }
            if preview {
                if !names.is_empty() {
                    msgs.push(format!("would brew uninstall {}", names.join(" ")));
                }
                if !casks.is_empty() {
                    msgs.push(format!("would brew uninstall --cask {}", casks.join(" ")));
                }
                return Ok(msgs);
            }
            if !names.is_empty() {
                let status = Command::new("brew")
                    .arg("uninstall")
                    .args(&names)
                    .arg("--ignore-dependencies")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .map_err(RigError::Io)?;
                msgs.push(if status.success() {
                    format!("brew uninstall {}", names.join(" "))
                } else {
                    format!("brew uninstall partial/failed ({status})")
                });
            }
            if !casks.is_empty() {
                let status = Command::new("brew")
                    .args(["uninstall", "--cask"])
                    .args(&casks)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .map_err(RigError::Io)?;
                msgs.push(if status.success() {
                    format!("brew uninstall --cask {}", casks.join(" "))
                } else {
                    format!("brew cask uninstall partial/failed ({status})")
                });
            }
        }
        OsKind::Linux => {
            let mut pkgs = Vec::new();
            for set in sets {
                let contents = packages::load_package_set(root, set)?;
                pkgs.extend(contents.apt);
            }
            pkgs.sort();
            pkgs.dedup();
            if pkgs.is_empty() {
                msgs.push("apt: no packages in recorded sets".into());
                return Ok(msgs);
            }
            if preview {
                msgs.push(format!("would apt-get remove {}", pkgs.join(" ")));
                return Ok(msgs);
            }
            let status = crate::ui::sudo_command()
                .args(["apt-get", "remove", "-y"])
                .args(&pkgs)
                .status()
                .map_err(RigError::Io)?;
            msgs.push(if status.success() {
                format!("apt-get remove {}", pkgs.join(" "))
            } else {
                format!("apt-get remove failed ({status})")
            });
        }
    }
    Ok(msgs)
}

fn brew_formula_names(entries: &[String]) -> (Vec<String>, Vec<String>) {
    let mut formulas = Vec::new();
    let mut casks = Vec::new();
    for e in entries {
        if let Some(name) = e.strip_prefix("brew:") {
            if !name.is_empty() {
                formulas.push(name.to_string());
            }
        } else if let Some(name) = e.strip_prefix("cask:") {
            if !name.is_empty() {
                casks.push(name.to_string());
            }
        }
    }
    formulas.sort();
    formulas.dedup();
    casks.sort();
    casks.dedup();
    (formulas, casks)
}

fn dirs_config() -> Option<PathBuf> {
    directories::ProjectDirs::from("dev", "rig", "rig").map(|d| d.config_dir().to_path_buf())
}

fn detect_os_for_clean() -> OsKind {
    crate::schema::detect_os()
}
