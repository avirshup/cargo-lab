use crate::Error;
use crate::ProjectPaths;
use crate::data::ScriptEntry;
use crate::manifest_editor::CargoDotToml;
use std::path::Path;
use std::{fs, process};

pub fn new_script(bin_name: &str, template_name: &str, paths: &ProjectPaths) -> crate::Result<()> {
    let mut cargo_doc = CargoDotToml::read(&paths.cargo_dot_toml)?;

    // copy the template to the destination if necessary
    let src_filename = _bin_name_to_src_filename(bin_name);
    let dest = paths.script_dir.join(&src_filename);
    if dest.is_file() {
        println!("Script '{}' already exists", paths.humanize(&dest));
    } else {
        let template_path = paths.template_path(template_name);
        fs::copy(&template_path, &dest)?;
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

pub fn list_scripts(quiet: bool, paths: &ProjectPaths) -> crate::Result<()> {
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

    for ScriptEntry { name, path } in script_iter {
        if quiet {
            println!("{name}");
        } else {
            println!(" - {name} -> {path}");
        }
    }
    Ok(())
}

pub fn inject_deps(
    bin_name: &str,
    dep_name: &str,
    dep_features: &[String],
    paths: &ProjectPaths,
) -> crate::Result<()> {
    // add the dependency with cargo add
    let cargo_add_result = process::Command::new("cargo")
        .current_dir(&paths.manifest_dir)
        .args(["add", dep_name, "--optional"])
        .spawn()?
        .wait()?;
    if !cargo_add_result.success() {
        return Err(Error::CargoFail(format!(
            "`cargo add` command reported failure (status:  {cargo_add_result})"
        )));
    }

    // build new cargo.toml
    let mut cargo_doc = CargoDotToml::read(&paths.cargo_dot_toml)?;
    cargo_doc.add_dep_to_feature(bin_name, dep_name, dep_features)?;
    _update_and_show_diff(&cargo_doc, &paths.cargo_dot_toml)?;

    Ok(())
}

// ───── Helpers ────────────────────────────────────────────────── //
fn _bin_name_to_src_filename(bin_name: &str) -> String {
    format!("src/{}.rs", bin_name.replace('-', "_"))
}

fn _update_and_show_diff(toml: &CargoDotToml, target: &Path) -> crate::errors::Result<()> {
    // back it up first
    let backup = target.with_added_extension("bak");
    fs::copy(target, &backup)?;

    // overwrite cargo.toml
    toml.write(target)?;

    println!("Updated Cargo.toml: ");
    let _ = process::Command::new("diff")
        .current_dir(target.parent().unwrap())
        .arg("--color=always") // TODO: only pass this if we are running in a TTY
        .args([&backup, target])
        .spawn()?
        .wait()?;

    Ok(())
}
