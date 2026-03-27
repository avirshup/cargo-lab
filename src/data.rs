pub const MISSING: &str = "<MISSING>";

/// A script (aka a `[[bin]]` table) in Cargo.toml.
/// If the `name` or `path` is missing, they will
/// have the value "<MISSING>".
pub struct ScriptEntry<'a> {
    pub name: &'a str,
    pub path: &'a str,
}
