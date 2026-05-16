use std::process;

use camino::{Utf8Path, Utf8PathBuf};

use crate::global_ctx::GlobalCtx;
use crate::manifest_editor::ManifestEditor;
use crate::{data, util};

// ───── Helpers ────────────────────────────────────────────────── //
/// Create a sensible `bin[].path` value from the script's name.
pub(super) fn _path_from_script_name(bin_name: &str) -> Utf8PathBuf {
    Utf8Path::new("src")
        .join(bin_name.replace('-', "_"))
        .with_extension("rs")
}

pub(super) fn _update_manifest_and_show_diff(
    toml: &ManifestEditor,
    target: &Utf8Path,
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
        .args([backup.file_name().unwrap(), target.file_name().unwrap()]);
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
pub(super) fn _run_cargo_add(
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

/// Built-in template for a minimal script
pub(super) fn _init_minimial_script(name: &str) -> String {
    format!(
        r#"// playground script: {name}

fn main() {{
    println!("hello from world")
}}
"#
    )
}
