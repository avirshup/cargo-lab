use std::env;
use std::path::{Path, PathBuf};

pub use Verbosity::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verbosity {
    Quiet,
    Normal,
    Verbose,
    Debug,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub manifest_dir: PathBuf,
    pub template_dir: PathBuf,
    pub cargo_dot_toml: PathBuf,
    pub cargo_exe: PathBuf,

    pub verbosity: Verbosity,
}

impl Config {
    pub fn from_env(verbosity: Verbosity) -> Self {
        let manifest_dir = PathBuf::from(
            env::var("CARGO_MANIFEST_DIR")
                .expect("CARGO_MANIFEST_DIR not set; is this running as a cargo command?"),
        );

        #[cfg(feature = "xtask")]
        let manifest_dir = manifest_dir.parent().unwrap().to_owned();

        // set by cargo when calling tools
        let cargo_exe = PathBuf::from(env::var("CARGO").unwrap_or("cargo".to_owned()));

        let template_dir = manifest_dir.join("xtask/templates");
        let cargo_dot_toml = manifest_dir.join("Cargo.toml");
        Self {
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

    pub fn relative_to_root<'a>(&'_ self, path: &'a Path) -> &'a Path {
        path.strip_prefix(self.manifest_dir.clone()).unwrap_or(path)
    }

    pub fn humanize<'a>(&'_ self, path: &'a Path) -> std::path::Display<'a> {
        self.relative_to_root(path).display()
    }
}
