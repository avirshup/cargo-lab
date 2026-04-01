pub use Verbosity::*;
use cfg_if::cfg_if;
use std::env;
use std::env::current_dir;
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
    pub template_dir: PathBuf, // TODO: this is not available in cargo subcmd mode
    pub cargo_dot_toml: PathBuf,
    pub cargo_exe: PathBuf,

    pub verbosity: Verbosity,
}

impl Config {
    // TODO: this needs to be split up into fallible parts (project paths
    //   which may not exist) and infallible parts (verbosity)
    pub fn from_env(verbosity: Verbosity) -> Self {
        let cwd = current_dir().expect("Failed to get cwd");

        cfg_if! {
            if #[cfg(feature = "xtask")] {
                let xtask_project_path = PathBuf::from(
                    env::var("CARGO_MANIFEST_DIR")
                    .expect("CARGO_MANIFEST_DIR not set; is this running as a cargo command?")
                );
                let template_dir = xtask_project_path.join("templates");

                // TODO: this is basically hardcoded that xtask is exactly
                //   located 1 level below project root. It probably should be configurable.
                let manifest_dir = xtask_project_path.parent().unwrap().to_owned();

            } else {
                let template_dir = _discover_template_dir().unwrap();
                let manifest_dir = _discover_manifest_dir().unwrap();
            }
        }

        // should be set by cargo, but can just default to `cargo`
        // TODO: would it be "cargo.exe" on windows??
        let cargo_exe = PathBuf::from(env::var("CARGO").unwrap_or("cargo".to_owned()));

        // FIXMEFIXME: this will not work unless xtask

        let cargo_dot_toml = manifest_dir.join("Cargo.toml");
        Self {
            cwd,
            manifest_dir,
            template_dir,
            cargo_exe,
            cargo_dot_toml,
            verbosity,
        }
    }

    pub fn template_path(&self, name: &str) -> PathBuf {
        self.template_dir.join(format!("{name}.rs.template"))
    }

    pub fn relpath_project_root<'a>(&'_ self, path: &'a Path) -> std::path::Display<'a> {
        path.strip_prefix(&self.manifest_dir)
            .unwrap_or(path)
            .display()
    }
}

/// Template dir when running as cargo subcmd (not xtask)
fn _discover_template_dir() -> Option<PathBuf> {
    // TODO: implement w/ xdg paths or something
    None
}

// tbh in valid cases it should not be more than 1 or _maybe_ 2 levels up
#[allow(dead_code)]
const MANIFEST_MAX_SEARCH_DEPTH: u8 = 5;

/// Return the first enclosing directory that contains a Cargo.toml
///
/// This does not do anything related to workspaces or whatever,
/// and really only works for the playground model.
fn _discover_manifest_dir() -> Option<PathBuf> {
    let cwd = current_dir().ok()?;

    let mut dir: &Path = cwd.as_ref();
    for _ in 0..MANIFEST_MAX_SEARCH_DEPTH {
        if dir.join("Cargo.toml").is_file() {
            return Some(dir.to_owned());
        } else {
            dir = dir.parent()?;
        }
    }

    None
}
