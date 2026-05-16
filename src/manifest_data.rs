/// Schemas for deserializing Cargo.toml.
///
/// This intentionally only parses a small subset of a full
/// Cargo.toml document, and as such is only be appropriate for
/// *de*-serialization (trying to write it back would cause
/// any unhandled fields to be deleted).
/// *Edits* of Cargo.toml are handled imperatively,
/// see `[./manifest_editor.rs](manifest_editor.rs)`.)
///
/// This was originally derived from cargo's source code,
/// although it's been almost fully ship-of-theseus'd now.
use std::collections::BTreeMap;
use std::str::FromStr;

use serde::Deserialize;

use crate::{data, util};

attribute_alias! {
   #[apply(TomlTable!)] = #[derive(Deserialize, Clone, Debug, PartialEq, Eq, Default)];
}

/// Deserialized relevant data from Cargo.toml
#[apply(TomlTable!)]
#[serde(rename_all = "kebab-case")]
pub struct ManifestData {
    pub package: Option<PackageTable>,

    #[serde(default = "BTreeMap::new")]
    pub features: BTreeMap<String, Vec<String>>,

    #[serde(default = "Vec::new")]
    pub bin: Vec<BinTable>,

    #[serde(default = "BTreeMap::new")]
    pub dependencies: BTreeMap<String, DepEntry>,
}

impl ManifestData {
    pub fn playground_config(&self) -> Option<&PlaygroundConfig> {
        self.package
            .as_ref()?
            .metadata
            .as_ref()?
            .cargo_playground
            .as_ref()
    }

    /// Returns true if cargo.toml already has this dependency and a version that's
    /// compatible with the request
    pub fn dep_satisfied(&self, request: &data::DepRequest) -> bool {
        self.get_dep(&request.depname)
            .map(|table| {
                // if we found a matching dep, check if the versions count as a match
                match (&request.version, table.version) {
                    (Some(req), Some(actual)) => *req == actual,
                    (None, _) => true, // no version requested, so any version matches
                    _otherwise => false, // nothing else counts as a match
                }
            })
            .unwrap_or(false)
    }

    pub fn get_dep(&self, name: &str) -> Option<DependencyTable> {
        let canon_request_name = util::canonicalize_crate_name(name);
        self.dependencies.iter().find_map(|(name, entry)| {
            if canon_request_name == util::canonicalize_crate_name(name) {
                Some(entry.into())
            } else {
                None
            }
        })
    }

    // ───── Script entry getters ──────────────────────────────────── //
    // NOTE: these don't directly yield the underlying data from the manifest;
    //   they transform the data into a `data::ScriptEntry` (and ignore any
    //   malformed manifest entries that can't be transformed into one).
    //   That makes these different than the depenency getters (above) that
    //   directly yield data from the TOML ...
    pub fn get_script(&self, input_name: &str) -> Option<data::ScriptEntry> {
        let canon_name = util::canonicalize_crate_name(input_name);

        self.iter_script_entries().find(|entry| {
            util::canonicalize_crate_name(&entry.name) == canon_name
        })
    }

    /// yield the well-formed scripts in this manifest.
    ///
    /// Note that this skips entries that are missing name or path (which
    /// are both, in fact, required)
    pub fn iter_script_entries(
        &self,
    ) -> impl Iterator<Item = data::ScriptEntry> {
        self.bin.iter().filter_map(|bin_table| {
            Some(data::ScriptEntry {
                name: bin_table.name.as_ref()?.clone(),
                path: bin_table.path.as_ref()?.into(),
                required_features: bin_table.required_features.clone(),
            })
        })
    }
}

/// The top-level `[package]` table
#[apply(TomlTable!)]
#[serde(rename_all = "kebab-case")]
pub struct PackageTable {
    pub name: Option<String>,
    pub metadata: Option<PackageMetadata>,
}

/// An entry in the `[[bin]]` array-of-tables
#[apply(TomlTable!)]
#[serde(rename_all = "kebab-case")]
pub struct BinTable {
    pub name: Option<String>,
    pub path: Option<String>,

    #[serde(default = "Vec::new")]
    pub required_features: Vec<String>,
    pub edition: Option<String>,
}

/// A value in the `[dependency]` key-value map
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum DepEntry {
    /// i.e., `package = "<version>"`
    VersionOnly(String),

    /// i.e., `package = { version = "<verion>", /* ...other fields */ }`
    Table(DependencyTable),
}

#[apply(TomlTable!)]
#[serde(rename_all = "kebab-case")]
pub struct DependencyTable {
    pub version: Option<String>,

    #[serde(default = "Vec::new")]
    pub features: Vec<String>,

    #[serde(default = "_false")]
    pub optional: bool,
}

/// convert from the union of possible TOML reperesentations
/// into the full table.
///
/// TODO: can serde be made to do this automatically ... without
///   having to mess around with a `Deserialize` impl?
impl From<&DepEntry> for DependencyTable {
    fn from(value: &DepEntry) -> Self {
        match value {
            DepEntry::VersionOnly(version) => Self {
                version: Some(version.clone()),
                ..Default::default()
            },
            DepEntry::Table(table) => table.clone(),
        }
    }
}

/// For converting from a plain `<package> = "<version>"`
/// specifier into a the full table
impl FromStr for DependencyTable {
    type Err = ();

    fn from_str(s: &str) -> Result<DependencyTable, ()> {
        Ok(Self {
            version: Some(s.to_owned()),
            ..Default::default()
        })
    }
}

#[apply(TomlTable!)]
#[serde(rename_all = "kebab-case")]
pub struct PackageMetadata {
    pub cargo_playground: Option<PlaygroundConfig>,
}

#[apply(TomlTable!)]
#[serde(rename_all = "kebab-case")]
pub struct PlaygroundConfig {
    #[serde(default = "_false")]
    pub enabled: bool,

    pub editor_cmd: Option<Vec<String>>,

    /// for using experimental cargo script frontmatter features
    /// see https://rust-lang.github.io/rfcs/3502-cargo-script.html
    #[serde(default = "_false")]
    pub experimental_rfc_3502_scripts: bool,
}

/// surely there is a better way to set defaults for bools in serde?
fn _false() -> bool {
    false
}

/// surely there is a better way to set defaults for bools in serde?
fn _true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn _s(s: &str) -> String {
        s.to_owned()
    }
    const TOML: &str = r#"
[package]
name = "packagename"

[dependencies]
plain_dep = "0.1"
inline-dep = {version = "0.2", optional=true, features=["oh-yes"]}

[dependencies.tabledep]
version = "0.3.0-dev4"
optional = true
features = ["oh-no"]

[package.metadata.cargo-playground]
enabled = true
editor-cmd = ["delphi", "-p"]  # not really

[[bin]]
name = "bin1"
path = "bin1.rs"
required-features = ["a1/a2"]
edition = "252525"
"#;

    #[test]
    fn test_dep_satisfaction_checks() {
        let manifest = parse_toml();

        assert!(manifest.dep_satisfied(&_depreq("Plain-Dep", Some("0.1"))));
        assert!(manifest.dep_satisfied(&_depreq("Plain-Dep", None)));

        // it's NOT satisfied if versions are specified but do not exactly match
        // (just pure string comparisons rn, no semver compatibility specs or anything)
        assert!(!manifest.dep_satisfied(&_depreq("Plain-Dep", Some("0.1.2"))));

        assert!(!manifest.dep_satisfied(&_depreq("doesnotexist", None)));
    }

    #[test]
    fn test_dep_getters() {
        let manifest = parse_toml();

        assert_eq!(manifest.get_dep("does-not-exist"), None);

        assert_eq!(
            manifest.get_dep("plain_dep"),
            Some(DependencyTable {
                version: Some(_s("0.1")),
                features: vec![],
                optional: false,
            })
        );

        assert_eq!(
            manifest.get_dep("inLine_Dep"),
            Some(DependencyTable {
                version: Some(_s("0.2")),
                features: vec![_s("oh-yes")],
                optional: true,
            })
        );
    }

    #[test]
    fn test_script_getters() {
        let manifest = parse_toml();

        assert_eq!(
            manifest.get_script("BiN1"),
            Some(data::ScriptEntry {
                name: _s("bin1"),
                path: "bin1.rs".into(),
                required_features: vec![_s("a1/a2")],
            })
        );

        // this is mispelled (has an extra dash), so does not retrieve it
        assert_eq!(manifest.get_script("bin-1"), None);
    }

    #[test]
    fn test_deserialize() {
        let actual = parse_toml();

        let expected = ManifestData {
            package: Some(PackageTable {
                name: Some(_s("packagename")),
                metadata: Some(PackageMetadata {
                    cargo_playground: Some(PlaygroundConfig {
                        enabled: true,
                        editor_cmd: Some(vec![_s("delphi"), _s("-p")]),
                        experimental_rfc_3502_scripts: false,
                    }),
                }),
            }),
            features: Default::default(),
            bin: vec![BinTable {
                name: Some(_s("bin1")),
                path: Some(_s("bin1.rs")),
                required_features: vec![_s("a1/a2")],
                edition: Some(_s("252525")),
            }],
            dependencies: [
                (
                    _s("plain_dep"), //
                    DepEntry::VersionOnly(_s("0.1")),
                ),
                (
                    _s("inline-dep"),
                    DepEntry::Table(DependencyTable {
                        version: Some(_s("0.2")),
                        features: vec![_s("oh-yes")],
                        optional: true,
                    }),
                ),
                (
                    _s("tabledep"),
                    DepEntry::Table(DependencyTable {
                        version: Some(_s("0.3.0-dev4")),
                        features: vec![_s("oh-no")],
                        optional: true,
                    }),
                ),
            ]
            .into(),
        };

        assert_eq!(actual, expected);
    }

    // ───── test helpers ───────────────────────────────────────────── //
    fn parse_toml() -> ManifestData {
        let de = toml_edit::de::Deserializer::parse(TOML)
            .expect("parsing test data as valid TOML");
        ManifestData::deserialize(de).expect("deserializing failed")
    }

    fn _depreq(dep: &str, version: Option<&str>) -> data::DepRequest {
        data::DepRequest {
            depname: dep.to_owned(),
            version: version.map(str::to_owned),
            input_string: "whatever".to_owned(),
        }
    }
}
