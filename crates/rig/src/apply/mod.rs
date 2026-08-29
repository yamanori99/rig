use crate::error::{Result, RigError};
use crate::schema::{Host, Role};
use crate::ui;
use std::path::Path;

mod clean;
mod cursor;
mod features;
mod gui;
mod inspect;
mod keys;
mod link;
mod omz;
mod packages;
mod plan;
mod ssh;
mod state;
mod tmux;

pub use clean::execute as clean;
pub use inspect::{print_live, LiveWanted};
pub use keys::distribute as distribute_keys;
pub use plan::build_plan;
pub use ssh::{generate as generate_ssh_config, write_ssh_config};
pub use state::load as load_state;

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
            "refusing to modify the system without --yes (-y)".into(),
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

    ui::blank();
    ui::section("run");

    st.note_step("validate", format!("role={} ok", host.role));
    ui::ok("validate", "");

    let thick_zsh = matches!(shell, crate::schema::ShellKind::Zsh);
    if thick_zsh {
        let omz = omz::ensure_omz_stack()?;
        st.note_step("omz", &omz.detail);
        if omz.ok {
            ui::ok("omz", &omz.detail);
        } else {
            ui::fail("omz", &omz.detail);
            let _ = state::save(&st);
            return Err(RigError::Msg(format!("omz failed: {}", omz.detail)));
        }
    }

    let link = link::link_shell(root, shell, thick_zsh)?;
    for p in &link.written {
        st.note_file(p);
    }
    for p in &link.touched_rcs {
        st.note_file(p);
    }
    st.note_step(
        "link-shell",
        format!(
            "config={} files={} rcs={} product_rc={}",
            link.config_dir.display(),
            link.written.len(),
            link.touched_rcs.len(),
            thick_zsh
        ),
    );
    ui::ok("link-shell", &format!("→ {}", link.config_dir.display()));
    if !link.sources.is_empty() {
        ui::item(format!("sources  {}", link.sources.join(", ")));
    }
    for p in &link.touched_rcs {
        ui::item(format!("snippet  {}", p.display()));
    }
    if thick_zsh {
        ui::item("product rc  OMZ + p10k");
    } else {
        ui::item("thin shell  common.sh; product rc optional");
    }

    let tmux = tmux::link_tmux(root)?;
    if let Some(p) = &tmux.linked {
        st.note_file(p);
    }
    for p in &tmux.extra {
        st.note_file(p);
    }
    st.note_step("link-tmux", &tmux.detail);
    ui::ok("link-tmux", &tmux.detail);

    if skip_packages || plan.package_sets.is_empty() {
        let reason = if skip_packages {
            "skipped (--skip-packages)"
        } else {
            "skipped (empty)"
        };
        st.note_step("packages", reason);
        ui::skip("packages", reason);
    } else {
        let report = packages::apply_packages(root, &plan.package_sets, os)?;
        st.note_step("packages", format!("{}: {}", report.backend, report.detail));
        if report.ok {
            ui::ok("packages", &format!("{}  {}", report.backend, report.detail));
        } else {
            ui::fail("packages", &format!("{}  {}", report.backend, report.detail));
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
    ui::ok("ssh-config", &format!("→ {}", ssh_path.display()));

    for step in &plan.steps {
        match step.id.as_str() {
            "remote-login" if !step.skip => {
                let report = features::apply_remote_login(os)?;
                finish_step(&mut st, "remote-login", report)?;
            }
            "thunderbolt" if !step.skip => {
                let ip = host.thunderbolt_ip().ok_or_else(|| {
                    RigError::Msg(
                        "thunderbolt step enabled but no [[ssh]] with link=thunderbolt".into(),
                    )
                })?;
                let report = features::apply_thunderbolt(&ip, os)?;
                finish_step(&mut st, "thunderbolt", report)?;
            }
            "tailscale" if !step.skip => {
                let report = features::apply_tailscale(os)?;
                finish_step(&mut st, "tailscale", report)?;
            }
            "stay-awake" if !step.skip => {
                let report = features::apply_stay_awake(os)?;
                finish_step(&mut st, "stay-awake", report)?;
            }
            "cursor" if !step.skip => {
                let (report, files) = cursor::apply_cursor(root, os)?;
                for p in &files {
                    st.note_file(p);
                }
                finish_step(&mut st, "cursor", report)?;
            }
            "gui" if !step.skip => {
                let report = gui::apply_gui(root, &plan.package_sets, os)?;
                finish_step(&mut st, "gui", report)?;
            }
            "gui" | "cursor" | "tailscale" | "thunderbolt" | "remote-login" | "stay-awake" => {
                // Enabled steps are handled above; remaining matches are skips.
                if step.skip {
                    st.note_step(&step.id, "skipped");
                    ui::skip(&step.id, &step.detail);
                }
            }
            _ => {}
        }
    }

    let state_path = state::save(&st)?;
    ui::blank();
    ui::kv("state", state_path.display());
    ui::title("done", false);
    ui::blank();
    ui::section("customize");
    ui::kv("host", format!("{}/hosts/{}.toml", root.display(), host.name));
    ui::kv("overlay", format!("{}/overlay/", root.display()));
    ui::item("leave templates/ alone — use overlay/");
    ui::kv("peers", "add hosts/<peer>.toml with [[ssh]], then rig ssh-config --yes");
    Ok(())
}

fn finish_step(st: &mut state::RigState, id: &str, report: features::StepReport) -> Result<()> {
    st.note_step(id, &report.detail);
    if report.ok {
        ui::ok(id, &report.detail);
        Ok(())
    } else {
        ui::fail(id, &report.detail);
        let _ = state::save(st);
        Err(RigError::Msg(format!("{id} failed: {}", report.detail)))
    }
}
