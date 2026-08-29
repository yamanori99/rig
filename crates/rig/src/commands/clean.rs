use crate::apply;
use miette::Result;

pub fn run(root: &std::path::Path, dry_run: bool, yes: bool, packages: bool) -> Result<()> {
    let state = crate::paths::state_path();
    println!("rig clean{}", if dry_run { " (dry-run)" } else { "" });
    println!("  root: {}", root.display());
    println!("  state file: {}", state.display());
    println!(
        "  packages: {}",
        if packages {
            "uninstall recorded sets (destructive)"
        } else {
            "keep (pass --packages to uninstall)"
        }
    );
    println!();

    if !dry_run && !yes {
        println!("refusing to clean without --yes (destructive)");
        println!("preview: rig clean --dry-run");
        println!("apply:   rig clean --yes");
        return Ok(());
    }

    let report = apply::clean(root, yes, dry_run, packages)?;
    println!("{}", report.detail);
    if dry_run {
        println!();
        println!("dry-run only — no changes made");
        println!("clean for real: rig clean --yes");
    }
    Ok(())
}
