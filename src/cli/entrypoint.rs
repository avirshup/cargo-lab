//! Tools for determining whether this was invoked as a cargo subcommand
//! or not and then parsing argv accordingly

use std::env;

use camino::Utf8Path;
use clap::{CommandFactory, FromArgMatches};

use crate::cli;

pub fn autocomplete_or_parse_args() -> cli::MainCli {
    // ───── Let clap decide whether or not we're generating completions ─────
    // this is essentially the same as `CompleteEnv::complete()` except
    // we're dispatching based on the specific autocomplete env var that's
    // been passed.
    if env::var(super::SUBCMD_COMPLETE_VAR)
        .map(|x| !x.is_empty())
        .unwrap_or(false)
    {
        _run_cargo_subcmd_autocomplete_and_exit()
    }
    if env::var(super::SUBCMD_COMPLETE_VAR)
        .map(|x| !x.is_empty())
        .unwrap_or(false)
    {
        _run_normal_autocomplete_and_exit()
    };
    // if the program didn't exit by now, then we're not in autocomplete mode

    // ───── Parse the CLI arguments and return them ─────
    // this is a slightly customized version of what `Parser::parse()` does
    let invocation = super::InvocationType::from_env();
    let cmd = invocation.build_cli_cmd();
    let argv = invocation.normalized_argv();
    cli::MainCli::from_arg_matches(&cmd.get_matches_from(argv))
        .unwrap_or_else(|e| e.exit())
}

fn _run_normal_autocomplete_and_exit() {
    clap_complete::CompleteEnv::with_factory(cli::MainCli::command)
        .var(super::DIRECT_COMPLETE_VAR)
        .complete();
}

/// Called when we think we're generating autocompletions for
/// an invocation of cargo itself. This needs to figure out 2 things:
/// 1) Is the user even trying to run `cargo playground`?
///    If not, we need to output nothing and exit as quickly as possible.
/// 2) If so, we need to figure out how "playground" is spelled here (i.e.,
///    work the same even if the user renamed an executable or whatever)
fn _run_cargo_subcmd_autocomplete_and_exit() {
    let mut arg_iter = env::args();

    // Figure out if we're being called as `cargo-subcmd -- cargo subcmd [args..]`
    // (where "subcmd" is usually "playground")
    // If it's of the form "cargo-subcmd -- cargo add" or something, then
    // we exit immediately - the user is running some other cargo cmd.
    let arg0 = arg_iter.next().expect("argv[0] exists");
    let cmd_suffix = crate::util::cargo_cmd_suffix(Utf8Path::new(&arg0));
    let subcmd_cli_arg = arg_iter.skip_while(|arg| arg != "--").nth(1);

    // IF we were called as "cargo-thing -- cargo thing", then "thing"
    // is the name of our subcommand, so go ahead and generate completions
    if let Some(suffix) = cmd_suffix
        && let Some(subcmd) = subcmd_cli_arg
        && suffix == subcmd
    {
        clap_complete::CompleteEnv::with_factory(|| {
            clap::Command::new("fake-cargo")
                .subcommand(cli::MainCli::command().name(subcmd.to_owned()))
        })
        .var(super::SUBCMD_COMPLETE_VAR)
        .complete(); // <- never returns
    }

    // whether or not autocomplete ran, we're done here
    std::process::exit(0);
}
