use crate::data;
use crate::{Error, Result};
use clap::{Args, Parser, Subcommand};

/// Manage scripts and dependencies in a playground project
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct ArgParser {
    #[command(subcommand)]
    pub cmd: SubCmd,
}

#[derive(Clone, Subcommand)]
pub enum SubCmd {
    /// Run a script
    #[command(name = "run")]
    RunScript {
        #[arg(help = "name of the script to run")]
        bin_name: String,

        #[arg(help = "Arguments forwarded to 'cargo run'")]
        args: Vec<String>,
    },

    /// Create a new script
    #[command(name = "new")]
    NewScript {
        bin_name: String,

        #[arg(short, long, default_value = "bare")]
        template: String,
    },

    /// List the scripts declared in `Cargo.toml`
    #[command(name = "list")]
    ListScripts {
        #[arg(long, short, help = "Only print script names, nothing else")]
        quiet: bool,
    },

    /// Add a dependency to a script
    #[command(name = "inject")]
    InjectDeps {
        #[arg(help = "name of the script to add dependencies to")]
        bin_name: String,

        #[arg(
            help = "Dependencies, with optional versions \
(depname)[@version], e.g., `clap` or `clap@0.1.2`. Any \
missing dependencies will be installed with `cargo add`",
            num_args = 0..,
            value_parser=parse_dep_arg,
        )]
        deps: Vec<data::DepRequest>,

        #[arg(
            short = 'F',
            long = "feature",
            help = "Dependency-qualified features to activate, \
of the form `[DEPNAME/](FEATURENAME)`. May be repeated.

The [DEPNAME/] prefix may be omitted if exactly one dependency has been specified.",
            value_parser = parse_feature_arg
        )]
        features: Vec<FeatureCliArg>,

        #[command(flatten, next_help_heading = "Optional arguments for `cargo add`")]
        cargo_add_args: CargoAddArgs,
    },
}

#[derive(Args, Clone)]
pub struct CargoAddArgs {
    #[arg(long)]
    path: Option<String>,
    #[arg(long)]
    base: Option<String>,
    #[arg(long)]
    git: Option<String>,
    #[arg(long)]
    branch: Option<String>,
    #[arg(long)]
    tag: Option<String>,
    #[arg(long)]
    rev: Option<String>,
    #[arg(long)]
    registry: Option<String>,
}

impl CargoAddArgs {
    pub fn cli_args(&self) -> Vec<String> {
        let mut vec = Vec::new();

        macro_rules! maybe_push_arg {
            ($attr:ident) => {
                if let Some(ref $attr) = self.$attr {
                    vec.push(format!(concat!("--", stringify!($attr), "={}"), $attr));
                }
            };
        }

        vec![1, 2];

        maybe_push_arg!(path);
        maybe_push_arg!(base);
        maybe_push_arg!(git);
        maybe_push_arg!(branch);
        maybe_push_arg!(tag);
        maybe_push_arg!(rev);
        maybe_push_arg!(registry);
        vec
    }
}

/// Parse a dependency name from the CLI
/// Does not implement
fn parse_dep_arg(dep_arg: &str) -> Result<data::DepRequest> {
    let mut field_iter = dep_arg.splitn(2, '@');
    let depname = field_iter
        .next()
        .ok_or_else(|| Error::InputError(dep_arg.to_string()))?;

    Ok(data::DepRequest {
        depname: depname.to_owned(),
        version: field_iter.next().map(str::to_owned),
        input_string: dep_arg.to_owned(),
    })
}

fn parse_feature_arg(feature_arg: &str) -> Result<FeatureCliArg> {
    let mut field_iter = feature_arg.splitn(2, '/');
    let part1 = field_iter
        .next()
        .ok_or_else(|| Error::InputError(feature_arg.to_string()))?;

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
pub(crate) struct FeatureCliArg {
    pub(crate) dep_qualifier: Option<String>,
    pub(crate) featurename: String,
    pub(crate) orig_input: String,
}

impl FeatureCliArg {
    pub(crate) fn to_feature_req(self) -> Result<data::FeatureRequest> {
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
