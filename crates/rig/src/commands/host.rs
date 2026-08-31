use crate::schema;
use crate::ui;
use miette::Result;

pub fn list(root: &std::path::Path) -> Result<()> {
    let hosts = schema::load_hosts(root)?;
    ui::title("host list", false);
    ui::kv("hosts", format!("{}/", root.join("hosts").display()));
    if hosts.is_empty() {
        ui::empty("none — copy hosts/examples/*.toml to hosts/ and edit");
        return Ok(());
    }
    ui::table_head(&format!(
        "{:<20} {:<12} {:<8} {:<6} {}",
        "name", "role", "os", "shell", "net"
    ));
    for (_, h) in hosts {
        let net = h
            .ssh_paths()
            .iter()
            .map(|p| format!("{}:{}", p.link.as_str(), p.alias))
            .collect::<Vec<_>>()
            .join(",");
        ui::table_row(format!(
            "{:<20} {:<12} {:<8} {:<6} {}",
            h.name,
            h.role,
            h.resolved_os().as_str(),
            h.resolved_shell().as_str(),
            if net.is_empty() { "-" } else { &net }
        ));
    }
    Ok(())
}

pub fn detect(root: &std::path::Path) -> Result<()> {
    let hosts = schema::load_hosts(root)?;
    let hn = schema::current_hostname();
    let short = hn.split('.').next().unwrap_or(&hn);
    ui::title("host detect", false);
    ui::kv("hostname", &hn);
    ui::kv("hosts", format!("{}/", root.join("hosts").display()));
    match schema::detect_current_host(&hosts) {
        Some(h) => {
            ui::kv("matched", format!("{}  role={}", h.name, h.role));
            ui::kv("edit", format!("{}/hosts/{}.toml", root.display(), h.name));
        }
        None => {
            ui::kv(
                "matched",
                format!("none — rig init or add hosts/{short}.toml"),
            );
            ui::kv("expected", format!("{}/hosts/{short}.toml", root.display()));
        }
    }
    Ok(())
}
