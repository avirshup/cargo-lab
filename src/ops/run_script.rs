use std::os::unix::prelude::CommandExt;
use std::process;

use crate::global_ctx::GlobalCtx;
use crate::util;

/// Run the requested script via `cargo run` and activating the desired features
///
/// ONLY RETURNS UPON FAILURE - if everything works right,
/// this `exec`s the cargo run
/// command, so the invoked process replaces this one.
pub fn run_script(
    bin_name: &str,
    args: &[String],
    ctx: GlobalCtx,
) -> crate::Result<()> {
    let script_entry = ctx
        .manifest_data()?
        .get_script(bin_name)
        .ok_or_else(|| crate::Error::ScriptNotFound(bin_name.to_owned()))?;

    let mut cmd = process::Command::new(&ctx.cargo_exe);
    cmd.args([
        "run",
        "--bin",
        &script_entry.name,
        "--features",
        &script_entry.required_features.join(","),
        "--",
    ])
    .args(args);

    util::show_invocation(&cmd);

    // this won't ever return under normal circumstances
    let exec_failure = cmd.exec();
    Err(crate::ioerr!(exec_failure, "Failed exec '{cmd:?}`"))
}
