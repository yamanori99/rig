use crate::error::{Result, RigError};
use crate::schema::OsKind;
use std::process::{Command, Stdio};

pub struct StepReport {
    pub ok: bool,
    pub detail: String,
}

/// Set the machine hostname to `name` (idempotent when already matching).
pub fn apply_hostname(name: &str, os: OsKind) -> Result<StepReport> {
    validate_hostname(name)?;
    let current = current_short_hostname();
    if current.eq_ignore_ascii_case(name) {
        return Ok(StepReport {
            ok: true,
            detail: format!("already {name}"),
        });
    }

    match os {
        OsKind::Macos => set_hostname_macos(name),
        OsKind::Linux => set_hostname_linux(name),
    }
}

/// Enable SSH remote login / ensure sshd is available.
pub fn apply_remote_login(os: OsKind) -> Result<StepReport> {
    match os {
        OsKind::Macos => enable_remote_login_macos(),
        OsKind::Linux => enable_remote_login_linux(),
    }
}

fn validate_hostname(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 63 {
        return Err(RigError::Msg(format!(
            "hostname must be 1..=63 chars, got {:?}",
            name
        )));
    }
    let ok = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.');
    if !ok || name.starts_with('-') || name.ends_with('-') || name.starts_with('.') {
        return Err(RigError::Msg(format!("invalid hostname: {name}")));
    }
    Ok(())
}

fn current_short_hostname() -> String {
    crate::schema::current_hostname()
        .split('.')
        .next()
        .unwrap_or("")
        .to_string()
}

fn set_hostname_macos(name: &str) -> Result<StepReport> {
    // LocalHostName cannot contain dots.
    let local = name.split('.').next().unwrap_or(name);
    for (key, value) in [
        ("ComputerName", name),
        ("LocalHostName", local),
        ("HostName", name),
    ] {
        let status = sudo(&["scutil", "--set", key, value])?;
        if !status {
            return Ok(StepReport {
                ok: false,
                detail: format!("scutil --set {key} failed"),
            });
        }
    }
    Ok(StepReport {
        ok: true,
        detail: format!("scutil → {name}"),
    })
}

fn set_hostname_linux(name: &str) -> Result<StepReport> {
    if which("hostnamectl").is_some() {
        if sudo(&["hostnamectl", "set-hostname", name])? {
            return Ok(StepReport {
                ok: true,
                detail: format!("hostnamectl → {name}"),
            });
        }
    }

    // Containers / minimal images: write /etc/hostname + runtime hostname.
    let write = Command::new("sudo")
        .args(["tee", "/etc/hostname"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match write {
        Ok(c) => c,
        Err(e) => {
            return Ok(StepReport {
                ok: false,
                detail: format!("sudo tee /etc/hostname: {e}"),
            });
        }
    };
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        if let Err(e) = stdin.write_all(format!("{name}\n").as_bytes()) {
            return Ok(StepReport {
                ok: false,
                detail: format!("write /etc/hostname: {e}"),
            });
        }
    }
    let status = child.wait().map_err(RigError::Io)?;
    if !status.success() {
        return Ok(StepReport {
            ok: false,
            detail: format!("tee /etc/hostname failed ({status})"),
        });
    }
    if !sudo(&["hostname", name])? {
        return Ok(StepReport {
            ok: false,
            detail: format!("hostname {name} failed"),
        });
    }
    Ok(StepReport {
        ok: true,
        detail: format!("/etc/hostname + hostname → {name}"),
    })
}

fn enable_remote_login_macos() -> Result<StepReport> {
    let out = Command::new("sudo")
        .args(["systemsetup", "-getremotelogin"])
        .output()
        .map_err(RigError::Io)?;
    let text = String::from_utf8_lossy(&out.stdout);
    if text.to_ascii_lowercase().contains("on") {
        return Ok(StepReport {
            ok: true,
            detail: "already On".into(),
        });
    }
    if sudo(&["systemsetup", "-setremotelogin", "on"])? {
        Ok(StepReport {
            ok: true,
            detail: "systemsetup -setremotelogin on".into(),
        })
    } else {
        Ok(StepReport {
            ok: false,
            detail: "systemsetup -setremotelogin on failed".into(),
        })
    }
}

fn enable_remote_login_linux() -> Result<StepReport> {
    if sshd_listening() {
        return Ok(StepReport {
            ok: true,
            detail: "sshd already listening".into(),
        });
    }

    for unit in ["ssh", "sshd"] {
        if sudo(&["systemctl", "enable", "--now", unit])? {
            return Ok(StepReport {
                ok: true,
                detail: format!("systemctl enable --now {unit}"),
            });
        }
    }

    // No systemd (Apple container smoke): start sshd in the background if present.
    if which("sshd").is_some() || std::path::Path::new("/usr/sbin/sshd").is_file() {
        let bin = which("sshd").unwrap_or_else(|| std::path::PathBuf::from("/usr/sbin/sshd"));
        let status = Command::new("sudo")
            .arg(&bin)
            .status()
            .map_err(RigError::Io)?;
        if status.success() || sshd_listening() {
            return Ok(StepReport {
                ok: true,
                detail: "started sshd".into(),
            });
        }
        return Ok(StepReport {
            ok: false,
            detail: format!("sshd start failed ({status})"),
        });
    }

    Ok(StepReport {
        ok: false,
        detail: "openssh-server / sshd not available".into(),
    })
}

fn sshd_listening() -> bool {
    // Cheap checks that work without systemd.
    if Command::new("pgrep")
        .args(["-x", "sshd"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return true;
    }
    std::fs::read_dir("/proc")
        .ok()
        .map(|entries| {
            entries.flatten().any(|e| {
                let name = e.file_name();
                let s = name.to_string_lossy();
                if !s.chars().all(|c| c.is_ascii_digit()) {
                    return false;
                }
                std::fs::read_to_string(e.path().join("comm"))
                    .map(|c| c.trim() == "sshd")
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn sudo(args: &[&str]) -> Result<bool> {
    let status = Command::new("sudo")
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(RigError::Io)?;
    Ok(status.success())
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
