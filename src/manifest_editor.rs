use crate::data::{MISSING, ScriptEntry};
use std::fs;
use std::path::Path;
use toml_edit::{Array, ArrayOfTables, DocumentMut, Item, Table};

pub struct CargoDotToml(DocumentMut);

impl CargoDotToml {
    // ───── Constructors ─────
    pub fn read(path: &Path) -> crate::Result<Self> {
        let toml_content = fs::read_to_string(path)?;
        let doc = toml_content.parse::<DocumentMut>()?;
        Ok(Self(doc))
    }

    #[allow(unused)] // for testing mostly
    pub fn from_string(content: &str) -> crate::Result<Self> {
        let doc = content.parse::<DocumentMut>()?;
        Ok(Self(doc))
    }

    // ───── Renderers ─────
    #[allow(unused)] // for testing mostly
    pub fn render(&self) -> String {
        self.0.to_string()
    }

    pub fn write(&self, target: &Path) -> crate::Result<()> {
        fs::write(target, self.0.to_string())?;
        Ok(())
    }

    // ───── Queries ─────
    pub fn list_scripts(&self) -> Option<impl Iterator<Item = ScriptEntry<'_>>> {
        self.0.get("bin")?.as_array_of_tables().map(|arr| {
            arr.iter().map(|table| ScriptEntry {
                name: table.get("name").and_then(Item::as_str).unwrap_or(MISSING),
                path: table.get("path").and_then(Item::as_str).unwrap_or(MISSING),
            })
        })
    }

    // ───── Mutators ─────
    pub fn add_new_bin(&mut self, bin_name: &str, src_path: &str) -> crate::Result<()> {
        let feature_name = bin_feature_name(bin_name);

        // ───── Part 1: insert bin array ───────────────────────────────── //
        // get or create top-level `[[bin]]` array
        let bin_array = self.0["bin"]
            .or_insert(ArrayOfTables::new().into())
            .as_array_of_tables_mut()
            .ok_or_else(|| {
                crate::Error::ManifestCorrupt(
                    "'[[bin]]' key exists but is not an array of tables".to_string(),
                )
            })?;

        // ensure this entry does not already exist
        if bin_array
            .iter()
            .any(|table| table["name"].as_str() == Some(bin_name))
        {
            return Err(crate::Error::ManifestCorrupt(format!(
                "Bin with name '{bin_name}' already exists"
            )));
        };

        // create the `[[bin]]` table entry
        let bin_table = {
            let mut bin_table = Table::new();
            bin_table["name"] = bin_name.into();
            bin_table["path"] = src_path.into();

            let mut required_features = Array::new();
            required_features.push(feature_name);
            bin_table["required-features"] = required_features.into();

            bin_table
        };

        bin_array.push(bin_table);

        // ───── Part 2: create a feature for it ────────────────────────── //
        // this also needs a mutable borrow, so it can't happen simultaneously
        // as the bin_array part

        // get or create the top-level feature table
        let feature_name = bin_feature_name(bin_name);
        let feature_table = self.0["features"]
            .or_insert(Table::new().into())
            .as_table_mut()
            .ok_or_else(|| {
                crate::Error::ManifestCorrupt("'feature' key exists but is not a table".to_string())
            })?;

        // insert new array into the table
        feature_table[&feature_name].or_insert(Array::new().into());

        Ok(())
    }

    /// Add a dependency and its features to a script's feature list.
    /// Does not insert duplicates entries in the array.
    /// Will fail if the dependency or script does not exist.
    ///
    /// ## Example
    /// For instance, calling
    /// `manifest.add_dep_to_feature("myscript", "thiserror", &["some_feature"])`
    /// would change this:
    /// ```toml
    /// [features]
    /// myscript_deps = ["dep:foo"]
    /// ```
    /// into this:
    /// ```toml
    /// [features]
    /// myscript_deps = ["dep:foo", "dep:thiserror", "thiserror/some_feature"]
    /// ```
    pub fn add_dep_to_feature(
        &mut self,
        input_bin_name: &str,
        input_depname: &str,
        dep_features: &[String],
    ) -> crate::Result<()> {
        // ───── Find the real names ───────────────────────────────────────────────── //
        // (these need to be cloned so we don't have a shared borrow
        // in the edit phase below)
        let bin_name = self
            .find_bin_name(input_bin_name)
            .ok_or_else(|| crate::Error::ScriptNotFound(input_bin_name.to_owned()))?
            .to_owned();
        let depname = self
            .find_dep_name(input_depname)
            .ok_or_else(|| crate::Error::DependencyNotFound(input_depname.to_owned()))?
            .to_owned();

        // ───── Edits ──────────────────────────────────────────────────── //
        let feature_name = bin_feature_name(&bin_name);
        let feature_array = &mut self.0["features"][&feature_name]
            .or_insert(Array::new().into())
            .as_array_mut()
            .ok_or_else(|| {
                crate::Error::ManifestCorrupt(format!(
                    "'features.{feature_name}' exists but is not an array"
                ))
            })?;

        add_unique_string_to_array(feature_array, &format!("dep:{depname}"));
        for dep_feature in dep_features {
            add_unique_string_to_array(feature_array, &format!("{depname}/{dep_feature}"));
        }

        Ok(())
    }

    /// Find a script matching the input name.
    /// To match `cargo` behavior, this is case- and
    //     /// undescore/hyphen-insensitive
    fn find_dep_name(&self, input_dep_name: &str) -> Option<&str> {
        fn _normalize_name(s: &str) -> String {
            s.replace('-', "_")
        }
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
    fn find_bin_name(&self, input_name: &str) -> Option<&str> {
        let bin_array = self.0.get("bin")?.as_array_of_tables()?;
        let canonicalized_input = _canonicalize_name(input_name);

        bin_array
            .iter()
            .filter_map(|table| table.get("name"))
            .filter_map(Item::as_str)
            .find(|realname| _canonicalize_name(realname) == canonicalized_input)
    }
}

/// Canonicalize a package name the same way cargo does it for
/// matching purposes. 2 names are equivalent if they both
/// canonicalize to the same string.
fn _canonicalize_name(s: &str) -> String {
    s.to_lowercase().replace('-', "_")
}

fn add_unique_string_to_array(arr: &mut Array, s: &str) {
    if !arr.iter().any(|item| item.as_str() == Some(s)) {
        arr.push(s);
    }
}

fn bin_feature_name(bin_name: &str) -> String {
    let delim = if bin_name.contains('_') { '_' } else { '-' };
    format!("{bin_name}{delim}deps")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_new_bin() {
        let toml = r#"
[package]
edition = "252525"  # now this song is going through your head

[dependencies]
"#;
        let mut doc = CargoDotToml::from_string(toml).unwrap();
        doc.add_new_bin("do-thing", "do_thing.rs").unwrap();
        let actual = doc.render();
        let expected = r#"
[package]
edition = "252525"  # now this song is going through your head

[dependencies]

[[bin]]
name = "do-thing"
path = "do_thing.rs"
required-features = ["do-thing-deps"]

[features]
do-thing-deps = []
"#;

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_add_to_array() {
        let toml = r#"
[features]
# rare doodah right here
existing-deps = ["dep:mydep", "1", "a/b",]
"#;

        let mut doc = CargoDotToml::from_string(toml).unwrap();

        doc.add_dep_to_feature("new", "mydep", &["harfbuzz".to_string()])
            .unwrap();
        doc.add_dep_to_feature("existing", "mydep", &["harfbuzz".to_string()])
            .unwrap();

        let actual = doc.render();
        let expected = r#"
[features]
# rare doodah right here
existing-deps = ["dep:mydep", "1", "a/b", "mydep/harfbuzz",]
new-deps = ["dep:mydep", "mydep/harfbuzz"]
"#;

        assert_eq!(actual, expected);
    }
}
