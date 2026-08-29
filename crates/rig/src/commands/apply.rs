use crate::apply::{self, build_plan};
use crate::error::RigError;
use crate::schema;
use miette::Result;

pub fn run(root: &std::path::Path, dry_run: bool, yes: bool, skip_packages: bool) -> Result<()> {
    let hosts = schema::load_hosts(root)?;
    let host = schema::detect_current_host(&hosts).ok_or_else(|| {
        RigError::Msg(format!(
            "this machine is not registered (hostname={}). run `rig init` first",
            schema::current_hostname()
        ))
    })?;
    let role = schema::load_role(root, &host.role)?;
    let plan = build_plan(host, &role);

    println!("rig apply{}", if dry_run { " (dry-run)" } else { "" });
    println!("  root={}", root.display());
    println!(
        "  host={}  role={}  os={}{}  shell={}  user={}",
        plan.host,
        plan.role,
        plan.os,
        if host.os.is_none() { " (detected)" } else { "" },
        plan.shell,
        plan.user
    );
    println!(
        "  packages: {}{}",
        plan.package_sets.join(" + "),
        if skip_packages { " (will skip)" } else { "" }
    );
    println!();
    for step in &plan.steps {
        let mark = if step.skip || (skip_packages && step.id == "packages") {
            "skip"
        } else {
            "do  "
        };
        println!("  [{mark}] {:<14} {}", step.id, step.detail);
    }

    if dry_run {
        println!();
        println!("dry-run only — no changes made");
        println!("apply for real: rig apply --yes");
        return Ok(());
    }

    apply::execute(root, host, &role, yes, skip_packages)?;
    Ok(())
}
