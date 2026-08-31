use crate::packages;
use crate::schema::{self, OsKind};
use crate::ui;
use miette::Result;

pub fn run(root: &std::path::Path, name: Option<&str>, os_filter: Option<&str>) -> Result<()> {
    let os_filter = match os_filter {
        None => None,
        Some("macos") => Some(OsKind::Macos),
        Some("linux") => Some(OsKind::Linux),
        Some(other) => {
            return Err(crate::error::RigError::Msg(format!(
                "unknown --os `{other}` (want: macos, linux)"
            ))
            .into());
        }
    };

    let names = match name {
        Some(n) => {
            let all = schema::list_roles(root)?;
            if !all.iter().any(|r| r == n) {
                return Err(crate::error::RigError::Msg(format!(
                    "unknown role `{n}` (have: {})",
                    all.join(", ")
                ))
                .into());
            }
            vec![n.to_string()]
        }
        None => schema::list_roles(root)?,
    };

    if names.is_empty() {
        ui::empty("no roles in roles/");
        return Ok(());
    }

    ui::title("roles", false);
    ui::kv("root", root.display());
    ui::blank();

    for (i, role_name) in names.iter().enumerate() {
        if i > 0 {
            ui::blank();
        }
        print_role(root, role_name, os_filter)?;
    }
    Ok(())
}

fn print_role(root: &std::path::Path, name: &str, os_filter: Option<OsKind>) -> Result<()> {
    let role = schema::load_role(root, name)?;
    ui::kv("role", name);
    ui::kvc(&role.description);

    if let Some(sh) = role.default_shell {
        ui::kv("shell", sh.as_str());
        ui::note("templates", format!("shell/common + shell/{}", sh.as_str()));
    } else {
        ui::kv("shell", "detect from $SHELL");
    }

    let f = &role.features;
    ui::section("features");
    ui::note("gui", yn(f.gui));
    ui::note("cursor", yn(f.cursor));
    ui::note("remote", yn(f.remote_login));
    ui::note("screen", yn(f.screen_sharing));
    ui::note("tailscale", yn(f.tailscale));
    ui::note("thunderbolt", yn(f.thunderbolt));
    ui::note("awake", yn(f.stay_awake));

    ui::kv("packages", role.packages.join(", "));

    let show_macos = os_filter.is_none() || os_filter == Some(OsKind::Macos);
    let show_linux = os_filter.is_none() || os_filter == Some(OsKind::Linux);

    if show_macos {
        ui::section("brew  macos");
        print_packages(root, &role.packages, OsKind::Macos)?;
    }
    if show_linux {
        ui::section("apt  linux");
        print_packages(root, &role.packages, OsKind::Linux)?;
    }

    Ok(())
}

fn print_packages(root: &std::path::Path, sets: &[String], os: OsKind) -> Result<()> {
    if sets.is_empty() {
        ui::item("none");
        return Ok(());
    }
    for set_name in sets {
        let set = packages::load_package_set(root, set_name)?;
        let pkgs = packages::packages_for_os(&set, os);
        ui::note("set", set_name);
        if pkgs.is_empty() {
            ui::item2("empty or missing file");
            continue;
        }
        for pkg in pkgs {
            ui::item2(pkg);
        }
    }
    Ok(())
}

fn yn(v: bool) -> &'static str {
    if v {
        "on"
    } else {
        "off"
    }
}
