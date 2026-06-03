pub const ALL_SUBCMDS: [&str; 13] = [
    "run",
    "init",
    "run",
    "new",
    "quick",
    "rename",
    "list",
    "check",
    "inject",
    "info",
    "edit",
    "completions",
    "help",
];
pub const TEST_SHELLS: [&str; 3] = ["fish", "bash", "zsh"];

pub const PROJECT_NAME: &str = "my_project";
pub const SECOND_SCRIPT: &str = "script2";
pub const TRY_CLAP_SCRIPT: &str = "try-clap";
pub const CLAP_RENAME: &str = "clap-greeter";

// ───── pub constants from the crate ───────────────────────────────── //
// Because they are observable properties of the program,
// these are hardcoded here (i.e., changing them in the code could be
// be considered a breaking change in the tool)
// from `ops` module:
pub const INIT_SCRIPT: &str = "my-first-script";

// from cli::invocations module:
pub const DIRECT_COMPLETE_VAR: &str = "COMPLETE_CARGO_LAB_DIRECT";
pub const SUBCMD_COMPLETE_VAR: &str = "COMPLETE_CARGO_LAB_SUBCMD";
