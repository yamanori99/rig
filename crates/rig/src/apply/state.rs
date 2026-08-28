use crate::error::{Result, RigError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RigState {
    pub schema_version: u32,
    pub host: String,
    pub role: String,
    pub managed_files: Vec<String>,
    pub package_sets: Vec<String>,
    pub steps: BTreeMap<String, String>,
}

impl RigState {
    pub fn new(host: &str, role: &str) -> Self {
        Self {
            schema_version: 1,
            host: host.to_string(),
            role: role.to_string(),
            managed_files: Vec::new(),
            package_sets: Vec::new(),
            steps: BTreeMap::new(),
        }
    }

    pub fn note_file(&mut self, path: impl AsRef<Path>) {
        let s = path.as_ref().display().to_string();
        if !self.managed_files.iter().any(|p| p == &s) {
            self.managed_files.push(s);
        }
    }

    pub fn note_step(&mut self, id: &str, detail: impl Into<String>) {
        self.steps.insert(id.to_string(), detail.into());
    }
}

#[allow(dead_code)] // used by clean / status later
pub fn load() -> Result<Option<RigState>> {
    let path = crate::paths::state_path();
    if !path.is_file() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path).map_err(RigError::Io)?;
    let state: RigState = serde_json::from_str(&raw).map_err(|e| {
        RigError::Msg(format!("failed to parse {}: {e}", path.display()))
    })?;
    Ok(Some(state))
}

pub fn save(state: &RigState) -> Result<PathBuf> {
    let path = crate::paths::state_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(RigError::Io)?;
    }
    let raw = serde_json::to_string_pretty(state)
        .map_err(|e| RigError::Msg(format!("state serialize: {e}")))?;
    fs::write(&path, raw).map_err(RigError::Io)?;
    Ok(path)
}
