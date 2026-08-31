use crate::error::Result;
use crate::schema::{self, Host};
use crate::ui;
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

    ui::title("check", false);
    ui::kv("self", &self_name);
    ui::kv("hosts", format!("{}/", root.join("hosts").display()));
    ui::blank();

    let peers: Vec<&Host> = hosts
        .iter()
        .map(|(_, h)| h)
        .filter(|h| h.name != self_name)
        .filter(|h| h.has_network())
        .collect();

    if peers.is_empty() {
        ui::empty("no peers with [[ssh]] — add alias/ip/link in hosts/*.toml");
        return Ok(());
    }

    let header = format!(
        "{:<16} {:<16} {:<4} {:<15} {:<5} {}",
        "peer", "alias", "link", "ip", "tcp", "ssh"
    );
    ui::table_head(&header);

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
            ui::table_row(format!(
                "{:<16} {:<16} {:<4} {:<15} {} {}",
                peer.name,
                path.alias,
                path.link.as_str(),
                path.ip,
                ui::mark_pad(tcp, 5),
                ui::mark_pad(ssh, 4)
            ));
        }
    }

    ui::blank();
    if any_ssh {
        ui::empty("at least one passwordless SSH path works");
    } else {
        ui::empty("no passwordless SSH yet — try: rig keys distribute --yes");
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
