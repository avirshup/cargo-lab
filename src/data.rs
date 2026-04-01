use std::path::{Path, PathBuf};

pub const MISSING: &str = "<MISSING>";

// ───── "Requests" from the user ───────────────────────────────── //
/// A dependency's feature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FeatureRequest {
    pub depname: String,
    pub featurename: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DepRequest {
    pub depname: String,
    pub version: Option<String>,
    pub input_string: String,
}

// ───── Data from cargo.toml ───────────────────────────────────── //
/// A dependency's feature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DepFeature {
    pub depname: String,
    pub featurename: String,
}

/// A script (aka a `[[bin]]` table) in Cargo.toml.
/// If the `name` or `path` is missing, they will
/// have the value "<MISSING>".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptEntry<'a> {
    pub name: &'a str,
    pub path: &'a str,
    pub required_features: Vec<&'a str>,
}

// ───── Environmental data ─────────────────────────────────────── //
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectPaths {
    pub manifest_dir: PathBuf,
    pub template_dir: PathBuf,
    pub cargo_dot_toml: PathBuf,

    #[allow(unused)] // for now
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
