use super::cursor;
use super::features::{self, LOGIND_DROPIN_PATH, SSHD_KEEPALIVE_PATH};
use crate::schema::OsKind;
use std::path::Path;
use std::process::{Command, Stdio};

/// Which live probes to run — only enabled role/host features.
#[derive(Clone, Copy, Default)]
pub struct LiveWanted {
    pub stay_awake: bool,
    pub remote_login: bool,
    pub thunderbolt: bool,
    pub tailscale: bool,
    pub cursor: bool,
}

impl LiveWanted {
    pub fn any(self) -> bool {
        self.stay_awake || self.remote_login || self.thunderbolt || self.tailscale || self.cursor
    }
}

/// Compact live settings for enabled features. No sudo.
pub fn print_live(os: OsKind, wanted: LiveWanted) {
    if !wanted.any() {
        return;
    }
    println!("  live");
    if wanted.stay_awake {
        print_stay_awake(os);
    }
    if wanted.remote_login {
        print_remote_login(os);
    }
    if wanted.thunderbolt {
        print_thunderbolt(os);
    }
    if wanted.tailscale {
        print_tailscale();
    }
    if wanted.cursor {
        print_cursor(os);
    }
}

fn print_stay_awake(os: OsKind) {
    println!("    stay-awake");
    match os {
        OsKind::Macos => {
            if let Some(s) = cmd_stdout("pmset", &["-g"]) {
                println!("      in-use  {}", pmset_keys(&s));
            } else {
                println!("      in-use  (pmset failed)");
            }
            if let Some(s) = cmd_stdout("pmset", &["-g", "custom"]) {
                for (name, body) in features::pmset_custom_sources(&s) {
                    println!("      {:<7} {}", name, pmset_keys(&body));
                }
            }
        }
        OsKind::Linux => {
            if !features::has_systemd() {
                println!("      no systemd — skip");
                return;
            }
            let path = Path::new(LOGIND_DROPIN_PATH);
            match std::fs::read_to_string(path) {
                Ok(s) if !s.trim().is_empty() => {
                    println!("      {}", path.display());
                    for line in s.lines() {
                        let t = line.trim();
                        if t.is_empty() || t.starts_with('#') {
                            continue;
                        }
                        println!("        {t}");
                    }
                }
                _ => println!("      {} (missing)", path.display()),
            }
        }
    }
}

fn print_remote_login(os: OsKind) {
    println!("    remote-login");
    let label = match os {
        OsKind::Macos => "sshd process",
        OsKind::Linux => "sshd listening",
    };
    println!(
        "      {label}: {}",
        if sshd_listening() { "yes" } else { "no" }
    );
    let path = Path::new(SSHD_KEEPALIVE_PATH);
    match std::fs::read_to_string(path) {
        Ok(s) => {
            let keys: Vec<&str> = s
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .collect();
            if keys.is_empty() {
                println!("      keepalive: {}", path.display());
            } else {
                println!("      keepalive: {}", keys.join(" "));
            }
        }
        Err(_) => println!("      keepalive: missing"),
    }
}

fn print_thunderbolt(os: OsKind) {
    println!("    thunderbolt");
    match cmd_stdout("ifconfig", &["bridge0"]) {
        Some(text) => {
            let status = ifconfig_field(&text, "status:");
            let inet = ifconfig_inet(&text);
            println!(
                "      bridge0  {}  inet={}",
                status.unwrap_or("-"),
                inet.unwrap_or("-")
            );
        }
        None => println!("      bridge0  (absent)"),
    }
    if matches!(os, OsKind::Macos) {
        let plist = "/Library/LaunchDaemons/dev.rig.thunderbolt-bridge.plist";
        let note = if Path::new(plist).is_file() {
            "present"
        } else {
            "missing"
        };
        println!("      plist    {note}");
    }
}

fn print_tailscale() {
    println!("    tailscale");
    let Some(ts) = which("tailscale") else {
        println!("      (not installed)");
        return;
    };
    let bin = ts.to_str().unwrap_or("tailscale");
    match cmd_stdout(bin, &["ip", "-4"]) {
        Some(ip) => println!("      ip  {}", ip.trim()),
        None => println!("      ip  (not connected)"),
    }
}

fn print_cursor(os: OsKind) {
    println!("    cursor");
    let Ok(dir) = cursor::cursor_user_dir(os) else {
        println!("      (HOME unset)");
        return;
    };
    let settings = dir.join("settings.json");
    if settings.is_file() {
        println!("      {}", settings.display());
    } else {
        println!("      settings.json missing");
    }
}

fn pmset_keys(text: &str) -> String {
    let mut sleep = "-";
    let mut display = "-";
    let mut disk = "-";
    let mut nap = "-";
    for line in text.lines() {
        let t = line.trim();
        let mut parts = t.split_whitespace();
        let key = parts.next().unwrap_or("");
        let val = parts.next().unwrap_or("");
        match key {
            "sleep" => sleep = val,
            "displaysleep" => display = val,
            "disksleep" => disk = val,
            "powernap" => nap = val,
            _ => {}
        }
    }
    format!("sleep={sleep} displaysleep={display} disksleep={disk} powernap={nap}")
}

fn ifconfig_field<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    text.lines().find_map(|l| {
        let t = l.trim();
        t.strip_prefix(prefix).map(str::trim)
    })
}

fn ifconfig_inet(text: &str) -> Option<&str> {
    text.lines().find_map(|l| {
        let t = l.trim();
        if t.starts_with("inet ") {
            t.split_whitespace().nth(1)
        } else {
            None
        }
    })
}

fn cmd_stdout(bin: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(bin)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).to_string();
    if s.trim().is_empty() {
        None
    } else {
        Some(s)
    }
}

fn sshd_listening() -> bool {
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

    #[test]
    fn pmset_keys_picks_sleep_fields() {
        let t = "\
         standby              1
         sleep                0
         displaysleep         120
         disksleep            10
         powernap             1
";
        assert_eq!(
            pmset_keys(t),
            "sleep=0 displaysleep=120 disksleep=10 powernap=1"
        );
    }
}
