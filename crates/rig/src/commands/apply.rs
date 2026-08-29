use crate::apply::{self, build_plan};
use crate::error::RigError;
use crate::schema;
use crate::ui;
use miette::Result;

pub fn run(root: &std::path::Path, yes: bool, skip_packages: bool) -> Result<()> {
    let hosts = schema::load_hosts(root)?;
    let host = schema::detect_current_host(&hosts).ok_or_else(|| {
        RigError::Msg(format!(
            "this machine is not registered (hostname={}). run `rig init` first",
            schema::current_hostname()
        ))
    })?;
    let role = schema::load_role(root, &host.role)?;
    let plan = build_plan(host, &role);

    ui::title("apply", !yes);
    ui::kv("root", root.display());
    ui::kv("host", &plan.host);
    ui::kv("role", &plan.role);
    ui::kv(
        "os",
        format!(
            "{}{}",
            plan.os,
            if host.os.is_none() { "  detected" } else { "" }
        ),
    );
    ui::kv("shell", &plan.shell);
    ui::kv("user", &plan.user);
    ui::kv(
        "packages",
        format!(
            "{}{}",
            plan.package_sets.join(" + "),
            if skip_packages { "  skip" } else { "" }
        ),
    );
    ui::blank();
    for step in &plan.steps {
        let do_it = !(step.skip || (skip_packages && step.id == "packages"));
        ui::plan(do_it, &step.id, &step.detail);
    }

    if !yes {
        ui::preview("apply");
        return Ok(());
    }

    apply::execute(root, host, &role, yes, skip_packages)?;
    Ok(())
}
