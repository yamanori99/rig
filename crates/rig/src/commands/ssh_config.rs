use crate::apply;
use crate::schema;
use crate::ui;
use miette::Result;

pub fn run(root: &std::path::Path, yes: bool) -> Result<()> {
    let hosts = schema::load_hosts(root)?;
    let text = apply::generate_ssh_config(root, &hosts);
    if !yes {
        ui::title("ssh-config", true);
        ui::kv("write", "~/.ssh/config.d/rig.conf");
        let aliases = apply::host_aliases(&text);
        ui::kv("aliases", aliases.len());
        for a in aliases {
            ui::note("host", a);
        }
        ui::preview("write");
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
            crate::paths::hosts_dir(root).display()
        ),
    );
    Ok(())
}
