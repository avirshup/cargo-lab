use crate::cli::CargoAddArgs;
use crate::data;
use crate::manifest_editor::CargoDotToml;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::{fs, process};

/// Run a script via `cargo run` and activating the desired features
///
/// ONLY RETURNS UPON FAILURE - if everything works right,
/// this `exec`s the cargo run
/// command, so the invoked process replaces this one.
pub fn run_script(
    bin_name: &str,
    args: &[String],
    paths: &data::ProjectPaths,
) -> crate::Result<()> {
    let cargo_doc = CargoDotToml::read(&paths.cargo_dot_toml)?;
    let script = cargo_doc
        .get_script(bin_name)
        .ok_or_else(|| crate::Error::ScriptNotFound(bin_name.to_owned()))?;

    let mut cmd = process::Command::new("cargo");
    cmd.args([
        "run",
        "--bin",
        script.name,
        "--features",
        &script.required_features.join(","),
    ])
    .args(args);

    println!("> {cmd:?}");

    Err(cmd.exec().into())
}

pub fn new_script(
    bin_name: &str,
    template_name: &str,
    paths: &data::ProjectPaths,
) -> crate::Result<()> {
    let mut cargo_doc = CargoDotToml::read(&paths.cargo_dot_toml)?;

    // copy the template to the destination if necessary
    let src_filename = _bin_name_to_src_filename(bin_name);
    let dest = paths.manifest_dir.join(&src_filename);
    if dest.is_file() {
        println!("Script '{}' already exists", paths.humanize(&dest));
    } else {
        let template_path = paths.template_path(template_name);
        _copy_file(&template_path, &dest)?;
        println!(
            "Created script: {} -> {}",
            paths.humanize(&template_path),
            paths.humanize(&dest)
        );
    }

    // update Cargo.toml
    cargo_doc.add_new_bin(bin_name, &src_filename)?;
    _update_and_show_diff(&cargo_doc, &paths.cargo_dot_toml)?;

    Ok(())
}

pub fn list_scripts(quiet: bool, paths: &data::ProjectPaths) -> crate::Result<()> {
    let cargo_doc = CargoDotToml::read(&paths.cargo_dot_toml)?;

    let Some(script_iter) = cargo_doc.list_scripts() else {
        if !quiet {
            println!(
                "No [[bin]] entries found in {}",
                paths.cargo_dot_toml.display()
            );
        }
        return Ok(());
    };

    if !quiet {
        println!("Scripts in {}: ", paths.cargo_dot_toml.display());
    }

    for data::ScriptEntry {
        name,
        path,
        required_features,
    } in script_iter
    {
        if quiet {
            println!("{name}");
        } else {
            println!("{name}:\n  path: {path}\n  features: {required_features:#?}");
        }
    }
    Ok(())
}

pub fn inject_deps(
    bin_name: &str,
    deps: &[data::DepRequest],
    features: &[data::FeatureRequest],
    paths: &data::ProjectPaths,
    cargo_add_args: &CargoAddArgs,
) -> crate::Result<()> {
    // TODO: check whether deps already in cargo before running cargo add?

    // first: add the dependencies
    if !deps.is_empty() {
        // NOTE that cargo automatically adds a feature
        //  for every optional dependency, however this is not
        //  actually necessary - the dep name can be listed
        //  directly as a "required-feature" (without
        //  the `dep:` qualifier for whatever reason)
        let cargo_add_result = _run_cmd(
            process::Command::new("cargo")
                .current_dir(&paths.manifest_dir)
                .args(["add", "--optional"])
                .args(cargo_add_args.cli_args())
                .args(deps.iter().map(|d| &d.input_string)),
        )?;
        if !cargo_add_result.success() {
            return Err(crate::Error::CargoFail(format!(
                "`cargo add` command reported failure (status:  {cargo_add_result})"
            )));
        }
    }

    // read the new (possibly modified) manifest
    let mut cargo_doc = CargoDotToml::read(&paths.cargo_dot_toml)?;

    // mutate it
    cargo_doc.activate_features(bin_name, deps, features)?;

    // update it
    _update_and_show_diff(&cargo_doc, &paths.cargo_dot_toml)?;

    Ok(())
}

// ───── Helpers ────────────────────────────────────────────────── //
fn _copy_file(src: &Path, dest: &Path) -> crate::Result<()> {
    fs::copy(src, dest).map_err(|e| crate::Error::CopyFailed {
        src: src.display().to_string(),
        dest: dest.display().to_string(),
        err: e,
    })?;

    Ok(())
}

/// print the command then run it
fn _run_cmd(cmd: &mut process::Command) -> std::io::Result<process::ExitStatus> {
    println!("> {cmd:?}");
    cmd.spawn()?.wait()
}

fn _bin_name_to_src_filename(bin_name: &str) -> String {
    format!("src/{}.rs", bin_name.replace('-', "_"))
}

fn _update_and_show_diff(toml: &CargoDotToml, target: &Path) -> crate::errors::Result<()> {
    // back it up first
    let backup = target.with_added_extension("bak");
    _copy_file(target, &backup)?;

    // overwrite cargo.toml
    toml.write(target)?;

    println!("Updated Cargo.toml: ");

    // we are ignoring any failure to run `diff` here
    // since it's only for illustrative purpose
    // and the file has already been modified
    let _ = _run_cmd(
        process::Command::new("diff")
            .current_dir(target.parent().unwrap())
            .arg("--color=always") // TODO: only pass this if we are running in a TTY
            .args([&backup, target]),
    );

    Ok(())
}
