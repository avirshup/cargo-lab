use all_the_errors::CollectAllTheErrors;

use crate::data;
use crate::errors::Error;

/// Figures out which "features" the user is requesting for the script
/// Note that "feature" is confusing, as it includes dependencies themselves
/// _and_ features to be activated for those dependencies.
///
/// (I.e., a script that needs to use "clap" with its derive feature
/// will have `required-features = ["clap", "clap/derive"]
///
/// This will fail if there is any ambiguity about which dependency
/// a given feature is being requested for.
pub fn resolve_feature_requests(
    input_deps: &[data::DepRequest],
    mut input_features: Vec<FeatureCliArg>,
) -> crate::errors::Result<Vec<data::FeatureRequest>> {
    // insert implicit feature dependency qualifiers
    // i.e., `inject depname -F feature` => `inject depname -F depname/feature`
    // but only if there is exactly one dependency listed
    if input_deps.len() == 1 {
        let implicit_depname = &input_deps.first().unwrap().depname;
        for input_feat in &mut input_features {
            if input_feat.dep_qualifier.is_none() {
                input_feat.dep_qualifier = Some(implicit_depname.to_owned());
            }
        }
    }

    // ensure all requested features have a dependency
    let features: Vec<data::FeatureRequest> = input_features
        .into_iter()
        .map(FeatureCliArg::into_feature_req)
        .collect_oks_or_iter_errs()
        .map_err(Error::from_nonempty_iter)?;
    Ok(features)
}

/// Parse a dependency name from the CLI
/// Does not implement
pub fn parse_dep_arg(dep_arg: &str) -> crate::errors::Result<data::DepRequest> {
    let mut field_iter = dep_arg.splitn(2, '@');
    let depname = field_iter
        .next()
        .ok_or_else(|| Error::InputErr(dep_arg.to_string()))?;

    Ok(data::DepRequest {
        depname: depname.to_owned(),
        version: field_iter.next().map(str::to_owned),
        input_string: dep_arg.to_owned(),
    })
}

pub fn parse_feature_arg(
    feature_arg: &str,
) -> crate::errors::Result<FeatureCliArg> {
    let mut field_iter = feature_arg.splitn(2, '/');
    let part1 = field_iter
        .next()
        .ok_or_else(|| Error::InputErr(feature_arg.to_string()))?;

    match field_iter.next() {
        Some(part2) => Ok(FeatureCliArg {
            dep_qualifier: Some(part1.to_owned()),
            featurename: part2.to_owned(),
            orig_input: feature_arg.to_string(),
        }),
        None => Ok(FeatureCliArg {
            dep_qualifier: None,
            featurename: part1.to_owned(),
            orig_input: feature_arg.to_string(),
        }),
    }
}

/// An *unvalidated* feature argument that may or may not
/// have its dependency qualifier attached
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureCliArg {
    pub dep_qualifier: Option<String>,
    pub featurename: String,
    pub orig_input: String,
}

impl FeatureCliArg {
    /// turn this into a request for a feature to be activated -
    /// succeeds only if the dependency has been provided
    pub fn into_feature_req(
        self,
    ) -> crate::errors::Result<data::FeatureRequest> {
        let Self {
            dep_qualifier,
            featurename,
            orig_input,
        } = self;
        if let Some(depname) = dep_qualifier {
            Ok(data::FeatureRequest {
                depname,
                featurename,
            })
        } else {
            Err(Error::AmbiguousFeature(orig_input))
        }
    }
}
