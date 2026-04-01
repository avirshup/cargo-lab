use crate::data;
pub use Verbosity::*;
use cfg_if::cfg_if;
use std::env;
use std::env::current_dir;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verbosity {
    Quiet,
    Normal,
    Verbose,
    Debug,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub cwd: PathBuf,
    pub manifest_dir: PathBuf,
    pub template_dir: PathBuf, // TODO: make this optional probably?
    pub cargo_dot_toml: PathBuf,
    pub cargo_exe: PathBuf,
    pub verbosity: Verbosity,
}

impl Config {
    // TODO: this needs to be split up into fallible parts (project paths
    //   which may not exist) and infallible parts (verbosity)
    pub fn from_env(verbosity: Verbosity) -> crate::Result<Self> {
        let cwd = current_dir().map_err(|ioerr| {
            crate::Error::IoFail("Failed to determine CWD".to_owned(), ioerr)
        })?;

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

        // should be set by cargo, but can just default to `cargo`
        // TODO: would it be "cargo.exe" on windows??
        let cargo_exe =
            PathBuf::from(env::var("CARGO").unwrap_or("cargo".to_owned()));

        // FIXMEFIXME: this will not work unless xtask

        let cargo_dot_toml = manifest_dir.join("Cargo.toml");
        Ok(Self {
            cwd,
            manifest_dir,
            template_dir,
            cargo_exe,
            cargo_dot_toml,
            verbosity,
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
            crate::Error::IoFail(
                format!(
                    "Could not access template dir: {}",
                    self.template_dir.to_string_lossy()
                ),
                ioerr,
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
