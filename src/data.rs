use std::cell::OnceCell;
use std::path::PathBuf;

use crate::util;

derive_alias! {
    #[derive(PlainData!)] = #[derive(Clone, Debug, PartialEq, Eq)];
}

// ───── Requests from the user ─────────────────────────────────── //
/// A request to add dependencies and activate dependency features
/// for a given script.
///
/// Like all other requests, this represents user input and so
/// all of the strings should be canonicalized/normalized to
/// the extent possible.
#[derive(PlainData!)]
pub struct ScriptConfigRequest {
    pub script: String,
    pub deps: Vec<DepRequest>,
    pub features: Vec<FeatureRequest>,
    pub cargo_args: Vec<String>,
}

/// A request to add a dependency.
///
/// Like all other requests, this represents user input and so
/// all of the strings should be canonicalized/normalized to
/// the extent possible.
#[derive(PlainData!)]
pub struct DepRequest {
    pub depname: String,
    pub version: Option<String>,
    pub input_string: String,
}

/// A request to activate a feature of dependency
///
/// Like all other requests, this represents user input and so
/// all of the strings should be canonicalized/normalized to
/// the extent possible.
#[derive(PlainData!)]
pub struct FeatureRequest {
    pub depname: String,
    pub featurename: String,
}

/// Newtype wrapper for script names w/ cached canonicalization
#[derive(Clone, Debug, Eq)]
pub struct ScriptName {
    pub name: String,
    _canonicalized: OnceCell<String>,
}

impl PartialEq for ScriptName {
    fn eq(&self, other: &Self) -> bool {
        self.canonical() == other.canonical()
    }
}

impl ScriptName {
    pub fn new(name: String) -> Self {
        Self {
            name,
            _canonicalized: Default::default(),
        }
    }

    pub fn canonical(&self) -> &str {
        self._canonicalized
            .get_or_init(|| util::canonicalize_crate_name(&self.name))
    }
}

// ───── Data from cargo.toml ───────────────────────────────────── //
// /// Metadata from `[package.metadata.cargo-playground]`
// #[derive(PlainData!)]
// pub struct PlaygroundMetadata {
//     pub enabled: bool,
//     pub editable: bool,
//     pub editor_cmd: Option<String>,
// }

/// A playground script (aka a `[[bin]]` table) in Cargo.toml.
#[derive(PlainData!)]
pub struct ScriptEntry {
    pub name: String,
    pub path: String,
    pub required_features: Vec<String>,
}

/// A template from the playground's templates directory
#[derive(PlainData!)]
pub struct ScriptTemplate {
    pub name: String,
    pub path: PathBuf,
}
