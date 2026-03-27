mod cli;
mod commands;
mod data;
mod errors;
mod manifest_editor;

use crate::cli::{ArgParser, SubCmd};
use crate::errors::{Error, Result};
use clap::Parser;
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    let args = ArgParser::parse();
    let paths = ProjectPaths::from_env();

    match args.cmd {
        SubCmd::NewScript { bin_name, template } => {
            commands::new_script(&bin_name, &template, &paths)?;
        }
        SubCmd::ListScripts { quiet } => {
            commands::list_scripts(quiet, &paths)?;
        }
        SubCmd::InjectDep {
            bin_name,
            dep_name,
            dep_features,
        } => {
            commands::inject_deps(&bin_name, &dep_name, &dep_features, &paths)?;
        }
    };

    Ok(())
}

// ───── Project tree helper ────────────────────────────────────── //
pub struct ProjectPaths {
    pub manifest_dir: PathBuf,
    pub template_dir: PathBuf,
    pub cargo_dot_toml: PathBuf,
    pub script_dir: PathBuf,
}

impl ProjectPaths {
    pub fn from_env() -> Self {
        let manifest_dir = Path::new(&env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let template_dir = manifest_dir.join("xtask/templates");
        let cargo_dot_toml = manifest_dir.join("Cargo.toml");
        let script_dir = manifest_dir.join("src");
        Self {
            manifest_dir,
            template_dir,
            script_dir,
            cargo_dot_toml,
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
