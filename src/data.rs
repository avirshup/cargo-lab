use std::path::PathBuf;

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
pub struct ScriptRequest {
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

// ───── Data from cargo.toml ───────────────────────────────────── //
/// Metadata from `[package.metadata.cargo-playground]`
#[derive(PlainData!)]
pub struct PlaygroundMetadata {
    pub enabled: bool,
    pub editable: bool,
    pub editor_cmd: Option<String>,
}

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
