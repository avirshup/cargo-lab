#[macro_use]
extern crate macro_rules_attribute;

mod cli;
mod config;
mod data;
mod errors;
mod manifest_data;
mod manifest_editor;
mod ops;
mod util;

#[cfg(feature = "experimental_cargo_script_rfc3502")]
mod cargo_script;

// lints / formatting disabled for vendored code
#[rustfmt::skip]
#[allow(clippy::all, unused)]
mod vendor_cargo;

use std::io;

use clap_complete::Shell;

use crate::cli::GeneratesArgs;
use crate::config::GlobalCtx;
use crate::errors::{Error, Result};

fn main() -> Result<()> {
    // if `$COMPLETE` env var is set, this will never return,
    // otherwise it's (more or less) a no-op
    cli::maybe_exec_dynamic_automcomplete();

    let args = cli::parse_argv();

    // nb: clap has already ensured that at most one of "-q/-v" was specified
    let verbosity = match (args.global_args.quiet, args.global_args.verbose) {
        (true, _) => config::Quiet,
        (false, 0) => config::Normal,
        (false, 1) => config::Verbose,
        (false, _more_than_1) => config::Debug,
    };
    let ctx = GlobalCtx::from_env(verbosity)?;

    run(args, ctx)
}

/// Interpret and validate CLI args and try to fulfill the request
///
/// Responsible for doing any additional parsing/
/// validation on top of what clap already did for us and then
/// dispatching the fully-interpreted user request off to the
/// relevant function.
fn run(args: cli::MainCli, ctx: GlobalCtx) -> Result<()> {
    if ctx.verbosity >= config::Debug {
        println!("Args: {args:#?}");
    }

    match args.cmd {
        cli::SubCmd::RunScript(cli::RunScript { bin_name, args }) => {
            let data = ctx.manifest_data()?;

            ops::run_script(&bin_name, &args, &ctx.cargo_exe, data)?;
        },

        cli::SubCmd::NewScript(cli::NewScript {
            bin_name,
            template,
            inject_args,
        }) => {
            let request = _build_script_request(bin_name, inject_args)?;
            ops::new_script(&request, &template, ctx)?;
        },

        cli::SubCmd::ListScripts => {
            ops::list_scripts(ctx)?;
        },

        cli::SubCmd::InjectDeps(cli::InjectDeps {
            bin_name,
            inject_args,
        }) => {
            // ───── Extra parsing for features ─────────────────────────────── //
            let request = _build_script_request(bin_name, inject_args)?;
            ops::inject_deps(&request, ctx)?;
        },

        cli::SubCmd::InstallCompletions(cli::InstallCompletions { shell }) => {
            // TODO: figure out this vs env-var path
            let requested_shell =
                shell.or_else(Shell::from_env).ok_or_else(|| {
                    Error::InputErr(
                        "Could not automatically determine your shell; please \
                         specify it with '--shell'"
                            .to_owned(),
                    )
                })?;

            cli::write_completion_script(requested_shell, io::stdout())?;
        },

        // mostly for debugging/testing. (should be hidden from CLI help)
        cli::SubCmd::DoNothing => (),
    };

    Ok(())
}

/// Extra processing to turn CLI inputs into a feature request object.
fn _build_script_request(
    input_script: String,
    cli::InjectArgs {
        deps,
        features,
        cargo_add_args,
    }: cli::InjectArgs,
) -> Result<data::ScriptRequest> {
    // this is such a common typo that we just go ahead and fix it here
    let script = input_script.trim_end_matches(".rs").to_owned();
    let feature_requests = cli::resolve_feature_requests(&deps, features)?;

    Ok(data::ScriptRequest {
        script,
        deps,
        features: feature_requests,
        cargo_args: cargo_add_args.cli_args(),
    })
}
