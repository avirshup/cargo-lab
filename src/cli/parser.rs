use std::path::PathBuf;

use clap::builder::Styles;
use clap::{Args, Parser, Subcommand};
use clap_complete::Shell;

use super::completions::{
    manifest_path_completer, script_name_completer, template_name_completer,
};
use super::derive_traits::GeneratesArgs;
use super::feature_parsers::{
    FeatureCliInput, parse_dep_arg, parse_feature_arg,
};
use crate::vendor_cargo::style;
use crate::{build_passthrough_long_args, data};

const STYLES: Styles = Styles::styled()
    .header(style::HEADER)
    .usage(style::USAGE)
    .literal(style::LITERAL)
    .placeholder(style::PLACEHOLDER)
    .error(style::ERROR)
    .valid(style::VALID)
    .invalid(style::INVALID);

/// Manage scripts and dependencies in a playground project
#[derive(Debug, Parser)]
#[command(version, about, long_about = None, styles = STYLES)]
pub struct MainCli {
    #[command(subcommand)]
    pub cmd: SubCmd,

    #[command(flatten, next_help_heading = "Global arguments")]
    pub global_args: GlobalArgs,
}

#[derive(Args, Clone, Debug)]
pub struct GlobalArgs {
    /// Path to the playground manifest directory
    #[arg(
        long,
        global = true,
        value_name = "PATH",
        add = manifest_path_completer()
    )]
    pub manifest_path: Option<PathBuf>,

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

// ───── Top-level subcmd enum ──────────────────────────────────── //
#[derive(Clone, Subcommand, Debug)]
pub enum SubCmd {
    /// Run a script
    #[command(name = "run")]
    RunScript(RunScript),

    /// Create a new script
    #[command(name = "new")]
    NewScript(NewScript),

    /// List the scripts declared in `Cargo.toml`
    #[command(name = "list")]
    ListScripts,

    /// Add a dependency to a script
    #[command(name = "inject")]
    InjectDeps(InjectDeps),

    /// Shell autocompletions
    #[command(
        name = "completions",
        long_about = "Print shell autocompletion script to stdout."
    )]
    InstallCompletions(InstallCompletions),

    /// For debugging, hidden from output
    #[command(name = "do-nothing", hide = true)]
    DoNothing,
}

// ──────────────────────────────────────────────────────────────────────── //
// ───── Subcommands                                                 ───── //
// ──────────────────────────────────────────────────────────────────────── //
/// Run an existing script from the playground
#[derive(Args, Clone, Debug)]
pub struct RunScript {
    #[arg(
            help = "name of the script to run",
            add = script_name_completer(),
            value_name = "SCRIPT"
    )]
    pub bin_name: String,

    #[arg(help = "Arguments forwarded to 'cargo run'")]
    pub args: Vec<String>,
}

/// Create a new playground script
#[derive(Args, Clone, Debug)]
pub struct NewScript {
    #[arg(help = "name of the script to create", value_name = "SCRIPT")]
    pub bin_name: String,

    #[arg(short, long, default_value = "bare", add = template_name_completer())]
    pub template: String,

    #[command(flatten, next_help_heading = "Dependencies")]
    pub inject_args: InjectArgs,
}

/// Add dependencies for a srcipt
#[derive(Clone, Args, Debug)]
pub struct InjectDeps {
    #[arg(
        help = "name of the script to add dependencies to",
        add = script_name_completer(),
        value_name = "SCRIPT"
    )]
    pub bin_name: String,

    #[command(flatten)]
    pub inject_args: InjectArgs,
}

/// Manage CLI completions for th
#[derive(Clone, Args, Debug)]
pub struct InstallCompletions {
    #[arg(
        short,
        long,
        help = "Shell to generate autocompletions for (if not pased, attempt \
                to detect current shell)"
    )]
    pub shell: Option<Shell>,
}

#[derive(Clone, Args, Debug)]
pub struct InjectArgs {
    #[arg(
        help = "Dependencies, with optional versions",
        long_help="Dependencies, with optional versions, of the form \
(depname)[@version], e.g., `clap` or `clap@0.1.2`. Any \
missing dependencies will be installed with `cargo add`",
        num_args = 0..,
        value_parser=parse_dep_arg,
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
        value_parser = parse_feature_arg
    )]
    pub features: Vec<Vec<FeatureCliInput>>,

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
