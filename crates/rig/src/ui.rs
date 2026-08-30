use std::fmt::Display;
use std::io::IsTerminal;
use std::path::Path;
use std::sync::OnceLock;

const KEY: usize = 10;
const MARK: usize = 4;
const ID: usize = 14;

const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
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

pub fn title(name: &str, preview: bool) {
    if preview {
        println!("{name}  {}", wrap(color_out(), DIM, "preview"));
    } else {
        println!("{name}");
    }
}

pub fn kv(key: &str, value: impl Display) {
    let k = format!("{key:<KEY$}");
    println!("  {} {value}", wrap(color_out(), DIM, &k));
}

pub fn kvc(value: impl Display) {
    println!("  {:KEY$} {value}", "");
}

pub fn blank() {
    println!();
}

pub fn section(name: &str) {
    println!("  {}", wrap(color_out(), DIM, name));
}

pub fn item(s: impl Display) {
    println!("    {s}");
}

pub fn item2(s: impl Display) {
    println!("      {s}");
}

pub fn item3(s: impl Display) {
    println!("        {s}");
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
        "skip" | "do" => YELLOW,
        _ => DIM,
    }
}

pub fn empty(msg: &str) {
    println!("  {msg}");
}

pub fn preview(action: &str) {
    blank();
    println!("  pass --yes (-y) to {action}");
}

pub fn next(cmd: &str) {
    kv("next", cmd);
}

pub fn error(msg: impl Display) {
    eprintln!("{}", wrap(color_err(), RED, "error"));
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
    eprintln!("{}", wrap(on, DIM, "data"));
    let root_k = wrap(on, DIM, &format!("{:<KEY$}", "root"));
    let hosts_k = wrap(on, DIM, &format!("{:<KEY$}", "hosts"));
    let overlay_k = wrap(on, DIM, &format!("{:<KEY$}", "overlay"));
    eprintln!("  {root_k} {}  os={os}", root.display());
    eprintln!("  {hosts_k} {}/", root.join("hosts").display());
    eprintln!("  {overlay_k} {}/", root.join("overlay").display());
}
