use crate::{Error, Result, cli_style};
use crate::{build_passthrough_long_args, data};
use all_the_errors::CollectAllTheErrors;
use clap::builder::Styles;
use clap::{Args, Parser, Subcommand};
use clap_complete::Shell;

/// Manage scripts and dependencies in a playground project
#[derive(Debug, Parser)]
#[command(version, about, long_about = None, styles = STYLES)]
pub struct PlaygroundCli {
    #[command(subcommand)]
    pub cmd: SubCmd,

    #[command(flatten, next_help_heading = "Output level")]
    pub general: OutputArgs,
}

const STYLES: Styles = Styles::styled()
    .header(cli_style::HEADER)
    .usage(cli_style::USAGE)
    .literal(cli_style::LITERAL)
    .placeholder(cli_style::PLACEHOLDER)
    .error(cli_style::ERROR)
    .valid(cli_style::VALID)
    .invalid(cli_style::INVALID);

#[derive(Args, Clone, Debug)]
pub struct OutputArgs {
    #[arg(
        short,
        long,
        global = true,
        action=clap::ArgAction::Count,
        help="Use verbose output (-vv = debugging output)"
    )]
    pub verbose: u8,

    #[arg(
        short,
        long,
        global = true,
        help = "Least output (suitable for piping)",
        conflicts_with = "verbose"
    )]
    pub quiet: bool,
}

#[derive(Clone, Subcommand, Debug)]
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

        #[command(flatten, next_help_heading = "Dependencies")]
        inject: InjectArgs,
    },

    /// List the scripts declared in `Cargo.toml`
    #[command(name = "list")]
    ListScripts {},

    /// Add a dependency to a script
    #[command(name = "inject")]
    InjectDeps {
        #[arg(help = "name of the script to add dependencies to")]
        bin_name: String,

        #[command(flatten)]
        inject: InjectArgs,
    },

    /// Shell autocompletions
    #[command(
        name = "completions",
        long_about = "Show or install autocompletion for supported shells.
By default, prints the script to STDOUT; will attempt to install it if --install is passed."
    )]
    InstallCompletions {
        #[arg(
            short,
            long,
            help = "Shell to generate autocompletions for (if not pased, attempt to detect current shell)"
        )]
        shell: Option<Shell>,

        #[arg(long, help = "Attempt to automatically install the completions")]
        install: bool,
    },

    // /// Open dependency's manifest in a browser (requires internet access)
    // ///
    // /// Attempts to open `https://docs.rs/crate/{$DEPNAME}/latest/source/Cargo.toml`
    // /// in a browser.
    // #[command(name = "show-manifest")]
    // OpenDepManifest {
    //     #[arg(help = "Dependency name. Must exactly match *package* name on crates.io.")]
    //     depname: String,
    //
    //     // see https://jwodder.github.io/kbits/posts/clap-bool-negate/
    //     #[clap(long = "show-url",
    //         action = clap::ArgAction::SetTrue,
    //         help="Just print the URL (don't try to open browser)",
    //         default_value="false",
    //     )]
    //     show: bool,
    // },
    /// For debugging, hidden from output
    #[command(name = "do-nothing", hide = true)]
    DoNothing {},
}

#[derive(Clone, Args, Debug)]
pub struct InjectArgs {
    #[arg(
        help = "Dependencies, with optional versions",
        long_help="Dependencies, with optional versions, of the form \
(depname)[@version], e.g., `clap` or `clap@0.1.2`. Any \
missing dependencies will be installed with `cargo add`",
        num_args = 0..,
        value_parser=_parse_dep_arg,
    )]
    pub deps: Vec<data::DepRequest>,

    #[arg(
        short = 'F',
        long = "feature",
        help="Dependency features to activate",
        long_help = "Dependency-qualified features to activate, \
of the form `[DEPNAME/](FEATURENAME)`, e.g., \"somecrate/somefeature\".

Run `cargo info (DEPNAME)` to see the features available for a given dependency.

The [DEPNAME/] prefix may be omitted if exactly one dependency has been specified.",
        value_parser = _parse_feature_arg
    )]
    pub features: Vec<FeatureCliArg>,

    // TODO: this takes up too many lines now??
    #[command(flatten, next_help_heading = "Arguments for \"cargo add\"")]
    pub cargo_add_args: CargoAddArgs,
}

build_passthrough_long_args!(
    /// Specific args to be forwarded to cargo add
    ///
    /// We only forward these specific flags - rather than everything - because
    /// we need control over `--optional`, `--features`, etc.
    #[derive(Args, Clone, Debug)]
    #[command(flatten_help=true)]
    pub struct CargoAddArgs {
        kv_flags(path, base, git, branch, tag, rev, registry),
        switch_flags(locked, offline, frozen),
    }
);

// ───── Dependency and feature parsing ─────────────────────────── //
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
) -> Result<Vec<data::FeatureRequest>> {
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
fn _parse_dep_arg(dep_arg: &str) -> Result<data::DepRequest> {
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

fn _parse_feature_arg(feature_arg: &str) -> Result<FeatureCliArg> {
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
pub(crate) struct FeatureCliArg {
    pub(crate) dep_qualifier: Option<String>,
    pub(crate) featurename: String,
    pub(crate) orig_input: String,
}

impl FeatureCliArg {
    /// turn this into a request for a feature to be activated -
    /// succeeds only if the dependency has been provided
    pub(crate) fn into_feature_req(self) -> Result<data::FeatureRequest> {
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
