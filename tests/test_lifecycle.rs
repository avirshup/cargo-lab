#![cfg(test)]
//! End-to-end tests of the produced binaries.
//!
//! These tests generally have more side effects than you
//! might expect from running `cargo test`
//! (i.e., they download dependencies, compile and run code, etc.)
//! so all tests here have `#[ignore]`;
//! Run these with `cargo test -- --ignored`, or run *all* tests
//! with `cargo test -- --include-ignored`

pub mod common;
use common::*;

/// Exercise the playground and script lifecycles.
///
/// This is an e2e test that should every subcommand
/// at least once:
/// - [x] init
/// - [x] run
/// - [x] new
/// - [x] quick
/// - [x] rename
/// - [x] list
/// - [x] info
/// - [x] inject
/// - [ ] edit (tbf pretty trivial)
/// - ~~[ ] completions~~ (checked elsewhere)
/// - [x] check
///
/// (As with everything here,
/// this won't run by default, use `cargo test -- --ignored`)
///
/// By default, this test invokes our executable from a tempdir, via
/// something along the lines of
/// `cargo run --manifest-path=$thisdir --bin=cargo-playground -- [..args]`
///
/// TODO: Test it (probably in containers) when installed:
///     1. as a cargo subcommand
///     2. as a standalone exe
///
/// TODO: add ability to disable color output (and test that it works ...
///      `runner.expect_no_ansi_escapes_in_stdout()` or something?)
///      so we can check the output more reliably.
#[test]
#[ignore]
fn test_playground_lifecycle() {
    // TODO: this runs our commands via `cargo run --bin cargo-playgronud -- [args]`.
    //   Should be testing this _as installed_ in both direct and subcmd mode.
    let tempdir = ScratchDir::new();
    let mut runner = Runner::new(&tempdir);

    // ───── 1. Newly initialized project ───────────────────────────── //
    // ───── Project creation ─────
    runner.run_cpg(&["init", PROJECT_NAME]).expect_ok();

    runner.cd(PROJECT_NAME);

    // check some (not all) files
    runner
        .expect_file("Cargo.toml")
        .expect_file(&projpath("src", INIT_SCRIPT, "rs"))
        .expect_file("templates/basic.rs.template");

    runner.consistency_checks();

    // ───── Commands in newly-created state ─────
    runner
        .run_cpg(&["list", "-q"])
        .expect_ok()
        .expect_stdout(INIT_SCRIPT);

    runner.run_cpg(&["run", INIT_SCRIPT]).expect_ok();

    // try to create an existing script
    runner.run_cpg(&["new", INIT_SCRIPT]).expect_fail();

    // ───── 2. Adding scripts ─────────────────────────────────────── //
    // ───── 2a. SCRIPT2: set name, no dependencies ─────
    // bare script
    let expect_script2_path = projpath("src", SECOND_SCRIPT, "rs");
    runner
        .run_cpg(&["new", SECOND_SCRIPT, "--template=basic"])
        .expect_ok();
    runner.expect_file(&expect_script2_path);
    runner.run_cpg(&["run", SECOND_SCRIPT]).expect_ok();
    runner
        .run_cpg(&["info", SECOND_SCRIPT])
        .expect_ok()
        .expect_stdout_contains(&expect_script2_path);

    // ───── 2b. TRY_CLAP_SCRIPT: Automatically named, w/ dependencies ─────
    runner
        .run_cpg(&[
            "quick",
            "clap",
            "--features",
            "clap/derive",
            "--template",
            "clap",
            "-q",
        ])
        .expect_ok()
        .expect_stdout_contains(TRY_CLAP_SCRIPT)
        .expect_stdout_contains(&projpath("src", TRY_CLAP_SCRIPT, "rs"));

    // TODO: don't require the '--' arg, like how `docker run [imagename]` works:
    //   any `cpg run` flag must come before the script name, and any arguments
    //   after the script name are passed to the script
    runner
        .run_cpg(&[
            "run",
            TRY_CLAP_SCRIPT,
            "--",
            "2",
            "--name",
            "marge simpson",
        ])
        .expect_ok()
        .expect_stdout_contains("oh hi marge simpson");

    // ───── 3. Modifying scripts ─────────────────────────────────────── //

    // ───── 3a. rename TRY_CLAP_SCRIPT -> CLAP_RENAME ─────
    let expect_rename_path = projpath("src", CLAP_RENAME, "rs");

    runner
        .run_cpg(&["rename", TRY_CLAP_SCRIPT, CLAP_RENAME])
        .expect_ok();
    runner.expect_file(&expect_rename_path);

    // test that query functions return it (and not the old one)
    runner.run_cpg(&["info", TRY_CLAP_SCRIPT]).expect_fail();
    runner
        .run_cpg(&["info", CLAP_RENAME])
        .expect_ok()
        .expect_stdout_contains(&expect_rename_path)
        .expect_stdout_contains("\"clap\"")
        .expect_stdout_contains("\"clap/derive\"");

    runner
        .run_cpg(&["list", "-q"])
        .expect_ok()
        .expect_stdout_contains(CLAP_RENAME);

    // ───── 3b. adding new dependencies ─────
    // dependency + implicit-dependency feature
    runner
        .run_cpg(&["inject", CLAP_RENAME, "anyhow", "-F", "backtrace"])
        .expect_ok();

    // explicit-dependency features
    runner
        .run_cpg(&["inject", CLAP_RENAME, "--features=clap/color"])
        .expect_ok();

    // check that they are all at least listed in the output
    runner
        .run_cpg(&["info", CLAP_RENAME])
        .expect_ok()
        .expect_stdout_contains("\"clap\"")
        .expect_stdout_contains("\"clap/derive\"")
        .expect_stdout_contains("\"clap/color\"")
        .expect_stdout_contains("\"anyhow\"")
        .expect_stdout_contains("\"anyhow/backtrace\"");

    // ───── Final checks ───────────────────────────────────────────── //
    runner.consistency_checks();
}
