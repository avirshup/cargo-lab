#![cfg(test)]
//! Basic autocomplete tests.
//!
//! These don't test the actual shell integration, they just
//! call the binaries (in fish mode, which is easy to work
//! with) to ensure the proper options are being generated.

pub mod common;
use common::*;

/// Ensure the completion scripts generators
/// correctly detect direct invocations
#[test]
fn test_completion_scripts_for_direct_invocation() {
    let tempdir = ScratchDir::new();
    let runner = Runner::new(&tempdir);

    // direct calls
    for shell in TEST_SHELLS {
        runner
            .run_lab(&["completions", shell])
            .expect_ok()
            .expect_stdout_contains(DIRECT_COMPLETE_VAR);
    }
}

/// Ensure the completion scripts generators
/// correctly detect cargo subcommand invocations
#[test]
fn test_completion_scripts_for_subcmd_invocation() {
    let tempdir = ScratchDir::new();
    let runner = Runner::new(&tempdir);

    // set up env vars as if being exec'd by cargo
    let runner_with_cargo_env_vars = runner.with_env([
        ("CARGO".to_owned(), "/path/to/cargo/bin/cargo".to_owned()),
        ("CARGO_HOME".to_owned(), "/path/to/cargo".to_owned()),
    ]);
    for shell in TEST_SHELLS {
        runner_with_cargo_env_vars
            .run_lab(&["lab", "completions", shell])
            .expect_ok()
            .expect_stdout_contains(SUBCMD_COMPLETE_VAR);
    }
}

#[test]
fn test_subcommand_name_autocomplete_for_direct_invocation() {
    let tempdir = ScratchDir::new();
    let runner = Runner::new(&tempdir);

    let direct_completion_runner =
        runner.with_env([(DIRECT_COMPLETE_VAR.to_owned(), "fish".to_owned())]);
    for subcmd in ALL_SUBCMDS {
        direct_completion_runner
            .run_lab(&["--", "cargo-lab", &subcmd[..2]])
            .expect_ok()
            .expect_stdout_contains(subcmd);
    }
}

/// Test whether autocomplete works for subcommand names.
/// directly invokes the autocompletion generator for fish
/// (does not test the actual shell integration)
#[test]
fn test_subcommand_name_autocomplete_for_subcmd_invocation() {
    let tempdir = ScratchDir::new();
    let runner = Runner::new(&tempdir);

    let subcmd_completion_runner =
        runner.with_env([(SUBCMD_COMPLETE_VAR.to_owned(), "fish".to_owned())]);
    for subcmd in ALL_SUBCMDS {
        subcmd_completion_runner
            .run_lab(&["--", "cargo", "lab", &subcmd[..2]])
            .expect_ok()
            .expect_stdout_contains(subcmd);
    }

    // for cargo subcommand mode, ensure that it does NOT autocomplete anything for
    // anything that doesn't start with "cargo lab"
    for cmd in [
        ["--", "cargo", ""],
        ["--", "cargo", "ini"],
        ["--", "cargo", "la"],
    ] {
        subcmd_completion_runner
            .run_lab(&cmd)
            .expect_ok()
            .expect_stdout("");
    }
}

/// Tests the results when invoked with the autocomplete
/// env vars set to "fish".
///
/// Note this definitely
/// DOES NOT test the shell integration itself,
/// just that our program generates the expected
/// completions when called correctly.
#[test]
fn test_dynamic_autocomplete_mode_for_fish() {
    // set up a new project w/ 2 scripts
    let tempdir = ScratchDir::new();
    let mut runner = Runner::new(&tempdir);
    runner.run_lab(&["init", PROJECT_NAME]).expect_ok();
    runner.cd(PROJECT_NAME);
    runner
        .run_lab(&["new", SECOND_SCRIPT, "--offline"])
        .expect_ok();

    // it's time for Will? It? Autocomplete!?!?!
    // (direct call edition)
    {
        let direct_completion_runner = runner
            .with_env([(DIRECT_COMPLETE_VAR.to_owned(), "fish".to_owned())]);
        direct_completion_runner
            .run_lab(&["--", "cargo-lab", "info", ""])
            .expect_ok()
            .expect_stdout_contains(INIT_SCRIPT)
            .expect_stdout_contains(SECOND_SCRIPT);

        for script in [INIT_SCRIPT, SECOND_SCRIPT] {
            direct_completion_runner
                .run_lab(&["--", "cargo-lab", "inject", &script[..3]])
                .expect_ok()
                .expect_stdout_contains(script);
        }
    }

    // it's time for Will? It? Autocomplete!?!?!
    // (cargo subcmd edition)
    {
        let subcmd_completion_runner = runner
            .with_env([(SUBCMD_COMPLETE_VAR.to_owned(), "fish".to_owned())]);
        subcmd_completion_runner
            .run_lab(&["--", "cargo", "lab", "inject", ""])
            .expect_ok()
            .expect_stdout_contains(INIT_SCRIPT)
            .expect_stdout_contains(SECOND_SCRIPT);

        for script in [INIT_SCRIPT, SECOND_SCRIPT] {
            subcmd_completion_runner
                .run_lab(&["--", "cargo", "lab", "info", &script[..3]])
                .expect_ok()
                .expect_stdout_contains(script);
        }
    }
}
