use crate::error::{Result, RigError};
use crate::ui;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_REPO: &str = "yamanori99/rig";

pub fn run(tag: Option<&str>, yes: bool, force: bool) -> Result<()> {
    let repo = std::env::var("RIG_REPO").unwrap_or_else(|_| DEFAULT_REPO.into());
    let current = env!("CARGO_PKG_VERSION");
    let target = detect_target()?;
    let dest = install_path()?;

    let (tag_name, url) = resolve_asset(&repo, tag, &target)?;
    let remote_ver = tag_name.trim_start_matches('v');

    ui::title("update", !yes);
    let running = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "?".into());
    ui::kv("current", format!("{current}  ({running})"));
    ui::kv("target", format!("{tag_name}  ({target})"));
    ui::kv("dest", dest.display());

    if !force && remote_ver == current {
        ui::empty("already up to date");
        return Ok(());
    }

    if !yes {
        ui::kv("download", url);
        ui::preview("install");
        return Ok(());
    }

    download_and_install(&url, &dest)?;
    ui::kv("installed", dest.display());
    if running != dest.display().to_string() {
        ui::kv("running", &running);
        ui::item(format!(
            "new binary is {} — open a new shell if PATH differs",
            dest.display()
        ));
    }
    ui::next("rig --version");
    Ok(())
}

fn install_path() -> Result<PathBuf> {
    let dir = match std::env::var_os("RIG_BIN_DIR") {
        Some(d) => PathBuf::from(d),
        None => dirs_home()?.join(".local/bin"),
    };
    Ok(dir.join("rig"))
}

fn dirs_home() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| RigError::Msg("HOME is not set".into()))
}

fn detect_target() -> Result<String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let os = match os {
        "macos" => "apple-darwin",
        "linux" => "unknown-linux-gnu",
        other => {
            return Err(RigError::Msg(format!("unsupported OS: {other}")));
        }
    };
    let arch = match arch {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        other => return Err(RigError::Msg(format!("unsupported arch: {other}"))),
    };
    Ok(format!("{arch}-{os}"))
}

fn resolve_asset(repo: &str, tag: Option<&str>, target: &str) -> Result<(String, String)> {
    let want = format!("rig-{target}.tar.gz");
    let (tag_name, json) = match tag {
        Some(t) => {
            let url = format!("https://api.github.com/repos/{repo}/releases/tags/{t}");
            let body = curl_get(&url)?;
            let json: Value = serde_json::from_str(&body)
                .map_err(|e| RigError::Msg(format!("release json: {e}")))?;
            let name = json
                .get("tag_name")
                .and_then(|v| v.as_str())
                .unwrap_or(t)
                .to_string();
            (name, json)
        }
        None => {
            let url = format!("https://api.github.com/repos/{repo}/releases/latest");
            let body = curl_get(&url)?;
            let json: Value = serde_json::from_str(&body)
                .map_err(|e| RigError::Msg(format!("release json: {e}")))?;
            let name = json
                .get("tag_name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| RigError::Msg("latest release has no tag_name".into()))?
                .to_string();
            (name, json)
        }
    };

    let url = json
        .get("assets")
        .and_then(|a| a.as_array())
        .into_iter()
        .flatten()
        .find_map(|a| {
            let name = a.get("name").and_then(|v| v.as_str())?;
            if name == want {
                a.get("browser_download_url")?.as_str().map(str::to_string)
            } else {
                None
            }
        })
        .ok_or_else(|| {
            RigError::Msg(format!(
                "no asset {want} on {tag_name} — see https://github.com/{repo}/releases"
            ))
        })?;
    Ok((tag_name, url))
}

fn curl_get(url: &str) -> Result<String> {
    let out = Command::new("curl")
        .args([
            "-fsSL",
            "-A",
            "rig",
            "-H",
            "Accept: application/vnd.github+json",
            url,
        ])
        .output()
        .map_err(RigError::Io)?;
    if !out.status.success() {
        return Err(RigError::Msg(format!(
            "curl failed ({}) for {url}",
            out.status
        )));
    }
    String::from_utf8(out.stdout).map_err(|e| RigError::Msg(format!("curl utf8: {e}")))
}

fn download_and_install(url: &str, dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(RigError::Io)?;
    }
    let tmp = tempfile_dir()?;
    let tarball = tmp.join("rig.tar.gz");
    let status = Command::new("curl")
        .args(["-fsSL", "-A", "rig", "-o"])
        .arg(&tarball)
        .arg(url)
        .status()
        .map_err(RigError::Io)?;
    if !status.success() {
        return Err(RigError::Msg(format!("download failed: {url}")));
    }
    let status = Command::new("tar")
        .args(["-xzf"])
        .arg(&tarball)
        .arg("-C")
        .arg(&tmp)
        .status()
        .map_err(RigError::Io)?;
    if !status.success() {
        return Err(RigError::Msg("tar extract failed".into()));
    }
    let bin = find_rig_bin(&tmp)?;
    replace_file(&bin, dest)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(dest, fs::Permissions::from_mode(0o755));
    }
    let _ = fs::remove_dir_all(&tmp);
    Ok(())
}

fn tempfile_dir() -> Result<PathBuf> {
    let base = std::env::temp_dir().join(format!("rig-update-{}", std::process::id()));
    fs::create_dir_all(&base).map_err(RigError::Io)?;
    Ok(base)
}

fn find_rig_bin(tmp: &Path) -> Result<PathBuf> {
    let direct = tmp.join("rig");
    if direct.is_file() {
        return Ok(direct);
    }
    for entry in fs::read_dir(tmp).map_err(RigError::Io)? {
        let path = entry.map_err(RigError::Io)?.path();
        let cand = path.join("rig");
        if cand.is_file() {
            return Ok(cand);
        }
    }
    Err(RigError::Msg("tarball has no rig binary".into()))
}

fn replace_file(src: &Path, dest: &Path) -> Result<()> {
    let tmp = dest.with_extension("new");
    fs::copy(src, &tmp).map_err(RigError::Io)?;
    fs::rename(&tmp, dest).map_err(RigError::Io)?;
    Ok(())
}
