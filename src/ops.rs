use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process;

use color_print::{ceprintln, cprintln};

use crate::global_ctx::GlobalCtx;
use crate::manifest_editor::ManifestEditor;
use crate::{data, global_ctx, util};

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

/// Create a new script from a template, w/ optional dependencies
pub fn new_script(
    request: &data::ScriptConfigRequest,
    template_name: &str,
    then_edit: bool,
    ctx: GlobalCtx,
) -> crate::Result<()> {
    // early exit if script matching this name already exists
    if ctx.manifest_data()?.get_script(&request.script).is_some() {
        return Err(crate::Error::ScriptNameConflict(request.script.clone()));
    }

    // ───── Part 1: run cargo add if necessary ─────────────────────── //
    // (if it was necessary, this returns a new context since
    // it will have modified Cargo.toml)
    // TODO: would it be nice to keep the original context around?
    let new_ctx = _run_cargo_add(&request.deps, &request.cargo_args, ctx)?;

    // ───── Part 2: create the new script ──────────────────────────── //
    let paths = new_ctx.project_paths()?;
    let src_filename = _script_name_to_filename(&request.script);
    let dest = paths.manifest_dir.join(&src_filename);
    if dest.is_file() {
        ceprintln!(
            "<yellow>warning</>: Script '{}' already exists",
            paths.relpath_project_root(&dest)
        );
    } else {
        let template_path = paths.template_path(template_name);
        util::copy_file(&template_path, &dest)?;
        ceprintln!(
            "<green>success</>: Created script: {} -> {}",
            paths.relpath_project_root(&template_path),
            paths.relpath_project_root(&dest)
        );
    }

    // update the in-memory Cargo.toml
    let mut manifest_editor = new_ctx.new_editor()?;
    manifest_editor.add_new_bin(&request.script, &src_filename)?;
    manifest_editor.activate_features(
        &request.script,
        &request.deps,
        &request.features,
    )?;

    // update it on disk
    _update_and_show_diff(&manifest_editor, &paths.cargo_dot_toml)?;

    // launch editor if requested
    if then_edit {
        edit_script(&request.script, &None, new_ctx.reload())?;
    }

    Ok(())
}

/// Add dependencies and activate features for an existing script
pub fn inject_deps(
    request: &data::ScriptConfigRequest,
    then_edit: bool,
    ctx: GlobalCtx,
) -> crate::Result<()> {
    // ───── Early exits ─────
    let orig_manifest = ctx.manifest_data()?;
    let orig_script = orig_manifest
        .get_script(&request.script)
        .ok_or_else(|| crate::Error::ScriptNotFound(request.script.clone()))?;
    if request.deps.is_empty() && request.features.is_empty() {
        return Ok(());
    }

    // ───── Actual operations ─────
    // run cargo add if necessary
    let new_ctx = _run_cargo_add(&request.deps, &request.cargo_args, ctx)?;
    let paths = new_ctx.project_paths()?;

    // update the in-memory Cargo.toml
    let mut manifest_editor = new_ctx.new_editor()?;
    manifest_editor.activate_features(
        &request.script,
        &request.deps,
        &request.features,
    )?;

    // update it on disk
    _update_and_show_diff(&manifest_editor, &paths.cargo_dot_toml)?;

    // launch editor if requested
    if then_edit {
        edit_script(&orig_script.name, &None, new_ctx.reload())?;
    }

    Ok(())
}

pub fn show_script(name: &str, ctx: GlobalCtx) -> crate::Result<()> {
    let manifest_data = ctx.manifest_data()?;

    if let Some(entry) = manifest_data.get_script(name) {
        _print_script_info(entry, &ctx);
        Ok(())
    } else {
        Err(crate::Error::ScriptNotFound(name.to_owned()))
    }
}

/// Display script information to stdout
pub fn list_scripts(ctx: GlobalCtx) -> crate::Result<()> {
    let manifest_data = ctx.manifest_data()?;
    let manifest_path = &ctx.project_paths()?.cargo_dot_toml;

    // if not quiet, show manifest path too
    if ctx.verbosity > global_ctx::Quiet {
        ceprintln!(
            "<blue>manifest:</>{}/<cyan>{}</> \n",
            manifest_path.parent().unwrap().display(),
            manifest_path.file_name().unwrap().display()
        );
    }

    // TODO: handle malformed entries? (they are currently just ignored
    //  by the iterator)
    let mut script_iter = manifest_data.iter_script_entries().peekable();

    // Warning if there are no scripts found
    if ctx.verbosity > global_ctx::Quiet && script_iter.peek().is_none() {
        ceprintln!(
            "<yellow>warning:</> No [[bin]] entries found in {}",
            manifest_path.display()
        );
    };

    for entry in script_iter {
        _print_script_info(entry, &ctx);
    }
    Ok(())
}

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

// ───── Helpers ────────────────────────────────────────────────── //
fn _script_name_to_filename(bin_name: &str) -> String {
    format!("src/{}.rs", bin_name.replace('-', "_"))
}

fn _update_and_show_diff(
    toml: &ManifestEditor,
    target: &Path,
) -> crate::errors::Result<()> {
    // back it up first
    let backup = target.with_added_extension("bak");
    util::copy_file(target, &backup)?;

    // overwrite cargo.toml
    toml.write(target)?;

    // TODO: better output if cargo.toml didn't change
    eprintln!("Updated Cargo.toml: ");

    let mut cmd = process::Command::new("diff");
    cmd.current_dir(target.parent().unwrap())
        .arg("--color=always") // TODO: this should be controlled globally
        .args([&backup, target]);
    util::show_invocation(&cmd);

    // we are ignoring any failure to run `diff` here
    // since it's only for illustrative purpose
    // and the file has already been modified
    let _ = util::run_subproc(cmd);
    Ok(())
}

/// If new dependencies are requested, runs `cargo add` to add them
/// and then returns a new context (to read the updated Cargo.toml).
///
/// Othewise returns the original context without changes.
fn _run_cargo_add(
    deps: &[data::DepRequest],
    cargo_add_args: &[String],
    ctx: GlobalCtx,
) -> crate::Result<GlobalCtx> {
    {
        let manifest_data = ctx.manifest_data()?;
        let new_deps: Vec<&data::DepRequest> = deps
            .iter()
            .filter(|dep| !manifest_data.dep_satisfied(dep))
            .collect();

        if new_deps.is_empty() {
            Ok(ctx)
        } else {
            let mut cmd = process::Command::new(&ctx.cargo_exe);
            cmd.current_dir(&ctx.project_paths()?.manifest_dir)
                .args(["add", "--optional"])
                .args(cargo_add_args)
                .args(deps.iter().map(|d| &d.input_string));
            util::show_invocation(&cmd);

            let cargo_add_result = util::run_subproc(cmd)?;
            if !cargo_add_result.success() {
                Err(crate::Error::CargoFail(format!(
                    "`cargo add` command reported failure (status:  \
                     {cargo_add_result})"
                )))
            } else {
                Ok(ctx.reload())
            }
        }
    }
}

fn _print_script_info(
    data::ScriptEntry {
        name,
        path,
        required_features,
    }: data::ScriptEntry,
    ctx: &GlobalCtx,
) {
    if ctx.verbosity > global_ctx::Quiet {
        cprintln!(
            "\
- <green>{name}</>:
    <blue>path:</> {path}
    <blue>dependencies:</> {required_features:?}
"
        );
    } else {
        println!("{name}");
    }
}
