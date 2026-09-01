use crate::schema;
use crate::ui;
use miette::Result;

fn col_w<'a>(min: usize, xs: impl Iterator<Item = &'a str>) -> usize {
    xs.map(|s| s.chars().count()).max().unwrap_or(min).max(min)
}

pub fn list(root: &std::path::Path) -> Result<()> {
    let hosts = schema::load_hosts(root)?;
    ui::title("host list", false);
    if hosts.is_empty() {
        ui::empty("none — copy hosts/examples/*.toml to ~/.rig-hosts/ and edit");
        return Ok(());
    }
    let nw = col_w(4, hosts.iter().map(|(_, h)| h.name.as_str())).max(4);
    let rw = col_w(4, hosts.iter().map(|(_, h)| h.role.as_str())).max(4);
    ui::table_head(&format!(
        "{}  {}  {:<5} {:<5} {}",
        ui::pad("name", nw),
        ui::pad("role", rw),
        "os",
        "shell",
        "net"
    ));
    for (_, h) in hosts {
        let net = h
            .ssh_paths()
            .iter()
            .map(|p| format!("{}:{}", p.link.as_str(), p.alias))
            .collect::<Vec<_>>()
            .join("  ");
        ui::table_row(format!(
            "{}  {}  {:<5} {:<5} {}",
            ui::pad(&h.name, nw),
            ui::pad(&h.role, rw),
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
    ui::kv("hosts", format!("{}/", crate::paths::hosts_dir(root).display()));
    match schema::detect_current_host(&hosts) {
        Some(h) => {
            ui::kv("matched", format!("{}  {}", h.name, h.role));
            if let Some(detail) = h.user_write_needed() {
                ui::note("user", detail);
            }
            ui::kv(
                "edit",
                crate::paths::hosts_dir(root)
                    .join(format!("{}.toml", h.name))
                    .display(),
            );
        }
        None => {
            ui::kv(
                "matched",
                format!("none — rig init or add ~/.rig-hosts/{short}.toml"),
            );
            ui::kv(
                "expected",
                crate::paths::hosts_dir(root)
                    .join(format!("{short}.toml"))
                    .display(),
            );
        }
    }
    Ok(())
}
