use std::cell::OnceCell;
use std::env;
use std::env::current_dir;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub use Verbosity::*;
use cfg_if::cfg_if;
use serde::Deserialize;

use crate::manifest_data::{ManifestData, PlaygroundConfig};
use crate::manifest_editor::ManifestEditor;
use crate::{data, util};

type CachedResult<'a, T> = Result<T, &'a crate::Error>;

/// Loads configuration values as needed for a given operation.
///
/// This is all done lazily since different operations have
/// different configuration needs (e.g., the "init"
/// command does not even need a manifest,
/// but the `run` command needs a fully configured manifest).
pub struct GlobalCtx {
    pub verbosity: Verbosity,
    pub cwd: PathBuf,
    pub cargo_exe: PathBuf,

    _project_paths: OnceCell<crate::Result<ProjectPaths>>,
    _manifest_raw: OnceCell<crate::Result<String>>,
    _manifest_data: OnceCell<crate::Result<ManifestData>>,
    _playground_config: OnceCell<Option<PlaygroundConfig>>,
    // NB: editable manifest TOML does not belong here because it's mutable
}

impl GlobalCtx {
    pub fn from_env(verbosity: Verbosity) -> crate::Result<Self> {
        let cwd = current_dir()
            .map_err(|ioerr| crate::ioerr!(ioerr, "Failed to determine CWD"))?;

        let cargo_exe =
            PathBuf::from(env::var("CARGO").unwrap_or("cargo".to_owned()));

        Ok(Self {
            verbosity,
            cwd,
            cargo_exe,
            _project_paths: Default::default(),
            _manifest_raw: Default::default(),
            _manifest_data: Default::default(),
            _playground_config: Default::default(),
        })
    }

    /// Create a new context to re-read the manifest after, e.g., it's changed on disk.
    ///
    /// Note this is still lazy so it doesn't actually re-read the manifest from disk
    /// until requested.
    pub fn reload(&self) -> GlobalCtx {
        Self {
            verbosity: self.verbosity,
            cwd: self.cwd.clone(),
            cargo_exe: self.cargo_exe.clone(),
            _project_paths: Default::default(),
            _manifest_raw: Default::default(),
            _manifest_data: Default::default(),
            _playground_config: Default::default(),
        }
    }

    pub fn manifest_raw(&'_ self) -> CachedResult<'_, &String> {
        self._manifest_raw
            .get_or_init(|| {
                self.project_paths()
                    .map_err(Clone::clone)
                    .and_then(|paths| util::read_file(&paths.cargo_dot_toml))
            })
            .as_ref()
    }

    pub fn project_paths(&'_ self) -> CachedResult<'_, &ProjectPaths> {
        self._project_paths
            .get_or_init(|| ProjectPaths::discover(&self.cwd))
            .as_ref()
    }

    pub fn manifest_data(&'_ self) -> CachedResult<'_, &ManifestData> {
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
                .and_then(|x| x.playground_config().map(|x| x.clone()))
        })
    }

    /// Return a _fresh_ editable copy of the manifest from its original
    /// state. Unlike most of the other methods here, this is *not* cached
    /// (it constructs a new editor every time it is called).
    pub fn new_editor(&self) -> crate::Result<ManifestEditor> {
        self.manifest_raw()
            .map_err(Clone::clone)
            .and_then(|s| ManifestEditor::from_string(s))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectPaths {
    pub manifest_dir: PathBuf,
    pub template_dir: PathBuf,
    pub cargo_dot_toml: PathBuf,
}

impl ProjectPaths {
    pub fn discover(cwd: &Path) -> crate::Result<Self> {
        cfg_if! {
            if #[cfg(feature = "xtask")] {
                let xtask_project_path = PathBuf::from(
                    _getenv("CARGO_MANIFEST_DIR")?
                );
                let template_dir = xtask_project_path.join("templates");

                // TODO: this is basically hardcoded that xtask is exactly
                //   located 1 level below project root. It probably should be configurable.
                let manifest_dir = xtask_project_path
                    .parent()
                    .ok_or_else(||
                        crate::Error::NoConfig(format!("Could not find parent directory of {cwd:?}?"))
                    )?
                    .to_owned();

            } else {
                let manifest_dir = _discover_manifest_dir(&cwd)?;
                let template_dir = manifest_dir.join("templates");
            }
        }

        // FIXMEFIXME: this will not work unless xtask

        let cargo_dot_toml = manifest_dir.join("Cargo.toml");
        Ok(Self {
            manifest_dir,
            template_dir,
            cargo_dot_toml,
        })
    }

    pub fn template_path(&self, name: &str) -> PathBuf {
        self.template_dir.join(format!("{name}.rs.template"))
    }

    pub fn relpath_project_root<'a>(
        &'_ self,
        path: &'a Path,
    ) -> std::path::Display<'a> {
        path.strip_prefix(&self.manifest_dir)
            .unwrap_or(path)
            .display()
    }

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
                self.template_dir.to_string_lossy()
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
            .map(|entry| entry.path())
            .filter_map(|path| {
                // yield templates with the extension ".rs.template"
                path.file_name()
                    .map(OsStr::to_string_lossy)
                    .and_then(|fname| {
                        fname.strip_suffix(".rs.template").map(str::to_owned)
                    })
                    .map(|name| data::ScriptTemplate { name, path })
            }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verbosity {
    Quiet,
    Normal,
    Verbose,
    Debug,
}

//
// #[derive(Clone, Debug, PartialEq, Eq)]
// pub struct Config {
//     pub cwd: PathBuf,
//     pub manifest_dir: PathBuf,
//     pub template_dir: PathBuf,
//     pub cargo_dot_toml: PathBuf,
//     pub cargo_exe: PathBuf,
//     pub verbosity: Verbosity,
// }

// tbh in valid cases it should not be more than 1 or _maybe_ 2 levels up
#[allow(dead_code)]
const MANIFEST_MAX_SEARCH_DEPTH: u8 = 5;

/// Return the first enclosing directory that contains a Cargo.toml
///
/// This does not do anything related to workspaces or whatever,
/// and really only works for the playground model.
fn _discover_manifest_dir(starting_dir: &Path) -> crate::Result<PathBuf> {
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

fn _getenv(name: &str) -> crate::Result<String> {
    env::var(name).map_err(|_| crate::Error::EnvVarMissing(name.to_owned()))
}
