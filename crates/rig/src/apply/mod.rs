use crate::error::{Result, RigError};
use crate::schema::{Host, Role};
use std::path::Path;

mod features;
mod link;
mod packages;
mod plan;
mod ssh;
mod state;

pub use plan::build_plan;
pub use ssh::{generate as generate_ssh_config, write_ssh_config};

/// Run a real apply (caller already printed the plan). Requires `yes` for safety.
pub fn execute(
    root: &Path,
    host: &Host,
    role: &Role,
    yes: bool,
    skip_packages: bool,
) -> Result<()> {
    if !yes {
        return Err(RigError::Msg(
            "refusing to modify the system without --yes (-y); try: rig apply --dry-run"
                .into(),
        ));
    }

    let plan = build_plan(host, role);
    let shell = host.shell.unwrap_or_else(|| {
        role.default_shell
            .unwrap_or_else(crate::schema::detect_shell)
    });
    let os = host.resolved_os();

    let mut st = state::RigState::new(&plan.host, &plan.role);
    st.package_sets = plan.package_sets.clone();

    println!();
    println!("applying…");

    st.note_step("validate", format!("role={} ok", host.role));
    println!("  [ok  ] validate");

    let link = link::link_shell(root, shell)?;
    for p in &link.written {
        st.note_file(p);
    }
    for p in &link.touched_rcs {
        st.note_file(p);
    }
    st.note_step(
        "link-shell",
        format!(
            "config={} files={} rcs={}",
            link.config_dir.display(),
            link.written.len(),
            link.touched_rcs.len()
        ),
    );
    println!("  [ok  ] link-shell  → {}", link.config_dir.display());
    for p in &link.touched_rcs {
        println!("           snippet → {}", p.display());
    }
    println!("           sources common.sh only (keeps existing OMZ/p10k).");
    println!(
        "           product rc: touch {}/shell/use-product-rc",
        link.config_dir.display()
    );

    if skip_packages || plan.package_sets.is_empty() {
        let reason = if skip_packages {
            "skipped (--skip-packages)"
        } else {
            "skipped (empty)"
        };
        st.note_step("packages", reason);
        println!("  [skip] packages  ({reason})");
    } else {
        let report = packages::apply_packages(root, &plan.package_sets, os)?;
        st.note_step(
            "packages",
            format!("{}: {}", report.backend, report.detail),
        );
        if report.ok {
            println!("  [ok  ] packages  ({}) {}", report.backend, report.detail);
        } else {
            println!("  [fail] packages  ({}) {}", report.backend, report.detail);
            let _ = state::save(&st);
            return Err(RigError::Msg(format!(
                "package step failed: {}",
                report.detail
            )));
        }
    }

    let hosts = crate::schema::load_hosts(root)?;
    let ssh_path = write_ssh_config(root, &hosts)?;
    st.note_file(&ssh_path);
    st.note_step("ssh-config", ssh_path.display().to_string());
    println!("  [ok  ] ssh-config → {}", ssh_path.display());

    for step in &plan.steps {
        match step.id.as_str() {
            "hostname" => {
                let report = features::apply_hostname(&host.name, os)?;
                finish_step(&mut st, "hostname", report)?;
            }
            "remote-login" if !step.skip => {
                let report = features::apply_remote_login(os)?;
                finish_step(&mut st, "remote-login", report)?;
            }
            "thunderbolt" if !step.skip => {
                let ip = host.thunderbolt.as_deref().ok_or_else(|| {
                    RigError::Msg("thunderbolt step enabled but host.thunderbolt is empty".into())
                })?;
                let report = features::apply_thunderbolt(ip, os)?;
                finish_step(&mut st, "thunderbolt", report)?;
            }
            "gui" | "cursor" | "tailscale" | "thunderbolt" | "remote-login" => {
                if step.skip {
                    st.note_step(&step.id, "skipped");
                    println!("  [skip] {:<14} {}", step.id, step.detail);
                } else {
                    st.note_step(&step.id, format!("not implemented yet: {}", step.detail));
                    println!(
                        "  [todo] {:<14} {} (not implemented yet)",
                        step.id, step.detail
                    );
                }
            }
            _ => {}
        }
    }

    let state_path = state::save(&st)?;
    println!();
    println!("state → {}", state_path.display());
    println!("apply complete.");
    Ok(())
}

fn finish_step(
    st: &mut state::RigState,
    id: &str,
    report: features::StepReport,
) -> Result<()> {
    st.note_step(id, &report.detail);
    if report.ok {
        println!("  [ok  ] {id:<14} {}", report.detail);
        Ok(())
    } else {
        println!("  [fail] {id:<14} {}", report.detail);
        let _ = state::save(st);
        Err(RigError::Msg(format!("{id} failed: {}", report.detail)))
    }
}
