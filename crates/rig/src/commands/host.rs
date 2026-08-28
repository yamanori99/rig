use crate::schema;
use miette::Result;

pub fn list(root: &std::path::Path) -> Result<()> {
    let hosts = schema::load_hosts(root)?;
    if hosts.is_empty() {
        println!("(no hosts registered — copy hosts/examples/*.toml to hosts/ and edit)");
        return Ok(());
    }
    println!(
        "{:<20} {:<12} {:<8} {:<6} {}",
        "NAME", "ROLE", "OS", "SHELL", "NET"
    );
    for (_, h) in hosts {
        let net = [
            h.vpn.as_deref().map(|_| "vpn"),
            h.lan.as_deref().map(|_| "lan"),
            h.thunderbolt.as_deref().map(|_| "tb"),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(",");
        println!(
            "{:<20} {:<12} {:<8} {:<6} {}",
            h.name,
            h.role,
            h.resolved_os().as_str(),
            h.resolved_shell().as_str(),
            if net.is_empty() { "-" } else { &net }
        );
    }
    Ok(())
}

pub fn detect(root: &std::path::Path) -> Result<()> {
    let hosts = schema::load_hosts(root)?;
    let hn = schema::current_hostname();
    println!("hostname: {hn}");
    match schema::detect_current_host(&hosts) {
        Some(h) => {
            println!("matched:  {} (role={})", h.name, h.role);
        }
        None => {
            println!("matched:  (none — run `rig init` or add hosts/{hn}.toml)");
        }
    }
    Ok(())
}
