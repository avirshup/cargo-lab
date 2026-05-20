use super::_common;
use crate::data;
use crate::global_ctx::GlobalCtx;

/// Add dependencies and activate features for an existing script
pub fn inject_deps(
    request: &data::ScriptConfigRequest,
    then_edit: bool,
    ctx: GlobalCtx,
) -> crate::Result<()> {
    // ───── Early exits ─────
    let orig_manifest_content = ctx.manifest_raw()?.to_owned();
    let orig_manifest = ctx.manifest_data()?;
    let orig_script = orig_manifest
        .get_script(&request.script)
        .ok_or_else(|| crate::Error::ScriptNotFound(request.script.clone()))?;
    if request.deps.is_empty() && request.features.is_empty() {
        return Ok(());
    }

    // ───── Actual operations ─────
    // run cargo add if necessary
    let new_ctx =
        _common::run_cargo_add(&request.deps, &request.cargo_args, ctx)?;
    let paths = new_ctx.project_paths()?;

    // update the in-memory Cargo.toml
    let mut manifest_editor = new_ctx.new_editor()?;
    manifest_editor.activate_features(
        &request.script,
        &request.deps,
        &request.features,
    )?;

    // update it on disk
    _common::update_manifest_and_show_diff(
        &orig_manifest_content,
        &manifest_editor,
        &paths.cargo_dot_toml,
    )?;

    // launch editor if requested
    if then_edit {
        super::launch_editor::edit_script(
            Some(&orig_script.name),
            &None,
            new_ctx.reload(),
        )?;
    }

    Ok(())
}
