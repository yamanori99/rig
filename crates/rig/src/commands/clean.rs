use crate::apply;
use crate::ui;
use miette::Result;

pub fn run(root: &std::path::Path, yes: bool, packages: bool) -> Result<()> {
    let state = crate::paths::state_path();
    ui::title("clean", !yes);
    ui::kv("root", root.display());
    ui::kv("state", state.display());
    ui::kv(
        "packages",
        if packages {
            "uninstall recorded sets (destructive)"
        } else {
            "keep (pass --packages to uninstall)"
        },
    );
    ui::blank();

    let report = apply::clean(root, yes, packages)?;
    if report.lines.is_empty() && report.errors.is_empty() {
        ui::empty("nothing to do");
    }
    for line in &report.lines {
        ui::item(line);
    }
    for err in &report.errors {
        ui::fail("clean", err);
    }
    if !yes {
        ui::preview("clean");
    }
    Ok(())
}
