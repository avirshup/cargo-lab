use std::collections::HashSet;

use all_the_errors::CollectAllTheErrors;
use camino::Utf8Path;
use serde::Deserialize;
use serde::de::IntoDeserializer;
use toml_edit::{Array, ArrayOfTables, Item, Table, TableLike, Value};

use crate::manifest_data::ManifestData;
use crate::{data, util};

/// In-memory, editable representation of Cargo.toml via [`toml_edit`]
pub struct ManifestEditor {
    doc: toml_edit::DocumentMut,
}

impl ManifestEditor {
    // ───── Constructors ─────
    #[allow(unused)] // for testing?
    /// Create a new in-memory "manifest" with nothing in it
    pub fn blank() -> Self {
        Self {
            doc: toml_edit::DocumentMut::new(),
        }
    }

    pub fn from_string(content: &str) -> crate::Result<Self> {
        let doc = content.parse::<toml_edit::DocumentMut>()?;
        Ok(Self { doc })
    }

    // ───── Renderers ─────
    pub fn render(&self) -> String {
        self.doc.to_string()
    }

    pub fn write(&self, target: &Utf8Path) -> crate::Result<()> {
        util::write_file(target, &self.render())
    }

    // ───── Old methods for testing ────────────────────────────────── //
    // TODO: remove these and update tests to use the ManifestData deserializer instead
    /// reify the current state of this editable document into a concrete data object
    /// (unlike `GlobalCtx.manifest_data()`, which deserializes the string that was read from disk)
    ///
    /// (currently just used for testing)
    #[allow(unused)]
    pub fn deserialize(&self) -> crate::Result<ManifestData> {
        let deserializer = self.doc.clone().into_deserializer();
        let result = ManifestData::deserialize(deserializer)?;
        Ok(result)
    }

    // ───── Queries ─────
    // TODO: these are probably not necessary anymore - use the serde
    //   deserializer for this. They are still useful for testing
    #[allow(unused)]
    pub fn get_script(&self, input_name: &str) -> Option<data::ScriptEntry> {
        let canon_name = util::canonicalize_crate_name(input_name);

        self.iter_scripts()?.find(|entry| {
            util::canonicalize_crate_name(&entry.name) == canon_name
        })
    }

    #[allow(unused)]
    pub fn iter_scripts(
        &self,
    ) -> Option<impl Iterator<Item = data::ScriptEntry>> {
        self.doc.get("bin")?.as_array_of_tables().map(|arr| {
            arr.iter()
                .filter_map(|table| Self::_table_to_script_entry(table).ok())
        })
    }

    // ───── Mutators ─────
    pub fn update_bin(
        &mut self,
        bin_name: &str,
        maybe_new_name: Option<&str>,
        maybe_new_path: Option<&str>,
    ) -> crate::Result<()> {
        let entry = self
            ._get_bin_entry_mut(bin_name)
            .ok_or_else(|| crate::Error::ScriptNotFound(bin_name.to_owned()))?;

        if let Some(name) = maybe_new_name {
            entry["name"] = name.into();
        }

        if let Some(path) = maybe_new_path {
            entry["path"] = path.into();
        }

        Ok(())
    }

    pub fn add_new_bin(
        &mut self,
        bin_name: &str,
        src_path: &str,
    ) -> crate::Result<()> {
        // ───── Part 1: insert bin array ───────────────────────────────── //
        // get or create top-level `[[bin]]` array
        let bin_array = self.doc["bin"]
            .or_insert(ArrayOfTables::new().into())
            .as_array_of_tables_mut()
            .ok_or_else(|| {
                crate::Error::ManifestStructureErr(
                    "'[[bin]]' key exists but is not an array of tables"
                        .to_string(),
                )
            })?;

        // ensure this entry does not already exist
        if bin_array
            .iter()
            .any(|table| table["name"].as_str() == Some(bin_name))
        {
            return Err(crate::Error::ScriptNameConflict(bin_name.to_owned()));
        };

        // create the `[[bin]]` table entry
        let bin_table = {
            let mut bin_table = Table::new();
            bin_table["name"] = bin_name.into();
            bin_table["path"] = src_path.into();
            bin_table["required-features"] = Array::new().into();

            bin_table
        };

        bin_array.push(bin_table);

        Ok(())
    }

    /// Activate denpency features for a script with `required-features`.
    /// Will fail if the dependencies or script does not exist.
    ///
    /// ## Example
    /// For instance, calling
    /// `manifest.add_dep_to_feature("myscript", &[DepFeature{pkg: "mypkg", feature: "myfeature")])`
    /// would change this:
    /// ```toml
    /// [[bin]]
    /// name = "myscript"
    /// path = "src/myscript.rs"
    /// required-features = []
    /// ```
    /// into this:
    /// ```toml
    /// # [...]
    /// required-features = ["mypkg/myfeature"]
    /// ```
    ///
    /// ## Notes
    /// You actually don't have to activate a dependency if you've
    /// activated one of its features - e.g., you don't have to list
    /// `dep:syn` if you've already listed `syn/parsing`.
    pub fn activate_features(
        &mut self,
        input_script_name: &str,
        dep_requests: &[data::DepRequest],
        feature_requests: &[data::FeatureRequest],
    ) -> crate::Result<()> {
        // ───── Part 1: figure out what we're adding ─────
        let dep_strs = dep_requests.iter().map(|dep| {
            self._normalize_dep_name(&dep.depname).map(str::to_owned)
        });

        let feature_activation_strs =
            feature_requests.iter().map(|feature_req| {
                self._normalize_dep_name(&feature_req.depname)
                    .map(|dep| format!("{}/{}", dep, feature_req.featurename))
            });

        let feature_strs: Vec<String> = dep_strs
            .chain(feature_activation_strs)
            .collect_oks_or_iter_errs()
            .map_err(crate::Error::from_nonempty_iter)?;

        // ───── part 2: add it ─────
        let bin_entry =
            self._get_bin_entry_mut(input_script_name).ok_or_else(|| {
                crate::Error::ScriptNotFound(input_script_name.to_owned())
            })?;

        let feature_array = bin_entry["required-features"]
            .or_insert(Array::new().into())
            .as_array_mut()
            .ok_or_else(|| {
                crate::Error::ManifestStructureErr(
                    "'required-features' for script 'input_bin_name' is not \
                     an array"
                        .to_owned(),
                )
            })?;

        _add_unique_strings_to_array(feature_array, feature_strs.into_iter());

        Ok(())
    }

    // ───── internal helpers ───────────────────────────────────────── //
    fn _normalize_dep_name(&self, input_name: &str) -> crate::Result<&str> {
        self._find_dep_name(input_name).ok_or_else(|| {
            crate::Error::DependencyNotFound((*input_name).to_owned())
        })
    }

    /// Find a script matching the input name
    /// To match `cargo` behavior, this is case- and
    /// undescore/hyphen-insensitive
    fn _find_dep_name(&self, input_dep_name: &str) -> Option<&str> {
        let canonicalized_input = util::canonicalize_crate_name(input_dep_name);
        let dep_table = self.doc.get("dependencies")?.as_table_like()?;

        dep_table.iter().map(|(k, _v)| k).find(|key| {
            util::canonicalize_crate_name(key) == canonicalized_input
        })
    }

    /// Find a script matching the input name.
    /// Similar to cargo's package-matching behavior, this is case- and
    /// undescore/hyphen-insensitive
    fn _get_bin_entry_mut(&mut self, input_name: &str) -> Option<&mut Table> {
        let bin_array = self.doc.get_mut("bin")?.as_array_of_tables_mut()?;
        let canonicalized_input = util::canonicalize_crate_name(input_name);

        bin_array.iter_mut().find(|table| {
            table
                .get("name")
                .and_then(Item::as_str)
                .map(|name| {
                    util::canonicalize_crate_name(name) == canonicalized_input
                })
                .unwrap_or(false)
        })
    }

    // pub fn find_bin_name(&self, input_name: &str) -> Option<&str> {
    //     let canonicalized_input = _canonicalize_name(input_name);
    //
    //     self.doc
    //         .get("bin")?
    //         .as_array_of_tables()?
    //         .iter()
    //         .filter_map(|table| table.get("name"))
    //         .filter_map(Item::as_str)
    //         .find(|realname| _canonicalize_name(realname) == canonicalized_input)
    // }

    // fn _table_to_metadata<'de, T>(
    //     table: &'de T,
    // ) -> crate::Result<data::PlaygroundMetadata<'de>>
    // where
    //     T: TableLike + IntoDeserializer<'de>,
    // {
    //     data::PlaygroundMetadata::deserialize(table.into_deserializer())
    //         .map_err(crate::Error::from_serde_err)
    // }

    fn _get_str_value<'src>(
        table: &'src impl TableLike,
        key: &'static str,
        err_place: &'static str,
    ) -> crate::Result<&'src str> {
        table
            .get(key)
            .ok_or_else(|| {
                crate::Error::ManifestStructureErr(format!(
                    "{err_place} is missing required key '{key}'"
                ))
            })?
            .as_str()
            .ok_or_else(|| {
                crate::Error::ManifestStructureErr(format!(
                    "In, {err_place} '{key}' must be a string"
                ))
            })
    }

    fn _table_to_script_entry(
        table: &Table,
    ) -> crate::Result<data::ScriptEntry> {
        let name =
            Self::_get_str_value(table, "name", "[[bin]] entry")?.to_owned();
        let path =
            Self::_get_str_value(table, "path", "[[bin]] entry")?.to_owned();

        let result = data::ScriptEntry {
            name,
            path: path.into(),
            required_features: table
                .get("required-features")
                .and_then(Item::as_array)
                .map(|array| {
                    array
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect::<Vec<String>>()
                })
                .unwrap_or(vec![]),
        };

        Ok(result)
    }
}

fn _add_unique_strings_to_array(
    arr: &mut Array,
    new_strs: impl Iterator<Item = String>,
) {
    let mut existing: HashSet<String> = arr
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)  // because we can't borrow here
        .collect();

    for new_str in new_strs {
        if !existing.contains(&new_str) {
            arr.push(&new_str);
            existing.insert(new_str);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_script() {
        const TOML: &str = r#"
[[bin]]
name = "my-script"
path = "src/my_script.rs"
required-features = ["hi", "hi/there"]
"#;

        let doc = ManifestEditor::from_string(TOML).unwrap();
        let expected = Some(data::ScriptEntry {
            name: "my-script".to_owned(),
            path: "src/my_script.rs".into(),
            required_features: vec!["hi".to_owned(), "hi/there".to_owned()],
        });

        for name in ["my-script", "My_ScriPt"] {
            let actual = doc.get_script(name);
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn test_add_new_script() {
        const TOML: &str = r#"
[package]
edition = "252525"  # now this song is going through your head

[dependencies]
"#;
        let mut doc = ManifestEditor::from_string(TOML).unwrap();
        doc.add_new_bin("do-thing", "do_thing.rs").unwrap();
        let actual = doc.render();
        let expected = r#"
[package]
edition = "252525"  # now this song is going through your head

[dependencies]

[[bin]]
name = "do-thing"
path = "do_thing.rs"
required-features = []
"#;

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_add_features_while_maintaining_order() {
        const TOML: &str = r#"
[dependencies]
d1 = {version = "1.2.3", optional=true}
d2 = {version = "1.2.3", optional=true}

[features]
d1 = ["dep:d1"]
d2 = ["dep:d2"]

[[bin]]
name = "s1"
path = "src/s1.rs"

[[bin]]
name = "s2"
path = "src/s2.rs"
required-features = ["d2", "something"]
"#;

        let mut doc = ManifestEditor::from_string(TOML).unwrap();

        doc.activate_features(
            "s1",
            &[_depreq("d1")],
            &[_featurereq("d2", "f2")],
        )
        .unwrap();
        doc.activate_features(
            "s2",
            &[_depreq("d1"), _depreq("d2")],
            &[_featurereq("d1", "f1"), _featurereq("d2", "f2")],
        )
        .unwrap();

        let actual = doc.render();
        let expected = r#"
[dependencies]
d1 = {version = "1.2.3", optional=true}
d2 = {version = "1.2.3", optional=true}

[features]
d1 = ["dep:d1"]
d2 = ["dep:d2"]

[[bin]]
name = "s1"
path = "src/s1.rs"
required-features = ["d1", "d2/f2"]

[[bin]]
name = "s2"
path = "src/s2.rs"
required-features = ["d2", "something", "d1", "d1/f1", "d2/f2"]
"#;
        assert_eq!(actual, expected);
    }

    // ───── test helpers ───────────────────────────────────────────── //
    fn _depreq(dep: &str) -> data::DepRequest {
        data::DepRequest {
            depname: dep.to_owned(),
            version: None,
            input_string: "whatever".to_owned(),
        }
    }

    fn _featurereq(dep: &str, feature: &str) -> data::FeatureRequest {
        data::FeatureRequest {
            depname: dep.to_owned(),
            featurename: feature.to_owned(),
        }
    }
}
