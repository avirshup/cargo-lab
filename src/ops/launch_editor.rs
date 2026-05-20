use std::process;

use camino::Utf8PathBuf;

use crate::global_ctx::GlobalCtx;
use crate::util;

/// Run a command to open an editor for the script
pub fn edit_script(
    maybe_script_name: Option<&str>,
    custom_cmd: &Option<Vec<String>>,
    ctx: GlobalCtx,
) -> crate::Result<()> {
    // figure out what path to pass to the editor
    let target_relpath = match maybe_script_name {
        Some(script_name) =>
            ctx.manifest_data()?
                .get_script(script_name)
                .ok_or_else(|| {
                    crate::Error::ScriptNotFound(script_name.to_owned())
                })?
                .path,
        None => Utf8PathBuf::from("."),
    };

    // figure out what the command they want to run
    let cmd_entrypoint = custom_cmd
        .as_ref()
        .or_else(|| {
            ctx.playground_config()
                .as_ref()
                .and_then(|cfg| cfg.editor_cmd.as_ref())
        })
        .filter(|v| !v.is_empty())
        .ok_or(crate::Error::NeedEditorCmd())?;

    // run it
    let mut proc = process::Command::new(&cmd_entrypoint[0]);
    proc.args(&cmd_entrypoint[1..])
        .arg(target_relpath)
        .current_dir(&ctx.project_paths()?.manifest_dir);
    util::show_invocation(&proc);
    util::run_subproc(proc)?;

    Ok(())
}
