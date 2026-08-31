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

    struct Row {
        peer: String,
        link: String,
        ip: String,
        status: String,
        ssh: bool,
    }

    let mut rows: Vec<Row> = Vec::new();
    for peer in &peers {
        let mut first = true;
        for path in peer.ssh_paths() {
            let tcp = tcp_port_open(&path.ip, 22);
            let ssh = tcp && ssh_batch_ok(&path.alias);
            let status = if ssh {
                "ssh ok".to_string()
            } else if tcp {
                "tcp/22 open  ssh fail".to_string()
            } else {
                "tcp/22 closed".to_string()
            };
            rows.push(Row {
                peer: if first {
                    peer.name.clone()
                } else {
                    String::new()
                },
                link: path.link.as_str().to_string(),
                ip: path.ip.clone(),
                status,
                ssh,
            });
            first = false;
        }
    }

    let pw = rows
        .iter()
        .map(|r| r.peer.chars().count())
        .max()
        .unwrap_or(4)
        .max(4);
    let iw = rows
        .iter()
        .map(|r| r.ip.chars().count())
        .max()
        .unwrap_or(2)
        .max(2);
    let sw = rows
        .iter()
        .map(|r| r.status.chars().count())
        .max()
        .unwrap_or(6)
        .max(6);

    ui::blank();
    ui::table_head(&format!(
        "{}  {}  {}  {}",
        ui::pad("peer", pw),
        ui::pad("link", 3),
        ui::pad("ip", iw),
        ui::pad("status", sw)
    ));
    let mut any_ssh = false;
    for r in &rows {
        any_ssh |= r.ssh;
        let st = ui::pad(&r.status, sw);
        let st = if r.ssh { ui::good(&st) } else { ui::bad(&st) };
        ui::table_row(format!(
            "{}  {}  {}  {}",
            ui::pad(&r.peer, pw),
            ui::pad(&r.link, 3),
            ui::pad(&r.ip, iw),
            st
        ));
    }

    if !any_ssh {
        ui::blank();
        ui::empty("ssh fail on every path — try: rig host keys -y");
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
