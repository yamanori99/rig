use std::fmt::Display;
use std::io::IsTerminal;
use std::path::Path;
use std::sync::OnceLock;

/// `  version    0.4.1` — labels share this column. Marks only on steps.
const INDENT: usize = 2;
const MARK: usize = 4;
const LABEL: usize = 11;
const VALUE_COL: usize = INDENT + LABEL + 1;

const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";
const MAGENTA: &str = "\x1b[35m";

fn color_out() -> bool {
    static V: OnceLock<bool> = OnceLock::new();
    *V.get_or_init(|| want_color() && std::io::stdout().is_terminal())
}

fn color_err() -> bool {
    static V: OnceLock<bool> = OnceLock::new();
    *V.get_or_init(|| want_color() && std::io::stderr().is_terminal())
}

fn want_color() -> bool {
    std::env::var_os("NO_COLOR").is_none()
}

fn wrap(on: bool, code: &str, s: &str) -> String {
    if on {
        format!("{code}{s}{RESET}")
    } else {
        s.to_string()
    }
}

fn paint(on: bool, codes: &[&str], s: &str) -> String {
    if on {
        format!("{}{s}{RESET}", codes.join(""))
    } else {
        s.to_string()
    }
}

fn term_cols() -> usize {
    if let Ok(v) = std::env::var("COLUMNS") {
        if let Ok(n) = v.parse::<usize>() {
            if n >= 40 {
                return n;
            }
        }
    }
    ioctl_cols().unwrap_or(80)
}

#[cfg(unix)]
fn ioctl_cols() -> Option<usize> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        #[repr(C)]
        struct WinSize {
            row: u16,
            col: u16,
            x: u16,
            y: u16,
        }
        extern "C" {
            fn ioctl(fd: i32, req: std::ffi::c_ulong, arg: *mut WinSize) -> i32;
        }
        #[cfg(target_os = "macos")]
        const TIOCGWINSZ: std::ffi::c_ulong = 0x4008_7468;
        #[cfg(target_os = "linux")]
        const TIOCGWINSZ: std::ffi::c_ulong = 0x5413;
        let mut ws = WinSize {
            row: 0,
            col: 0,
            x: 0,
            y: 0,
        };
        let fd = if std::io::stdout().is_terminal() {
            1
        } else {
            0
        };
        let r = unsafe { ioctl(fd, TIOCGWINSZ, &mut ws) };
        if r == 0 && ws.col >= 40 {
            Some(ws.col as usize)
        } else {
            None
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

#[cfg(not(unix))]
fn ioctl_cols() -> Option<usize> {
    None
}

fn tilde_home(s: &str) -> String {
    let Some(home) = std::env::var_os("HOME") else {
        return s.to_string();
    };
    let home = home.to_string_lossy();
    if let Some(rest) = s.strip_prefix(home.as_ref()) {
        format!("~{rest}")
    } else {
        s.to_string()
    }
}

fn ellipsize(s: &str, max: usize) -> String {
    let n = s.chars().count();
    if n <= max {
        return s.to_string();
    }
    if max < 8 {
        return s.chars().take(max).collect();
    }
    let keep = max.saturating_sub(1);
    let head = keep / 2;
    let tail = keep - head;
    let chars: Vec<char> = s.chars().collect();
    let h: String = chars[..head].iter().collect();
    let t: String = chars[n - tail..].iter().collect();
    format!("{h}…{t}")
}

fn tidy(s: &str) -> String {
    let s = tilde_home(s);
    if !std::io::stdout().is_terminal() {
        return s;
    }
    let budget = term_cols().saturating_sub(VALUE_COL + MARK + 1).max(20);
    ellipsize(&s, budget)
}

fn val(on: bool, s: &str) -> String {
    wrap(on, YELLOW, s)
}

fn label(key: &str) -> String {
    format!("{key:<LABEL$}")
}

fn mark_field(mark: &str) -> String {
    format!("{mark:<MARK$}")
}

pub fn title(name: &str, preview: bool) {
    let on = color_out();
    let n = paint(on, &[BOLD, CYAN], name);
    if preview {
        println!("  {n}  {}", wrap(on, DIM, "preview"));
    } else {
        println!("  {n}");
    }
}

pub fn kv(key: &str, value: impl Display) {
    let on = color_out();
    let k = wrap(on, DIM, &label(key));
    println!("  {k} {}", val(on, &tidy(&value.to_string())));
}

pub fn kvc(value: impl Display) {
    let on = color_out();
    let k = wrap(on, DIM, &label(""));
    let v = wrap(on, DIM, &tidy(&value.to_string()));
    println!("  {k} {v}");
}

pub fn blank() {
    println!();
}

pub fn section(name: &str) {
    println!("  {}", wrap(color_out(), DIM, name));
}

/// Nested labeled line — same value column as `kv` / steps.
pub fn note(key: &str, value: impl Display) {
    kv(key, value);
}

pub fn item(s: impl Display) {
    let on = color_out();
    let k = wrap(on, DIM, &label(""));
    println!("  {k} {}", wrap(on, DIM, &tidy(&s.to_string())));
}

pub fn item2(s: impl Display) {
    let on = color_out();
    let k = wrap(on, DIM, &label(""));
    println!("  {k} {}", val(on, &tidy(&s.to_string())));
}

pub fn progress(s: impl Display) {
    let on = color_out();
    let k = wrap(on, DIM, &label(""));
    println!("  {k} {}", wrap(on, CYAN, &tidy(&s.to_string())));
}

pub fn plan(do_it: bool, id: &str, detail: &str) {
    step(if do_it { "do" } else { "skip" }, id, detail);
}

pub fn ok(id: &str, detail: &str) {
    step("ok", id, detail);
}

pub fn fail(id: &str, detail: &str) {
    step("fail", id, detail);
}

pub fn skip(id: &str, detail: &str) {
    step("skip", id, detail);
}

fn step(mark: &str, id: &str, detail: &str) {
    let on = color_out();
    let mark_c = wrap(on, mark_code(mark), &mark_field(mark));
    let id_c = wrap(on, CYAN, &label(id));
    let d = tidy(detail.trim());
    if d.is_empty() {
        println!("  {mark_c} {id_c}");
    } else {
        println!("  {mark_c} {id_c} {}", val(on, &d));
    }
}

fn mark_code(mark: &str) -> &'static str {
    match mark {
        "ok" => GREEN,
        "fail" => RED,
        "do" => CYAN,
        _ => DIM,
    }
}

pub fn empty(msg: &str) {
    println!("  {}", wrap(color_out(), DIM, msg));
}

pub fn preview(action: &str) {
    blank();
    empty(&format!("pass --yes (-y) to {action}"));
}

pub fn next(cmd: &str) {
    kv("next", cmd);
}

pub fn error(msg: impl Display) {
    let on = color_err();
    let k = wrap(on, DIM, &label("error"));
    eprintln!("  {k} {}", paint(on, &[BOLD, RED], &tidy(&msg.to_string())));
}

pub fn error_help(help: impl Display) {
    let on = color_err();
    let k = wrap(on, DIM, &label("help"));
    eprintln!("  {k} {help}");
}

pub fn error_cause(msg: impl Display) {
    let on = color_err();
    let k = wrap(on, DIM, &label("cause"));
    eprintln!("  {k} {msg}");
}

pub fn data_hint(root: &Path, os: &str) {
    let on = color_err();
    let k = wrap(on, DIM, &label("data"));
    let path = val(on, &tidy(&format!("{}/", root.display())));
    let os_s = wrap(on, DIM, os);
    eprintln!("  {k} {path}  {os_s}");
}

pub fn group(name: &str) {
    println!("  {}", wrap(color_out(), CYAN, name));
}

pub fn table_head(line: &str) {
    println!("  {}", wrap(color_out(), DIM, line));
}

pub fn table_row(line: impl Display) {
    println!("  {line}");
}

/// Pad to `width` using Unicode scalar count (host names are ASCII).
pub fn pad(s: &str, width: usize) -> String {
    let n = s.chars().count();
    if n >= width {
        s.to_string()
    } else {
        format!("{s}{}", " ".repeat(width - n))
    }
}

pub fn sudo_command() -> std::process::Command {
    std::process::Command::new("sudo")
}

/// Same indent and label width as `kv`.
pub fn help_head(name: &str) -> String {
    let on = color_out();
    format!("  {}", paint(on, &[BOLD, CYAN], name))
}

pub fn help_row(id: &str, note: &str) -> String {
    let on = color_out();
    let k = wrap(on, CYAN, &label(id));
    format!("  {k} {note}")
}

/// Wordmark for `rig` / `rig -h`. One hue per line.
pub fn banner() -> String {
    let on = color_out();
    let art = r#"        _
   ____(_)___ _
  / __/ / __ `/
 / / / / /_/ /
/_/ /_/\__, /
      /____/"#;
    let hues = [YELLOW, GREEN, CYAN, MAGENTA, CYAN, CYAN];
    let mut lines: Vec<String> = art
        .lines()
        .enumerate()
        .map(|(i, line)| paint(on, &[BOLD, hues[i.min(hues.len() - 1)]], line))
        .collect();
    if let Some(last) = lines.last_mut() {
        last.push_str("  ");
        last.push_str(&wrap(on, DIM, &format!("v{}", env!("CARGO_PKG_VERSION"))));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::{ellipsize, tilde_home};

    #[test]
    fn ellipsize_keeps_short() {
        assert_eq!(ellipsize("abc", 10), "abc");
    }

    #[test]
    fn ellipsize_middle() {
        let s = ellipsize("abcdefghijklmnopqrstuvwxyz", 11);
        assert_eq!(s.chars().count(), 11);
        assert!(s.contains('…'));
        assert!(s.starts_with("abcde"));
        assert!(s.ends_with("wxyz"));
    }

    #[test]
    fn tilde_is_prefix_only() {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/you".into());
        let p = format!("{home}/Library/foo");
        assert_eq!(tilde_home(&p), "~/Library/foo");
        assert_eq!(tilde_home("/other"), "/other");
    }
}
