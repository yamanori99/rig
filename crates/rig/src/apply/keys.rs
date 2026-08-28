use crate::error::{Result, RigError};
use crate::schema::Host;
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

pub struct DistributeReport {
    pub detail: String,
}

/// Copy this machine's ed25519 pubkey to peers via Host aliases (-lan / -tb / -ts).
///
/// Prefers LAN and Thunderbolt (system sshd + authorized_keys). Falls back to
/// Tailscale alias only when no LAN/TB path answers on TCP/22.
pub fn distribute(
    root: &Path,
    self_name: &str,
    yes: bool,
    dry_run: bool,
) -> Result<DistributeReport> {
    let pubkey = default_pubkey()?;
    if !pubkey.is_file() {
        return Ok(DistributeReport {
            detail: format!(
                "no public key at {} — run: ssh-keygen -t ed25519",
                pubkey.display()
            ),
        });
    }

    let hosts = crate::schema::load_hosts(root)?;
    let peers: Vec<&Host> = hosts
        .iter()
        .map(|(_, h)| h)
        .filter(|h| h.name != self_name)
        .filter(|h| h.vpn.is_some() || h.lan.is_some() || h.thunderbolt.is_some())
        .collect();

    if peers.is_empty() {
        return Ok(DistributeReport {
            detail: "no peers with vpn/lan/thunderbolt in hosts/".into(),
        });
    }

    let mut successes = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();

    for peer in peers {
        let candidates = candidate_paths(peer);
        let reachable: Vec<(String, String)> = candidates
            .into_iter()
            .filter(|(_, ip)| tcp_port_open(ip, 22))
            .collect();
        if reachable.is_empty() {
            skipped.push(format!("{} (offline)", peer.name));
            continue;
        }

        let mut copy_targets: Vec<String> = reachable
            .iter()
            .filter(|(alias, _)| alias.ends_with("-lan") || alias.ends_with("-tb"))
            .map(|(alias, _)| alias.clone())
            .collect();
        if copy_targets.is_empty() {
            if let Some((alias, _)) = reachable.into_iter().find(|(a, _)| a.ends_with("-ts")) {
                copy_targets.push(alias);
            }
        }

        if dry_run {
            successes.push(format!(
                "{} → would copy via {}",
                peer.name,
                copy_targets.join(",")
            ));
            continue;
        }

        if !yes {
            skipped.push(format!(
                "{} (pass --yes to copy via {})",
                peer.name,
                copy_targets.join(",")
            ));
            continue;
        }

        let mut peer_ok = false;
        let mut last_err = String::new();
        for target in &copy_targets {
            match ssh_copy_id(&pubkey, target) {
                Ok(()) => {
                    successes.push(format!("{} via {target}", peer.name));
                    peer_ok = true;
                    break;
                }
                Err(e) => last_err = e,
            }
        }
        if !peer_ok {
            failed.push(format!("{} ({last_err})", peer.name));
        }
    }

    let mut parts = Vec::new();
    if !successes.is_empty() {
        parts.push(format!("ok: {}", successes.join("; ")));
    }
    if !skipped.is_empty() {
        parts.push(format!("skip: {}", skipped.join("; ")));
    }
    if !failed.is_empty() {
        parts.push(format!("fail: {}", failed.join("; ")));
    }
    if parts.is_empty() {
        parts.push("nothing to do".into());
    }

    Ok(DistributeReport {
        detail: parts.join(" | "),
    })
}

fn default_pubkey() -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| RigError::Msg("HOME is not set".into()))?;
    Ok(home.join(".ssh/id_ed25519.pub"))
}

fn candidate_paths(peer: &Host) -> Vec<(String, String)> {
    let mut v = Vec::new();
    if let Some(ip) = &peer.lan {
        v.push((format!("{}-lan", peer.name), ip.clone()));
    }
    if let Some(ip) = &peer.thunderbolt {
        v.push((format!("{}-tb", peer.name), ip.clone()));
    }
    if let Some(ip) = &peer.vpn {
        v.push((format!("{}-ts", peer.name), ip.clone()));
    }
    v
}

fn tcp_port_open(ip: &str, port: u16) -> bool {
    let Ok(addr) = format!("{ip}:{port}").parse::<SocketAddr>() else {
        return false;
    };
    TcpStream::connect_timeout(&addr, Duration::from_secs(3)).is_ok()
}

fn ssh_opts() -> [&'static str; 10] {
    [
        "-o",
        "ConnectTimeout=5",
        "-o",
        "ConnectionAttempts=1",
        "-o",
        "BatchMode=yes",
        "-o",
        "StrictHostKeyChecking=accept-new",
        "-o",
        "GSSAPIAuthentication=no",
    ]
}

fn ssh_copy_id(pubkey: &Path, alias: &str) -> std::result::Result<(), String> {
    if which("ssh-copy-id").is_some() {
        let status = Command::new("ssh-copy-id")
            .args(ssh_opts())
            .arg("-i")
            .arg(pubkey)
            .arg(alias)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| e.to_string())?;
        if status.success() {
            return Ok(());
        }
    }

    let pub_text = std::fs::read_to_string(pubkey).map_err(|e| e.to_string())?;
    let pub_line = pub_text.trim();
    if pub_line.is_empty() {
        return Err("empty pubkey".into());
    }
    let q = shell_single_quote(pub_line);

    let check = Command::new("ssh")
        .args(ssh_opts())
        .arg(alias)
        .arg(format!(
            "grep -qxF {q} ~/.ssh/authorized_keys 2>/dev/null"
        ))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| e.to_string())?;
    if check.success() {
        return Ok(());
    }

    let install = Command::new("ssh")
        .args(ssh_opts())
        .arg(alias)
        .arg(format!(
            "mkdir -p ~/.ssh && chmod 700 ~/.ssh && \
             touch ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys && \
             grep -qxF {q} ~/.ssh/authorized_keys || echo {q} >> ~/.ssh/authorized_keys"
        ))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| e.to_string())?;
    if install.success() {
        Ok(())
    } else {
        Err(format!(
            "could not install on {alias} (need one-time password access, then re-run)"
        ))
    }
}

fn shell_single_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\"'\"'"))
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
