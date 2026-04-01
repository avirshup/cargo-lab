pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("TOML parsing error: {0}")]
    ParseErr(#[from] toml_edit::TomlError),

    // TODO: This does not actually produce helpful errors
    #[error("IO operation error: {0} ({1})")]
    IoFail(String, std::io::Error),

    #[error("Required env var missing: {0}")]
    EnvVarMissing(String),

    #[error("Failed to copy {src} to {dest}: {err}")]
    CopyFailed {
        src: String,
        dest: String,
        err: std::io::Error,
    },

    #[error("Config discovery failed: {0}")]
    NoConfig(String),

    #[error("{0}")]
    AlreadyExists(String),

    #[error("{0}")]
    CargoFail(String),

    #[error("{0}")]
    ManifestCorrupt(String),

    #[error("No script matching '{0}' found in Cargo.toml")]
    ScriptNotFound(String),

    #[error("No dependency matching '{0}' found in Cargo.toml")]
    DependencyNotFound(String),

    #[error(
        "Feature '{0}' is ambiguous, please qualify with its dependency, \
         i.e., 'depname/{0}'."
    )]
    AmbiguousFeature(String),

    #[error("Failed to parse CLI argument '{0}'")]
    InputErr(String),

    #[error("Encountered multiple errors")] // TODO: display them
    MultipleErrors(Vec<Error>),
}

impl Error {
    /// Consolidates errors into MulitpleErrors if there's more than one.
    /// Panics if there are zero.
    pub fn from_nonempty_iter(mut iter: impl Iterator<Item = Self>) -> Self {
        let first_err = iter
            .next()
            .expect("`Error::from_nonempty_iter` called with empty iter");

        // multiple errors, or just the one?
        if let Some(second_err) = iter.next() {
            let mut errvec = vec![first_err, second_err];
            errvec.extend(iter);
            Error::MultipleErrors(errvec)
        } else {
            first_err
        }
    }
}
