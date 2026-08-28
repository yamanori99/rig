use crate::error::Result;
use crate::schema::{self, Host};
use std::net::{SocketAddr, TcpStream};
use std::process::{Command, Stdio};
use std::time::Duration;

/// Probe TCP/22 and BatchMode SSH for each peer [[ssh]] path.
pub fn run(root: &std::path::Path) -> Result<()> {
    let hosts = schema::load_hosts(root)?;
    let self_name = schema::detect_current_host(&hosts)
        .map(|h| h.name.clone())
        .unwrap_or_else(|| {
            let hn = schema::current_hostname();
            hn.split('.').next().unwrap_or(&hn).to_string()
        });

    println!("rig check");
    println!("  self={self_name}");
    println!("  hosts={}/", root.join("hosts").display());
    println!();

    let peers: Vec<&Host> = hosts
        .iter()
        .map(|(_, h)| h)
        .filter(|h| h.name != self_name)
        .filter(|h| h.has_network())
        .collect();

    if peers.is_empty() {
        println!("(no peers with [[ssh]] — add alias/ip/link in hosts/*.toml)");
        return Ok(());
    }

    println!(
        "{:<18} {:<18} {:<4} {:<18} {:<5} {}",
        "PEER", "ALIAS", "LINK", "IP", "TCP", "SSH"
    );

    let mut any_ssh = false;
    for peer in peers {
        for path in peer.ssh_paths() {
            let tcp = if tcp_port_open(&path.ip, 22) {
                "ok"
            } else {
                "fail"
            };
            let ssh = if tcp == "ok" && ssh_batch_ok(&path.alias) {
                any_ssh = true;
                "ok"
            } else if tcp == "ok" {
                "fail"
            } else {
                "-"
            };
            println!(
                "{:<18} {:<18} {:<4} {:<18} {:<5} {ssh}",
                peer.name,
                path.alias,
                path.link.as_str(),
                path.ip,
                tcp
            );
        }
    }

    println!();
    if any_ssh {
        println!("at least one passwordless SSH path works");
    } else {
        println!("no passwordless SSH yet — try: rig keys distribute --yes");
    }
    Ok(())
}

fn tcp_port_open(ip: &str, port: u16) -> bool {
    let Ok(addr) = format!("{ip}:{port}").parse::<SocketAddr>() else {
        return false;
    };
    TcpStream::connect_timeout(&addr, Duration::from_secs(3)).is_ok()
}

fn ssh_batch_ok(alias: &str) -> bool {
    Command::new("ssh")
        .args([
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
        ])
        .arg(alias)
        .arg("exit 0")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
