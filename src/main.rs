mod cli;
mod commands;
mod config;
mod data;
mod errors;
mod manifest_editor;
mod util;

use std::io;

use clap_complete::Shell;
use config::Config;

use crate::errors::{Error, Result};

fn main() -> Result<()> {
    // note that this won't return if `$COMPLETE` env var is set
    cli::maybe_exec_dynamic_automcomplete();

    // clap ensures that at most one of "-q" or "-v[v]" was specified
    let args = cli::parse_argv();
    let verbosity = match (args.general.quiet, args.general.verbose) {
        (true, _) => config::Quiet,
        (false, 0) => config::Normal,
        (false, 1) => config::Verbose,
        (false, _more_than_1) => config::Debug,
    };
    let cfg = Config::from_env(verbosity)?;

    run(args, cfg)
}

/// Interpret and validate CLI args and try to fulfill the request
///
/// Responsible for doing any additional parsing/
/// validation on top of what clap already did for us and then
/// dispatching the fully-interpreted user request off to the
/// relevant function.
fn run(args: cli::PlaygroundCli, cfg: Config) -> Result<()> {
    if cfg.verbosity >= config::Verbose {
        println!("Config: {cfg:#?}");
    }
    if cfg.verbosity >= config::Debug {
        println!("Args: {args:#?}");
    }

    match args.cmd {
        cli::SubCmd::RunScript { bin_name, args } => {
            commands::run_script(&bin_name, &args, &cfg)?;
        },

        cli::SubCmd::NewScript {
            bin_name,
            template,
            inject:
                cli::InjectArgs {
                    deps: input_deps,
                    features: input_features,
                    cargo_add_args,
                },
        } => {
            // create the new script (if it does not already exist)
            commands::new_script(&bin_name, &template, &cfg).or_else(
                |err| {
                    if let Error::AlreadyExists(_) = err {
                        Ok(())
                    } else {
                        Err(err)
                    }
                },
            )?;

            // if dependencies/features were requested, install them now
            let feature_requests =
                cli::resolve_feature_requests(&input_deps, input_features)?;
            commands::inject_deps(
                &bin_name,
                &input_deps,
                &feature_requests,
                &cfg,
                &cargo_add_args,
            )?;
        },

        cli::SubCmd::ListScripts {} => {
            commands::list_scripts(&cfg)?;
        },

        cli::SubCmd::InjectDeps {
            bin_name,
            inject:
                cli::InjectArgs {
                    deps: input_deps,
                    features: input_features,
                    cargo_add_args,
                },
        } => {
            // ───── Extra parsing for features ─────────────────────────────── //
            let feature_requests =
                cli::resolve_feature_requests(&input_deps, input_features)?;
            commands::inject_deps(
                &bin_name,
                &input_deps,
                &feature_requests,
                &cfg,
                &cargo_add_args,
            )?;
        },

        cli::SubCmd::InstallCompletions { shell } => {
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

        cli::SubCmd::DoNothing {} => (),
    };

    Ok(())
}
