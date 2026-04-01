mod cli;
mod cli_arg_macros;
mod cli_completions;
mod cli_style;
mod commands;
mod config;
mod data;
mod errors;
mod manifest_editor;
mod util;

use crate::errors::{Error, Result};
use clap::Parser;
use clap_complete::Shell;
use config::Config;
use std::ffi::{OsStr, OsString};
use std::fmt::Write;
use std::path::Path;

fn main() -> Result<()> {
    let args = parse_argv();

    // clap ensures that at most one of "-q" or "-v[v]" was specified
    let verbosity = match (args.general.quiet, args.general.verbose) {
        (true, _) => config::Quiet,
        (false, 0) => config::Normal,
        (false, 1) => config::Verbose,
        (false, _more_than_1) => config::Debug,
    };
    let cfg = Config::from_env(verbosity);

    run(args, cfg)
}

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
        }

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
            commands::new_script(&bin_name, &template, &cfg).or_else(|err| {
                if let Error::AlreadyExists(_) = err {
                    Ok(())
                } else {
                    Err(err)
                }
            })?;

            // if dependencies/features were requested, install them now
            let feature_requests = cli::resolve_feature_requests(&input_deps, input_features)?;
            commands::inject_deps(
                &bin_name,
                &input_deps,
                &feature_requests,
                &cfg,
                &cargo_add_args,
            )?;
        }

        cli::SubCmd::ListScripts {} => {
            commands::list_scripts(&cfg)?;
        }

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
            let feature_requests = cli::resolve_feature_requests(&input_deps, input_features)?;
            commands::inject_deps(
                &bin_name,
                &input_deps,
                &feature_requests,
                &cfg,
                &cargo_add_args,
            )?;
        }

        cli::SubCmd::InstallCompletions { shell, install } => {
            let requested_shell = shell.or_else(Shell::from_env).ok_or_else(|| {
                Error::InputErr(
                    "Could not automatically determine your shell; please specify it with '--shell'"
                        .to_owned()
                )
            })?;

            cli_completions::generate_completions(requested_shell, install);
        }

        cli::SubCmd::DoNothing {} => (),
    };

    Ok(())
}

/// Extra handling before letting clap parse the CLI args.
///
/// When invoked as `cargo playground $args`, cargo will exec
/// this with an argv of
/// ["cargo-playground", "playground", ..args], so:
/// 1. we need to hide one of the arguments for our parse to work, and
/// 2. we need to change the bin name to "cargo playground" (one arg with a space)
///    for the docs to be consistent
///
/// This is based on how "cargo tauri" does it:
/// https://github.com/tauri-apps/tauri/blob/36eee37/crates/tauri-cli/src/main.rs
fn parse_argv() -> cli::PlaygroundCli {
    let mut arg_vec: Vec<OsString> = std::env::args_os().collect();

    // yes, you CAN get mutable references to multiple elements in the same vec
    // if you believe in yourself (OR the *real* mutable references to elements
    // of the same vec are the friends we made along the way)
    let mut arg_iter_mut = arg_vec.iter_mut().take(2);
    let arg0 = arg_iter_mut.next().expect("Empty argv?");
    let maybe_arg1 = arg_iter_mut.next();

    let bin_name = {
        let bin_path = Path::new(arg0);
        bin_path.file_stem().and_then(OsStr::to_str)
    };

    let argv_slice =
        // if it was invoked as "cargo-[something] [something]", we need to get rid of
        // one of the two arguments, and for helpstring purposes rename the
        // new first argument to "cargo something"
        if let Some(cargo_subcmd) = bin_name.and_then(|s| s.strip_prefix("cargo-"))
            && let Some(arg1) = maybe_arg1
            && arg1 == cargo_subcmd
        {
            // overwrite argv[1] and send it
            arg1.clear();
            arg1.write_str("cargo ").unwrap();
            arg1.write_str(cargo_subcmd).unwrap();
            &arg_vec[1..]
        } else {
            &arg_vec
        };

    cli::PlaygroundCli::parse_from(argv_slice)
}
