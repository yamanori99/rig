use crate::apply;
use crate::error::RigError;
use crate::schema;
use miette::Result;

pub fn distribute(root: &std::path::Path, yes: bool) -> Result<()> {
    let hosts = schema::load_hosts(root)?;
    let self_host = schema::detect_current_host(&hosts).ok_or_else(|| {
        RigError::Msg(format!(
            "this machine is not registered (hostname={}). run `rig init` first",
            schema::current_hostname()
        ))
    })?;

    println!(
        "rig keys distribute{}",
        if yes { "" } else { "  (preview)" }
    );
    println!("  self={}  pubkey=~/.ssh/id_ed25519.pub", self_host.name);
    println!("  hosts={}/", root.join("hosts").display());
    println!("  order per peer: lan/tb links first, then vpn");
    println!();

    let report = apply::distribute_keys(root, &self_host.name, yes)?;
    println!("{}", report.detail);
    if !yes {
        println!();
        println!("preview — pass --yes (-y) to copy keys");
    }
    Ok(())
}
