#![cfg(feature = "experimental_cargo_script_rfc3502")]
//! EXPERIMENTAL experiments for experimentally supporting RFC 3502 - single-file
//! cargo scripts with embedded "frontmatter" in a YAML (I think?)-style `---` fence.
//! AIUI this extends _the rust language itself_ with a new type of metadata syntax,
//! so it needs IDE support.
//!
//! It's somewhate unclear how to enable this, at least from the docs.
//! Certainly need to enable nightly toolchain
//! (`[toolchain] \n channel = "nightly"` in `rust-toolchain.toml`).
//!  - Maybe also `[unstable] \n script = true` in `.cargo/config.toml`?
//!  - and/or `unstable.frontmatter=true` in `.cargo/config.toml`?
//!  - and/or `#![feature(frontmatter)]` in the script file?
//!
//! Also unclear what the tooling will (or even should) do if a given source file
//! is **both** part of a regular cargo project AND has its own script metadata
//! like this.
//!
//! When/if scripts are fully stabilized and supported, instead of maintaining
//! the cargo-script metadata in the source file all the time (which also
//! has issues around keeping it in sync with cargo.toml), maybe we want
//! a command like `cargo playground eject-script $scriptname -o dest`
//! that takes an existing script (without metadata) and writes it out as
//! a standalone single-file script w/ the relevant metadata.
//!
//! ## Does it work at all?
//! (Perhaps see [the tracking issue](https://github.com/rust-lang/cargo/issues/12207)
//! instead of manually testing it?)
//!
//! When testing this in April 2026:
//!
//! ### `RustRover`
//! Mostly does *not* handle it:
//! it reports the frontmatter as a syntax error, _but_ at least it does not seem to
//! impact the rest of the code analysis for the file. Does not seem to be
//! affected by activating nightly features.
//! (It's too bad that frontmatter is not just special comments so rustrover could
//! just ignore it.)
//!
//! (Rustrover in general is probably not going to like opening single files
//! rather than a project tree anyway)
//!
//! ### `rust-analyzer` + `vscodium`
//! In a normal cargo project:
//!  -  _recognizes_ the frontmatter and reports it as an error
//!    because we haven't enabled that experimental feature)
//!  - enabling the nightly toolchain and setting all the unstable
//!    feature flags everywhere I could did not make it happy, it then
//!    seemed to think the frontmatter was a syntax error
//!
//! When opening a **single file** outside of a cargo project root:
//!    - lots of errors in the logs from not locating cargo.toml, very unclear
//!      what's going on.
//!
//! (Quite possible that there are issues arising from the _interactions_
//! between rust-analyzer and vscode here, could test analyzer separately)

use crate::manifest_data::ManifestData;
use crate::vendor_cargo::embedded;

/// Parse embedded RFC-3502 manifest in a script, if it exists.
///
/// See also [RFC 3502-cargo-script](https://rust-lang.github.io/rfcs/3502-cargo-script.html)
///
/// May return the following variants:
///  - `Err(...)` if there are any parsing errors - either of the script
///    file itself embedded TOML within a its manifest;
///  - `Ok(None)` if there is no RFC 3502 shebang/embedded manifest in the file; and
///  - `Ok(Some(...))` if the manifests exists and is parsed successfully.
///
/// # Notes
///
/// ## The vendored implementaiton
/// `expand_manifest` returns all the lines of the original file, up to the end of the
/// frontmatter. Each non-frontmatter line is preprended with a toml line comment '#'
/// so that the resulting string can be parsed as toml.
///
/// ## The "spec"
/// The example in the text of RFC 3502 is not prescriptive, and
/// in does not seem to be syntactically allowed with the current implementation.
/// I think the the actual spec is going to be figured from
/// the implementation work, so we need to derive our examples (and tests)
/// from cargo itself.
///
/// Their [tests](https://github.com/rust-lang/cargo/blob/0f14d9d2fa/tests/testsuite/script/cargo.rs) might actually be the most reliable source of truth here.
///
/// ## TODOs
/// - probably this should have specific errors for different error types and
///   return a more specific error enum, not just `crate::Error` enum.
pub fn parse_embedded_manifest(
    script_src: &'_ str,
) -> crate::Result<Option<ManifestData>> {
    let manifest_str = embedded::expand_manifest(script_src)?;

    if manifest_str.is_empty() {
        return Ok(None);
    }

    let result: ManifestData = toml_edit::de::from_str(&manifest_str)?;
    Ok(Some(result))
}

/// Modify script source to set the state of the embedded manifest in it.
///
/// Not actually implemented yet, and maybe shouldn't be:
///
/// TODO: need to use _edit_ existing manifests rather than _replacing_ them
///   (repalcing them will destroy any and all user-controlled data and comments in them)
///   This would require carefully replacing the existing spans BUT ALSO ignoring the comments
///   that expand_manifest added?
pub fn set_embedded_manifest(
    orig_script: &str,
    manifest: Option<&ManifestData>,
) -> crate::Result<String> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest_data::{DepEntry, DependencyTable};

    fn _s(s: &str) -> String {
        s.to_owned()
    }

    fn clap_manifest() -> ManifestDocument {
        ManifestDocument {
            package: None,
            features: [].into(),
            bin: vec![],
            dependencies: [(
                _s("clap"),
                DepEntry::Table(DependencyTable {
                    version: Some(_s("4.2")),
                    features: vec![_s("derive")],
                    optional: false,
                }),
            )]
            .into(),
        }
    }

    const EMPTY: &str = "use foo::Bar;\n";
    const NO_SHEBANG: &str = r#"
---
[dependencies]
clap = { version = "4.2", features = ["derive"] }
---
use foo::Bar;"#;

    const JUST_SHEBANG: &str = r#"!/usr/bin/env cargo
use foo::Bar;"#;

    const RFC_3502_EXAMPLE: &str = r#"#!/usr/bin/env cargo
---
[dependencies]
clap = { version = "4.2", features = ["derive"] }
---

use foo::Bar;
"#;

    #[test]
    fn test_parse_rfc_example() {
        let actual =
            parse_embedded_manifest(RFC_3502_EXAMPLE).expect("failed to parse");
        let expected = Some(clap_manifest());
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_empty_examples() {
        for script_src in [JUST_SHEBANG, EMPTY] {
            let actual =
                parse_embedded_manifest(script_src).expect("failed to parse");
            assert_eq!(actual, None);
        }
    }

    #[test]
    fn test_add_manifest_roundtrip() {
        let expect_manifest = clap_manifest();

        let with_manifest =
            set_embedded_manifest(EMPTY, Some(&expect_manifest)).unwrap();

        let found_manifest = parse_embedded_manifest(&with_manifest);
        assert!(
            found_manifest.is_ok(),
            "Failed to parse manifest after adding it"
        );

        assert_eq!(found_manifest.unwrap(), Some(expect_manifest));
    }
}
