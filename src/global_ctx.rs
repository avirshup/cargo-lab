use std::cell::OnceCell;
use std::env;
use std::env::current_dir;

pub use Verbosity::*;
use camino::{Utf8Path, Utf8PathBuf};
use serde::Deserialize;

use crate::manifest_data::{ManifestData, PlaygroundConfig};
use crate::manifest_editor::ManifestEditor;
use crate::{data, util};

#[cfg(feature = "xtask")]
const ENV_XTASK_MANIFEST_PATH: &str = "CARGO_MANIFEST_DIR";
pub const CARGO_PLAYGROUND_MANIFEST_DIR: &str = "CARGO_PLAYGROUND_MANIFEST_DIR";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verbosity {
    /// No stdout at all, few warnings (for use with dynamic autocomplete)
    NearlySilent,

    /// Minimal output, generally suitable for line-oriented piping
    Quiet,
    Normal,
    Verbose,
    Debug,
}

type CachedResult<'a, T> = Result<&'a T, &'a crate::Error>;

/// Loads configuration values as needed for a given operation.
///
/// This is all done lazily since different operations have
/// different configuration needs (e.g., the "init"
/// command does not even need a manifest,
/// but the `run` command needs a fully configured manifest).
///
/// TODO:
/// I don't like calling things "context" since it's an
/// overloaded word that means 50 different things in various
/// ... uh, well, contexts ... but "GlobalCtx" is what cargo calls it.
/// Although this is not actually really Global?
///
/// # How manifest discovery works
/// The manifest directory we use  will be the first one of these that is not `None`:
///
/// 1. The `override_manifest_path` passed to [`GlobalCtx::new`]
///    (usually this will come from the `--manifest-path` CLI option),
/// 2. the value of the `CARGO_PLAYGROUND_MANIFEST_DIR` env var;
/// 3. in `cfg(feature=xtask)` mode only, the first
///    parent of the `$CARGO_MANIFEST_DIR` env var
/// 4. The `$cwd` or first parent thereof contains Cargo.toml.
///
/// Note "discovering" a manifest here, for now, just means identifying
/// some directory that we _assume_ contains Cargo.toml. If this comes
/// from some user input, we don't bother to validate this fact (or
/// even that the directory exists) right now (it will of course
/// fail later if the operation actually requires parsing the manifest).
pub struct GlobalCtx {
    pub verbosity: Verbosity,
    pub cwd: Utf8PathBuf,
    pub cargo_exe: Utf8PathBuf,

    _override_manifest_path: Option<Utf8PathBuf>,
    _project_paths: OnceCell<crate::Result<ProjectPaths>>,
    _manifest_raw: OnceCell<crate::Result<String>>,
    _manifest_data: OnceCell<crate::Result<ManifestData>>,
    _playground_config: OnceCell<Option<PlaygroundConfig>>,
}

impl GlobalCtx {
    /// Construct a new global context.
    ///
    /// The path to the manifest directory (or the manifest itself, that's fine too)
    /// can be passed as the second argument, otherwise  we'll try to discover it
    /// upon demand (see [class docs](GlobalCtx).)
    ///
    /// Currently panics if CWD doesn't exist.
    pub fn new(
        verbosity: Verbosity,
        override_manifest_path: Option<String>,
    ) -> Self {
        let cwd: Utf8PathBuf = current_dir()
            .expect("Process has CWD")
            .try_into()
            .expect("CWD is UTF-8"); // panic so we don't have to return a Result just for this case

        // TODO: path / cmd to invoke cargo should be configurable?
        let cargo_exe =
            Utf8PathBuf::from(env::var("CARGO").unwrap_or("cargo".to_owned()));

        Self {
            verbosity,
            cwd,
            cargo_exe,
            _override_manifest_path: override_manifest_path
                .map(Utf8PathBuf::from),
            _project_paths: Default::default(),
            _manifest_raw: Default::default(),
            _manifest_data: Default::default(),
            _playground_config: Default::default(),
        }
    }

    /// Creates a new context with custom project paths
    pub fn with_paths(&self, paths: ProjectPaths) -> GlobalCtx {
        Self {
            verbosity: self.verbosity,
            cwd: self.cwd.clone(),
            cargo_exe: self.cargo_exe.clone(),
            _override_manifest_path: None,
            _project_paths: OnceCell::from(Ok(paths)),
            _manifest_raw: Default::default(),
            _manifest_data: Default::default(),
            _playground_config: Default::default(),
        }
    }

    /// Create a new context to re-read the manifest after, e.g., it's changed on disk.
    ///
    /// Note this is still lazy so it doesn't actually re-read the manifest from disk
    /// until requested. But it at least doesn't need to re-do the manifest discovery parts..
    pub fn reload(&self) -> GlobalCtx {
        Self {
            verbosity: self.verbosity,
            cwd: self.cwd.clone(),
            cargo_exe: self.cargo_exe.clone(),
            _override_manifest_path: self._override_manifest_path.clone(),
            _project_paths: self._project_paths.clone(),
            _manifest_raw: Default::default(),
            _manifest_data: Default::default(),
            _playground_config: Default::default(),
        }
    }

    // ───── Paths ──────────────────────────────────────────────────── //
    pub fn project_paths(&'_ self) -> CachedResult<'_, ProjectPaths> {
        self._project_paths
            .get_or_init(|| {
                if let Some(input_path) = &self._override_manifest_path {
                    ProjectPaths::from_input_path(input_path)
                } else {
                    ProjectPaths::discover(&self.cwd)
                }
            })
            .as_ref()
    }

    // ───── Manifest data ──────────────────────────────────────────── //
    pub fn manifest_raw(&'_ self) -> CachedResult<'_, str> {
        self._manifest_raw
            .get_or_init(|| {
                self.project_paths()
                    .map_err(Clone::clone)
                    .and_then(|paths| util::read_file(&paths.cargo_dot_toml))
            })
            .as_deref()
    }

    pub fn manifest_data(&'_ self) -> CachedResult<'_, ManifestData> {
        self._manifest_data
            .get_or_init(|| {
                self.manifest_raw().map_err(Clone::clone).and_then(|s| {
                    let de = toml_edit::de::Deserializer::parse(s)?;
                    let data = ManifestData::deserialize(de)?;
                    Ok(data)
                })
            })
            .as_ref()
    }

    pub fn playground_config(&self) -> &Option<PlaygroundConfig> {
        self._playground_config.get_or_init(|| {
            self.manifest_data()
                .as_ref()
                .ok()
                .and_then(|x| x.playground_config().cloned())
        })
    }

    /// Return a _fresh_ editable copy of the manifest from its original
    /// state. Unlike most of the other methods here, this is *not* cached
    /// (it constructs a new editor every time it is called).
    pub fn new_editor(&self) -> crate::Result<ManifestEditor> {
        self.manifest_raw()
            .map_err(Clone::clone)
            .and_then(ManifestEditor::from_string)
    }
}

/// Paths for a given playground project.
///
/// This struct's constructors will check that  `manifest_dir` is
/// a directory that exists (at construction time anyway), but none
/// of the other paths are checked.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive] // <- require use of constructors
pub struct ProjectPaths {
    pub manifest_dir: Utf8PathBuf,
    pub template_dir: Utf8PathBuf,
    pub cargo_dot_toml: Utf8PathBuf,
    pub script_dir: Utf8PathBuf,
}

impl ProjectPaths {
    // ───── Constructors ───────────────────────────────────────────── //

    /// Find path to the project based on user's explicit input
    pub fn from_input_path(input_path: &Utf8Path) -> crate::Result<Self> {
        let normalized_path = if input_path.is_file()
            && input_path.file_name() == Some("Cargo.toml")
        {
            input_path.parent().unwrap()
        } else {
            input_path
        };

        Self::from_manifest_dir(normalized_path)
    }

    /// Try to figure out which project path the user wants, based on env vars and CWD.
    pub fn discover(cwd: &Utf8Path) -> crate::Result<Self> {
        if let Ok(path) = _getenv::<Utf8PathBuf>(CARGO_PLAYGROUND_MANIFEST_DIR)
        {
            return Self::from_manifest_dir(&path);
        }

        #[cfg(feature = "xtask")]
        if let Ok(xtask_dir) = _getenv::<Utf8PathBuf>(ENV_XTASK_MANIFEST_PATH)
            && let Ok(path) = _first_parent_dir_with_a_manifest_in_it(
                xtask_dir
                    .parent()
                    .expect("`CARGO_MANIFEST_DIR` parent exists"),
            )
        {
            return Self::from_manifest_dir(&path);
        }

        _first_parent_dir_with_a_manifest_in_it(cwd)
            .map(Self::_from_manifest_dir_path_unchecked)
    }

    /// Construct project paths from the path to the manifest dir, ensuring
    /// that it really is a readable directory.
    pub fn from_manifest_dir(path: &Utf8Path) -> crate::Result<Self> {
        if path.is_dir() {
            // They passed a directory, we're gtg
            Ok(Self::_from_manifest_dir_path_unchecked(path.to_owned()))
        } else if !path.exists() {
            Err(crate::Error::FileErr {
                path: path.to_owned(),
                description: "provided manifest path does not exist".to_owned(),
            })
        } else {
            Err(crate::Error::FileErr {
                description: "provided manifest path is not a directory or \
                              manifest file"
                    .to_owned(),
                path: path.to_owned(),
            })
        }
    }

    fn _from_manifest_dir_path_unchecked(path: Utf8PathBuf) -> Self {
        Self {
            cargo_dot_toml: path.join("Cargo.toml"),
            template_dir: path.join("templates"),
            script_dir: path.join("src"),
            manifest_dir: path,
        }
    }

    // ───── Path getters ───────────────────────────────────────────── //
    /// Return path to a template of the given name.
    /// Note that this always succeeds - it does not check whether the
    /// file actually exists or not.
    pub fn template_path(&self, name: &str) -> Utf8PathBuf {
        self.template_dir.join(format!("{name}.rs.template"))
    }

    // ───── Formatting helpers ─────────────────────────────────────── //
    /// Format a path relative to the root of the project
    pub fn relpath_project_root(&self, path: &Utf8Path) -> Utf8PathBuf {
        path.strip_prefix(&self.manifest_dir)
            .unwrap_or(path)
            .to_owned()
    }

    // ───── Directory traversel ────────────────────────────────────── //
    /// Iterate over templates found in the template dir, if it exists
    ///
    /// Note that this currently ignores any errors if the initial
    /// `read_dir` call is succesful.
    pub fn iter_templates(
        &self,
    ) -> crate::Result<impl Iterator<Item = data::ScriptTemplate>> {
        let dir_reader = self.template_dir.read_dir().map_err(|ioerr| {
            crate::ioerr!(
                ioerr,
                "Could not access template dir: {}",
                self.template_dir
            )
        })?;

        // match all filies in the template directy named "X.rs.template" and turn
        // them into ScriptTemplate structs. Currently just ignores any entries
        // that we fail to read
        Ok(dir_reader
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| {
                // ignore anything that's not a file
                entry
                    .file_type()
                    .map(|ftype| ftype.is_file())
                    .unwrap_or(false)
            })
            .filter_map(|entry| Utf8PathBuf::try_from(entry.path()).ok())
            .filter_map(|path| {
                // yield templates with the extension ".rs.template"
                path.file_name()
                    .and_then(|fname| {
                        fname.strip_suffix(".rs.template").map(str::to_owned)
                    })
                    .map(|name| data::ScriptTemplate { name, path })
            }))
    }
}

// ───── Internal helpers ───────────────────────────────────────── //
const MANIFEST_MAX_SEARCH_DEPTH: u8 = 5;

/// Return the first enclosing directory that contains a Cargo.toml
///
/// This does not do anything related to workspaces or whatever,
/// and really only works for the playground model.
fn _first_parent_dir_with_a_manifest_in_it(
    starting_dir: &Utf8Path,
) -> crate::Result<Utf8PathBuf> {
    let mut dir = starting_dir;
    for _ in 0..MANIFEST_MAX_SEARCH_DEPTH {
        if dir.join("Cargo.toml").is_file() {
            return Ok(dir.to_owned());
        } else if let Some(next_dir) = dir.parent() {
            dir = next_dir
        } else {
            break;
        }
    }

    Err(crate::Error::NoConfig(
        "Could not locate Cargo.toml".to_owned(),
    ))
}

fn _getenv<T>(name: &str) -> crate::Result<T>
where
    T: From<String>,
{
    env::var(name)
        .map(T::from)
        .map_err(|_| crate::Error::EnvVarMissing(name.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_paths() {
        let paths = ProjectPaths::_from_manifest_dir_path_unchecked(
            "/tmp/project/hyvät".into(),
        );
        assert_eq!(
            paths
                .relpath_project_root("/tmp/project/hyvät/herrat".into())
                .as_str(),
            "herrat"
        );
        assert_eq!(
            paths.template_path("ahti").as_str(),
            "/tmp/project/hyvät/templates/ahti.rs.template"
        );
        assert_eq!(
            paths.cargo_dot_toml.as_str(),
            "/tmp/project/hyvät/Cargo.toml"
        );
    }
}
