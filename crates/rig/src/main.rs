mod apply;
mod commands;
mod embed;
mod error;
mod packages;
mod paths;
mod schema;
mod ui;

use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{ArgAction, ColorChoice, CommandFactory, Parser, Subcommand};
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
    let h = ui::help_head;
    let r = ui::help_row;
    format!(
        "\
{}
{}
{}
{}
{}

{}
{}
{}
{}

{}
{}
{}
{}
{}

{}
{}
{}
{}
{}
",
        h("Commands"),
        r("init, i", "Write hosts/<name>.toml"),
        r("apply, a", "Apply this host (preview; -y writes)"),
        r("status, s", "Show this machine"),
        r("host, h", "Peers and SSH"),
        h("Options"),
        r("-v", "Print version"),
        r("-r", "Product root (cwd / RIG_ROOT / unpack)"),
        r("-h", "Print help (see --help)"),
        h("Flow"),
        r("init", "-R workstation|compute"),
        r("apply", "-y"),
        r("host", "check"),
        r("host", "keys -y"),
        h("Layout"),
        r("hosts/", "This host + peers"),
        r("overlay/", "Your shell / tmux / Cursor"),
        r("templates/", "Product defaults — do not edit"),
        r("product/", DATA_DEFAULT),
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
thunderbolt, stay-awake). --skip-packages leaves brew/apt alone.

--undo reverses apply (preview; --yes deletes). --packages with
--undo also uninstalls role brew/apt packages.";

const CLEAN_LONG: &str = "\
Preview deletions, then run with --yes.

Same as `rig apply --undo`. Removes apply state and managed links.
--packages also uninstalls role brew/apt packages (destructive).";

const SSH_CONFIG_LONG: &str = "\
Build ~/.ssh/config.d/rig.conf from every hosts/*.toml [[ssh]] path.

Preview first; --yes (alias --write) installs the Include snippet
in ~/.ssh/config if needed.";

const CHECK_LONG: &str = "\
For each peer path: TCP/22, then ssh -o BatchMode=yes.

Does not copy keys. Use `rig host keys -y` after check fails
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
Make pubkey login work on peers.

--yes creates ~/.ssh/id_ed25519 if missing (empty passphrase),
then copies the pubkey. Password is asked once per new peer.
Preview without --yes.";

#[derive(Parser, Debug)]
#[command(
    name = "rig",
    version,
    disable_version_flag = true,
    propagate_version = true,
    arg_required_else_help = true,
    subcommand_required = true,
    disable_help_subcommand = true,
    color = ColorChoice::Auto,
    styles = clap_styles(),
    about = "Opinionated setup for workstation and compute machines",
    long_about = LONG_ABOUT,
    before_help = crate::ui::banner(),
    after_help = after_help(),
    help_template = "\
{before-help}\
{about-with-newline}\
{usage-heading} {usage}{after-help}\
"
)]
struct Cli {
    /// Print version
    #[arg(short = 'v', short_alias = 'V', long = "version", action = ArgAction::Version, global = true)]
    _version: (),

    /// Product root (cwd / RIG_ROOT / embedded unpack)
    #[arg(
        short = 'r',
        long,
        global = true,
        env = "RIG_ROOT",
        hide_env_values = true
    )]
    root: Option<std::path::PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Write hosts/<name>.toml
    #[command(alias = "i", long_about = INIT_LONG)]
    Init {
        /// Role: workstation or compute
        #[arg(short = 'R', long, default_value = "workstation")]
        role: String,
        /// Inventory name (default: short hostname)
        #[arg(short = 'n', long)]
        name: Option<String>,
    },
    /// Apply this host (preview; -y writes)
    #[command(alias = "a", long_about = APPLY_LONG)]
    Apply {
        /// Write (default is preview)
        #[arg(short = 'y', long = "yes")]
        yes: bool,
        /// Skip brew/apt (shell + ssh-config + features)
        #[arg(short = 'S', long, conflicts_with = "undo")]
        skip_packages: bool,
        /// Reverse apply (preview; -y deletes)
        #[arg(long = "undo")]
        undo: bool,
        /// With --undo, also uninstall role packages
        #[arg(short = 'p', long, requires = "undo")]
        packages: bool,
    },
    /// Show this machine
    #[command(alias = "s", long_about = STATUS_LONG)]
    Status,
    /// Peers and SSH
    #[command(alias = "h", subcommand)]
    Host(HostCmd),
    /// Role features and packages
    #[command(hide = true, long_about = ROLES_LONG)]
    Roles {
        /// Role name (omit for all)
        name: Option<String>,
        /// macos or linux
        #[arg(short = 'o', long)]
        os: Option<String>,
    },
    /// Undo apply links (preview; -y deletes)
    #[command(hide = true, long_about = CLEAN_LONG)]
    Clean {
        /// Delete (default is preview)
        #[arg(short = 'y', long = "yes")]
        yes: bool,
        /// Also uninstall role packages
        #[arg(short = 'p', long)]
        packages: bool,
    },
    /// Install a GitHub Release
    #[command(hide = true, visible_alias = "u", long_about = UPDATE_LONG)]
    Update {
        /// Release tag (default: latest)
        #[arg(short = 't', long)]
        tag: Option<String>,
        /// Install (default is preview)
        #[arg(short = 'y', long = "yes")]
        yes: bool,
        /// Reinstall even if versions match
        #[arg(short = 'f', long)]
        force: bool,
    },
    /// Print the product data path
    #[command(hide = true, long_about = ROOT_LONG)]
    Root,
    /// Probe peer TCP/22 and SSH
    #[command(hide = true, visible_alias = "c", long_about = CHECK_LONG)]
    Check,
    /// Write ~/.ssh/config.d/rig.conf
    #[command(hide = true, visible_alias = "ssh", long_about = SSH_CONFIG_LONG)]
    SshConfig {
        /// Write config (alias: --write)
        #[arg(short = 'y', long = "yes", visible_alias = "write")]
        yes: bool,
    },
}

#[derive(Subcommand, Debug)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    disable_help_subcommand = true
)]
enum HostCmd {
    /// List hosts/*.toml
    #[command(long_about = HOST_LIST_LONG)]
    List,
    /// Match this hostname to a host file
    #[command(hide = true, long_about = HOST_DETECT_LONG)]
    Detect,
    /// Probe peer TCP/22 and SSH
    #[command(long_about = CHECK_LONG)]
    Check,
    /// Local ed25519 + copy to peers
    #[command(long_about = KEYS_DIST_LONG)]
    Keys {
        /// Write (default is preview)
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },
    /// Write ~/.ssh/config.d/rig.conf
    #[command(hide = true, name = "ssh-config", alias = "ssh", long_about = SSH_CONFIG_LONG)]
    SshConfig {
        /// Write config (alias: --write)
        #[arg(short = 'y', long = "yes", visible_alias = "write")]
        yes: bool,
    },
}

fn main() {
    if std::env::args_os().nth(1).is_none() {
        let mut cmd = Cli::command();
        let _ = cmd.print_help();
        std::process::exit(0);
    }
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
        Commands::Root
            | Commands::Status
            | Commands::Update { .. }
            | Commands::Host(_)
            | Commands::Check
    ) {
        paths::eprint_data_hint(&root);
    }

    match cli.command {
        Commands::Init { role, name } => commands::init::run(&root, &role, name.as_deref())?,
        Commands::Host(HostCmd::List) => commands::host::list(&root)?,
        Commands::Host(HostCmd::Detect) => commands::host::detect(&root)?,
        Commands::Host(HostCmd::Check) | Commands::Check => commands::check::run(&root)?,
        Commands::Host(HostCmd::Keys { yes }) => commands::keys::run(&root, yes)?,
        Commands::Host(HostCmd::SshConfig { yes }) | Commands::SshConfig { yes } => {
            commands::ssh_config::run(&root, yes)?
        }
        Commands::Apply {
            yes,
            skip_packages,
            undo,
            packages,
        } => {
            if undo {
                commands::clean::run(&root, yes, packages)?
            } else {
                commands::apply::run(&root, yes, skip_packages)?
            }
        }
        Commands::Clean { yes, packages } => commands::clean::run(&root, yes, packages)?,
        Commands::Roles { name, os } => {
            commands::roles::run(&root, name.as_deref(), os.as_deref())?
        }
        Commands::Status => commands::status::run(&root)?,
        Commands::Root => {
            println!("{}", root.display());
            crate::ui::title("root", false);
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
