use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Diagnostic, Debug)]
pub enum RigError {
    #[error("rig root not found")]
    #[diagnostic(help(
        "run from a rig checkout, set --root / RIG_ROOT, or install a release binary (product files are embedded)"
    ))]
    #[allow(dead_code)] // kept for callers; discover_root now falls back to embed
    RootNotFound,

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to parse {path}: {source}")]
    Toml {
        path: String,
        #[source]
        source: toml::de::Error,
    },

    #[error("{0}")]
    Msg(String),
}

pub type Result<T> = std::result::Result<T, RigError>;
