use std::fmt::Display;
use std::io::IsTerminal;
use std::path::Path;
use std::sync::OnceLock;

/// `  ok  screen-sharing  detail` — values share this column.
const INDENT: usize = 2;
const MARK: usize = 4;
const LABEL: usize = 16;
const VALUE_COL: usize = INDENT + MARK + 1 + LABEL + 1; // 24

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
    let budget = term_cols().saturating_sub(VALUE_COL).max(24);
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
        println!("{n}  {}", wrap(on, DIM, "preview"));
    } else {
        println!("{n}");
    }
}

pub fn kv(key: &str, value: impl Display) {
    let on = color_out();
    let m = wrap(on, DIM, &mark_field(""));
    let k = wrap(on, DIM, &label(key));
    println!("  {m} {k} {}", val(on, &tidy(&value.to_string())));
}

pub fn kvc(value: impl Display) {
    let on = color_out();
    let m = wrap(on, DIM, &mark_field(""));
    let k = wrap(on, DIM, &label(""));
    let v = wrap(on, DIM, &tidy(&value.to_string()));
    println!("  {m} {k} {v}");
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
    let m = wrap(on, DIM, &mark_field(""));
    let k = wrap(on, DIM, &label(""));
    println!("  {m} {k} {}", wrap(on, DIM, &tidy(&s.to_string())));
}

pub fn item2(s: impl Display) {
    let on = color_out();
    let m = wrap(on, DIM, &mark_field(""));
    let k = wrap(on, DIM, &label(""));
    println!("  {m} {k} {}", val(on, &tidy(&s.to_string())));
}

pub fn progress(s: impl Display) {
    let on = color_out();
    let m = wrap(on, DIM, &mark_field(""));
    let k = wrap(on, DIM, &label(""));
    println!("  {m} {k} {}", wrap(on, CYAN, &tidy(&s.to_string())));
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
    kvc(msg);
}

pub fn preview(action: &str) {
    blank();
    kvc(format!("pass --yes (-y) to {action}"));
}

pub fn next(cmd: &str) {
    kv("next", cmd);
}

pub fn error(msg: impl Display) {
    eprintln!("{}", paint(color_err(), &[BOLD, RED], "error"));
    eprintln!("  {}", tidy(&msg.to_string()));
}

pub fn error_help(help: impl Display) {
    let on = color_err();
    let m = wrap(on, DIM, &mark_field(""));
    let k = wrap(on, DIM, &label("help"));
    eprintln!("  {m} {k} {help}");
}

pub fn error_cause(msg: impl Display) {
    let on = color_err();
    let m = wrap(on, DIM, &mark_field(""));
    let k = wrap(on, DIM, &label("cause"));
    eprintln!("  {m} {k} {msg}");
}

pub fn data_hint(root: &Path, os: &str) {
    let on = color_err();
    eprintln!("{}", paint(on, &[BOLD, CYAN], "data"));
    let m = wrap(on, DIM, &mark_field(""));
    let root_k = wrap(on, DIM, &label("root"));
    let hosts_k = wrap(on, DIM, &label("hosts"));
    let overlay_k = wrap(on, DIM, &label("overlay"));
    let root_s = val(on, &tidy(&root.display().to_string()));
    let os_s = format!("os={}", val(on, os));
    eprintln!("  {m} {root_k} {root_s}  {os_s}");
    eprintln!(
        "  {m} {hosts_k} {}",
        val(on, &tidy(&format!("{}/", root.join("hosts").display())))
    );
    eprintln!(
        "  {m} {overlay_k} {}",
        val(on, &tidy(&format!("{}/", root.join("overlay").display())))
    );
}

pub fn table_head(line: &str) {
    println!("  {}", wrap(color_out(), DIM, line));
}

pub fn table_row(line: impl Display) {
    println!("  {line}");
}

pub fn mark_pad(mark: &str, width: usize) -> String {
    let pad = format!("{mark:<width$}");
    wrap(color_out(), mark_code(mark), &pad)
}

pub fn sudo_command() -> std::process::Command {
    std::process::Command::new("sudo")
}

/// Wordmark for `rig` / `rig -h`. Original slant art; hues are ours, not OMZ.
pub fn banner() -> String {
    let on = color_out();
    let art = r#"        _
   ____(_)___ _
  / __/ / __ `/
 / / / / /_/ /
/_/ /_/\__, /
      /____/"#;
    let hues = [YELLOW, GREEN, CYAN, MAGENTA];
    let mut i = 0usize;
    art.lines()
        .map(|line| {
            let mut out = String::from("  ");
            for ch in line.chars() {
                if ch == ' ' {
                    out.push(' ');
                    continue;
                }
                let s = ch.to_string();
                out.push_str(&paint(on, &[BOLD, hues[i % hues.len()]], &s));
                i += 1;
            }
            out
        })
        .collect::<Vec<_>>()
        .join("\n")
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
