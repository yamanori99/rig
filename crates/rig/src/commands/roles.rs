use crate::packages;
use crate::schema::{self, OsKind};
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
        println!("(no roles in roles/)");
        return Ok(());
    }

    for (i, role_name) in names.iter().enumerate() {
        if i > 0 {
            println!();
            println!("{}", "-".repeat(60));
            println!();
        }
        print_role(root, role_name, os_filter)?;
    }
    Ok(())
}

fn print_role(
    root: &std::path::Path,
    name: &str,
    os_filter: Option<OsKind>,
) -> Result<()> {
    let role = schema::load_role(root, name)?;
    println!("role: {name}");
    println!("  {}", role.description);

    if let Some(sh) = role.default_shell {
        println!("  default_shell: {}", sh.as_str());
        println!(
            "  shell templates: templates/shell/common + templates/shell/{}",
            sh.as_str()
        );
    } else {
        println!("  default_shell: (detect from $SHELL)");
    }

    let f = &role.features;
    println!("  features:");
    println!(
        "    gui={}  cursor={}  remote_login={}  tailscale={}  thunderbolt={}",
        yn(f.gui),
        yn(f.cursor),
        yn(f.remote_login),
        yn(f.tailscale),
        yn(f.thunderbolt)
    );
    if f.gui {
        println!("    → GUI apps / workstation extras");
    }
    if f.cursor {
        println!("    → Cursor user settings link");
    }
    if f.remote_login {
        println!("    → remote login / sshd");
    }
    if f.tailscale {
        println!("    → Tailscale");
    }
    if f.thunderbolt {
        println!("    → Thunderbolt bridge0 when [[ssh]] has link=thunderbolt");
    }

    println!("  package sets: {}", role.packages.join(", "));

    let show_macos = os_filter.is_none() || os_filter == Some(OsKind::Macos);
    let show_linux = os_filter.is_none() || os_filter == Some(OsKind::Linux);

    if show_macos {
        println!("  brew (macos):");
        print_packages(root, &role.packages, OsKind::Macos)?;
    }
    if show_linux {
        println!("  apt (linux):");
        print_packages(root, &role.packages, OsKind::Linux)?;
    }

    Ok(())
}

fn print_packages(
    root: &std::path::Path,
    sets: &[String],
    os: OsKind,
) -> Result<()> {
    if sets.is_empty() {
        println!("    (none)");
        return Ok(());
    }
    for set_name in sets {
        let set = packages::load_package_set(root, set_name)?;
        let pkgs = packages::packages_for_os(&set, os);
        println!("    [{set_name}]");
        if pkgs.is_empty() {
            println!("      (empty or missing file)");
            continue;
        }
        for pkg in pkgs {
            println!("      {pkg}");
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
