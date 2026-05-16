use std::process;

use crate::global_ctx::GlobalCtx;
use crate::util;

/// Run a command to open an editor for the script
pub fn edit_script(
    script_name: &str,
    custom_cmd: &Option<Vec<String>>,
    ctx: GlobalCtx,
) -> crate::Result<()> {
    let script_entry = ctx
        .manifest_data()?
        .get_script(script_name)
        .ok_or_else(|| crate::Error::ScriptNotFound(script_name.to_owned()))?;

    let cmd = custom_cmd
        .as_ref()
        .or_else(|| {
            ctx.playground_config()
                .as_ref()
                .and_then(|cfg| cfg.editor_cmd.as_ref())
        })
        .filter(|v| !v.is_empty())
        .ok_or(crate::Error::NeedEditorCmd())?;

    let project_root = &ctx.project_paths()?.manifest_dir;
    let mut proc = process::Command::new(&cmd[0]);
    proc.current_dir(project_root)
        .args(&cmd[1..])
        .arg(script_entry.path);

    util::show_invocation(&proc);
    util::run_subproc(proc)?;

    Ok(())
}
