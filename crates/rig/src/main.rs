mod apply;
mod commands;
mod embed;
mod error;
mod packages;
mod paths;
mod schema;
mod ui;

use clap::{Parser, Subcommand};
use miette::{Report, Result};

// Per-target help: each release binary only mentions its own default data dir.
#[cfg(target_os = "macos")]
const AFTER_HELP: &str = "\
data
  hosts      this host + peers  ([[ssh]], packages add/remove)
  overlay    personal shell / tmux / Cursor overrides
  templates  product defaults — do not edit; use overlay/

  default    ~/Library/Application Support/dev.rig.rig/product/
  path       rig root
";

#[cfg(target_os = "linux")]
const AFTER_HELP: &str = "\
data
  hosts      this host + peers  ([[ssh]], packages add/remove)
  overlay    personal shell / tmux / Cursor overrides
  templates  product defaults — do not edit; use overlay/

  default    ~/.local/share/rig/product/
  path       rig root
";

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
const AFTER_HELP: &str = "\
data
  hosts      this host + peers  ([[ssh]], packages add/remove)
  overlay    personal shell / tmux / Cursor overrides
  templates  product defaults — do not edit; use overlay/

  default    pass --root / RIG_ROOT
  path       rig root
";

#[derive(Parser, Debug)]
#[command(
    name = "rig",
    version,
    about = "Opinionated setup for workstation and compute machines",
    after_help = AFTER_HELP
)]
struct Cli {
    /// Product root containing hosts/ overlay/ templates/ (cwd / RIG_ROOT / embedded)
    #[arg(long, global = true, env = "RIG_ROOT")]
    root: Option<std::path::PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create local host.toml and print paths to edit
    #[command(after_help = AFTER_HELP)]
    Init {
        /// Role to seed: workstation or compute
        #[arg(long, default_value = "workstation")]
        role: String,
        /// Inventory name / SSH prefix (defaults to current short hostname for matching)
        #[arg(long)]
        name: Option<String>,
    },
    /// Show registered hosts
    #[command(subcommand)]
    Host(HostCmd),
    /// Apply configuration for this machine (preview; `--yes` writes)
    #[command(after_help = AFTER_HELP)]
    Apply {
        #[arg(short = 'y', long = "yes")]
        yes: bool,
        /// Skip brew/apt (shell + ssh-config + state only; useful in testenv)
        #[arg(long)]
        skip_packages: bool,
    },
    /// Remove apply artifacts (preview; `--yes` deletes)
    Clean {
        #[arg(short = 'y', long = "yes")]
        yes: bool,
        #[arg(long)]
        packages: bool,
    },
    /// Generate SSH config from hosts (preview; `--yes` writes)
    SshConfig {
        #[arg(short = 'y', long = "yes", visible_alias = "write")]
        yes: bool,
    },
    /// Probe peer connectivity (TCP/22 + BatchMode SSH per path)
    Check,
    /// Manage SSH keys for passwordless peer access
    #[command(subcommand)]
    Keys(KeysCmd),
    /// List roles with packages, features, and shell settings
    Roles {
        /// Role name (omit to show all)
        name: Option<String>,
        /// Show packages for one OS only: macos | linux
        #[arg(long)]
        os: Option<String>,
    },
    /// Snapshot of this machine (host, apply, enabled live, ssh)
    Status,
    /// Print product data root (hosts / overlay live here)
    #[command(after_help = AFTER_HELP)]
    Root,
    /// Replace ~/.local/bin/rig with a GitHub Release binary (preview; `--yes` installs)
    Update {
        /// Release tag (default: latest)
        #[arg(long)]
        tag: Option<String>,
        #[arg(short = 'y', long = "yes")]
        yes: bool,
        /// Reinstall even if versions match
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand, Debug)]
enum HostCmd {
    List,
    Detect,
}

#[derive(Subcommand, Debug)]
enum KeysCmd {
    /// Copy this machine's pubkey to peers (prefer lan/tb links, then vpn)
    Distribute {
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },
}

fn main() {
    if let Err(report) = try_main() {
        print_error(&report);
        std::process::exit(1);
    }
}

fn print_error(report: &Report) {
    crate::ui::error(format!("{report}"));
    let mut causes = report.chain();
    let _ = causes.next();
    for c in causes {
        crate::ui::error_cause(c.to_string());
    }
    if let Some(help) = report.help() {
        crate::ui::error_help(help);
    }
}

fn try_main() -> Result<()> {
    // Empty RIG_ROOT= must not be treated as --root with a missing value.
    if std::env::var_os("RIG_ROOT").is_some_and(|v| v.is_empty()) {
        std::env::remove_var("RIG_ROOT");
    }

    let cli = Cli::parse();
    let root = paths::discover_root(cli.root)?;

    // Always remind — paths are easy to forget. stderr keeps stdout pipe-safe.
    if !matches!(
        cli.command,
        Commands::Root | Commands::Status | Commands::Update { .. }
    ) {
        paths::eprint_data_hint(&root);
    }

    match cli.command {
        Commands::Init { role, name } => commands::init::run(&root, &role, name.as_deref())?,
        Commands::Host(HostCmd::List) => commands::host::list(&root)?,
        Commands::Host(HostCmd::Detect) => commands::host::detect(&root)?,
        Commands::Apply { yes, skip_packages } => commands::apply::run(&root, yes, skip_packages)?,
        Commands::Clean { yes, packages } => commands::clean::run(&root, yes, packages)?,
        Commands::SshConfig { yes } => commands::ssh_config::run(&root, yes)?,
        Commands::Check => commands::check::run(&root)?,
        Commands::Keys(KeysCmd::Distribute { yes }) => commands::keys::distribute(&root, yes)?,
        Commands::Roles { name, os } => {
            commands::roles::run(&root, name.as_deref(), os.as_deref())?
        }
        Commands::Status => commands::status::run(&root)?,
        Commands::Root => {
            // First line = path only (scripts: `$(rig root | head -1)`).
            println!("{}", root.display());
            crate::ui::kv("os", schema::detect_os().as_str());
            crate::ui::kvc("(auto-detect; override in hosts/*.toml)");
            crate::ui::kv("hosts", format!("{}/", root.join("hosts").display()));
            crate::ui::kv("overlay", format!("{}/", root.join("overlay").display()));
            crate::ui::kv(
                "templates",
                format!(
                    "{}/  (product — prefer overlay/)",
                    root.join("templates").display()
                ),
            );
        }
        Commands::Update { tag, yes, force } => commands::update::run(tag.as_deref(), yes, force)?,
    }
    Ok(())
}
