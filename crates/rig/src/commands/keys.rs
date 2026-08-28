use crate::apply;
use crate::error::RigError;
use crate::schema;
use miette::Result;

pub fn distribute(root: &std::path::Path, yes: bool, dry_run: bool) -> Result<()> {
    let hosts = schema::load_hosts(root)?;
    let self_host = schema::detect_current_host(&hosts).ok_or_else(|| {
        RigError::Msg(format!(
            "this machine is not registered (hostname={}). run `rig init` first",
            schema::current_hostname()
        ))
    })?;

    println!(
        "rig keys distribute{}",
        if dry_run { " (dry-run)" } else { "" }
    );
    println!("  self={}  pubkey=~/.ssh/id_ed25519.pub", self_host.name);
    println!("  hosts={}/", root.join("hosts").display());
    println!("  order per peer: lan/tb links first, then vpn");
    if !dry_run && !yes {
        println!("  tip: pass --yes (-y) to install; without it only probes / plans");
    }
    println!();

    let report = apply::distribute_keys(root, &self_host.name, yes, dry_run)?;
    println!("{}", report.detail);
    if dry_run {
        println!();
        println!("dry-run only — no keys copied");
        println!("install: rig keys distribute --yes");
    }
    Ok(())
}
