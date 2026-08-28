use crate::apply;
use crate::schema;
use miette::Result;

pub fn run(root: &std::path::Path, dry_run: bool, write: bool) -> Result<()> {
    let hosts = schema::load_hosts(root)?;
    let text = apply::generate_ssh_config(root, &hosts);
    if dry_run || !write {
        print!("{text}");
        if !write {
            println!("# tip: pass --write to install → ~/.ssh/config.d/rig.conf");
        }
        return Ok(());
    }
    let path = apply::write_ssh_config(root, &hosts)?;
    println!("wrote {}", path.display());
    println!("ensured Include config.d/*.conf in ~/.ssh/config");
    println!(
        "source hosts: {}/  (edit [[ssh]] there, not the generated file)",
        root.join("hosts").display()
    );
    Ok(())
}
