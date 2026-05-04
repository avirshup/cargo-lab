#![cfg(test)]

mod helpers;
use helpers::*;

const PROJECT_NAME: &str = "my_project";
const FIRST_SCRIPT_NAME: &str = "my-first-script"; // !this is currently hardcoded in `ops` module!
const SCRIPT2: &str = "script2";
const TRY_CLAP_SCRIPT: &str = "try-clap";
const CLAP_RENAME: &str = "clap-greeter";

/// Exercise the playground and script lifecycles:
/// (create, configure, query, rename, run, etc.)
///
/// This includes stuff that you don't want to run by
/// default (download dependencies, compile and run code)
/// so it's ignored by default,
/// run it with `cargo test -- --ignored`
///
/// By default, this test invokes our executable from a tempdir, via
/// something along the lines of
/// `cargo run --manifest-path=$thisdir --bin=cargo-playground -- [..args]`
///
/// TODO: Test it (probably in containers) when installed:
///     1. as a cargo subcommand
///     2. as a standalone exe
///
#[test]
#[ignore]
fn test_playground_lifecycle() {
    let mut runner = Runner::new();

    // ───── Project creation ─────
    runner
        .run(&["init", PROJECT_NAME])
        .expect_ok("failed to init project");

    runner.cd(PROJECT_NAME);

    // check some (not all) files
    runner
        .expect_file("Cargo.toml")
        .expect_file(&format!("src/{}.rs", FIRST_SCRIPT_NAME.replace('-', "_")))
        .expect_file("templates/basic.rs.template");

    runner.consistency_checks();

    // probably check it every time? TODO: implement "check" (instead of `do-nothing` maybe)
    // runner.run(&["check"]).expect_ok("Project check failed");

    // ───── Test whether shell autocomplete works ─────
    // (just a sanity check)

    // ───── Commands in newly-created state ─────
    runner
        .run(&["list", "-q"])
        .expect_ok("failed to list initial project")
        .expect_stdout(FIRST_SCRIPT_NAME);

    runner
        .run(&["run", FIRST_SCRIPT_NAME])
        .expect_ok("Project check failed");

    // try to create an existing script
    runner
        .run(&["new", FIRST_SCRIPT_NAME])
        .expect_fail("name collision");

    // ───── Creating new commands ─────
    // bare script
    runner
        .run(&["new", SCRIPT2, "--template=basic"])
        .expect_ok("failed to create new script");
    runner.expect_file(&format!("src/{SCRIPT2}.rs"));
    runner
        .run(&["run", SCRIPT2])
        .expect_ok("failed to run new script");

    // and now with dependencies
    runner
        .run(&[
            "quick",
            "clap",
            "--features",
            "clap/derive",
            "--template",
            "clap",
            "-q",
        ])
        .expect_ok("quick script with dependencies")
        .expect_stdout_contains(TRY_CLAP_SCRIPT)
        .expect_stdout_contains(&format!(
            "src/{}.rs",
            TRY_CLAP_SCRIPT.replace('-', "_"),
        ));

    // TODO: don't require the '--' arg, like how `docker run [imagename]` works
    runner
        .run(&["run", TRY_CLAP_SCRIPT, "--", "2", "--name", "marge simpson"])
        .expect_ok("running clap template script")
        .expect_stdout_contains("oh hi marge simpson");

    // renaming scripts
    runner
        .run(&["rename", TRY_CLAP_SCRIPT, CLAP_RENAME])
        .expect_ok("failed to rename script");
    runner
        .run(&["list", "-q"])
        .expect_ok("listing after rename")
        .expect_stdout_contains(CLAP_RENAME);
    runner.expect_file(&format!("src/{}.rs", CLAP_RENAME.replace('-', "_")));

    // internal consistency checks again
    runner.consistency_checks();

    // TODO: Test autocomplete
}
