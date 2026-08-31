use crate::error::{Result, RigError};
use crate::schema::{Host, SshPath};
use std::io::IsTerminal;
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

pub struct DistributeReport {
    pub key: Option<String>,
    pub ok: Vec<String>,
    pub skip: Vec<String>,
    pub fail: Vec<String>,
}

fn empty_report() -> DistributeReport {
    DistributeReport {
        key: None,
        ok: Vec::new(),
        skip: Vec::new(),
        fail: Vec::new(),
    }
}

/// Copy this machine's ed25519 pubkey to peers via their [[ssh]] aliases.
///
/// Prefers `link = lan|thunderbolt` (system sshd + authorized_keys). Falls back
/// to `link = vpn` only when no LAN/TB path answers on TCP/22.
///
/// Already-authorized hosts use BatchMode. First-time copy inherits the TTY
/// so ssh can ask for the login password once.
pub fn distribute(root: &Path, self_name: &str, yes: bool) -> Result<DistributeReport> {
    let pubkey = default_pubkey()?;
    let key_note = match ensure_local_ed25519(yes)? {
        KeyEnsure::Existing => None,
        KeyEnsure::WouldGenerate => Some(format!(
            "would generate {} (empty passphrase)",
            default_privkey()?.display()
        )),
        KeyEnsure::Generated => Some(format!(
            "generated {} (empty passphrase)",
            default_privkey()?.display()
        )),
        KeyEnsure::DerivedPub => Some("wrote .pub from existing private key".into()),
        KeyEnsure::WouldDerivePub => Some("would write .pub from existing private key".into()),
    };

    if !pubkey.is_file() {
        let mut report = empty_report();
        report.key = key_note;
        if !yes {
            return Ok(report);
        }
        report.fail.push(format!(
            "no public key at {} after generate",
            pubkey.display()
        ));
        return Ok(report);
    }

    let hosts = crate::schema::load_hosts(root)?;
    let peers: Vec<&Host> = hosts
        .iter()
        .map(|(_, h)| h)
        .filter(|h| h.name != self_name)
        .filter(|h| h.has_network())
        .collect();

    if peers.is_empty() {
        return Ok(DistributeReport {
            key: key_note,
            ok: Vec::new(),
            skip: vec!["no peers with [[ssh]] in hosts/".into()],
            fail: Vec::new(),
        });
    }

    let mut successes = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();

    for peer in peers {
        let candidates = peer.ssh_paths();
        let reachable: Vec<_> = candidates
            .into_iter()
            .filter(|p| tcp_port_open(&p.ip, 22))
            .collect();
        if reachable.is_empty() {
            skipped.push(format!("{} (offline)", peer.name));
            continue;
        }

        let mut copy_targets: Vec<SshPath> = reachable
            .iter()
            .filter(|p| p.link.prefer_for_keys())
            .cloned()
            .collect();
        if copy_targets.is_empty() {
            if let Some(p) = reachable
                .into_iter()
                .find(|p| p.link == crate::schema::LinkKind::Vpn)
            {
                copy_targets.push(p);
            }
        }

        if !yes {
            successes.push(format!(
                "{} → would copy via {}",
                peer.name,
                copy_targets
                    .iter()
                    .map(|p| p.alias.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            ));
            continue;
        }

        let user = peer.resolved_user();
        let mut peer_ok = false;
        let mut last_err = String::new();
        for path in &copy_targets {
            match ssh_copy_id(&pubkey, peer, path, &user) {
                Ok(how) => {
                    successes.push(format!("{} via {} ({how})", peer.name, path.alias));
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

    Ok(DistributeReport {
        key: key_note,
        ok: successes,
        skip: skipped,
        fail: failed,
    })
}

enum KeyEnsure {
    Existing,
    WouldGenerate,
    Generated,
    WouldDerivePub,
    DerivedPub,
}

fn ssh_dir() -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| RigError::Msg("HOME is not set".into()))?;
    Ok(home.join(".ssh"))
}

fn default_privkey() -> Result<PathBuf> {
    Ok(ssh_dir()?.join("id_ed25519"))
}

fn default_pubkey() -> Result<PathBuf> {
    Ok(ssh_dir()?.join("id_ed25519.pub"))
}

fn ensure_local_ed25519(yes: bool) -> Result<KeyEnsure> {
    let dir = ssh_dir()?;
    let privkey = dir.join("id_ed25519");
    let pubkey = dir.join("id_ed25519.pub");

    if privkey.is_file() && pubkey.is_file() {
        return Ok(KeyEnsure::Existing);
    }

    if privkey.is_file() && !pubkey.is_file() {
        if !yes {
            return Ok(KeyEnsure::WouldDerivePub);
        }
        derive_pubkey(&privkey, &pubkey)?;
        return Ok(KeyEnsure::DerivedPub);
    }

    if pubkey.is_file() && !privkey.is_file() {
        return Err(RigError::Msg(format!(
            "public key at {} but no private key — not generating over that",
            pubkey.display()
        )));
    }

    if !yes {
        return Ok(KeyEnsure::WouldGenerate);
    }

    std::fs::create_dir_all(&dir).map_err(RigError::Io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
            .map_err(RigError::Io)?;
    }

    let comment = key_comment();
    let status = Command::new("ssh-keygen")
        .args(["-t", "ed25519", "-N", "", "-q", "-f"])
        .arg(&privkey)
        .arg("-C")
        .arg(&comment)
        .status()
        .map_err(|e| RigError::Msg(format!("ssh-keygen: {e}")))?;
    if !status.success() {
        return Err(RigError::Msg(format!(
            "ssh-keygen failed ({status}) writing {}",
            privkey.display()
        )));
    }
    Ok(KeyEnsure::Generated)
}

fn derive_pubkey(privkey: &Path, pubkey: &Path) -> Result<()> {
    let out = Command::new("ssh-keygen")
        .args(["-y", "-f"])
        .arg(privkey)
        .output()
        .map_err(|e| RigError::Msg(format!("ssh-keygen -y: {e}")))?;
    if !out.status.success() {
        return Err(RigError::Msg(format!(
            "ssh-keygen -y failed ({})",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    std::fs::write(pubkey, out.stdout).map_err(RigError::Io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(pubkey, std::fs::Permissions::from_mode(0o644))
            .map_err(RigError::Io)?;
    }
    Ok(())
}

fn key_comment() -> String {
    let user = std::env::var("USER").unwrap_or_else(|_| "rig".into());
    let host = hostname_short();
    format!("{user}@{host}")
}

fn hostname_short() -> String {
    Command::new("hostname")
        .arg("-s")
        .output()
        .ok()
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            (!s.is_empty()).then_some(s)
        })
        .unwrap_or_else(|| "localhost".into())
}

fn tcp_port_open(ip: &str, port: u16) -> bool {
    let Ok(addr) = format!("{ip}:{port}").parse::<SocketAddr>() else {
        return false;
    };
    TcpStream::connect_timeout(&addr, Duration::from_secs(3)).is_ok()
}

fn ssh_batch_opts() -> [&'static str; 10] {
    [
        "-o",
        "ConnectTimeout=8",
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

fn ssh_interactive_opts() -> [&'static str; 14] {
    [
        "-o",
        "ConnectTimeout=20",
        "-o",
        "ConnectionAttempts=1",
        "-o",
        "NumberOfPasswordPrompts=1",
        "-o",
        "StrictHostKeyChecking=accept-new",
        "-o",
        "GSSAPIAuthentication=no",
        "-o",
        "PreferredAuthentications=publickey,keyboard-interactive,password",
        "-o",
        "LogLevel=ERROR",
    ]
}

fn ssh_copy_id(
    pubkey: &Path,
    peer: &Host,
    path: &SshPath,
    user: &str,
) -> std::result::Result<&'static str, String> {
    let alias = path.alias.as_str();
    if pubkey_already_on(pubkey, alias) {
        return Ok("already");
    }

    if run_copy_id(pubkey, alias)? {
        return Ok("key");
    }

    if !std::io::stdin().is_terminal() {
        return Err(format!(
            "{alias}: first copy needs a TTY for the login password"
        ));
    }

    prompt_which_machine(peer, path, user);
    if install_via_ssh(pubkey, alias, false)? {
        return Ok("password");
    }

    Err(format!(
        "could not install on {alias} (wrong password or sshd refused pubkey)"
    ))
}

fn prompt_which_machine(peer: &Host, path: &SshPath, user: &str) {
    crate::ui::blank();
    crate::ui::kv("peer", &peer.name);
    crate::ui::kv("via", format!("{}  ({})", path.alias, path.link.comment()));
    crate::ui::kv("login", format!("{user}@{}", path.ip));
    crate::ui::item("password for that peer (once)");
    let _ = std::io::Write::flush(&mut std::io::stdout());
    let _ = std::io::Write::flush(&mut std::io::stderr());
}

fn pubkey_already_on(pubkey: &Path, alias: &str) -> bool {
    let Ok(pub_text) = std::fs::read_to_string(pubkey) else {
        return false;
    };
    let pub_line = pub_text.trim();
    if pub_line.is_empty() {
        return false;
    }
    let q = shell_single_quote(pub_line);
    Command::new("ssh")
        .args(ssh_batch_opts())
        .arg(alias)
        .arg(format!("grep -qxF {q} ~/.ssh/authorized_keys 2>/dev/null"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn run_copy_id(pubkey: &Path, alias: &str) -> std::result::Result<bool, String> {
    if which("ssh-copy-id").is_some() {
        let status = Command::new("ssh-copy-id")
            .args(ssh_batch_opts())
            .arg("-i")
            .arg(pubkey)
            .arg(alias)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| e.to_string())?;
        if status.success() {
            return Ok(true);
        }
    }

    install_via_ssh(pubkey, alias, true)
}

fn install_via_ssh(pubkey: &Path, alias: &str, batch: bool) -> std::result::Result<bool, String> {
    let pub_text = std::fs::read_to_string(pubkey).map_err(|e| e.to_string())?;
    let pub_line = pub_text.trim();
    if pub_line.is_empty() {
        return Err("empty pubkey".into());
    }
    let q = shell_single_quote(pub_line);
    let remote = format!(
        "mkdir -p ~/.ssh && chmod 700 ~/.ssh && \
         touch ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys && \
         grep -qxF {q} ~/.ssh/authorized_keys || echo {q} >> ~/.ssh/authorized_keys"
    );

    let mut cmd = Command::new("ssh");
    if batch {
        cmd.args(ssh_batch_opts())
            .arg(alias)
            .arg(&remote)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
    } else {
        cmd.args(ssh_interactive_opts()).arg(alias).arg(&remote);
    }
    let status = cmd.status().map_err(|e| e.to_string())?;
    Ok(status.success())
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
