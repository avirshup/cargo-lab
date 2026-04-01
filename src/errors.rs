use camino::Utf8PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

// TODO: clean this up, it might be time to Box<dyn> it
#[derive(Debug, thiserror::Error, Clone)]
pub enum Error {
    #[error("Encountered multiple errors")] // TODO: display them
    MultipleErrors(Vec<Error>),

    // ───── General runtime env problems ─────
    #[error("Required env var missing: {0}")]
    EnvVarMissing(String),

    #[error("Config discovery failed: {0}")]
    NoConfig(String),

    #[error("Unknown shell: {0}")]
    UnknownShell(String),

    // ───── I/o / subproc related ─────
    #[error("IO operation error: {desc} ({err})")]
    IoFail { desc: String, err: String },

    #[error("Failed to copy {src} to {dest}: {err}")]
    CopyFailed {
        src: Utf8PathBuf,
        dest: Utf8PathBuf,
        err: String,
    },

    #[error("{0}")]
    CargoFail(String),

    #[error("Path '{path}': {description})")]
    FileErr {
        path: Utf8PathBuf,
        description: String,
    },

    // ───── TOML-related ─────
    #[error("TOML parsing error: {0}")]
    TomlParseErr(#[from] toml_edit::TomlError),

    // When there's an error deserializing the (already-parsed)
    // TOML into the expected structure
    #[error("TOML structure error: {0}")]
    TomlStructureErr(#[from] toml_edit::de::Error),

    // This is basically the same thing as `TomlStructureErr` but
    // it arises when navigating the TOML manually
    #[error("{0}")]
    ManifestStructureErr(String),

    #[cfg(feature = "experimental_cargo_script_rfc3502")]
    #[error("RFC 3502 parsing error: {0}")]
    EmbeddedScriptErr(
        #[from] crate::vendor_cargo::frontmatter::FrontmatterError,
    ),

    // ───── Input-related errors ─────
    #[error("Failed to parse CLI argument '{0}'")]
    CliArgParseFail(String),

    #[error(
        "To launch an editor, please set the 'editor-cmd' key in the \
         '[project.metadata.cargo-playground]' table in `Cargo.toml`"
    )]
    NeedEditorCmd(),

    #[error("No script matching '{0}' found in Cargo.toml")]
    ScriptNotFound(String),

    #[error("Script named '{0}' already exists in Cargo.toml")]
    ScriptNameConflict(String),

    #[error("'{0}' is not a valid filename for a script")]
    InvalidScriptFilename(Utf8PathBuf),

    #[error("No dependency matching '{0}' found in Cargo.toml")]
    DependencyNotFound(String),

    #[error(
        "Feature '{0}' is ambiguous, please qualify with its dependency, \
         i.e., 'depname/{0}'."
    )]
    AmbiguousFeature(String),
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
