use crate::{Error, Result};
use crate::{build_passthrough_long_args, data};
use clap::{Args, Parser, Subcommand};

/// Manage scripts and dependencies in a playground project
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct ArgParser {
    #[command(subcommand)]
    pub cmd: SubCmd,

    #[command(flatten, next_help_heading = "Output level")]
    pub general: OutputArgs,
}

#[derive(Args, Clone)]
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
    ListScripts {},

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
of the form `[DEPNAME/](FEATURENAME)`, e.g., \"somecrate/somefeature\".

Run `cargo info (DEPNAME)` to see the features available for a given dependency.

The [DEPNAME/] prefix may be omitted if exactly one dependency has been specified.",
            value_parser = parse_feature_arg
        )]
        features: Vec<FeatureCliArg>,

        #[command(
            flatten,
            next_help_heading = "Optional arguments to be forwarded to `cargo add`"
        )]
        cargo_add_args: CargoAddArgs,
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

build_passthrough_long_args!(
    /// Specific args to be forwarded to cargo add
    ///
    /// We only forward these specific flags - rather than everything - because
    /// we need control over `--optional`, `--features`, etc.
    #[derive(Args, Clone)]
    CargoAddArgs {
        kv_flags: (path, base, git, branch, tag, rev, registry),
        switch_flags: (locked, offline, frozen),
    }
);

// ───── Dependency and feature parsing ─────────────────────────── //
/// Parse a dependency name from the CLI
/// Does not implement
fn parse_dep_arg(dep_arg: &str) -> Result<data::DepRequest> {
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

fn parse_feature_arg(feature_arg: &str) -> Result<FeatureCliArg> {
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
