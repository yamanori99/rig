mod apply;
mod commands;
mod embed;
mod error;
mod packages;
mod paths;
mod schema;
mod ui;

use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{ColorChoice, Parser, Subcommand};
use miette::{Report, Result};

fn clap_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::BrightBlack.on_default() | Effects::BOLD)
        .usage(AnsiColor::BrightBlack.on_default() | Effects::BOLD)
        .literal(AnsiColor::Cyan.on_default())
        .placeholder(AnsiColor::BrightBlack.on_default())
        .error(AnsiColor::Red.on_default() | Effects::BOLD)
        .valid(AnsiColor::Green.on_default())
        .invalid(AnsiColor::Red.on_default())
}

const LONG_ABOUT: &str = "\
Opinionated setup for workstation and compute machines.

Roles drive shell, packages, and features. Host files under the product
root hold identity and [[ssh]] paths. Overlay holds personal edits;
do not change templates/.";

#[cfg(target_os = "macos")]
const DATA_DEFAULT: &str = "~/Library/Application Support/dev.rig.rig/product/";

#[cfg(target_os = "linux")]
const DATA_DEFAULT: &str = "~/.local/share/rig/product/";

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
const DATA_DEFAULT: &str = "pass --root / RIG_ROOT";

fn after_help() -> String {
    format!(
        "\
Typical flow:
  rig init --role workstation|compute
  edit hosts/<name>.toml  (and peer files with [[ssh]])
  rig apply --yes
  rig check
  rig keys distribute --yes

Data (rig root):
  hosts      this host + peers  ([[ssh]], packages add/remove)
  overlay    personal shell / tmux / Cursor overrides
  templates  product defaults — do not edit; use overlay/
  default    {DATA_DEFAULT}
  path       rig root   (or --root / RIG_ROOT)

Examples:
  rig status
  rig apply
  rig apply -y
  rig roles compute --os macos
  rig host detect
"
    )
}

const INIT_LONG: &str = "\
Write hosts/<name>.toml from a role template (workstation or compute).

`name` defaults to the short hostname so `rig apply` can match this
machine. Edit [[ssh]] on peer files, not only on this host.";

const APPLY_LONG: &str = "\
Preview the plan, then write with --yes.

Links shell/tmux, installs brew/apt sets, writes ssh config, then
role features (gui, cursor, remote-login, screen-sharing, tailscale,
thunderbolt, stay-awake). --skip-packages leaves brew/apt alone.";

const CLEAN_LONG: &str = "\
Preview deletions, then run with --yes.

Removes apply state and managed links. --packages also uninstalls
role brew/apt packages (destructive).";

const SSH_CONFIG_LONG: &str = "\
Build ~/.ssh/config.d/rig.conf from every hosts/*.toml [[ssh]] path.

Preview first; --yes (alias --write) installs the Include snippet
in ~/.ssh/config if needed.";

const CHECK_LONG: &str = "\
For each peer path: TCP/22, then ssh -o BatchMode=yes.

Does not copy keys. Use `rig keys distribute` after check fails
on auth.";

const ROLES_LONG: &str = "\
Print role features and package lists from roles/ + packages/.

Omit NAME to list every role. --os macos|linux filters brew vs apt.";

const STATUS_LONG: &str = "\
This machine: matched host, features, extra packages, last apply,
live probes, generated ssh config.";

const ROOT_LONG: &str = "\
Print the product data root. First stdout line is the path only
(scripts: `$(rig root | head -1)`).";

const UPDATE_LONG: &str = "\
Install a GitHub Release binary to ~/.local/bin/rig.

Preview first; --yes downloads. --force reinstalls the same tag.";

const HOST_LIST_LONG: &str = "\
List hosts/*.toml: name, role, os, shell, network paths.";

const HOST_DETECT_LONG: &str = "\
Match the current short hostname to hosts/<name>.toml.

Apply uses the same match. If none, run `rig init`.";

const KEYS_DIST_LONG: &str = "\
Install ~/.ssh/id_ed25519.pub on peers (ssh-copy-id).

If the key is already there, this is silent. Otherwise --yes
prompts for the peer login password once (needs a TTY).
Prefers lan/thunderbolt, then vpn. Preview without --yes.";

#[derive(Parser, Debug)]
#[command(
    name = "rig",
    version,
    color = ColorChoice::Auto,
    styles = clap_styles(),
    about = "Opinionated setup for workstation and compute machines",
    long_about = LONG_ABOUT,
    after_help = after_help()
)]
struct Cli {
    /// Product root (cwd / RIG_ROOT / embedded unpack)
    #[arg(long, global = true, env = "RIG_ROOT")]
    root: Option<std::path::PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Seed hosts/<name>.toml from a role
    #[command(long_about = INIT_LONG)]
    Init {
        /// Role: workstation or compute
        #[arg(long, default_value = "workstation")]
        role: String,
        /// Inventory name (default: short hostname)
        #[arg(long)]
        name: Option<String>,
    },
    /// List or detect registered hosts
    #[command(subcommand)]
    Host(HostCmd),
    /// Apply this machine's role (preview; --yes writes)
    #[command(long_about = APPLY_LONG)]
    Apply {
        /// Write (default is preview)
        #[arg(short = 'y', long = "yes")]
        yes: bool,
        /// Skip brew/apt (shell + ssh-config + features)
        #[arg(long)]
        skip_packages: bool,
    },
    /// Remove apply artifacts (preview; --yes deletes)
    #[command(long_about = CLEAN_LONG)]
    Clean {
        /// Delete (default is preview)
        #[arg(short = 'y', long = "yes")]
        yes: bool,
        /// Also uninstall role packages
        #[arg(long)]
        packages: bool,
    },
    /// Generate SSH config from hosts (preview; --yes writes)
    #[command(long_about = SSH_CONFIG_LONG)]
    SshConfig {
        /// Write config (alias: --write)
        #[arg(short = 'y', long = "yes", visible_alias = "write")]
        yes: bool,
    },
    /// Probe peer TCP/22 and BatchMode SSH
    #[command(long_about = CHECK_LONG)]
    Check,
    /// Copy SSH keys to peers
    #[command(subcommand)]
    Keys(KeysCmd),
    /// Show role features and packages
    #[command(long_about = ROLES_LONG)]
    Roles {
        /// Role name (omit for all)
        name: Option<String>,
        /// macos or linux
        #[arg(long)]
        os: Option<String>,
    },
    /// Snapshot host, extras, apply, live, ssh
    #[command(long_about = STATUS_LONG)]
    Status,
    /// Print product data root
    #[command(long_about = ROOT_LONG)]
    Root,
    /// Install a GitHub Release binary (preview; --yes)
    #[command(long_about = UPDATE_LONG)]
    Update {
        /// Release tag (default: latest)
        #[arg(long)]
        tag: Option<String>,
        /// Install (default is preview)
        #[arg(short = 'y', long = "yes")]
        yes: bool,
        /// Reinstall even if versions match
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand, Debug)]
enum HostCmd {
    /// List hosts/*.toml
    #[command(long_about = HOST_LIST_LONG)]
    List,
    /// Match this hostname to a host file
    #[command(long_about = HOST_DETECT_LONG)]
    Detect,
}

#[derive(Subcommand, Debug)]
enum KeysCmd {
    /// Copy this pubkey to peers (preview; --yes)
    #[command(long_about = KEYS_DIST_LONG)]
    Distribute {
        /// Copy keys (default is preview)
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
    if std::env::var_os("RIG_ROOT").is_some_and(|v| v.is_empty()) {
        std::env::remove_var("RIG_ROOT");
    }

    let cli = Cli::parse();
    let root = paths::discover_root(cli.root)?;

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
