use crate::paths;
use miette::Result;

pub fn run(
    _root: &std::path::Path,
    dry_run: bool,
    yes: bool,
    packages: bool,
) -> Result<()> {
    let state = paths::state_path();
    println!("rig clean{}", if dry_run { " (dry-run)" } else { "" });
    println!("  state file: {}", state.display());
    if !state.exists() {
        println!("  (no state yet — nothing to clean; apply once first)");
        return Ok(());
    }
    println!("  packages: {}", if packages { "remove recorded" } else { "keep" });
    if dry_run {
        println!("dry-run only");
        return Ok(());
    }
    if !yes {
        println!("refusing to clean without --yes (destructive)");
        return Ok(());
    }
    println!("clean execution will reverse the apply manifest (next slice)");
    Ok(())
}
