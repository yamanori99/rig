use super::cursor;
use super::features::{self, LOGIND_DROPIN_PATH, SSHD_KEEPALIVE_PATH};
use crate::schema::OsKind;
use std::path::Path;
use std::process::{Command, Stdio};

/// Print live OS / daemon settings (not just last apply notes).
/// Does not use sudo — `rig status` must stay non-interactive.
pub fn print_live(os: OsKind) {
    println!("  live");
    print_stay_awake(os);
    print_remote_login(os);
    print_file_block("sshd keepalive", Path::new(SSHD_KEEPALIVE_PATH));
    print_thunderbolt(os);
    print_tailscale();
    print_cursor(os);
    print_dotfile("tmux", &home().join(".tmux.conf"));
}

fn print_stay_awake(os: OsKind) {
    println!("    stay-awake");
    match os {
        OsKind::Macos => {
            dump_cmd("pmset -g custom", "pmset", &["-g", "custom"]);
            dump_cmd("pmset -g (in use)", "pmset", &["-g"]);
        }
        OsKind::Linux => {
            if !features::has_systemd() {
                println!("      no systemd — skip");
                return;
            }
            print_file_block("logind drop-in", Path::new(LOGIND_DROPIN_PATH));
        }
    }
}

fn print_remote_login(os: OsKind) {
    println!("    remote-login");
    match os {
        OsKind::Macos => {
            dump_cmd(
                "systemsetup -getremotelogin",
                "systemsetup",
                &["-getremotelogin"],
            );
            println!(
                "      sshd process: {}",
                if sshd_listening() { "yes" } else { "no" }
            );
        }
        OsKind::Linux => {
            println!(
                "      sshd listening: {}",
                if sshd_listening() { "yes" } else { "no" }
            );
        }
    }
}

fn print_thunderbolt(os: OsKind) {
    println!("    thunderbolt");
    let out = Command::new("ifconfig").arg("bridge0").output();
    match out {
        Ok(o) if o.status.success() => {
            indent_body(&String::from_utf8_lossy(&o.stdout));
        }
        _ => println!("      (no bridge0)"),
    }
    if matches!(os, OsKind::Macos) {
        print_file_block(
            "thunderbolt plist",
            Path::new("/Library/LaunchDaemons/dev.rig.thunderbolt-bridge.plist"),
        );
    }
}

fn print_tailscale() {
    println!("    tailscale");
    let Some(ts) = which("tailscale") else {
        println!("      (not installed)");
        return;
    };
    dump_cmd("ip -4", ts.to_str().unwrap_or("tailscale"), &["ip", "-4"]);
    dump_cmd(
        "status",
        ts.to_str().unwrap_or("tailscale"),
        &["status", "--self"],
    );
}

fn print_cursor(os: OsKind) {
    println!("    cursor");
    let Ok(dir) = cursor::cursor_user_dir(os) else {
        println!("      (HOME unset)");
        return;
    };
    if !dir.is_dir() {
        println!("      (no {})", dir.display());
        return;
    }
    for name in ["settings.json", "keybindings.json"] {
        print_file_block(name, &dir.join(name));
    }
}

fn print_dotfile(label: &str, path: &Path) {
    println!("    {label}");
    print_file_block(&path.display().to_string(), path);
}

fn print_file_block(label: &str, path: &Path) {
    println!("      {label}  {}", path.display());
    match std::fs::read_to_string(path) {
        Ok(s) if !s.trim().is_empty() => indent_body_at(&s, 8),
        Ok(_) => println!("        (empty)"),
        Err(_) => println!("        (missing)"),
    }
}

fn dump_cmd(label: &str, bin: &str, args: &[&str]) {
    println!("      {label}");
    let out = Command::new(bin)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    match out {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            if text.trim().is_empty() {
                println!("        (empty)");
            } else {
                indent_body_at(&text, 8);
            }
        }
        Ok(o) => {
            let err = String::from_utf8_lossy(&o.stderr);
            let msg = err.trim();
            if msg.is_empty() {
                println!("        (failed — may need admin)");
            } else {
                indent_body_at(msg, 8);
            }
        }
        Err(_) => println!("        (not available)"),
    }
}

fn indent_body(text: &str) {
    indent_body_at(text, 6);
}

fn indent_body_at(text: &str, spaces: usize) {
    let pad = " ".repeat(spaces);
    for line in text.lines() {
        println!("{pad}{line}");
    }
}

fn home() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
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
