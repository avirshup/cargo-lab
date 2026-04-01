use std::collections::HashSet;
use std::fs;
use std::path::Path;

use all_the_errors::CollectAllTheErrors;
use toml_edit::{Array, ArrayOfTables, Item, Table, Value};

use crate::data;
use crate::data::{MISSING, ScriptEntry};

pub struct CargoDotToml(toml_edit::DocumentMut);

impl CargoDotToml {
    // ───── Constructors ─────
    pub fn read(path: &Path) -> crate::Result<Self> {
        let toml_content = fs::read_to_string(path).map_err(|ioerr| {
            crate::Error::IoFail(
                format!("Failed to read '{}'", path.to_string_lossy()),
                ioerr,
            )
        })?;
        let doc = toml_content.parse::<toml_edit::DocumentMut>()?;
        Ok(Self(doc))
    }

    #[allow(unused)] // for testing mostly
    pub fn from_string(content: &str) -> crate::Result<Self> {
        let doc = content.parse::<toml_edit::DocumentMut>()?;
        Ok(Self(doc))
    }

    // ───── Renderers ─────
    #[allow(unused)] // for testing mostly
    pub fn render(&self) -> String {
        self.0.to_string()
    }

    pub fn write(&self, target: &Path) -> crate::Result<()> {
        fs::write(target, self.0.to_string()).map_err(|ioerr| {
            crate::Error::IoFail(
                format!("Failed to write to {}", target.to_string_lossy()),
                ioerr,
            )
        })?;
        Ok(())
    }

    // ───── Queries ─────
    pub fn get_script(&self, input_name: &str) -> Option<ScriptEntry<'_>> {
        let canon_name = _canonicalize_name(input_name);

        self.list_scripts()?
            .find(|entry| _canonicalize_name(entry.name) == canon_name)
    }

    pub fn list_scripts(
        &self,
    ) -> Option<impl Iterator<Item = ScriptEntry<'_>>> {
        self.0
            .get("bin")?
            .as_array_of_tables()
            .map(|arr| arr.iter().map(Self::table_to_script_entry))
    }

    // ───── Mutators ─────
    pub fn add_new_bin(
        &mut self,
        bin_name: &str,
        src_path: &str,
    ) -> crate::Result<()> {
        // ───── Part 1: insert bin array ───────────────────────────────── //
        // get or create top-level `[[bin]]` array
        let bin_array = self.0["bin"]
            .or_insert(ArrayOfTables::new().into())
            .as_array_of_tables_mut()
            .ok_or_else(|| {
                crate::Error::ManifestCorrupt(
                    "'[[bin]]' key exists but is not an array of tables"
                        .to_string(),
                )
            })?;

        // ensure this entry does not already exist
        if bin_array
            .iter()
            .any(|table| table["name"].as_str() == Some(bin_name))
        {
            return Err(crate::Error::AlreadyExists(format!(
                "Bin with name '{bin_name}' already exists"
            )));
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
            self.normalize_dep_name(&dep.depname).map(str::to_owned)
        });

        let feature_activation_strs =
            feature_requests.iter().map(|feature_req| {
                self.normalize_dep_name(&feature_req.depname)
                    .map(|dep| format!("{}/{}", dep, feature_req.featurename))
            });

        let feature_strs: Vec<String> = dep_strs
            .chain(feature_activation_strs)
            .collect_oks_or_iter_errs()
            .map_err(crate::Error::from_nonempty_iter)?;

        // ───── part 2: add it ─────
        let bin_entry =
            self.get_bin_entry_mut(input_script_name).ok_or_else(|| {
                crate::Error::ScriptNotFound(input_script_name.to_owned())
            })?;

        let feature_array = bin_entry["required-features"]
            .or_insert(Array::new().into())
            .as_array_mut()
            .ok_or_else(|| {
                crate::Error::ManifestCorrupt(
                    "'required-features' for script 'input_bin_name' is not \
                     an array"
                        .to_owned(),
                )
            })?;

        _add_unique_strings_to_array(feature_array, feature_strs.into_iter());

        Ok(())
    }

    // ───── internal helpers ───────────────────────────────────────── //
    fn normalize_dep_name(&self, input_name: &str) -> crate::Result<&str> {
        self.find_dep_name(input_name).ok_or_else(|| {
            crate::Error::DependencyNotFound((*input_name).to_owned())
        })
    }

    /// Find a script matching the input name
    /// To match `cargo` behavior, this is case- and
    /// undescore/hyphen-insensitive
    fn find_dep_name(&self, input_dep_name: &str) -> Option<&str> {
        let canonicalized_input = _canonicalize_name(input_dep_name);
        let dep_table = self.0.get("dependencies")?.as_table_like()?;

        dep_table
            .iter()
            .map(|(k, _v)| k)
            .find(|key| _canonicalize_name(key) == canonicalized_input)
    }

    /// Find a script matching the input name.
    /// Similar to cargo's package-matching behavior, this is case- and
    /// undescore/hyphen-insensitive
    fn get_bin_entry_mut(&mut self, input_name: &str) -> Option<&mut Table> {
        let bin_array = self.0.get_mut("bin")?.as_array_of_tables_mut()?;
        let canonicalized_input = _canonicalize_name(input_name);

        bin_array.iter_mut().find(|table| {
            table
                .get("name")
                .and_then(Item::as_str)
                .map(|name| _canonicalize_name(name) == canonicalized_input)
                .unwrap_or(false)
        })
    }

    // pub fn find_bin_name(&self, input_name: &str) -> Option<&str> {
    //     let canonicalized_input = _canonicalize_name(input_name);
    //
    //     self.0
    //         .get("bin")?
    //         .as_array_of_tables()?
    //         .iter()
    //         .filter_map(|table| table.get("name"))
    //         .filter_map(Item::as_str)
    //         .find(|realname| _canonicalize_name(realname) == canonicalized_input)
    // }

    fn table_to_script_entry(table: &Table) -> ScriptEntry<'_> {
        ScriptEntry {
            name: table.get("name").and_then(Item::as_str).unwrap_or(MISSING),
            path: table.get("path").and_then(Item::as_str).unwrap_or(MISSING),
            required_features: table
                .get("required-features")
                .and_then(Item::as_array)
                .map(|array| {
                    array
                        .iter() // MAYBE: maybe don't ignore non-string items? (i.e., errors)
                        .filter_map(Value::as_str)
                        .collect::<Vec<&str>>()
                })
                .unwrap_or(vec![]),
        }
    }
}

/// Canonicalize a name for matching purposes
/// (i.e., 2 names "match" if they both canonicalize to the same string)
fn _canonicalize_name(s: &str) -> String {
    s.to_lowercase().replace('-', "_")
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

        let doc = CargoDotToml::from_string(TOML).unwrap();
        let expected = Some(ScriptEntry {
            name: "my-script",
            path: "src/my_script.rs",
            required_features: vec![],
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
        let mut doc = CargoDotToml::from_string(TOML).unwrap();
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

    fn depreq(dep: &str) -> data::DepRequest {
        data::DepRequest {
            depname: dep.to_owned(),
            version: None,
            input_string: "whatever".to_owned(),
        }
    }

    fn featurereq(dep: &str, feature: &str) -> data::FeatureRequest {
        data::FeatureRequest {
            depname: dep.to_owned(),
            featurename: feature.to_owned(),
        }
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

        let mut doc = CargoDotToml::from_string(TOML).unwrap();

        doc.activate_features("s1", &[depreq("d1")], &[featurereq("d2", "f2")])
            .unwrap();
        doc.activate_features(
            "s2",
            &[depreq("d1"), depreq("d2")],
            &[featurereq("d1", "f1"), featurereq("d2", "f2")],
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
}
