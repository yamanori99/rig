use crate::apply;
use miette::Result;

pub fn run(root: &std::path::Path, yes: bool, packages: bool) -> Result<()> {
    let state = crate::paths::state_path();
    println!("rig clean{}", if yes { "" } else { "  (preview)" });
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

    let report = apply::clean(root, yes, packages)?;
    println!("{}", report.detail);
    if !yes {
        println!();
        println!("preview — pass --yes (-y) to clean");
    }
    Ok(())
}
