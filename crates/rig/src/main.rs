mod apply;
mod commands;
mod error;
mod packages;
mod paths;
mod schema;

use clap::{Parser, Subcommand};
use miette::Result;

#[derive(Parser, Debug)]
#[command(name = "rig", version, about = "Opinionated setup for workstation and compute machines")]
struct Cli {
    /// Root of the rig project (defaults to discovery from cwd / env)
    #[arg(long, global = true, env = "RIG_ROOT")]
    root: Option<std::path::PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create local host config from an example
    Init {
        /// Role to seed: workstation or compute
        #[arg(long, default_value = "workstation")]
        role: String,
        /// Host name (defaults to current short hostname)
        #[arg(long)]
        name: Option<String>,
    },
    /// Show registered hosts
    #[command(subcommand)]
    Host(HostCmd),
    /// Apply configuration for this machine
    Apply {
        #[arg(long)]
        dry_run: bool,
        #[arg(short = 'y', long = "yes")]
        yes: bool,
        /// Skip brew/apt (shell + ssh-config + state only; useful in testenv)
        #[arg(long)]
        skip_packages: bool,
    },
    /// Show what clean would remove (and remove with --yes)
    Clean {
        #[arg(long)]
        dry_run: bool,
        #[arg(short = 'y', long = "yes")]
        yes: bool,
        #[arg(long)]
        packages: bool,
    },
    /// Generate SSH config from hosts
    SshConfig {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        write: bool,
    },
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
}

#[derive(Subcommand, Debug)]
enum HostCmd {
    List,
    Detect,
}

#[derive(Subcommand, Debug)]
enum KeysCmd {
    /// Copy this machine's pubkey to peers (-lan / -tb preferred, then -ts)
    Distribute {
        #[arg(long)]
        dry_run: bool,
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = paths::discover_root(cli.root)?;

    match cli.command {
        Commands::Init { role, name } => commands::init::run(&root, &role, name.as_deref())?,
        Commands::Host(HostCmd::List) => commands::host::list(&root)?,
        Commands::Host(HostCmd::Detect) => commands::host::detect(&root)?,
        Commands::Apply {
            dry_run,
            yes,
            skip_packages,
        } => commands::apply::run(&root, dry_run, yes, skip_packages)?,
        Commands::Clean {
            dry_run,
            yes,
            packages,
        } => commands::clean::run(&root, dry_run, yes, packages)?,
        Commands::SshConfig { dry_run, write } => {
            commands::ssh_config::run(&root, dry_run, write)?
        }
        Commands::Keys(KeysCmd::Distribute { dry_run, yes }) => {
            commands::keys::distribute(&root, yes, dry_run)?
        }
        Commands::Roles { name, os } => {
            commands::roles::run(&root, name.as_deref(), os.as_deref())?
        }
    }
    Ok(())
}
