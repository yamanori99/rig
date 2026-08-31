use std::fmt::Display;
use std::io::IsTerminal;
use std::path::Path;
use std::sync::OnceLock;

const KEY: usize = 10;
const MARK: usize = 4;
const ID: usize = 14;

const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const CYAN: &str = "\x1b[36m";

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
    let k = format!("{key:<KEY$}");
    println!("  {} {value}", wrap(color_out(), DIM, &k));
}

pub fn kvc(value: impl Display) {
    let v = wrap(color_out(), DIM, &value.to_string());
    println!("  {:KEY$} {v}", "");
}

pub fn blank() {
    println!();
}

pub fn section(name: &str) {
    println!("  {}", wrap(color_out(), DIM, name));
}

/// Nested labeled line under a step (`    key        value`).
pub fn note(key: &str, value: impl Display) {
    let k = format!("{key:<KEY$}");
    println!("    {} {value}", wrap(color_out(), DIM, &k));
}

pub fn item(s: impl Display) {
    println!("    {}", wrap(color_out(), DIM, &s.to_string()));
}

pub fn item2(s: impl Display) {
    println!("      {}", wrap(color_out(), DIM, &s.to_string()));
}

/// Live install / fetch (not dim).
pub fn progress(s: impl Display) {
    println!("    {}", wrap(color_out(), CYAN, &s.to_string()));
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
    let mark_pad = format!("{mark:<MARK$}");
    let id_pad = format!("{id:<ID$}");
    let mark_c = wrap(on, mark_code(mark), &mark_pad);
    let id_c = wrap(on, CYAN, &id_pad);
    let d = detail.trim();
    if d.is_empty() {
        println!("  {mark_c} {id_c}");
    } else {
        println!("  {mark_c} {id_c} {d}");
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
    let msg = format!("pass --yes (-y) to {action}");
    println!("  {}", wrap(color_out(), DIM, &msg));
}

pub fn next(cmd: &str) {
    kv("next", cmd);
}

pub fn error(msg: impl Display) {
    eprintln!("{}", paint(color_err(), &[BOLD, RED], "error"));
    eprintln!("  {msg}");
}

pub fn error_help(help: impl Display) {
    let k = format!("{:<KEY$}", "help");
    eprintln!("  {} {help}", wrap(color_err(), DIM, &k));
}

pub fn error_cause(msg: impl Display) {
    let k = format!("{:<KEY$}", "cause");
    eprintln!("  {} {msg}", wrap(color_err(), DIM, &k));
}

pub fn data_hint(root: &Path, os: &str) {
    let on = color_err();
    eprintln!("{}", paint(on, &[BOLD, CYAN], "data"));
    let root_k = wrap(on, DIM, &format!("{:<KEY$}", "root"));
    let hosts_k = wrap(on, DIM, &format!("{:<KEY$}", "hosts"));
    let overlay_k = wrap(on, DIM, &format!("{:<KEY$}", "overlay"));
    eprintln!(
        "  {root_k} {}  {}",
        root.display(),
        wrap(on, DIM, &format!("os={os}"))
    );
    eprintln!("  {hosts_k} {}/", root.join("hosts").display());
    eprintln!("  {overlay_k} {}/", root.join("overlay").display());
}

pub fn table_head(line: &str) {
    println!("  {}", wrap(color_out(), DIM, line));
}

pub fn table_row(line: impl Display) {
    println!("  {line}");
}

/// Color `ok` / `fail` / `skip` for table cells; pad to `width` first.
pub fn mark_pad(mark: &str, width: usize) -> String {
    let pad = format!("{mark:<width$}");
    wrap(color_out(), mark_code(mark), &pad)
}

/// `sudo` with a prompt that matches kv layout (`  password `).
pub fn sudo_command() -> std::process::Command {
    let on = color_err();
    let label = wrap(on, DIM, "password");
    let mut c = std::process::Command::new("sudo");
    c.arg("-p").arg(format!("  {label} "));
    c
}
