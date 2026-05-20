use camino::Utf8PathBuf;
use clap::builder::Styles;
use clap::{Args, Command, Parser, Subcommand};
use color_print::{cformat, cstr};

use super::completions::{
    manifest_path_completer, script_name_completer, template_name_completer,
};
use super::feature_parsers::{
    FeatureCliInput, parse_dep_arg, parse_feature_arg,
};
use super::passthrough_arg_macro::GeneratesArgs;
use crate::cli::invocations::InvocationType;
use crate::vendor_cargo::style;
use crate::{build_passthrough_long_args, data};

attribute_alias! {
   #[apply(DeriveArg)] = #[derive(Clone, Debug)];
}

pub(super) const STYLES: Styles = Styles::styled()
    .header(style::HEADER)
    .usage(style::USAGE)
    .literal(style::LITERAL)
    .placeholder(style::PLACEHOLDER)
    .error(style::ERROR)
    .valid(style::VALID)
    .invalid(style::INVALID);

/// Manage script playgrounds as cargo projects
#[apply(DeriveArg)]
#[derive(Parser)]
#[command(version, about, long_about = None, styles = STYLES)]
pub struct MainCli {
    #[command(subcommand)]
    pub cmd: SubCmd,

    #[command(flatten, next_help_heading = "Global arguments")]
    pub global_args: GlobalArgs,
}

#[apply(DeriveArg)]
#[derive(Args)]
pub struct GlobalArgs {
    /// Path to the playground's manifest directory
    #[arg(
        long,
        global = true,
        value_name = "PATH",
        add = manifest_path_completer()
    )]
    pub manifest_path: Option<String>,

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
#[apply(DeriveArg)]
#[derive(Subcommand)]
pub enum SubCmd {
    /// Create a new playground
    #[command(name = "init")]
    InitPlayground(InitPlayground),

    /// Run a script
    #[command(name = "run")]
    RunScript(RunScript),

    /// Create a new script
    #[command(name = "new")]
    NewScript(NewScript),

    /// Create a new script with an automatically generated name
    #[command(name = "quick")]
    QuickScript(NewScriptOpts),

    /// Rename script
    #[command(name = "rename")]
    RenameScript(RenameScript),

    /// List the scripts declared in `Cargo.toml`
    #[command(name = "list")]
    ListScripts,

    /// Show info about a script
    #[command(name = "info")]
    ShowScriptInfo(ShowScriptInfo),

    /// Add a dependency to a script
    #[command(name = "inject")]
    InjectDeps(InjectDeps),

    /// Open a script in your editor
    #[command(name = "edit")]
    EditScript(EditScript),

    /// Generate shell autocomplete scripts
    #[command(
        name = "completions",
        defer=WriteCompletionScript::add_after_help_examples,
    )]
    WriteCompletionScript(WriteCompletionScript),

    /// Project configuration health checks
    #[command(name = "check")]
    Check,
}

// ──────────────────────────────────────────────────────────────────────── //
// ───── Subcommands                                                 ───── //
// ──────────────────────────────────────────────────────────────────────── //
/// Create a new playground
#[apply(DeriveArg)]
#[derive(Args)]
pub struct InitPlayground {
    /// Path to initialize a as playground.
    ///
    /// Can be: a path that does not exist, an empty directory, or
    /// an existing cargo project (must also pass `--existing`)
    #[arg()]
    pub path: Utf8PathBuf,

    // // TODO: either use arg group to make `--existing` mutually
    // //   exclusive of others, or make separate commands
    // /// Name of the playground "package", if not already set.
    // /// (defaults to "{directory-name}-playground")
    // #[arg(action=clap::ArgAction::SetTrue)]
    // pub existing: bool,
    //
    /// Name of the playground "package", if not already set.
    /// (defaults to "{directory-name}-playground")
    #[arg()]
    pub name: Option<String>,

    /// Rust edition to use (default: "2024")
    /// Ignored if `--existing` is passed.
    // TODO: enforce rather than ignore
    #[arg(default_value = "2024")]
    pub edition: String,
}

/// Run an existing script from the playground
#[derive(Args, Clone, Debug)]
pub struct RunScript {
    #[arg(
            help = "name of the script to run",
            add = script_name_completer(),
            value_name = "SCRIPT"
    )]
    pub script_name: String,

    #[arg(help = "Arguments forwarded to 'cargo run'")]
    pub args: Vec<String>,
}

/// Create a new playground script
#[apply(DeriveArg)]
#[derive(Args)]
pub struct NewScript {
    /// Name of the script to create (optional).
    /// If not provided, a random human-readable name will be generated
    #[arg(value_name = "SCRIPT")]
    pub script_name: String,

    #[command(flatten)]
    pub opts: NewScriptOpts,
}

/// All the arguments for new scripts except its name.
#[apply(DeriveArg)]
#[derive(Args)]
pub struct NewScriptOpts {
    #[arg(short, long, add = template_name_completer())]
    pub template: Option<String>,

    #[arg(
        long,
        help = "Open the script in editor after creating it.",
        long_help = "Open the script in editor after creating it \
                     (`package.metadata.cargo-playground.editor-cmd` must be \
                     set in manifest.)"
    )]
    pub edit: bool,

    #[command(flatten, next_help_heading = "Dependencies")]
    pub inject_args: InjectArgs,
}

/// Create a new playground script
#[apply(DeriveArg)]
#[derive(Args)]
pub struct RenameScript {
    /// Name of the script to be renamed
    #[arg(value_name = "SCRIPT", add = script_name_completer())]
    pub old_name: String,

    /// New script name
    pub new_name: String,

    /// Open the script in editor after this operation.
    ///
    /// (`package.metadata.cargo-playground.editor-cmd` must be set in the manifest.)"
    #[arg(long)]
    pub edit: bool,
}

// TODO: scripts should say which dependencies they have available?
//   Maybe "--add-use-statements" to add those? Maybe keep
//   commented-out cargo frontmatter up-to-date? Or maybe
//   `#[cfg(feature)]` attributes (or not, those are very noisy-looking)

/// Add dependencies and/or activate features for a script
#[apply(DeriveArg)]
#[derive(Args)]
pub struct InjectDeps {
    /// name of the script to add dependencies to
    #[arg(add = script_name_completer(), value_name = "SCRIPT")]
    pub script_name: String,

    /// Open the script in editor after installing the dependencies.
    ///
    /// (`package.metadata.cargo-playground.editor-cmd` must be set in the manifest.)"
    #[arg(long)]
    pub edit: bool,

    #[command(flatten)]
    pub inject_args: InjectArgs,
}

/// Print information about script
#[apply(DeriveArg)]
#[derive(Args)]
pub struct ShowScriptInfo {
    #[arg(add = script_name_completer())]
    pub script_name: String,
}

/// Open script in editor
#[apply(DeriveArg)]
#[derive(Args)]
pub struct EditScript {
    /// name of the script to edit (if not present, opens the playground's root dir)
    #[arg(add = script_name_completer(), value_name = "SCRIPT")]
    pub script_name: Option<String>,

    /// Command to invoke editor
    ///
    /// Can be omitted if `package.metadata.cargo-playground.editor-cmd` is set.
    ///
    /// All arguments after this flag ('--cmd') will be interpreted as
    /// the arguments to invoke the editor.
    #[arg(
        long="cmd",
        allow_hyphen_values = true,
        num_args = 0..,
    )]
    pub editor_cmd: Option<Vec<String>>,
}

/// Print CLI completion commands to stdout.
#[apply(DeriveArg)]
#[derive(Args)]
pub struct WriteCompletionScript {
    // TODO: list supported shells (
    // see `clap_complete::env::Shells::builtins()`)
    /// Shell name (bash, fish, and zsh are officially supported)
    #[arg()]
    pub shell: String,
}

impl WriteCompletionScript {
    /// Dynamically generates long help text for this command.
    ///
    /// This is necessary because we want to provide example
    /// commands, so need to know exactly how the command was invoked.
    ///
    /// (If this needs to be done more than once, could also use
    /// a more general mechanism to expand placeholders
    /// within use a regex or something to expand a placeholder
    /// within the docstrings?)
    fn add_after_help_examples(cmd: Command) -> Command {
        let invocation = InvocationType::from_env();

        // `get_bin_name()` is not the bin name, it
        // actually returns the bin name _with the subcommand already appended to it_
        // (e.g., "cargo-playground completions")
        let this_cmd =
            cmd.get_bin_name().unwrap_or("cargo-playground completions");

        let bash_and_zsh_cargo_subcmd_caveat = match &invocation {
            InvocationType::CargoSubcmd { .. } => cstr!(
                r#"<yellow>NB:</> To avoid conflicts with completions for other cargo commands, activate <cyan>cargo</>'s completions <underline>first</>!
(e.g., run <cyan>source <<(rustup completions cargo bash)</> before this one in your profile)
"#
            ),
            InvocationType::Direct(_) => "",
        };

        let helpstr = cformat!(
            r#"This command emits <italic>shell scripts</> that can be used to enable dynamic tab completion in various shells.

<bright-green,bold>Shell autocompletion tips</>
Please note that these steps should work for most common shell setups, but may require further customization to the emitted scripts; you may need to check your shell's documentation.

<bold,underline>Fish:</>
To activate completions for the current session:

  $ <bright-cyan,bold>{this_cmd} fish | source</>

Place this command in your <blue>~/.config/fish/config.fish</>
to activate it for every session.

<bold,underline>Bash/zsh:</>
{bash_and_zsh_cargo_subcmd_caveat}
To activate completions for the current session, either:

  $ <bright-cyan,bold>source <<({this_cmd} bash)</>
or
  $ <bright-cyan,bold>source <<({this_cmd} zsh)</>
"#
        );

        // // NOTE: in fish, lazy loading a script with `cargo playground completions fish | source`
        // //    doesn't work, and in fact breaks autocomplete for cargo entirely
        // //    (note - eval doesn't work either, nor does even just sourcing it from another file?).
        // //    No idea why.
        // let invoked_cmd = invocation.invoked_cmd();
        //  r"#To lazily load these completions for every session, run
        //  $ <bright-cyan,bold>{this_cmd} fish > ~/.config/fish/completions/{invoked_cmd}.fish"#</>

        cmd.after_help(helpstr)
    }
}

#[apply(DeriveArg)]
#[derive(Args)]
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

    // TODO: could this be flattened using Arg::value_delimeter?
    //   Is there a way to have more than one delimiter? (i.e.,
    //   here, commas OR any whitespace?)
    #[arg(
        short = 'F',
        long = "features",
        help="Dependency features to activate",
        long_help = "Dependency-qualified features to activate, \
of the form `[DEPNAME/](FEATURENAME)`, e.g., \"somecrate/somefeature\".

Run `cargo info (DEPNAME)` to see the features available for a given dependency.

The [DEPNAME/] prefix may be omitted if exactly one dependency has been specified.",
        value_parser = parse_feature_arg
    )]
    pub features: Vec<Vec<FeatureCliInput>>,

    // TODO: this takes up too many lines now??
    #[command(flatten, next_help_heading = "Arguments for `cargo add`")]
    pub cargo_add_args: CargoAddArgs,
}

// MAYBE: this should just be a regular struct definition w/ a derive macro
//   (use `macro_rules_attribute` and/or `derive_deftly`)?
//   Although ... using a DSL is tbh way way easier
//   because it doesn't have to support everything?
// TODO: when you `--help`, each one of these arguments takes up 3 lines even though
//       it has no help. Overriding the command's help template won't help because
//       it doesn't control arg formatting.
build_passthrough_long_args!(
    /// Specific args to be forwarded to cargo add
    ///
    /// We only forward these specific flags - rather than everything - because
    /// we need control over `--optional`, `--features`, etc.
    #[derive(Args, Clone, Debug)]
    #[command(flatten_help = true)]
    pub struct CargoAddArgs {
        kv_flags(path, base, git, branch, tag, rev, registry),
        switch_flags(locked, offline, frozen),
    }
);
