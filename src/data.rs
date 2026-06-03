use camino::Utf8PathBuf;

// ───── Requests from the user ─────────────────────────────────── //
/// A request to add dependencies and activate dependency features
/// for a given script.
///
/// Like all other requests, this represents user input and so
/// all of the strings should be canonicalized/normalized to
/// the extent possible.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ScriptConfigRequest {
    pub script: String,
    pub deps: Vec<DepRequest>,
    pub features: Vec<FeatureRequest>,
    pub cargo_args: Vec<String>,
}

impl ScriptConfigRequest {
    pub fn nodeps(script: String) -> Self {
        Self {
            script,
            ..Default::default()
        }
    }
}

/// A request to add a dependency.
///
/// Like all other requests, this represents user input and so
/// all of the strings should be canonicalized/normalized to
/// the extent possible.
#[derive(Clone, Debug, PartialEq, Eq)]
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureRequest {
    pub depname: String,
    pub featurename: String,
}

// ───── Data from cargo.toml ───────────────────────────────────── //
// This is data that's been normalized and validated from the schemas
// in `manifest_data.rs`. Probably should make it harder to accidentally
// use the manifest data directly when you really want to use these instead.

/// A lab script (aka a `[[bin]]` table) in Cargo.toml.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptEntry {
    pub name: String,
    pub path: Utf8PathBuf, // relative to manifest root!
    pub required_features: Vec<String>,
}

/// A template from the lab's templates directory
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptTemplate {
    pub name: String,
    pub path: Utf8PathBuf,
}
