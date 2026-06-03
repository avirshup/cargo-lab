use std::fmt::Display;

use camino::Utf8PathBuf;
use color_print::{cformat, cprintln};

use crate::global_ctx::{GlobalCtx, ProjectPaths};
use crate::manifest_data::{BinTable, ManifestData};

pub fn check_project(ctx: GlobalCtx) -> crate::Result<()> {
    // ───── Basic project config ───────────────────────────────────── //
    // we can't check much if any of these fail
    cprintln!("Checking cargo lab configuration ...");

    // Cargo.toml discovery
    let path_result = ctx.project_paths();
    _print_check_result(
        "manifest path",
        path_result
            .map(|paths| paths.cargo_dot_toml.as_str())
            .map_err(|_| "not found"),
    );
    let paths = path_result?;

    // Cargo.toml parsing
    let manifest_data_result = ctx.manifest_data();
    _print_check_result(
        "parse Cargo.toml",
        path_result.map(|_| "success").map_err(|_| "failed"),
    );
    let manifest_data = manifest_data_result?;

    // ───── Inspections ────────────────────────────────────────────── //
    // we don't have to stop after the first error anymore
    let mut errs = Vec::new();

    // [package.metadata.cargo-lab]
    let lab_cfg_result = ctx.lab_config().as_ref().ok_or_else(|| {
        crate::Error::NoConfig("package.metadata.cargo-lab".to_owned())
    });
    _print_check_result(
        "Cargo.toml::[package.metadata.cargo-lab]",
        path_result.map(|_| "valid").map_err(|_| "not found"),
    );
    if let Err(e) = lab_cfg_result {
        errs.push(e);
    }

    // the scripts
    cprintln!("\nScripts:");
    for (bin_num, script_toml) in manifest_data.bin.iter().enumerate() {
        if let Err(err) =
            _check_script_entry(paths, script_toml, manifest_data, bin_num)
        {
            errs.push(err);
        }
    }

    // ───── Report overall result ──────────────────────────────────── //
    if !errs.is_empty() {
        Err(crate::Error::from_nonempty_iter(errs.into_iter()))
    } else {
        Ok(())
    }
}

fn _print_check_result<T, E>(description: &str, value: Result<T, E>)
where
    T: Display,
    E: Display,
{
    match value {
        Ok(val) => cprintln!("<green> [ok]</> {description}: <blue>{val}</>"),
        Err(val) => cprintln!("<red>[err]</> {description}: <yellow>{val}</>"),
    }
}

fn _check_script_entry(
    paths: &ProjectPaths,
    script_toml: &BinTable,
    manifest_data: &ManifestData,
    bin_num: usize,
) -> crate::Result<()> {
    // does it even have a name? If not, refer to it by index
    let name_result: crate::Result<&String> =
        script_toml
            .name
            .as_ref()
            .ok_or(crate::Error::ManifestStructureErr(
                "'name' field not found".to_owned(),
            ));

    let display_name = &name_result
        .as_deref()
        .cloned()
        .unwrap_or_else(|_| format!("entry #{bin_num}"));

    let check_result: crate::Result<Utf8PathBuf> = name_result
        .and_then(|name| {
            // is the script entry well-formed?
            manifest_data.get_script(name).ok_or(
                crate::Error::ManifestStructureErr(
                    "Could not parse script entry".to_owned(),
                ),
            )
        })
        .and_then(|entry| {
            // does the path exist?
            if paths.manifest_dir.join(&entry.path).is_file() {
                Ok(entry.path)
            } else {
                Err(crate::Error::FileErr {
                    path: entry.path.to_owned(),
                    description: "Not a file".to_owned(),
                })
            }
        });

    _print_check_result(
        &cformat!("  <cyan>{display_name}</>"),
        check_result.as_ref(),
    );

    // final result
    check_result.map(|_| ())
}
