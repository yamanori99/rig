use crate::error::{Result, RigError};
use crate::paths;
use crate::schema::OsKind;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PackageSetContents {
    pub brew: Vec<String>,
    pub apt: Vec<String>,
}

pub fn load_package_set(root: &Path, name: &str) -> Result<PackageSetContents> {
    let brew_path = paths::packages_dir(root)
        .join("brew")
        .join(format!("{name}.Brewfile"));
    let apt_path = paths::packages_dir(root)
        .join("apt")
        .join(format!("{name}.list"));

    Ok(PackageSetContents {
        brew: if brew_path.is_file() {
            parse_brewfile(&std::fs::read_to_string(&brew_path).map_err(RigError::Io)?)
        } else {
            Vec::new()
        },
        apt: if apt_path.is_file() {
            parse_apt_list(&std::fs::read_to_string(&apt_path).map_err(RigError::Io)?)
        } else {
            Vec::new()
        },
    })
}

pub fn packages_for_os(set: &PackageSetContents, os: OsKind) -> &[String] {
    match os {
        OsKind::Macos => &set.brew,
        OsKind::Linux => &set.apt,
    }
}

/// Parse Homebrew Brewfile lines into `kind:name` entries (e.g. `brew:git`, `cask:cursor`).
pub fn parse_brewfile(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in body.lines() {
        let line = strip_comment(line).trim();
        if line.is_empty() {
            continue;
        }
        if let Some(pkg) = parse_brew_directive(line) {
            out.push(pkg);
        }
    }
    out
}

fn parse_brew_directive(line: &str) -> Option<String> {
    for kind in ["brew", "cask", "tap", "mas"] {
        let prefix = format!("{kind} ");
        if let Some(rest) = line.strip_prefix(&prefix) {
            let name = rest
                .trim()
                .trim_matches(|c| c == '"' || c == '\'')
                .split(',')
                .next()?
                .trim()
                .trim_matches(|c| c == '"' || c == '\'');
            if !name.is_empty() {
                return Some(format!("{kind}:{name}"));
            }
        }
    }
    None
}

pub fn parse_apt_list(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in body.lines() {
        let line = strip_comment(line).trim();
        if line.is_empty() {
            continue;
        }
        out.push(line.to_string());
    }
    out
}

fn strip_comment(line: &str) -> &str {
    line.split('#').next().unwrap_or(line)
}
