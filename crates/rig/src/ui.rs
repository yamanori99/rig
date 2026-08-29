use std::fmt::Display;
use std::path::Path;

const KEY: usize = 10;
const MARK: usize = 4;
const ID: usize = 14;

pub fn title(name: &str, preview: bool) {
    if preview {
        println!("{name}  preview");
    } else {
        println!("{name}");
    }
}

pub fn kv(key: &str, value: impl Display) {
    println!("  {key:<KEY$} {value}");
}

pub fn kvc(value: impl Display) {
    println!("  {:KEY$} {value}", "");
}

pub fn blank() {
    println!();
}

pub fn section(name: &str) {
    println!("  {name}");
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
    let d = detail.trim();
    if d.is_empty() {
        println!("  {mark:<MARK$} {id:<ID$}");
    } else {
        println!("  {mark:<MARK$} {id:<ID$} {d}");
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
    eprintln!("error");
    eprintln!("  {msg}");
}

pub fn error_help(help: impl Display) {
    eprintln!("  help     {help}");
}

pub fn error_cause(msg: impl Display) {
    eprintln!("  cause    {msg}");
}

pub fn data_hint(root: &Path, os: &str) {
    eprintln!("data");
    eprintln!("  root     {}  os={os}", root.display());
    eprintln!("  hosts    {}/", root.join("hosts").display());
    eprintln!("  overlay  {}/", root.join("overlay").display());
}
