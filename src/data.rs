use std::path::PathBuf;

pub const MISSING: &str = "<MISSING>";

// ───── "Requests" from the user ───────────────────────────────── //
/// A dependency's feature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureRequest {
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
// /// A dependency's feature.
// #[derive(Clone, Debug, PartialEq, Eq)]
// pub struct DepFeature {
//     pub depname: String,
//     pub featurename: String,
// }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptTemplate {
    pub name: String,
    pub path: PathBuf,
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
