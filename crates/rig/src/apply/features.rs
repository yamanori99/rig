use crate::error::{Result, RigError};
use crate::schema::OsKind;
use std::process::{Command, Stdio};

pub struct StepReport {
    pub ok: bool,
    pub detail: String,
}

/// Enable SSH remote login / ensure sshd is available.
pub fn apply_remote_login(os: OsKind) -> Result<StepReport> {
    match os {
        OsKind::Macos => enable_remote_login_macos(),
        OsKind::Linux => enable_remote_login_linux(),
    }
}

/// Assign a Thunderbolt Bridge (`bridge0`) IPv4 on macOS and persist via LaunchDaemon.
///
/// `ip` is this machine's address (from [[ssh]] with link=thunderbolt). Linux is a no-op.
pub fn apply_thunderbolt(ip: &str, os: OsKind) -> Result<StepReport> {
    validate_ipv4(ip)?;
    match os {
        OsKind::Macos => set_thunderbolt_macos(ip),
        OsKind::Linux => Ok(StepReport {
            ok: true,
            detail: "skipped on linux (macOS bridge0 only)".into(),
        }),
    }
}

/// Ensure Tailscale daemon is configured; enable Tailscale SSH when already logged in.
///
/// Does not run interactive `tailscale up` (needs browser / auth key). Soft-ok when
/// the binary is missing or the node is not yet connected.
pub fn apply_tailscale(os: OsKind) -> Result<StepReport> {
    let ts = which("tailscale");
    let Some(ts) = ts else {
        return Ok(StepReport {
            ok: true,
            detail: "tailscale not installed — install via packages, then re-apply".into(),
        });
    };

    let mut notes = Vec::new();
    match os {
        OsKind::Macos => {
            if let Some(msg) = ensure_tailscaled_macos()? {
                notes.push(msg);
            }
        }
        OsKind::Linux => {
            if let Some(msg) = ensure_tailscaled_linux()? {
                notes.push(msg);
            }
        }
    }

    // Connected?
    let status = Command::new(&ts)
        .arg("status")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(RigError::Io)?;
    if !status.success() {
        notes.push("not connected — run: sudo tailscale up --ssh".into());
        return Ok(StepReport {
            ok: true,
            detail: notes.join("; "),
        });
    }

    let ip = Command::new(&ts)
        .args(["ip", "-4"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());

    // Enable Tailscale SSH if possible (best-effort).
    let set = Command::new(&ts)
        .args(["set", "--ssh=true", "--accept-risk=lose-ssh"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match set {
        Ok(s) if s.success() => notes.push("tailscale ssh on".into()),
        _ => notes.push("tailscale ssh set skipped".into()),
    }

    if let Some(ip) = ip {
        notes.insert(0, format!("connected {ip}"));
    } else {
        notes.insert(0, "connected".into());
    }

    Ok(StepReport {
        ok: true,
        detail: notes.join("; "),
    })
}

fn ensure_tailscaled_macos() -> Result<Option<String>> {
    let daemon = which("tailscaled")
        .or_else(|| {
            for p in [
                "/opt/homebrew/bin/tailscaled",
                "/usr/local/bin/tailscaled",
            ] {
                let path = std::path::PathBuf::from(p);
                if path.is_file() {
                    return Some(path);
                }
            }
            None
        });
    let Some(daemon) = daemon else {
        return Ok(Some("tailscaled binary not found".into()));
    };

    let label = "dev.rig.tailscaled";
    let path = format!("/Library/LaunchDaemons/{label}.plist");
    let desired = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
"#,
        daemon.display()
    );

    let existing = std::fs::read_to_string(&path).ok();
    if existing.as_deref() != Some(desired.as_str()) {
        let mut child = Command::new("sudo")
            .args(["tee", &path])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(RigError::Io)?;
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin.write_all(desired.as_bytes()).map_err(RigError::Io)?;
        }
        let status = child.wait().map_err(RigError::Io)?;
        if !status.success() {
            return Ok(Some(format!("launchdaemon write failed ({status})")));
        }
        let _ = sudo(&["chmod", "644", &path]);
        let _ = sudo(&["launchctl", "bootout", "system", &path]);
        let _ = sudo(&["launchctl", "bootstrap", "system", &path]);
        let _ = sudo(&["launchctl", "unload", &path]);
        let _ = sudo(&["launchctl", "load", &path]);
        // Prefer system LaunchDaemon over brew user service.
        let _ = Command::new("brew")
            .args(["services", "stop", "tailscale"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }

    if !pgrep("tailscaled") {
        let _ = sudo(&["launchctl", "load", &path]);
        let _ = sudo(&["launchctl", "kickstart", "-k", &format!("system/{label}")]);
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    Ok(Some(if pgrep("tailscaled") {
        "tailscaled running".into()
    } else {
        "tailscaled not running yet".into()
    }))
}

fn ensure_tailscaled_linux() -> Result<Option<String>> {
    for unit in ["tailscaled", "tailscale"] {
        if sudo(&["systemctl", "enable", "--now", unit])? {
            return Ok(Some(format!("systemctl enable --now {unit}")));
        }
    }
    if pgrep("tailscaled") {
        return Ok(Some("tailscaled already running".into()));
    }
    Ok(Some(
        "no systemd unit — install Tailscale from https://tailscale.com/download".into(),
    ))
}

fn pgrep(name: &str) -> bool {
    Command::new("pgrep")
        .args(["-x", name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn validate_ipv4(ip: &str) -> Result<()> {
    let parts: Vec<_> = ip.split('.').collect();
    if parts.len() != 4 {
        return Err(RigError::Msg(format!("thunderbolt IP must be IPv4: {ip}")));
    }
    for p in parts {
        if p.parse::<u8>().is_err() {
            return Err(RigError::Msg(format!("thunderbolt IP must be IPv4: {ip}")));
        }
    }
    Ok(())
}

fn set_thunderbolt_macos(ip: &str) -> Result<StepReport> {
    let bridge_out = Command::new("ifconfig")
        .arg("bridge0")
        .output()
        .map_err(RigError::Io)?;
    if !bridge_out.status.success() {
        return Ok(StepReport {
            ok: true,
            detail: "no bridge0 — connect Thunderbolt and re-apply".into(),
        });
    }
    let text = String::from_utf8_lossy(&bridge_out.stdout);
    let current = text
        .lines()
        .find_map(|l| {
            let t = l.trim();
            if t.starts_with("inet ") {
                t.split_whitespace().nth(1).map(str::to_string)
            } else {
                None
            }
        });

    let mut notes = Vec::new();
    if current.as_deref() == Some(ip) {
        notes.push(format!("bridge0 already {ip}"));
    } else {
        if !sudo(&[
            "ifconfig",
            "bridge0",
            "inet",
            ip,
            "netmask",
            "255.255.255.0",
            "up",
        ])? {
            return Ok(StepReport {
                ok: false,
                detail: format!("ifconfig bridge0 inet {ip} failed"),
            });
        }
        notes.push(format!("bridge0 → {ip}/24"));
    }

    match ensure_thunderbolt_launchdaemon(ip)? {
        Some(msg) => notes.push(msg),
        None => notes.push("launchdaemon ok".into()),
    }

    Ok(StepReport {
        ok: true,
        detail: notes.join("; "),
    })
}

const TB_PLIST_LABEL: &str = "dev.rig.thunderbolt-bridge";

fn ensure_thunderbolt_launchdaemon(ip: &str) -> Result<Option<String>> {
    let path = format!("/Library/LaunchDaemons/{TB_PLIST_LABEL}.plist");
    let desired = thunderbolt_plist(ip);
    let existing = std::fs::read_to_string(&path).ok();
    if existing.as_deref() == Some(desired.as_str()) {
        return Ok(None);
    }

    // Write via sudo tee
    let mut child = Command::new("sudo")
        .args(["tee", &path])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(RigError::Io)?;
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin.write_all(desired.as_bytes()).map_err(RigError::Io)?;
    }
    let status = child.wait().map_err(RigError::Io)?;
    if !status.success() {
        return Ok(Some(format!("launchdaemon write failed ({status})")));
    }
    let _ = sudo(&["chmod", "644", &path]);
    // Reload best-effort (macOS variants differ on bootout/bootstrap).
    let _ = sudo(&["launchctl", "bootout", "system", &path]);
    let _ = sudo(&["launchctl", "bootstrap", "system", &path]);
    let _ = sudo(&["launchctl", "enable", &format!("system/{TB_PLIST_LABEL}")]);
    let _ = sudo(&["launchctl", "kickstart", "-k", &format!("system/{TB_PLIST_LABEL}")]);
    // Older fallback
    let _ = sudo(&["launchctl", "unload", &path]);
    let _ = sudo(&["launchctl", "load", &path]);
    Ok(Some("launchdaemon installed".into()))
}

fn thunderbolt_plist(ip: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{TB_PLIST_LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>/sbin/ifconfig</string>
        <string>bridge0</string>
        <string>inet</string>
        <string>{ip}</string>
        <string>netmask</string>
        <string>255.255.255.0</string>
        <string>up</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>StartInterval</key>
    <integer>60</integer>
</dict>
</plist>
"#
    )
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
