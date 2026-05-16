use color_print::{ceprintln, cprintln};

use crate::global_ctx::GlobalCtx;
use crate::{data, global_ctx};

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
            manifest_path.parent().unwrap(),
            manifest_path.file_name().unwrap()
        );
    }

    // TODO: handle malformed entries? (they are currently just ignored
    //  by the iterator)
    let mut script_iter = manifest_data.iter_script_entries().peekable();

    // Warning if there are no scripts found
    if ctx.verbosity > global_ctx::Quiet && script_iter.peek().is_none() {
        ceprintln!(
            "<yellow>warning:</> No [[bin]] entries found in {manifest_path}"
        );
    };

    for entry in script_iter {
        _print_script_info(entry, &ctx);
    }
    Ok(())
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
- <cyan>{name}</>:
    <blue>path:</> {path}
    <blue>dependencies:</> {required_features:?}
"
        );
    } else {
        println!("{name}");
    }
}
