#[macro_use]
extern crate macro_rules_attribute;

mod cli;
mod data;
mod errors;
mod global_ctx;
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

use crate::cli::GeneratesArgs;
use crate::errors::{Error, Result};
use crate::global_ctx::GlobalCtx;

fn main() -> Result<()> {
    // if `$COMPLETE` env var is set, this will never return,
    // otherwise it's (more or less) a no-op
    cli::maybe_exec_dynamic_automcomplete();

    let args = cli::parse_argv();
    run(args)
}

/// Interpret and validate CLI args, then try to fulfill the request
///
/// Does any additional CLI arg parsing
/// necessary to interpret the user's request, then
/// dispatches it to the appropriate operation to fulfill it.
///
/// Any additional validation (e.g., against the manifest
/// or config) should be done in the operation itself.
fn run(args: cli::MainCli) -> Result<()> {
    // nb: clap has already ensured that at most one of "-q/-v" was specified
    let verbosity = match (args.global_args.quiet, args.global_args.verbose) {
        (true, _) => global_ctx::Quiet,
        (false, 0) => global_ctx::Normal,
        (false, 1) => global_ctx::Verbose,
        (false, _more_than_1) => global_ctx::Debug,
    };

    let ctx = GlobalCtx::new(verbosity, args.global_args.manifest_path.clone());

    if ctx.verbosity >= global_ctx::Debug {
        println!("Args: {args:#?}");
    }

    match args.cmd {
        cli::SubCmd::RunScript(cli::RunScript {
            script_name: bin_name,
            args,
        }) => {
            ops::run_script(&bin_name, &args, ctx)?;
        },

        cli::SubCmd::NewScript(cli::NewScript {
            script_name: bin_name,
            template,
            inject_args,
            edit,
        }) => {
            let request = _build_script_request(bin_name, inject_args)?;
            ops::new_script(&request, &template, edit, ctx)?;
        },

        cli::SubCmd::ListScripts => {
            ops::list_scripts(ctx)?;
        },

        cli::SubCmd::ShowScriptInfo(cli::ShowScriptInfo { script_name }) => {
            ops::show_script(&script_name, ctx)?;
        },

        cli::SubCmd::InjectDeps(cli::InjectDeps {
            script_name: bin_name,
            inject_args,
            edit,
        }) => {
            // ───── Extra parsing for features ─────────────────────────────── //
            let request = _build_script_request(bin_name, inject_args)?;
            ops::inject_deps(&request, edit, ctx)?;
        },

        cli::SubCmd::EditScript(cli::EditScript {
            script_name: bin_name,
            editor_cmd,
        }) => {
            ops::edit_script(&bin_name, &editor_cmd, ctx)?;
        },

        cli::SubCmd::WriteCompletionScript(cli::WriteCompletionScript {
            shell,
        }) => {
            cli::print_completion_script(&shell, io::stdout())?;
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
) -> Result<data::ScriptConfigRequest> {
    // this is such a common typo that we just go ahead and fix it here
    let script = input_script.trim_end_matches(".rs").to_owned();
    let feature_requests = cli::resolve_feature_requests(&deps, features)?;

    Ok(data::ScriptConfigRequest {
        script,
        deps,
        features: feature_requests,
        cargo_args: cargo_add_args.cli_args(),
    })
}
