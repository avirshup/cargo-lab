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
mod random_names;
mod script_name_newtype;

use std::io;
use std::path::PathBuf;

use color_print::ceprintln;

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
/// Anything that requires reading the real manifest or project should
/// be done within the operation itself.
fn run(args: cli::MainCli) -> Result<()> {
    let verbosity = match (args.global_args.quiet, args.global_args.verbose) {
        (true, 0) => global_ctx::Quiet,
        (true, _more_than_zero) =>
            panic!("clap allowed -q and -v to be passed at the same time?"),
        (false, 0) => global_ctx::Normal,
        (false, 1) => global_ctx::Verbose,
        (false, _more_than_1) => global_ctx::Debug,
    };

    let ctx = GlobalCtx::new(verbosity, args.global_args.manifest_path.clone());

    if ctx.verbosity >= global_ctx::Debug {
        println!("Args: {args:#?}");
    }

    match args.cmd {
        cli::SubCmd::InitPlayground(cli::InitPlayground {
            path: path_str,
            name: input_name,
            edition,
        }) => {
            let path = PathBuf::from(path_str);
            let name: String = input_name.unwrap_or_else(|| {
                path.file_name()
                    .expect("non-empty filename")
                    .to_string_lossy()
                    .into()
            });

            ops::init_new_playground(&path, &name, &edition, ctx)?;
        },

        cli::SubCmd::RunScript(cli::RunScript {
            script_name: bin_name,
            args,
        }) => {
            ops::run_script(&bin_name, &args, ctx)?;
        },

        cli::SubCmd::NewScript(cli::NewScript {
            script_name,
            opts:
                cli::NewScriptOpts {
                    template,
                    inject_args,
                    edit,
                },
        }) => {
            let request = _build_script_request(script_name, inject_args)?;
            ops::new_script(&request, template.as_deref(), edit, ctx)?;
        },

        cli::SubCmd::QuickScript(cli::NewScriptOpts {
            template,
            inject_args,
            edit,
        }) => {
            // automatically generate the name
            // TODO: base it on the dependencies if any are requested
            let manifest_data = ctx.manifest_data()?;
            let script_name =
                _generate_script_name_from_deps(&inject_args.deps, |name| {
                    manifest_data.get_script(name).is_none()
                });
            ceprintln!("Generated script name: <blue>{script_name}</>");

            let request = _build_script_request(script_name, inject_args)?;
            ops::new_script(&request, template.as_deref(), edit, ctx)?;
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
    // TODO: we should probably be validating the names (e.g., no special chars)
    //    around here
    let script = input_script.trim_end_matches(".rs").to_owned();
    let feature_requests = cli::resolve_feature_requests(&deps, features)?;

    Ok(data::ScriptConfigRequest {
        script,
        deps,
        features: feature_requests,
        cargo_args: cargo_add_args.cli_args(),
    })
}

const MAX_UNIQUE_NAME_TRIES: u8 = 100;

/// generate a random script name based on depependency names
fn _generate_script_name_from_deps(
    deps: &[data::DepRequest],
    pred: impl Fn(&str) -> bool,
) -> String {
    if deps.is_empty() {
        return _generate_script_name(pred);
    }

    // these will always have name of the form "try-dep1-dep2-[...]", with
    // an adjective at the end if needed to make it unique
    let basename: String = format!(
        "try-{}",
        util::join_str_iter(deps.iter().map(|req| req.depname.as_ref()), "-")
    );
    if pred(&basename) {
        return basename;
    }

    for _i in 0..MAX_UNIQUE_NAME_TRIES {
        let name = format!("{basename}-{}", random_names::random_adjective());
        if pred(&name) {
            return name;
        };
    }

    // if this can't find a unique name within 100 tries something is very wrong
    panic!("Failed to find a unique name?");
}

/// generate a random script name from the ether
fn _generate_script_name(pred: impl Fn(&str) -> bool) -> String {
    for _i in 0..MAX_UNIQUE_NAME_TRIES {
        let name = random_names::random_name();
        if pred(&name) {
            return name;
        };
    }

    // if this can't find a unique name within 100 tries something is very wrong
    panic!("Failed to find a unique name?");
}
