use crate::error::{Result, RigError};
use crate::packages;
use crate::paths;
use crate::schema::OsKind;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub struct PackageReport {
    pub backend: &'static str,
    #[allow(dead_code)]
    pub sets: Vec<String>,
    pub ok: bool,
    pub detail: String,
}

pub fn apply_packages(root: &Path, sets: &[String], os: OsKind) -> Result<PackageReport> {
    match os {
        OsKind::Macos => brew_bundle(root, sets),
        OsKind::Linux => apt_install(root, sets),
    }
}

fn brew_bundle(root: &Path, sets: &[String]) -> Result<PackageReport> {
    if which("brew").is_none() {
        return Ok(PackageReport {
            backend: "brew",
            sets: sets.to_vec(),
            ok: false,
            detail: "brew not found on PATH".into(),
        });
    }

    let mut notes = Vec::new();
    let mut ok = true;
    for set in sets {
        let file = paths::packages_dir(root)
            .join("brew")
            .join(format!("{set}.Brewfile"));
        if !file.is_file() {
            notes.push(format!("missing {set}.Brewfile"));
            ok = false;
            continue;
        }
        let path = file.display().to_string();
        let started = Instant::now();
        let (success, already) = run_brew_stats(&["bundle", "--file", &path, "--no-upgrade"])?;
        let took = fmt_dur(started.elapsed().as_secs());
        if success {
            crate::ui::note(set, format!("{already} already  {took}"));
            notes.push(format!("{set}: ok"));
        } else {
            crate::ui::note(set, format!("fail  {took}"));
            notes.push(format!("{set}: brew bundle failed"));
            ok = false;
        }
    }

    Ok(PackageReport {
        backend: "brew",
        sets: sets.to_vec(),
        ok,
        detail: notes.join("; "),
    })
}

pub(crate) fn brew_banner(title: &str) {
    crate::ui::progress(title);
    let _ = io::stdout().flush();
}

/// Run brew with compact, color-forced output (no --verbose dump).
pub(crate) fn run_brew(args: &[&str]) -> Result<bool> {
    Ok(run_brew_stats(args)?.0)
}

fn run_brew_stats(args: &[&str]) -> Result<(bool, usize)> {
    let already = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut child = Command::new("brew")
        .args(args)
        .env("HOMEBREW_COLOR", "1")
        .env("HOMEBREW_NO_ENV_HINTS", "1")
        .env("HOMEBREW_NO_INSTALL_CLEANUP", "1")
        .env("HOMEBREW_NO_AUTO_UPDATE", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(RigError::Io)?;

    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let last = Arc::new(Mutex::new(Instant::now()));
    let stop = Arc::new(AtomicBool::new(false));
    let start = Instant::now();

    let last_out = last.clone();
    let already_out = already.clone();
    let t_out = thread::spawn(move || pump_brew(stdout, last_out, already_out));
    let last_err = last.clone();
    let already_err = already.clone();
    let t_err = thread::spawn(move || pump_brew(stderr, last_err, already_err));

    let stop_beat = stop.clone();
    let last_beat = last.clone();
    let t_beat = thread::spawn(move || {
        while !stop_beat.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(200));
            let quiet = last_beat
                .lock()
                .map(|t| t.elapsed() >= Duration::from_secs(8))
                .unwrap_or(false);
            if quiet {
                eprint!("\r    …  {}          ", fmt_dur(start.elapsed().as_secs()));
                let _ = io::stderr().flush();
            }
        }
        clear_spinner();
    });

    let status = child.wait().map_err(RigError::Io)?;
    stop.store(true, Ordering::Relaxed);
    let _ = t_out.join();
    let _ = t_err.join();
    let _ = t_beat.join();
    Ok((status.success(), already.load(Ordering::Relaxed)))
}

fn run_prefixed(mut cmd: Command) -> Result<(bool, usize)> {
    let already = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(RigError::Io)?;
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let last = Arc::new(Mutex::new(Instant::now()));
    let t_out = {
        let last = last.clone();
        let already = already.clone();
        thread::spawn(move || pump_apt(stdout, last, already))
    };
    let t_err = {
        let last = last.clone();
        let already = already.clone();
        thread::spawn(move || pump_apt(stderr, last, already))
    };
    let status = child.wait().map_err(RigError::Io)?;
    let _ = t_out.join();
    let _ = t_err.join();
    Ok((status.success(), already.load(Ordering::Relaxed)))
}

fn pump_apt(
    stream: impl io::Read + Send,
    last: Arc<Mutex<Instant>>,
    already: Arc<std::sync::atomic::AtomicUsize>,
) {
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        let Ok(line) = line else {
            break;
        };
        if let Ok(mut t) = last.lock() {
            *t = Instant::now();
        }
        match restyle_apt_line(&line) {
            BrewShow::Skip => {}
            BrewShow::Already => {
                already.fetch_add(1, Ordering::Relaxed);
            }
            BrewShow::Progress(s) => crate::ui::progress(s),
            BrewShow::Line(s) => crate::ui::item(s),
        }
    }
}

fn restyle_apt_line(raw: &str) -> BrewShow {
    let line = strip_ansi(raw);
    let line = line.trim();
    if line.is_empty() {
        return BrewShow::Skip;
    }
    let lower = line.to_ascii_lowercase();
    if lower.contains("already the newest") || lower.contains("already installed") {
        return BrewShow::Already;
    }
    if lower.starts_with("e:")
        || lower.starts_with("err:")
        || lower.starts_with("error")
        || lower.contains("unable to")
    {
        return BrewShow::Line(line.to_string());
    }
    if lower.starts_with("get:")
        || lower.starts_with("unpacking")
        || lower.starts_with("setting up")
    {
        return BrewShow::Progress(line.to_string());
    }
    BrewShow::Skip
}

fn pump_brew(
    stream: impl io::Read + Send,
    last: Arc<Mutex<Instant>>,
    already: Arc<std::sync::atomic::AtomicUsize>,
) {
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        let Ok(line) = line else {
            break;
        };
        if let Ok(mut t) = last.lock() {
            *t = Instant::now();
        }
        match restyle_brew_line(&line) {
            BrewShow::Skip => {}
            BrewShow::Already => {
                already.fetch_add(1, Ordering::Relaxed);
            }
            BrewShow::Progress(s) => {
                clear_spinner();
                crate::ui::progress(s);
            }
            BrewShow::Line(s) => {
                clear_spinner();
                crate::ui::item(s);
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum BrewShow {
    Skip,
    Already,
    Progress(String),
    Line(String),
}

fn restyle_brew_line(raw: &str) -> BrewShow {
    let line = strip_ansi(raw);
    let line = line.trim();
    if line.is_empty() {
        return BrewShow::Skip;
    }
    let lower = line.to_ascii_lowercase();
    if lower.contains("brewfile dependencies")
        || lower.contains("`brew bundle` complete")
        || lower.contains("circular dependency")
    {
        return BrewShow::Skip;
    }
    if line.starts_with("Using ") {
        return BrewShow::Already;
    }
    if let Some(name) = line.strip_prefix("Installing ") {
        return BrewShow::Progress(format!("install  {}", name.trim()));
    }
    if let Some(rest) = line.strip_prefix("==> ") {
        if rest.to_ascii_lowercase().starts_with("pouring") {
            return BrewShow::Skip;
        }
        return BrewShow::Progress(rest.to_string());
    }
    if lower.starts_with("warning") {
        return BrewShow::Skip;
    }
    if lower.starts_with("error") || lower.starts_with("fatal") {
        return BrewShow::Line(line.to_string());
    }
    BrewShow::Skip
}

fn clear_spinner() {
    eprint!("\r                    \r");
    let _ = io::stderr().flush();
}

fn fmt_dur(secs: u64) -> String {
    if secs >= 60 {
        format!("{}m{:02}s", secs / 60, secs % 60)
    } else {
        format!("{secs}s")
    }
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for x in chars.by_ref() {
                    if x.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

fn apt_install(root: &Path, sets: &[String]) -> Result<PackageReport> {
    if which("apt-get").is_none() {
        return Ok(PackageReport {
            backend: "apt",
            sets: sets.to_vec(),
            ok: false,
            detail: "apt-get not found".into(),
        });
    }

    let mut pkgs = Vec::new();
    for set in sets {
        let contents = packages::load_package_set(root, set)?;
        pkgs.extend(contents.apt);
    }
    pkgs.sort();
    pkgs.dedup();
    if pkgs.is_empty() {
        return Ok(PackageReport {
            backend: "apt",
            sets: sets.to_vec(),
            ok: true,
            detail: "no packages".into(),
        });
    }

    brew_banner("apt-get install");
    let started = Instant::now();
    let mut cmd = crate::ui::sudo_command();
    cmd.args(["apt-get", "install", "-y"]);
    cmd.args(&pkgs);
    let (success, already) = run_prefixed(cmd)?;
    let took = fmt_dur(started.elapsed().as_secs());
    if success {
        crate::ui::note("apt", format!("{already} already  {took}"));
    } else {
        crate::ui::note("apt", format!("fail  {took}"));
    }
    Ok(PackageReport {
        backend: "apt",
        sets: sets.to_vec(),
        ok: success,
        detail: if success {
            format!("installed {}", pkgs.join(" "))
        } else {
            "apt-get failed".into()
        },
    })
}

const EXTRA_SHOW: usize = 24;
const APT_NOISE: usize = 40;

/// Formulae/casks (or apt manuals) installed but not in this host's package sets.
pub fn extras_not_recommended(
    root: &Path,
    sets: &[String],
    os: OsKind,
) -> Result<Option<Vec<String>>> {
    let recommended = packages::recommended_for_os(root, sets, os)?;
    let Some(installed) = (match os {
        OsKind::Macos => brew_installed_requested(),
        OsKind::Linux => apt_manual_installed(),
    }) else {
        return Ok(None);
    };
    Ok(Some(packages::extras_sorted(&installed, &recommended)))
}

fn brew_installed_requested() -> Option<Vec<String>> {
    if which("brew").is_none() {
        return None;
    }
    let mut formulae = brew_list_lines(&["list", "--formula", "--installed-on-request", "-1"]);
    if formulae.is_empty() {
        formulae = brew_list_lines(&["leaves", "-1"]);
    }
    let mut out: Vec<String> = formulae.into_iter().map(|n| format!("brew:{n}")).collect();
    out.extend(
        brew_list_lines(&["list", "--cask", "-1"])
            .into_iter()
            .map(|n| format!("cask:{n}")),
    );
    Some(out)
}

pub(crate) fn brew_installed_casks() -> Vec<String> {
    brew_list_lines(&["list", "--cask", "-1"])
}

fn brew_list_lines(args: &[&str]) -> Vec<String> {
    let out = Command::new("brew").args(args).output();
    let Ok(out) = out else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

fn apt_manual_installed() -> Option<Vec<String>> {
    if which("apt-mark").is_none() {
        return None;
    }
    let out = Command::new("apt-mark").args(["showmanual"]).output();
    let Ok(out) = out else {
        return None;
    };
    if !out.status.success() {
        return None;
    }
    let lines: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    Some(lines)
}

pub fn print_extras(root: &Path, sets: &[String], os: OsKind) -> Result<()> {
    let Some(extra) = extras_not_recommended(root, sets, os)? else {
        return Ok(());
    };
    if os == OsKind::Linux && extra.len() > APT_NOISE {
        crate::ui::kv(
            "extra",
            format!(
                "{} apt manuals — skipped (noisy vs role lists)",
                extra.len()
            ),
        );
        return Ok(());
    }
    if extra.is_empty() {
        crate::ui::kv("extra", "none besides role sets");
        return Ok(());
    }
    crate::ui::kv("extra", format!("{} not in role sets", extra.len()));
    crate::ui::kvc("installed besides recommended (brew deps omitted)");
    let show = extra.len().min(EXTRA_SHOW);
    for name in &extra[..show] {
        crate::ui::note("pkg", name);
    }
    if extra.len() > EXTRA_SHOW {
        crate::ui::item(format!("… {} more", extra.len() - EXTRA_SHOW));
    }
    Ok(())
}

fn which(bin: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        for dir in std::env::split_paths(&paths) {
            let p = dir.join(bin);
            if p.is_file() {
                return Some(p);
            }
        }
        None
    })
}

#[cfg(test)]
mod tests {
    use super::{restyle_apt_line, restyle_brew_line, BrewShow};

    #[test]
    fn restyle_using_and_install() {
        assert_eq!(restyle_brew_line("Using cmake"), BrewShow::Already);
        assert_eq!(
            restyle_brew_line("Installing llvm"),
            BrewShow::Progress("install  llvm".into())
        );
        assert_eq!(
            restyle_brew_line("==> Pouring llvm.bottle.tar.gz"),
            BrewShow::Skip
        );
        assert_eq!(
            restyle_brew_line("==> Fetching llvm"),
            BrewShow::Progress("Fetching llvm".into())
        );
        assert_eq!(
            restyle_brew_line("`brew bundle` complete! 38 Brewfile dependencies now installed."),
            BrewShow::Skip
        );
        assert_eq!(
            restyle_brew_line(
                "Warning: Formulae dependency graph sorting found a circular dependency:"
            ),
            BrewShow::Skip
        );
    }

    #[test]
    fn restyle_apt_hides_noise() {
        assert_eq!(
            restyle_apt_line("git is already the newest version (1:2.0)."),
            BrewShow::Already
        );
        assert_eq!(restyle_apt_line("Reading package lists..."), BrewShow::Skip);
        assert_eq!(
            restyle_apt_line("E: Unable to locate package foo"),
            BrewShow::Line("E: Unable to locate package foo".into())
        );
    }
}
