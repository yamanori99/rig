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

    let mut ssh_ok: Vec<(String, String, String)> = Vec::new();
    for peer in peers {
        ui::group(&peer.name);
        for path in peer.ssh_paths() {
            let tcp = tcp_port_open(&path.ip, 22);
            let ssh = tcp && ssh_batch_ok(&path.alias);
            let link = path.link.as_str();
            let detail = if ssh {
                ssh_ok.push((
                    peer.name.clone(),
                    path.alias.clone(),
                    path.link.comment().to_string(),
                ));
                format!("{}  {}  ssh ok", path.alias, path.ip)
            } else if tcp {
                format!(
                    "{}  {}  tcp open  ssh no (try: rig host keys -y)",
                    path.alias, path.ip
                )
            } else {
                format!("{}  {}  no tcp/22", path.alias, path.ip)
            };
            if ssh {
                ui::ok(link, &detail);
            } else {
                ui::fail(link, &detail);
            }
        }
    }

    ui::blank();
    if ssh_ok.is_empty() {
        ui::empty("no passwordless SSH yet — try: rig host keys -y");
    } else {
        ui::section("can ssh");
        for (peer, alias, how) in ssh_ok {
            ui::kv("ssh", format!("ssh {alias}  {peer}  {how}"));
        }
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
