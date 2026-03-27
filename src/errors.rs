pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("TOML parsing error: {0}")]
    ParseErr(#[from] toml_edit::TomlError),

    #[error("IO operation error: {0}")]
    IoFail(#[from] std::io::Error),

    #[error("{0}")]
    CargoFail(String),

    #[error("{0}")]
    ManifestCorrupt(String),

    #[error("No script matching '{0}' found in Cargo.toml")]
    ScriptNotFound(String),

    #[error("No dependency matching '{0}' found in Cargo.toml")]
    DependencyNotFound(String),
}
