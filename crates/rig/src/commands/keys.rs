use crate::apply;
use crate::error::RigError;
use crate::schema;
use crate::ui;
use miette::Result;

pub fn distribute(root: &std::path::Path, yes: bool) -> Result<()> {
    let hosts = schema::load_hosts(root)?;
    let self_host = schema::detect_current_host(&hosts)
        .ok_or_else(|| RigError::Msg(schema::unregistered_hint(root, &hosts)))?;

    ui::title("keys distribute", !yes);
    ui::kv("self", &self_host.name);
    ui::kv("pubkey", "~/.ssh/id_ed25519.pub");
    ui::kv("hosts", format!("{}/", root.join("hosts").display()));
    ui::kv("order", "lan/tb first, then vpn");
    ui::kv("auth", "existing key, else one-time password");
    ui::blank();

    let report = apply::distribute_keys(root, &self_host.name, yes)?;
    if report.ok.is_empty() && report.skip.is_empty() && report.fail.is_empty() {
        ui::empty("nothing to do");
    }
    for line in &report.ok {
        ui::ok("copy", line);
    }
    for line in &report.skip {
        ui::skip("copy", line);
    }
    for line in &report.fail {
        ui::fail("copy", line);
    }
    if !yes {
        ui::preview("copy keys");
    }
    Ok(())
}
