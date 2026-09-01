use crate::error::{Result, RigError};
use crate::schema::{OsKind, ShellKind};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::features::StepReport;

/// Set the account login shell to match the role (Homebrew bash/zsh on macOS).
pub fn apply_login_shell(shell: ShellKind, os: OsKind, user: &str) -> Result<StepReport> {
    let Some(wanted) = wanted_shell_path(shell, os) else {
        return Ok(StepReport {
            ok: false,
            detail: match (shell, os) {
                (ShellKind::Bash, OsKind::Macos) => {
                    "Homebrew bash not found (brew install bash)".into()
                }
                (ShellKind::Zsh, OsKind::Macos) => {
                    "Homebrew zsh not found (brew install zsh)".into()
                }
                (ShellKind::Zsh, OsKind::Linux) => "zsh not found on PATH".into(),
                (ShellKind::Bash, OsKind::Linux) => "bash not found on PATH".into(),
            },
        });
    };

    if is_apple_stock_bash(&wanted) {
        return Ok(StepReport {
            ok: false,
            detail: "refusing /bin/bash (macOS ships 3.2); install Homebrew bash".into(),
        });
    }

    let wanted_s = wanted.display().to_string();
    if let Some(cur) = current_login_shell(user, os) {
        if same_shell(&cur, &wanted) {
            return Ok(StepReport {
                ok: true,
                detail: format!("already {wanted_s}"),
            });
        }
    }

    if !shells_file_contains(&wanted)? {
        if !ensure_sudo_ticket()? {
            return Ok(StepReport {
                ok: false,
                detail: "sudo -v failed (needed to add shell to /etc/shells)".into(),
            });
        }
        if !append_etc_shells(&wanted)? {
            return Ok(StepReport {
                ok: false,
                detail: format!("could not add {wanted_s} to /etc/shells"),
            });
        }
    }

    if !chsh_to(&wanted, user)? {
        return Ok(StepReport {
            ok: false,
            detail: format!("chsh -s {wanted_s} failed"),
        });
    }

    Ok(StepReport {
        ok: true,
        detail: format!("chsh → {wanted_s}"),
    })
}

pub fn plan_detail(shell: ShellKind, os: OsKind) -> String {
    match (shell, os) {
        (ShellKind::Bash, OsKind::Macos) => {
            "chsh to Homebrew bash (not /bin/bash 3.2)".into()
        }
        (ShellKind::Zsh, OsKind::Macos) => "chsh to Homebrew zsh (not /bin/zsh)".into(),
        (ShellKind::Bash, OsKind::Linux) => "chsh to bash".into(),
        (ShellKind::Zsh, OsKind::Linux) => "chsh to zsh".into(),
    }
}

fn wanted_shell_path(shell: ShellKind, os: OsKind) -> Option<PathBuf> {
    match (shell, os) {
        (ShellKind::Bash, OsKind::Macos) => brew_formula_bin("bash"),
        (ShellKind::Zsh, OsKind::Macos) => brew_formula_bin("zsh"),
        (ShellKind::Bash, OsKind::Linux) => which("bash").or_else(|| {
            let p = PathBuf::from("/usr/bin/bash");
            p.is_file().then_some(p)
        }),
        (ShellKind::Zsh, OsKind::Linux) => which("zsh").or_else(|| {
            let p = PathBuf::from("/usr/bin/zsh");
            p.is_file().then_some(p)
        }),
    }
}

fn brew_formula_bin(formula: &str) -> Option<PathBuf> {
    if let Some(brew) = which("brew") {
        if let Ok(out) = Command::new(brew)
            .args(["--prefix", formula])
            .stdin(Stdio::null())
            .output()
        {
            if out.status.success() {
                let prefix = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !prefix.is_empty() {
                    let p = PathBuf::from(prefix).join("bin").join(formula);
                    if p.is_file() {
                        return Some(p);
                    }
                }
            }
        }
    }
    for p in [
        PathBuf::from("/opt/homebrew/bin").join(formula),
        PathBuf::from("/usr/local/bin").join(formula),
    ] {
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

fn is_apple_stock_bash(path: &Path) -> bool {
    path == Path::new("/bin/bash")
}

fn current_login_shell(user: &str, os: OsKind) -> Option<PathBuf> {
    match os {
        OsKind::Macos => {
            let out = Command::new("dscl")
                .args([".", "-read", &format!("/Users/{user}"), "UserShell"])
                .stdin(Stdio::null())
                .output()
                .ok()?;
            if !out.status.success() {
                return None;
            }
            let text = String::from_utf8_lossy(&out.stdout);
            // "UserShell: /bin/zsh"
            text.split_whitespace().nth(1).map(PathBuf::from)
        }
        OsKind::Linux => {
            let out = Command::new("getent")
                .args(["passwd", user])
                .stdin(Stdio::null())
                .output()
                .ok()?;
            if !out.status.success() {
                return None;
            }
            let line = String::from_utf8_lossy(&out.stdout);
            line.trim().rsplit(':').next().map(|s| PathBuf::from(s.trim()))
        }
    }
}

fn same_shell(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

fn shells_file_contains(path: &Path) -> Result<bool> {
    let text = match fs::read_to_string("/etc/shells") {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(RigError::Io(e)),
    };
    let want = path.to_string_lossy();
    Ok(text.lines().any(|l| {
        let t = l.trim();
        !t.starts_with('#') && t == want
    }))
}

fn append_etc_shells(path: &Path) -> Result<bool> {
    let line = format!("{}\n", path.display());
    let status = crate::ui::sudo_command()
        .args(["tee", "-a", "/etc/shells"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .and_then(|mut child| {
            if let Some(stdin) = child.stdin.as_mut() {
                use std::io::Write;
                stdin.write_all(line.as_bytes())?;
            }
            child.wait()
        })
        .map_err(RigError::Io)?;
    Ok(status.success())
}

fn chsh_to(path: &Path, user: &str) -> Result<bool> {
    let path_s = path.to_string_lossy();
    let status = Command::new("chsh")
        .args(["-s", path_s.as_ref(), user])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(RigError::Io)?;
    if status.success() {
        return Ok(true);
    }
    let status = crate::ui::sudo_command()
        .args(["chsh", "-s", path_s.as_ref(), user])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(RigError::Io)?;
    Ok(status.success())
}

fn ensure_sudo_ticket() -> Result<bool> {
    let status = crate::ui::sudo_command()
        .arg("-v")
        .status()
        .map_err(RigError::Io)?;
    Ok(status.success())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apple_stock_bash_is_rejected() {
        assert!(is_apple_stock_bash(Path::new("/bin/bash")));
        assert!(!is_apple_stock_bash(Path::new("/opt/homebrew/bin/bash")));
    }

    #[test]
    fn macos_bash_plan_mentions_homebrew() {
        let d = plan_detail(ShellKind::Bash, OsKind::Macos);
        assert!(d.contains("Homebrew"));
        assert!(d.contains("/bin/bash"));
    }

    #[test]
    fn macos_zsh_plan_mentions_homebrew() {
        let d = plan_detail(ShellKind::Zsh, OsKind::Macos);
        assert!(d.contains("Homebrew"));
        assert!(d.contains("/bin/zsh"));
    }
}
