mod helpers;

use helpers::*;

const PROJECT_NAME: &str = "my_project";
const FIRST_SCRIPT_NAME: &str = "my-first-script"; // this is currently hardcoded in `ops` module

#[test]
fn test_lifecycle() {
    let mut runner = Runner::new_tempdir();

    // ───── Project creation ─────
    runner
        .run(&["init", PROJECT_NAME])
        .expect_ok("failed to init project");
    runner.cd(PROJECT_NAME);

    // probably check it every time? TODO: implement "check" (instead of `do-nothing` maybe)
    runner.run(&["check"]).expect_ok("Project check failed");

    // ───── Commands in newly-created state ─────
    runner
        .run(&["list", "-q"])
        .expect_ok("failed to list initial project")
        .expect_stdout(FIRST_SCRIPT_NAME);

    runner
        .run(&["run", "my-first-script"])
        .expect_ok("Project check failed");

    // ───── Creating new commands ─────
}
