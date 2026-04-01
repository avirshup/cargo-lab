use crate::cli::CargoAddArgs;
use crate::manifest_editor::CargoDotToml;
use crate::util;
use crate::{config, data};
use color_print::{ceprintln, cprintln};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::{fs, process};
// /// Attempt to open a dependency's manifest in a browser.
// ///
// ///
// /// ## Q: Why in a browser?
// /// The cargo CLI does not expose any way to get a dependency's manifest (AFACT).
// /// The cargo _library_ crate might, but it is explicitly not stable and you
// /// are expressly discouraged from using it. It would also be a huge dependency.
// ///
// pub fn open_dep_manifest(depname: String, just_show_url: bool) -> crate::Result<()> {
//     let url = format!("https://docs.rs/crate/{depname}/latest/source/Cargo.toml");
//     if just_show_url {
//         println!("{url}");
//     } else {
//         println!("Manifest link: {url}");
//
//         // Q: Why calling `open`, `xdg-open`, etc. instead using `webbrowser` crate?
//         //    Or why not download it directly and display on CLI?
//         // A: Too big dependencies. Too many transitive dependencies for a non-essential feature.
//         cfg_if::cfg_if! {
//             if #[cfg(target_os = "macos")] {
//                 _run_cmd(process::Command::new("open").arg(&url))?;
//             } else if #[cfg(target_family = "unix")] {
//                 _run_cmd(process::Command::new("xdg-open").arg(&url))?;
//             } else if #[cfg(target_family = "windows")] {
//                 _run_cmd(process::Command::new("start").arg(&url))?;
//             } else {
//                 println!("No URL opener found, please open {url} in a browser.");
//             }
//         }
//     }
//
//     Ok(())
// }

/// Run the requested script via `cargo run` and activating the desired features
///
/// ONLY RETURNS UPON FAILURE - if everything works right,
/// this `exec`s the cargo run
/// command, so the invoked process replaces this one.
pub fn run_script(bin_name: &str, args: &[String], cfg: &config::Config) -> crate::Result<()> {
    let cargo_doc = CargoDotToml::read(&cfg.cargo_dot_toml)?;
    let script = cargo_doc
        .get_script(bin_name)
        .ok_or_else(|| crate::Error::ScriptNotFound(bin_name.to_owned()))?;

    let mut cmd = process::Command::new(&cfg.cargo_exe);
    cmd.args([
        "run",
        "--bin",
        script.name,
        "--features",
        &script.required_features.join(","),
        "--",
    ])
    .args(args);

    util::show_invocation(&cmd);

    let exec_failure = cmd.exec(); // this shouldn't actually ever be set

    Err(crate::Error::IoFail(
        format!("Failed exec '{cmd:?}`"),
        exec_failure,
    ))
}

pub fn new_script(
    input_bin_name: &str,
    template_name: &str,
    cfg: &config::Config,
) -> crate::Result<()> {
    let mut cargo_doc = CargoDotToml::read(&cfg.cargo_dot_toml)?;

    // remove trailing .rs in input bin name since it happens so much
    // MAYBE: validate the name and resulting filename
    //  What are the rules tho? Could this be delegated to `cargo` somehow?
    let bin_name = input_bin_name.trim_end_matches(".rs");

    // copy the template to the destination if necessary
    let src_filename = _bin_name_to_src_filename(bin_name);
    let dest = cfg.manifest_dir.join(&src_filename);
    if dest.is_file() {
        ceprintln!(
            "<yellow>warning</>: Script '{}' already exists",
            cfg.relpath_project_root(&dest)
        );
    } else {
        let template_path = cfg.template_path(template_name);
        util::copy_file(&template_path, &dest)?;
        ceprintln!(
            "<green>success</>: Created script: {} -> {}",
            cfg.relpath_project_root(&template_path),
            cfg.relpath_project_root(&dest)
        );
    }

    // update Cargo.toml
    cargo_doc.add_new_bin(bin_name, &src_filename)?;
    _update_and_show_diff(&cargo_doc, &cfg.cargo_dot_toml)?;

    Ok(())
}

pub fn list_scripts(cfg: &config::Config) -> crate::Result<()> {
    let cargo_doc = CargoDotToml::read(&cfg.cargo_dot_toml)?;

    let Some(script_iter) = cargo_doc.list_scripts() else {
        if cfg.verbosity > config::Quiet {
            ceprintln!(
                "<yellow>warning:</> No [[bin]] entries found in {}",
                cfg.cargo_dot_toml.display()
            );
        }
        return Ok(());
    };

    if cfg.verbosity > config::Quiet {
        // TODO: windows path style, probably need a custom formatter
        ceprintln!(
            "<blue>manifest:</>{}/<cyan>{}</> \n",
            cfg.cargo_dot_toml.parent().unwrap().display(),
            cfg.cargo_dot_toml.file_name().unwrap().display()
        );
    }

    for data::ScriptEntry {
        name,
        path,
        required_features,
    } in script_iter
    {
        if cfg.verbosity > config::Quiet {
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
    Ok(())
}

pub fn inject_deps(
    bin_name: &str,
    deps: &[data::DepRequest],
    features: &[data::FeatureRequest],
    cfg: &config::Config,
    cargo_add_args: &CargoAddArgs,
) -> crate::Result<()> {
    if deps.is_empty() && features.is_empty() {
        return Ok(());
    }

    // first: add the dependencies
    // PROBABLY: also check whether deps already in manifest?
    if !deps.is_empty() {
        // NOTE that cargo automatically adds a feature
        //  for every optional dependency, however this is not
        //  actually necessary - the dep name can be listed
        //  directly as a "required-feature" (without even
        //  the `dep:` qualifier, for whatever reason)
        let mut cmd = process::Command::new(&cfg.cargo_exe);
        cmd.current_dir(&cfg.manifest_dir)
            .args(["add", "--optional"])
            .args(cargo_add_args.cli_args())
            .args(deps.iter().map(|d| &d.input_string));
        util::show_invocation(&cmd);

        let cargo_add_result = util::run_subproc(cmd)?;
        if !cargo_add_result.success() {
            return Err(crate::Error::CargoFail(format!(
                "`cargo add` command reported failure (status:  {cargo_add_result})"
            )));
        }
    }

    // read the new (possibly modified) manifest
    let mut cargo_doc = CargoDotToml::read(&cfg.cargo_dot_toml)?;

    // mutate it (in memory)
    cargo_doc.activate_features(bin_name, deps, features)?;

    // update the file on disk
    _update_and_show_diff(&cargo_doc, &cfg.cargo_dot_toml)?;

    Ok(())
}

// ───── Helpers ────────────────────────────────────────────────── //
fn _bin_name_to_src_filename(bin_name: &str) -> String {
    format!("src/{}.rs", bin_name.replace('-', "_"))
}

fn _update_and_show_diff(toml: &CargoDotToml, target: &Path) -> crate::errors::Result<()> {
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
