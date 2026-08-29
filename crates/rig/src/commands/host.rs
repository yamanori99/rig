use crate::schema;
use miette::Result;

pub fn list(root: &std::path::Path) -> Result<()> {
    let hosts = schema::load_hosts(root)?;
    println!("hosts dir: {}/", root.join("hosts").display());
    if hosts.is_empty() {
        println!("(no hosts registered — copy hosts/examples/*.toml to hosts/ and edit)");
        return Ok(());
    }
    println!(
        "{:<20} {:<12} {:<8} {:<6} {}",
        "NAME", "ROLE", "OS", "SHELL", "NET"
    );
    for (_, h) in hosts {
        let net = h
            .ssh_paths()
            .iter()
            .map(|p| format!("{}:{}", p.link.as_str(), p.alias))
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "{:<20} {:<12} {:<8} {:<6} {}",
            h.name,
            h.role,
            h.resolved_os().as_str(),
            h.resolved_shell().as_str(),
            if net.is_empty() { "-" } else { &net }
        );
    }
    Ok(())
}

pub fn detect(root: &std::path::Path) -> Result<()> {
    let hosts = schema::load_hosts(root)?;
    let hn = schema::current_hostname();
    let short = hn.split('.').next().unwrap_or(&hn);
    println!("hostname: {hn}");
    println!("hosts:    {}/", root.join("hosts").display());
    match schema::detect_current_host(&hosts) {
        Some(h) => {
            println!("matched:  {} (role={})", h.name, h.role);
            println!("edit:     {}/hosts/{}.toml", root.display(), h.name);
        }
        None => {
            println!("matched:  (none — run `rig init` or add hosts/{short}.toml)");
            println!("expected: {}/hosts/{short}.toml", root.display());
        }
    }
    Ok(())
}
