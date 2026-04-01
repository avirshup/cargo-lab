use crate::cli_completions::{
    manifest_path_completer, script_name_completer, template_name_completer,
};
use crate::cli_parsers;
use crate::cli_style;
use crate::{build_passthrough_long_args, data};
use clap::builder::Styles;
use clap::{Args, Parser, Subcommand};
use clap_complete::Shell;
use std::path::PathBuf;

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
    /// Path to the playground manifest directory
    #[arg(
        long,
        global = true,
        value_name = "PATH",
        add = manifest_path_completer()
    )]
    manifest_path: Option<PathBuf>,

    /// Use verbose output (-vv = debugging output)
    #[arg(
        short,
        long,
        global = true,
        action=clap::ArgAction::Count,
        conflicts_with = "quiet"
    )]
    pub verbose: u8,

    /// Least output (suitable for piping)
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    pub quiet: bool,
}

#[derive(Clone, Subcommand, Debug)]
pub enum SubCmd {
    /// Run a script
    #[command(name = "run")]
    RunScript {
        #[arg(
            help = "name of the script to run",
            add = script_name_completer(),
        )]
        bin_name: String,

        #[arg(help = "Arguments forwarded to 'cargo run'")]
        args: Vec<String>,
    },

    /// Create a new script
    #[command(name = "new")]
    NewScript {
        bin_name: String,

        #[
            arg(short, long, default_value = "bare",
            add = template_name_completer(),
        )]
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
        #[arg(
            help = "name of the script to add dependencies to",
            add = script_name_completer(),
        )]
        bin_name: String,

        #[command(flatten)]
        inject: InjectArgs,
    },

    /// Shell autocompletions
    #[command(
        name = "completions",
        long_about = "Print shell autocompletion script to stdout."
    )]
    InstallCompletions {
        #[arg(
            short,
            long,
            help = "Shell to generate autocompletions for (if not pased, attempt to detect current shell)"
        )]
        shell: Option<Shell>,
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
        value_parser=cli_parsers::parse_dep_arg,
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
        value_parser = cli_parsers::parse_feature_arg
    )]
    pub features: Vec<cli_parsers::FeatureCliArg>,

    // TODO: this takes up too many lines now??
    #[command(flatten, next_help_heading = "Arguments for \"cargo add\"")]
    pub cargo_add_args: CargoAddArgs,
}

// TODO: this should just be a regular struct definition w/ a macro
//   (use `macro_rules_attribute` and/or `derive_deftly`)
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
