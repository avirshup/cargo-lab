use color_print::ceprintln;

use super::_common;
use crate::{data, global_ctx, util};

/// Create a new script from a template, w/ optional dependencies
///
/// If the source file for the script already exists but does not have
/// an entry in the manifest, this will add it to the manifest without modifying
/// the source file.
pub fn new_script(
    request: &data::ScriptConfigRequest,
    template: Option<&str>,
    then_edit: bool,
    ctx: global_ctx::GlobalCtx,
) -> crate::Result<()> {
    // TODO: split this up, needs to have one entrypoint for being
    //  called from `main.rs`, another when being called from other ops.

    // early exit if script matching this name already exists
    if ctx.manifest_data()?.get_script(&request.script).is_some() {
        return Err(crate::Error::ScriptNameConflict(request.script.clone()));
    }
    // TODO: handle source file name collisions too

    // ───── Part 1: run cargo add (if necessary) ───────────────────── //
    // (if it was necessary, this returns a new context since
    // it will have modified Cargo.toml)
    let new_ctx =
        _common::_run_cargo_add(&request.deps, &request.cargo_args, ctx)?;

    // ───── Part 2: create the source file ─────────────────────────── //
    // TODO: less TOCTOU here, use `File::create_new`?
    let paths = new_ctx.project_paths()?;
    let src_filename = _common::_path_from_script_name(&request.script);
    let dest = paths.manifest_dir.join(&src_filename);
    if dest.is_file() {
        // if the _source file_ already exists, don't overwrite it,
        // but (for now) proceed with the rest
        ceprintln!(
            "<yellow>warning</>: Script '{}' already exists",
            paths.relpath_project_root(&dest)
        );
    } else if let Some(template_name) = template {
        // create the source file by copying the template

        // TODO: add a comment to the top the same as when using the builtin template
        let template_path = paths.template_path(template_name);
        util::copy_file(&template_path, &dest)?;
        if new_ctx.verbosity > global_ctx::Quiet {
            ceprintln!(
                "<green>success</>: Created script from template: {} -> {}",
                paths.relpath_project_root(&template_path),
                paths.relpath_project_root(&dest)
            );
        }
    } else {
        // create the source file using our built-in super-minimal script
        util::write_file(
            &dest,
            &_common::_init_minimial_script(&request.script),
        )?;

        if new_ctx.verbosity > global_ctx::Quiet {
            ceprintln!(
                "<green>success</>: Created minimal script at: {}",
                paths.relpath_project_root(&dest)
            );
        }
    }

    // ───── Part 3: update the manifest ────────────────────────────── //
    let mut manifest_editor = new_ctx.new_editor()?;
    manifest_editor.add_new_bin(&request.script, src_filename.as_str())?;
    manifest_editor.activate_features(
        &request.script,
        &request.deps,
        &request.features,
    )?;
    _common::_update_manifest_and_show_diff(
        &manifest_editor,
        &paths.cargo_dot_toml,
    )?;

    // ───── finish up ──────────────────────────────────────────────── //
    // if in "quiet" mode, the only output is the name of the newly created script
    if new_ctx.verbosity == global_ctx::Quiet {
        println!("{}", &request.script);
    }

    // launch editor if requested
    if then_edit {
        super::launch_editor::edit_script(
            &request.script,
            &None,
            new_ctx.reload(),
        )?;
    }

    Ok(())
}
