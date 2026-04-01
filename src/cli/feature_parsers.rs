use all_the_errors::CollectAllTheErrors;

use crate::data;

/// An *unvalidated* feature argument that may or may not
/// have its dependency qualifier attached.
/// Its dependency qualifier must be resolved, turning it into a
/// [`data::FeatureCliArg`], before it can be used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureCliInput {
    pub dep_qualifier: Option<String>,
    pub featurename: String,
    pub orig_input: String,
}

/// Parse a dependency name from the CLI
pub fn parse_dep_arg(dep_arg: &str) -> crate::Result<data::DepRequest> {
    let mut field_iter = dep_arg.splitn(2, '@');
    let depname = field_iter
        .next()
        .ok_or_else(|| crate::Error::InputErr(dep_arg.to_string()))?
        .to_owned();
    let version = field_iter.next().map(str::to_owned);

    Ok(data::DepRequest {
        depname,
        version,
        input_string: dep_arg.to_owned(),
    })
}

/// parse an argument from `-F` / `--feature`
///
/// In general this is a space-or-comma-separated
/// list of features (although it can also just be
/// one, and the flag itself can also be repeated)
pub fn parse_feature_arg(
    feature_arglist: &str,
) -> crate::errors::Result<Vec<FeatureCliInput>> {
    feature_arglist
        .split(&[' ', ','])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(_parse_one_feature)
        .collect()
}

fn _parse_one_feature(
    feature_arg: &str,
) -> crate::errors::Result<FeatureCliInput> {
    let mut field_iter = feature_arg.splitn(2, '/');
    let part1 = field_iter
        .next()
        .ok_or_else(|| crate::Error::InputErr(feature_arg.to_string()))?;

    match field_iter.next() {
        Some(part2) => Ok(FeatureCliInput {
            dep_qualifier: Some(part1.to_owned()),
            featurename: part2.to_owned(),
            orig_input: feature_arg.to_string(),
        }),
        None => Ok(FeatureCliInput {
            dep_qualifier: None,
            featurename: part1.to_owned(),
            orig_input: feature_arg.to_string(),
        }),
    }
}

/// Figures out which "features" the user is requesting for the script.
/// This will fail if there is any ambiguity about which dependency
/// a given feature is being requested for.
///
/// Note that "feature" is confusing, as it includes dependencies themselves
/// _and_ features to be activated for those dependencies.
/// (e.g., a script that needs to use "clap" with its derive feature
/// will have `required-features = ["clap", "clap/derive"]`. Also,
/// sometimes it can be spelled `"dep:clap"` and sometimes just `"clap"`)
pub fn resolve_feature_requests(
    input_deps: &[data::DepRequest],
    input_features: Vec<Vec<FeatureCliInput>>,
) -> crate::errors::Result<Vec<data::FeatureRequest>> {
    // features do not need to include a dependency name
    // if there is exactly one dependency specified
    let implicit_depname = if input_deps.len() == 1 {
        input_deps.first().map(|dep| dep.depname.as_str())
    } else {
        None
    };

    // ensure all requested features have a dependency
    input_features
        .into_iter()
        .flatten()
        .map(|input_feature| {
            _resolve_one_feature(input_feature, &implicit_depname)
        })
        .collect_oks_or_iter_errs()
        .map_err(crate::Error::from_nonempty_iter)
}

fn _resolve_one_feature(
    input_feature: FeatureCliInput,
    implicit_depname: &Option<&str>,
) -> crate::errors::Result<data::FeatureRequest> {
    let FeatureCliInput {
        dep_qualifier,
        featurename,
        orig_input,
    } = input_feature;

    if let Some(depname) =
        dep_qualifier.or_else(|| implicit_depname.map(str::to_owned))
    {
        Ok(data::FeatureRequest {
            depname,
            featurename,
        })
    } else {
        Err(crate::Error::AmbiguousFeature(orig_input))
    }
}
