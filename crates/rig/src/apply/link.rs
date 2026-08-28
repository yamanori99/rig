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
    pub sources: Vec<String>,
}

pub fn config_dir() -> PathBuf {
    directories::ProjectDirs::from("dev", "rig", "rig")
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".rig-config"))
}

/// Copy shell templates into ~/.config/rig/shell and ensure rc files source them.
/// Prefers `overlay/shell/...` over `templates/shell/...` when present.
///
/// When `enable_product_rc` is true (workstation / zsh), create `use-product-rc`
/// so the thick product zshrc (OMZ + p10k) is sourced.
pub fn link_shell(root: &Path, shell: ShellKind, enable_product_rc: bool) -> Result<LinkReport> {
    let cfg = config_dir();
    let shell_dir = cfg.join("shell");
    fs::create_dir_all(&shell_dir).map_err(RigError::Io)?;

    let mut written = Vec::new();
    let mut sources = Vec::new();

    let (common_src, common_kind) = resolve_shell_file(root, "shell/common/profile.sh")?;
    let common_dst = shell_dir.join("common.sh");
    copy_file(&common_src, &common_dst)?;
    written.push(common_dst);
    sources.push(format!("common←{common_kind}"));

    let (rc_name, profile_name, tpl_rc, tpl_profile, rc_dst_name, profile_dst_name) =
        match shell {
            ShellKind::Zsh => (
                ".zshrc",
                ".zprofile",
                "shell/zsh/zshrc",
                "shell/zsh/zprofile",
                "zshrc",
                "zprofile",
            ),
            ShellKind::Bash => (
                ".bashrc",
                ".bash_profile",
                "shell/bash/bashrc",
                "shell/bash/bash_profile",
                "bashrc",
                "bash_profile",
            ),
        };

    let (rc_src, rc_kind) = resolve_shell_file(root, tpl_rc)?;
    let (profile_src, profile_kind) = resolve_shell_file(root, tpl_profile)?;
    let rc_dst = shell_dir.join(rc_dst_name);
    let profile_dst = shell_dir.join(profile_dst_name);
    copy_file(&rc_src, &rc_dst)?;
    copy_file(&profile_src, &profile_dst)?;
    written.push(rc_dst.clone());
    written.push(profile_dst);
    sources.push(format!("rc←{rc_kind}"));
    sources.push(format!("profile←{profile_kind}"));

    if matches!(shell, ShellKind::Zsh) {
        // p10k + helper assets for workstation product shell
        if let Ok((p10k_src, kind)) = resolve_shell_file(root, "shell/zsh/p10k.zsh") {
            let p10k_dst = shell_dir.join("p10k.zsh");
            copy_file(&p10k_src, &p10k_dst)?;
            written.push(p10k_dst.clone());
            sources.push(format!("p10k←{kind}"));
            // Keep ~/.p10k.zsh pointing at managed copy (p10k tooling expects this path).
            let home_p10k = dirs_home()?.join(".p10k.zsh");
            link_or_replace_symlink(&p10k_dst, &home_p10k)?;
            written.push(home_p10k);
        }
        copy_tree_if_present(
            root,
            "shell/scripts",
            &shell_dir.join("scripts"),
            &mut written,
            &mut sources,
        )?;
        copy_tree_if_present(
            root,
            "shell/fastfetch",
            &shell_dir.join("fastfetch"),
            &mut written,
            &mut sources,
        )?;
        if let Ok((tips_src, kind)) = resolve_shell_file(root, "shell/tips.txt") {
            let tips_dst = shell_dir.join("tips.txt");
            copy_file(&tips_src, &tips_dst)?;
            written.push(tips_dst);
            sources.push(format!("tips←{kind}"));
        }
        // Make scripts executable
        if let Ok(entries) = fs::read_dir(shell_dir.join("scripts")) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_file() {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = fs::set_permissions(&p, fs::Permissions::from_mode(0o755));
                    }
                }
            }
        }
    }

    let marker = shell_dir.join("use-product-rc");
    if enable_product_rc {
        fs::write(&marker, b"").map_err(RigError::Io)?;
        written.push(marker);
        sources.push("product-rc=on".into());
    }

    let home = dirs_home()?;
    let mut touched = Vec::new();
    // Login profile: PATH / common only. Interactive product rc (fastfetch etc.)
    // must not run here — login shells also source ~/.zshrc afterward.
    let profile_snip = managed_snippet(root, &cfg, shell, false);
    let rc_snip = managed_snippet(root, &cfg, shell, true);
    let profile_path = home.join(profile_name);
    ensure_snippet(&profile_path, &profile_snip)?;
    touched.push(profile_path);
    let rc_path = home.join(rc_name);
    ensure_snippet(&rc_path, &rc_snip)?;
    touched.push(rc_path);

    Ok(LinkReport {
        config_dir: cfg,
        written,
        touched_rcs: touched,
        sources,
    })
}

fn copy_tree_if_present(
    root: &Path,
    rel: &str,
    dst_dir: &Path,
    written: &mut Vec<PathBuf>,
    sources: &mut Vec<String>,
) -> Result<()> {
    let overlay = root.join("overlay").join(rel);
    let tpl = paths::templates_dir(root).join(rel);
    let src_dir = if overlay.is_dir() {
        sources.push(format!("{rel}←overlay"));
        overlay
    } else if tpl.is_dir() {
        sources.push(format!("{rel}←templates"));
        tpl
    } else {
        return Ok(());
    };
    fs::create_dir_all(dst_dir).map_err(RigError::Io)?;
    for e in fs::read_dir(&src_dir).map_err(RigError::Io)? {
        let e = e.map_err(RigError::Io)?;
        let src = e.path();
        if !src.is_file() {
            continue;
        }
        let dst = dst_dir.join(e.file_name());
        fs::copy(&src, &dst).map_err(RigError::Io)?;
        written.push(dst);
    }
    Ok(())
}

fn link_or_replace_symlink(src: &Path, dst: &Path) -> Result<()> {
    if dst.is_symlink() {
        if let Ok(cur) = fs::read_link(dst) {
            if cur == src {
                return Ok(());
            }
        }
        fs::remove_file(dst).map_err(RigError::Io)?;
    } else if dst.is_file() {
        let bak = PathBuf::from(format!("{}.bak.{}", dst.display(), epoch_secs()));
        fs::rename(dst, &bak).map_err(RigError::Io)?;
    } else if dst.exists() {
        return Err(RigError::Msg(format!(
            "refusing to replace non-file: {}",
            dst.display()
        )));
    }
    std::os::unix::fs::symlink(src, dst).map_err(RigError::Io)?;
    Ok(())
}

fn epoch_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn resolve_shell_file(root: &Path, rel: &str) -> Result<(PathBuf, &'static str)> {
    let overlay = root.join("overlay").join(rel);
    if overlay.is_file() {
        return Ok((overlay, "overlay"));
    }
    let tpl = paths::templates_dir(root).join(rel);
    if tpl.is_file() {
        return Ok((tpl, "templates"));
    }
    Err(RigError::Msg(format!(
        "missing shell template: overlay/{rel} or templates/{rel}"
    )))
}

fn managed_snippet(root: &Path, cfg: &Path, shell: ShellKind, with_product_rc: bool) -> String {
    let rc = match shell {
        ShellKind::Zsh => "zshrc",
        ShellKind::Bash => "bashrc",
    };
    let product = if with_product_rc {
        format!(
            "# product rc (OMZ/p10k on workstation): touch \"$RIG_CONFIG/shell/use-product-rc\"\n\
             [ -f \"$RIG_CONFIG/shell/use-product-rc\" ] && [ -f \"$RIG_CONFIG/shell/{rc}\" ] && . \"$RIG_CONFIG/shell/{rc}\"\n"
        )
    } else {
        String::new()
    };
    format!(
        "{BEGIN}\n\
         # managed by rig — do not edit this block\n\
         export RIG_ROOT=\"{root}\"\n\
         export RIG_CONFIG=\"{cfg}\"\n\
         [ -f \"$RIG_CONFIG/shell/common.sh\" ] && . \"$RIG_CONFIG/shell/common.sh\"\n\
         {product}\
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
        return Err(RigError::Msg(format!("missing template: {}", src.display())));
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
