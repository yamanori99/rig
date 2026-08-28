use crate::error::Result;
use crate::schema::{self, Host};
use std::net::{SocketAddr, TcpStream};
use std::process::{Command, Stdio};
use std::time::Duration;

/// Probe TCP/22 and BatchMode SSH for each peer path (-lan / -tb / -ts).
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
    println!();

    let peers: Vec<&Host> = hosts
        .iter()
        .map(|(_, h)| h)
        .filter(|h| h.name != self_name)
        .filter(|h| h.vpn.is_some() || h.lan.is_some() || h.thunderbolt.is_some())
        .collect();

    if peers.is_empty() {
        println!("(no peers with vpn/lan/thunderbolt — add addresses in hosts/*.toml)");
        return Ok(());
    }

    println!(
        "{:<18} {:<4} {:<18} {:<5} {}",
        "PEER", "PATH", "IP", "TCP", "SSH"
    );

    let mut any_ssh = false;
    for peer in peers {
        for (path, ip) in peer_paths(peer) {
            let tcp = if tcp_port_open(&ip, 22) { "ok" } else { "fail" };
            let ssh = if tcp == "ok" && ssh_batch_ok(&format!("{}-{path}", peer.name)) {
                any_ssh = true;
                "ok"
            } else if tcp == "ok" {
                "fail"
            } else {
                "-"
            };
            println!(
                "{:<18} {:<4} {:<18} {:<5} {ssh}",
                peer.name, path, ip, tcp
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

fn peer_paths(peer: &Host) -> Vec<(&'static str, String)> {
    let mut v = Vec::new();
    if let Some(ip) = &peer.lan {
        v.push(("lan", ip.clone()));
    }
    if let Some(ip) = &peer.thunderbolt {
        v.push(("tb", ip.clone()));
    }
    if let Some(ip) = &peer.vpn {
        v.push(("ts", ip.clone()));
    }
    v
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
