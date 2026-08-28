use crate::error::{Result, RigError};
use crate::paths;
use crate::schema::ShellKind;
use std::fs;
use std::path::{Path, PathBuf};

const BEGIN: &str = "# >>> begin rig >>>";
const END: &str = "# <<< end rig <<<";

pub struct LinkReport {
    pub config_dir: PathBuf,
    pub written: Vec<PathBuf>,
    pub touched_rcs: Vec<PathBuf>,
}

pub fn config_dir() -> PathBuf {
    directories::ProjectDirs::from("dev", "rig", "rig")
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".rig-config"))
}

/// Copy shell templates into ~/.config/rig/shell and ensure rc files source them.
pub fn link_shell(root: &Path, shell: ShellKind) -> Result<LinkReport> {
    let cfg = config_dir();
    let shell_dir = cfg.join("shell");
    fs::create_dir_all(&shell_dir).map_err(RigError::Io)?;

    let mut written = Vec::new();
    let common_src = paths::templates_dir(root).join("shell/common/profile.sh");
    let common_dst = shell_dir.join("common.sh");
    copy_file(&common_src, &common_dst)?;
    written.push(common_dst);

    let (rc_name, profile_name, tpl_rc, tpl_profile) = match shell {
        ShellKind::Zsh => (
            ".zshrc",
            ".zprofile",
            "shell/zsh/zshrc",
            "shell/zsh/zprofile",
        ),
        ShellKind::Bash => (
            ".bashrc",
            ".bash_profile",
            "shell/bash/bashrc",
            "shell/bash/bash_profile",
        ),
    };

    let rc_dst = shell_dir.join(match shell {
        ShellKind::Zsh => "zshrc",
        ShellKind::Bash => "bashrc",
    });
    let profile_dst = shell_dir.join(match shell {
        ShellKind::Zsh => "zprofile",
        ShellKind::Bash => "bash_profile",
    });
    copy_file(&paths::templates_dir(root).join(tpl_rc), &rc_dst)?;
    copy_file(
        &paths::templates_dir(root).join(tpl_profile),
        &profile_dst,
    )?;
    written.push(rc_dst.clone());
    written.push(profile_dst.clone());

    let home = dirs_home()?;
    let mut touched = Vec::new();
    let snippet = managed_snippet(root, &cfg, shell);
    for name in [rc_name, profile_name] {
        let path = home.join(name);
        ensure_snippet(&path, &snippet)?;
        touched.push(path);
    }

    Ok(LinkReport {
        config_dir: cfg,
        written,
        touched_rcs: touched,
    })
}

fn managed_snippet(root: &Path, cfg: &Path, shell: ShellKind) -> String {
    let rc = match shell {
        ShellKind::Zsh => "zshrc",
        ShellKind::Bash => "bashrc",
    };
    format!(
        "{BEGIN}\n\
         # managed by rig — do not edit this block\n\
         export RIG_ROOT=\"{root}\"\n\
         export RIG_CONFIG=\"{cfg}\"\n\
         [ -f \"$RIG_CONFIG/shell/common.sh\" ] && . \"$RIG_CONFIG/shell/common.sh\"\n\
         # optional product rc (PROMPT etc.): touch \"$RIG_CONFIG/shell/use-product-rc\"\n\
         [ -f \"$RIG_CONFIG/shell/use-product-rc\" ] && [ -f \"$RIG_CONFIG/shell/{rc}\" ] && . \"$RIG_CONFIG/shell/{rc}\"\n\
         {END}\n",
        root = root.display(),
        cfg = cfg.display(),
    )
}

fn ensure_snippet(path: &Path, snippet: &str) -> Result<()> {
    let existing = if path.is_file() {
        fs::read_to_string(path).map_err(RigError::Io)?
    } else {
        String::new()
    };

    let new_body = if let (Some(start), Some(end)) =
        (existing.find(BEGIN), existing.find(END))
    {
        let end_at = end + END.len();
        let mut out = String::new();
        out.push_str(&existing[..start]);
        out.push_str(snippet);
        // skip trailing newline after END if present once
        let rest = existing[end_at..].trim_start_matches('\n');
        if !rest.is_empty() {
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(rest);
            if !out.ends_with('\n') {
                out.push('\n');
            }
        }
        out
    } else if existing.is_empty() {
        snippet.to_string()
    } else {
        let mut out = existing;
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push('\n');
        out.push_str(snippet);
        out
    };

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(RigError::Io)?;
    }
    fs::write(path, new_body).map_err(RigError::Io)?;
    Ok(())
}

fn copy_file(src: &Path, dst: &Path) -> Result<()> {
    if !src.is_file() {
        return Err(RigError::Msg(format!("missing template: {}", src.display())).into());
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(RigError::Io)?;
    }
    fs::copy(src, dst).map_err(RigError::Io)?;
    Ok(())
}

fn dirs_home() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| RigError::Msg("HOME is not set".into()))
}
