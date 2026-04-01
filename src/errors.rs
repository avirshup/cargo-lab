pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error, Clone)]
pub enum Error {
    #[error("TOML parsing error: {0}")]
    ParseErr(#[from] toml_edit::TomlError),

    #[error("IO operation error: {desc} ({err})")]
    IoFail { desc: String, err: String },

    #[cfg(feature = "experimental_cargo_script_rfc3502")]
    #[error("RFC 3502 parsing error: {0}")]
    EmbeddedScriptErr(
        #[from] crate::vendor_cargo::frontmatter::FrontmatterError,
    ),

    #[error("Required env var missing: {0}")]
    EnvVarMissing(String),

    #[error("TOML parsing error: {0}")]
    TomlParsingFailed(#[from] toml_edit::de::Error),

    #[error("Failed to copy {src} to {dest}: {err}")]
    CopyFailed {
        src: String,
        dest: String,
        err: String,
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

    #[error("Script named '{0}' already exists in Cargo.toml")]
    ScriptNameConflict(String),

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

    pub fn from_serde_err(err: impl serde::de::Error) -> Self {
        Self::ManifestCorrupt(err.to_string())
    }
}

/// Lets us apply `?` to borrowed errors (which we get from cached results)
///
/// Implementing `From` for references via cloning seems like a weird idea
/// in general, really it would be nice to implement this via the (currently
/// experimental) [`Try`] trait instead?
impl From<&Error> for Error {
    fn from(err: &Error) -> Self {
        err.clone()
    }
}

/// These are kind of annoying to handle, so just putting them in
/// a macro for now to maintain flexibility ...
///
/// Usage: `ioerr!( IOERR, *args_for_format!)`
#[macro_export]
macro_rules! ioerr {
    ( $err:ident, $($arg:tt)+ ) => {
        $crate::Error::IoFail{
            err: $err.to_string(),
            desc: format!( $($arg)+ )
        }
    };
}
