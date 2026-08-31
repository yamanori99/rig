use crate::error::{Result, RigError};
use crate::schema::OsKind;
use std::process::{Command, Stdio};

pub struct StepReport {
    pub ok: bool,
    pub detail: String,
}

/// Enable macOS Screen Sharing (VNC :5900). Linux is a no-op.
pub fn apply_screen_sharing(os: OsKind) -> Result<StepReport> {
    if !matches!(os, OsKind::Macos) {
        return Ok(StepReport {
            ok: true,
            detail: "skipped on linux (macOS only)".into(),
        });
    }
    if !ensure_sudo_ticket()? {
        return Ok(StepReport {
            ok: false,
            detail: "sudo -v failed (password not accepted or cancelled)".into(),
        });
    }
    enable_screen_sharing_macos()
}

/// Enable SSH remote login / ensure sshd is available.
pub fn apply_remote_login(os: OsKind) -> Result<StepReport> {
    if !ensure_sudo_ticket()? {
        return Ok(StepReport {
            ok: false,
            detail: "sudo -v failed (password not accepted or cancelled)".into(),
        });
    }
    let report = match os {
        OsKind::Macos => enable_remote_login_macos()?,
        OsKind::Linux => enable_remote_login_linux()?,
    };
    if !report.ok {
        return Ok(report);
    }
    let keepalive = ensure_sshd_client_alive()?;
    Ok(StepReport {
        ok: true,
        detail: match keepalive {
            Some(msg) => format!("{}; {}", report.detail, msg),
            None => report.detail,
        },
    })
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
            detail: "tailscale not installed — skipped".into(),
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

pub(crate) const SSHD_KEEPALIVE_PATH: &str = "/etc/ssh/sshd_config.d/99-rig-keepalive.conf";
pub(crate) const LOGIND_DROPIN_PATH: &str = "/etc/systemd/logind.conf.d/99-rig-stay-awake.conf";
pub(crate) const LOGIND_DESIRED: &str = "\
# managed by rig — compute stay-awake
[Login]
IdleAction=ignore
HandleLidSwitch=ignore
HandleLidSwitchExternalPower=ignore
HandleLidSwitchDocked=ignore
";

/// Prevent idle sleep so a headless node stays reachable.
pub fn apply_stay_awake(os: OsKind) -> Result<StepReport> {
    match os {
        OsKind::Macos => apply_stay_awake_macos(),
        OsKind::Linux => apply_stay_awake_linux(),
    }
}

pub(crate) fn has_systemd() -> bool {
    std::path::Path::new("/run/systemd/system").is_dir()
}

fn apply_stay_awake_macos() -> Result<StepReport> {
    if pmset_all_sources_awake() {
        return Ok(StepReport {
            ok: true,
            detail: "pmset -a already sleep=0 displaysleep=0 disksleep=0 powernap=0".into(),
        });
    }
    if sudo(&[
        "pmset",
        "-a",
        "sleep",
        "0",
        "displaysleep",
        "0",
        "disksleep",
        "0",
        "powernap",
        "0",
    ])? {
        Ok(StepReport {
            ok: true,
            detail: "pmset -a sleep=0 displaysleep=0 disksleep=0 powernap=0".into(),
        })
    } else {
        Ok(StepReport {
            ok: false,
            detail: "pmset -a failed".into(),
        })
    }
}

/// True when every source in `pmset -g custom` has sleep/display/disk off
/// (powernap too, when that key exists).
fn pmset_all_sources_awake() -> bool {
    let Ok(out) = Command::new("pmset").args(["-g", "custom"]).output() else {
        return false;
    };
    if !out.status.success() {
        return false;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    pmset_custom_all_awake(&text)
}

pub(crate) fn pmset_custom_all_awake(custom: &str) -> bool {
    let sources = pmset_custom_sources(custom);
    !sources.is_empty() && sources.iter().all(|(_, body)| pmset_stanza_awake(body))
}

pub(crate) fn pmset_custom_sources(custom: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut name = String::new();
    let mut body = String::new();
    let flush = |name: &mut String, body: &mut String, out: &mut Vec<(String, String)>| {
        if !name.is_empty() {
            out.push((name.clone(), body.clone()));
        }
        name.clear();
        body.clear();
    };
    for line in custom.lines() {
        let t = line.trim();
        let header = t
            .strip_suffix(':')
            .and_then(|h| h.strip_suffix(" Power"))
            .filter(|h| matches!(*h, "Battery" | "AC" | "UPS"));
        if let Some(h) = header {
            flush(&mut name, &mut body, &mut out);
            name = h.to_string();
            continue;
        }
        if !name.is_empty() {
            body.push_str(line);
            body.push('\n');
        }
    }
    flush(&mut name, &mut body, &mut out);
    out
}

fn pmset_stanza_awake(body: &str) -> bool {
    let mut sleep = None;
    let mut display = None;
    let mut disk = None;
    let mut nap = None;
    for line in body.lines() {
        let t = line.trim();
        let mut parts = t.split_whitespace();
        let key = parts.next().unwrap_or("");
        let val = parts.next().unwrap_or("");
        match key {
            "sleep" => sleep = Some(val == "0"),
            "displaysleep" => display = Some(val == "0"),
            "disksleep" => disk = Some(val == "0"),
            "powernap" => nap = Some(val == "0"),
            _ => {}
        }
    }
    sleep == Some(true) && display == Some(true) && disk == Some(true) && nap.unwrap_or(true)
}

fn apply_stay_awake_linux() -> Result<StepReport> {
    if !has_systemd() {
        return Ok(StepReport {
            ok: true,
            detail: "no systemd — skip stay-awake".into(),
        });
    }

    let existing = std::fs::read_to_string(LOGIND_DROPIN_PATH).unwrap_or_default();
    if existing == LOGIND_DESIRED {
        return Ok(StepReport {
            ok: true,
            detail: format!("{LOGIND_DROPIN_PATH} already"),
        });
    }

    let dir = "/etc/systemd/logind.conf.d";
    if !sudo(&["mkdir", "-p", dir])? {
        return Ok(StepReport {
            ok: false,
            detail: format!("mkdir {dir} failed"),
        });
    }
    if !sudo_write(LOGIND_DROPIN_PATH, LOGIND_DESIRED)? {
        return Ok(StepReport {
            ok: false,
            detail: format!("write {LOGIND_DROPIN_PATH} failed"),
        });
    }
    let _ = sudo(&["chmod", "644", LOGIND_DROPIN_PATH]);
    let restarted = sudo(&["systemctl", "restart", "systemd-logind"])?;
    Ok(StepReport {
        ok: true,
        detail: if restarted {
            format!("{LOGIND_DROPIN_PATH} (logind restarted)")
        } else {
            format!("{LOGIND_DROPIN_PATH} (wrote; restart logind skipped)")
        },
    })
}

fn sudo_write(path: &str, contents: &str) -> Result<bool> {
    let mut child = crate::ui::sudo_command()
        .args(["tee", path])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(RigError::Io)?;
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin.write_all(contents.as_bytes()).map_err(RigError::Io)?;
    }
    let status = child.wait().map_err(RigError::Io)?;
    Ok(status.success())
}

fn ensure_tailscaled_macos() -> Result<Option<String>> {
    let daemon = which("tailscaled").or_else(|| {
        for p in ["/opt/homebrew/bin/tailscaled", "/usr/local/bin/tailscaled"] {
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
        let mut child = crate::ui::sudo_command()
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
    let current = text.lines().find_map(|l| {
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
    let mut child = crate::ui::sudo_command()
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
    let _ = sudo(&[
        "launchctl",
        "kickstart",
        "-k",
        &format!("system/{TB_PLIST_LABEL}"),
    ]);
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

const SSH_PLIST: &str = "/System/Library/LaunchDaemons/ssh.plist";
const SSH_SERVICE: &str = "system/com.openssh.sshd";
const SCREEN_SHARING_PLIST: &str = "/System/Library/LaunchDaemons/com.apple.screensharing.plist";
const SCREEN_SHARING_SERVICE: &str = "system/com.apple.screensharing";

fn enable_screen_sharing_macos() -> Result<StepReport> {
    if screen_sharing_is_on() {
        return Ok(StepReport {
            ok: true,
            detail: "already On".into(),
        });
    }

    let launch = bootstrap_screen_sharing()?;
    std::thread::sleep(std::time::Duration::from_millis(500));
    if screen_sharing_is_on() {
        return Ok(StepReport {
            ok: true,
            detail: "launchctl bootstrap com.apple.screensharing".into(),
        });
    }

    let mut detail = "Screen Sharing still off".to_string();
    let launch = launch.trim();
    if !launch.is_empty() {
        detail.push_str(": ");
        detail.push_str(launch);
    }
    detail.push_str(
        ". System Settings > General > Sharing > Screen Sharing, or grant Terminal Full Disk Access",
    );
    Ok(StepReport { ok: false, detail })
}

fn bootstrap_screen_sharing() -> Result<String> {
    let mut notes = Vec::new();
    let (en_ok, en_out) = sudo_output(&["launchctl", "enable", SCREEN_SHARING_SERVICE])?;
    if !en_ok {
        notes.push(format!("enable: {}", en_out.trim()));
    }
    let (boot_ok, boot_out) =
        sudo_output(&["launchctl", "bootstrap", "system", SCREEN_SHARING_PLIST])?;
    let boot = boot_out.to_ascii_lowercase();
    if !boot_ok && !boot.contains("already") && !boot.contains("in progress") {
        notes.push(format!("bootstrap: {}", boot_out.trim()));
        let (load_ok, load_out) = sudo_output(&["launchctl", "load", "-w", SCREEN_SHARING_PLIST])?;
        if !load_ok {
            notes.push(format!("load -w: {}", load_out.trim()));
        }
    }
    let (kick_ok, kick_out) =
        sudo_output(&["launchctl", "kickstart", "-k", SCREEN_SHARING_SERVICE])?;
    if !kick_ok {
        notes.push(format!("kickstart: {}", kick_out.trim()));
    }
    Ok(notes.join("; "))
}

fn screen_sharing_is_on() -> bool {
    vnc_listening()
}

pub fn vnc_listening() -> bool {
    let Ok(addr) = "127.0.0.1:5900".parse() else {
        return false;
    };
    std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(400)).is_ok()
}

fn enable_remote_login_macos() -> Result<StepReport> {
    if remote_login_is_on()? {
        return Ok(StepReport {
            ok: true,
            detail: "already On".into(),
        });
    }

    // Without -f, systemsetup asks yes/no on a hidden stdout; the password
    // the user just typed is consumed as that answer and the step fails.
    // On current macOS it also needs Full Disk Access; then we bootstrap sshd.
    let (ok, out) = sudo_output(&["/usr/sbin/systemsetup", "-f", "-setremotelogin", "on"])?;
    if ok && remote_login_is_on()? {
        return Ok(StepReport {
            ok: true,
            detail: "systemsetup -f -setremotelogin on".into(),
        });
    }

    let launch = bootstrap_sshd()?;
    std::thread::sleep(std::time::Duration::from_millis(500));
    if remote_login_is_on()? {
        return Ok(StepReport {
            ok: true,
            detail: "launchctl bootstrap com.openssh.sshd (systemsetup needs Full Disk Access)"
                .into(),
        });
    }

    let mut detail = "remote login still off".to_string();
    let extra = out.trim();
    if !extra.is_empty() {
        detail.push_str(": ");
        detail.push_str(extra);
    }
    let launch = launch.trim();
    if !launch.is_empty() {
        detail.push_str("; ");
        detail.push_str(launch);
    }
    detail.push_str(
        ". System Settings > General > Sharing > Remote Login, or grant Terminal Full Disk Access",
    );
    Ok(StepReport { ok: false, detail })
}

/// Enable Apple sshd without systemsetup (no Full Disk Access).
fn bootstrap_sshd() -> Result<String> {
    let mut notes = Vec::new();
    let (en_ok, en_out) = sudo_output(&["launchctl", "enable", SSH_SERVICE])?;
    if !en_ok {
        notes.push(format!("enable: {}", en_out.trim()));
    }
    let (boot_ok, boot_out) = sudo_output(&["launchctl", "bootstrap", "system", SSH_PLIST])?;
    let boot = boot_out.to_ascii_lowercase();
    if !boot_ok && !boot.contains("already") && !boot.contains("in progress") {
        notes.push(format!("bootstrap: {}", boot_out.trim()));
        let (load_ok, load_out) = sudo_output(&["launchctl", "load", "-w", SSH_PLIST])?;
        if !load_ok {
            notes.push(format!("load -w: {}", load_out.trim()));
        }
    }
    let (kick_ok, kick_out) = sudo_output(&["launchctl", "kickstart", "-k", SSH_SERVICE])?;
    if !kick_ok {
        notes.push(format!("kickstart: {}", kick_out.trim()));
    }
    Ok(notes.join("; "))
}

fn remote_login_is_on() -> Result<bool> {
    if ssh_port_open() {
        return Ok(true);
    }
    let (ok, text) = sudo_output(&["/usr/sbin/systemsetup", "-getremotelogin"])?;
    let lower = text.to_ascii_lowercase();
    // Do not match the word "on" inside "on or off requires Full Disk Access".
    Ok(ok && lower.contains("remote login: on"))
}

fn ssh_port_open() -> bool {
    let Ok(addr) = "127.0.0.1:22".parse() else {
        return false;
    };
    std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(400)).is_ok()
}

/// Drop-in so idle SSH sessions survive NAT / flaky paths (pairs with client ServerAlive*).
fn ensure_sshd_client_alive() -> Result<Option<String>> {
    let dir = std::path::Path::new("/etc/ssh/sshd_config.d");
    if !dir.is_dir() {
        return Ok(None);
    }
    let path = std::path::Path::new(SSHD_KEEPALIVE_PATH);
    let desired = "\
# managed by rig — client keepalive companion
ClientAliveInterval 30
ClientAliveCountMax 6
";
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    if existing == desired {
        return Ok(None);
    }

    let mut child = crate::ui::sudo_command()
        .args(["tee", SSHD_KEEPALIVE_PATH])
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
        return Ok(Some(format!("sshd keepalive write failed ({status})")));
    }

    // Best-effort reload (macOS / linux / containers differ).
    let _ = sudo(&["launchctl", "kickstart", "-k", "system/com.openssh.sshd"]);
    let _ = sudo(&["systemctl", "reload", "ssh"]);
    let _ = sudo(&["systemctl", "reload", "sshd"]);
    let _ = sudo(&["service", "ssh", "reload"]);
    Ok(Some("sshd ClientAliveInterval=30".into()))
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
        let status = crate::ui::sudo_command()
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

fn ensure_sudo_ticket() -> Result<bool> {
    let status = crate::ui::sudo_command()
        .arg("-v")
        .status()
        .map_err(RigError::Io)?;
    Ok(status.success())
}

fn sudo(args: &[&str]) -> Result<bool> {
    let status = crate::ui::sudo_command()
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(RigError::Io)?;
    Ok(status.success())
}

fn sudo_output(args: &[&str]) -> Result<(bool, String)> {
    let out = crate::ui::sudo_command()
        .args(args)
        .output()
        .map_err(RigError::Io)?;
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    let err = String::from_utf8_lossy(&out.stderr);
    if !err.is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&err);
    }
    Ok((out.status.success(), text))
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

#[cfg(test)]
mod tests {
    use super::*;

    const CUSTOM: &str = "\
Battery Power:
 sleep                1
 displaysleep         10
 disksleep            10
 powernap             1
AC Power:
 sleep                0
 displaysleep         0
 disksleep            0
 powernap             0
";

    #[test]
    fn custom_sources_splits_battery_and_ac() {
        let s = pmset_custom_sources(CUSTOM);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].0, "Battery");
        assert_eq!(s[1].0, "AC");
    }

    #[test]
    fn custom_awake_false_if_battery_still_sleeps() {
        assert!(!pmset_custom_all_awake(CUSTOM));
    }

    #[test]
    fn custom_awake_true_when_every_source_is_zero() {
        let t = "\
AC Power:
 sleep                0
 displaysleep         0
 disksleep            0
 powernap             0
";
        assert!(pmset_custom_all_awake(t));
    }
}
