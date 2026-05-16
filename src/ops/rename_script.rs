use color_print::ceprintln;

use super::_common;
use crate::global_ctx::GlobalCtx;
use crate::{global_ctx, util};

/// Change script's name or path
pub fn rename_script(
    input_old_name: &str,
    new_name: &str,
    then_edit: bool,
    ctx: GlobalCtx,
) -> crate::Result<()> {
    let paths = ctx.project_paths()?;
    let orig_manifest = ctx.manifest_data()?;
    let orig_script =
        orig_manifest.get_script(input_old_name).ok_or_else(|| {
            crate::Error::ScriptNotFound(input_old_name.to_owned())
        })?;

    // ───── Part 1: sanity checks ──────────────────────────────────── //
    // check names
    if orig_script.name == new_name {
        if ctx.verbosity > global_ctx::Quiet {
            ceprintln!(
                "<yellow>warning</> Renaming '{new_name}' to itself (nothing \
                 changed)"
            );
        }
        return Ok(());
    }
    if orig_manifest.get_script(new_name).is_some() {
        return Err(crate::Error::ScriptNameConflict(new_name.to_owned()));
    }

    // check paths
    let new_path = _common::_path_from_script_name(new_name);
    if paths.manifest_dir.join(&new_path).exists() {
        return Err(crate::Error::FileErr {
            path: new_path,
            description: "Cannot rename; would overwrite existing file"
                .to_owned(),
        });
    }

    // ───── Part 2: apply the changes ──────────────────────────────── //

    // update manifest in memory
    // (do this first so it will fail before changing anything on disk)
    let mut manifest_editor = ctx.new_editor()?;
    manifest_editor.update_bin(
        &orig_script.name,
        Some(new_name),
        Some(new_path.as_str()),
    )?;

    // Make changes on disk
    if orig_script.path != new_path {
        ceprintln!(
            "<blue>mv</> <yellow>{}</> -> <green>{}</>",
            orig_script.path,
            new_path
        );
        util::rename_file(
            &paths.manifest_dir.join(&orig_script.path),
            &paths.manifest_dir.join(&new_path),
        )?;
    };
    _common::_update_manifest_and_show_diff(
        &manifest_editor,
        &paths.cargo_dot_toml,
    )?;

    // done
    if then_edit {
        super::launch_editor::edit_script(
            &orig_script.name,
            &None,
            ctx.reload(),
        )?;
    }
    Ok(())
}
