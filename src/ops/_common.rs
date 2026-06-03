use std::process;

use camino::{Utf8Path, Utf8PathBuf};

use crate::global_ctx::GlobalCtx;
use crate::manifest_editor::ManifestEditor;
use crate::{data, util};

/// Create a sensible `bin[].path` value from the script's name.
pub(super) fn path_from_script_name(bin_name: &str) -> Utf8PathBuf {
    Utf8Path::new("src")
        .join(bin_name.replace('-', "_"))
        .with_extension("rs")
}

/// utility function for committing changes to Cargo.toml.
///
/// TODO: this needs to have output control
///     (verbosity, out vs err, color)
pub(super) fn update_manifest_and_show_diff(
    orig_content: &str,
    toml: &ManifestEditor,
    target: &Utf8Path,
) -> crate::errors::Result<()> {
    // render the toml, see if we even need to do anything
    let new_content = toml.render();
    if orig_content == new_content {
        // early return if content is unchanged
        println!("No changes to Cargo.toml.");
        return Ok(());
    }

    // create backup then overwrite
    let backup = target.with_added_extension("bak");
    util::copy_file(target, &backup)?;
    util::write_file(target, &new_content)?;

    println!("Updated Cargo.toml: ");
    util::display_file_diff(orig_content, &new_content);

    Ok(())
}

/// If new dependencies are requested, runs `cargo add` to add them
/// and then returns a new context (to read the updated Cargo.toml).
///
/// Othewise returns the original context without changes.
pub(super) fn run_cargo_add(
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
pub(super) fn minimal_script(name: &str) -> String {
    format!(
        r#"// lab script: {name}

fn main() {{
    println!("hello from world")
}}
"#
    )
}
