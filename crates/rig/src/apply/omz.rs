use super::features::StepReport;
use crate::error::{Result, RigError};
use std::path::Path;
use std::process::{Command, Stdio};

/// Ensure Oh My Zsh + Powerlevel10k + common plugins (workstation / zsh).
pub fn ensure_omz_stack() -> Result<StepReport> {
    let home = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .ok_or_else(|| RigError::Msg("HOME is not set".into()))?;
    let zsh = home.join(".oh-my-zsh");
    let mut notes = Vec::new();

    if !zsh.is_dir() {
        let status = Command::new("sh")
            .args([
                "-c",
                "curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh | sh -s -- --unattended",
            ])
            .env("CHSH", "no")
            .env("RUNZSH", "no")
            .stdin(Stdio::null())
            .status()
            .map_err(RigError::Io)?;
        if !status.success() {
            return Ok(StepReport {
                ok: false,
                detail: format!("oh-my-zsh install failed ({status})"),
            });
        }
        notes.push("installed oh-my-zsh".into());
    } else {
        notes.push("oh-my-zsh present".into());
    }

    let custom = zsh.join("custom");
    ensure_git_clone(
        "https://github.com/romkatv/powerlevel10k.git",
        &custom.join("themes/powerlevel10k"),
        &mut notes,
        "powerlevel10k",
    )?;
    ensure_git_clone(
        "https://github.com/zsh-users/zsh-autosuggestions",
        &custom.join("plugins/zsh-autosuggestions"),
        &mut notes,
        "zsh-autosuggestions",
    )?;
    ensure_git_clone(
        "https://github.com/zsh-users/zsh-syntax-highlighting.git",
        &custom.join("plugins/zsh-syntax-highlighting"),
        &mut notes,
        "zsh-syntax-highlighting",
    )?;

    Ok(StepReport {
        ok: true,
        detail: notes.join("; "),
    })
}

fn ensure_git_clone(url: &str, dest: &Path, notes: &mut Vec<String>, label: &str) -> Result<()> {
    if dest.is_dir() {
        notes.push(format!("{label} present"));
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(RigError::Io)?;
    }
    let status = Command::new("git")
        .args(["clone", "--depth=1", url])
        .arg(dest)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(RigError::Io)?;
    if status.success() {
        notes.push(format!("cloned {label}"));
    } else {
        notes.push(format!("{label} clone failed ({status})"));
    }
    Ok(())
}
