use crate::apply;
use crate::error::RigError;
use crate::schema;
use crate::ui;
use miette::Result;

pub fn run(root: &std::path::Path, yes: bool) -> Result<()> {
    let hosts = schema::load_hosts(root)?;
    let self_host = schema::detect_current_host(&hosts)
        .ok_or_else(|| RigError::Msg(schema::unregistered_hint(root, &hosts)))?;

    ui::title("host keys", !yes);
    ui::kv("self", &self_host.name);
    ui::blank();

    let report = apply::distribute_keys(root, &self_host.name, yes)?;
    if let Some(key) = &report.key {
        ui::ok("key", key);
    }
    if report.ok.is_empty()
        && report.skip.is_empty()
        && report.fail.is_empty()
        && report.key.is_none()
    {
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
        ui::preview("write key and copy");
    }
    Ok(())
}
