use crate::apply;
use crate::schema;
use crate::ui;
use miette::Result;

pub fn run(root: &std::path::Path, yes: bool) -> Result<()> {
    let hosts = schema::load_hosts(root)?;
    let text = apply::generate_ssh_config(root, &hosts);
    if !yes {
        eprintln!("ssh-config  preview");
        eprintln!("  write     pass --yes / --write → ~/.ssh/config.d/rig.conf");
        print!("{text}");
        return Ok(());
    }
    let path = apply::write_ssh_config(root, &hosts)?;
    ui::title("ssh-config", false);
    ui::kv("wrote", path.display());
    ui::kv("include", "config.d/*.conf in ~/.ssh/config");
    ui::kv(
        "source",
        format!(
            "{}/  (edit [[ssh]] there, not the generated file)",
            root.join("hosts").display()
        ),
    );
    Ok(())
}
