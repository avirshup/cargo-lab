use crate::cli::CargoAddArgs;
use crate::manifest_editor::CargoDotToml;
use crate::{config, data};
use color_print::cprintln;
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
pub fn run_script(bin_name: &str, args: &[String], paths: &config::Config) -> crate::Result<()> {
    let cargo_doc = CargoDotToml::read(&paths.cargo_dot_toml)?;
    let script = cargo_doc
        .get_script(bin_name)
        .ok_or_else(|| crate::Error::ScriptNotFound(bin_name.to_owned()))?;

    let mut cmd = process::Command::new(&paths.cargo_exe);
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
    paths: &config::Config,
) -> crate::Result<()> {
    let mut cargo_doc = CargoDotToml::read(&paths.cargo_dot_toml)?;

    // copy the template to the destination if necessary
    let src_filename = _bin_name_to_src_filename(bin_name);
    let dest = paths.manifest_dir.join(&src_filename);
    if dest.is_file() {
        cprintln!(
            "<yellow>warning</yellow>: Script '{}' already exists",
            paths.humanize(&dest)
        );
    } else {
        let template_path = paths.template_path(template_name);
        _copy_file(&template_path, &dest)?;
        cprintln!(
            "<green>success</green>: Created script: {} -> {}",
            paths.humanize(&template_path),
            paths.humanize(&dest)
        );
    }

    // update Cargo.toml
    cargo_doc.add_new_bin(bin_name, &src_filename)?;
    _update_and_show_diff(&cargo_doc, &paths.cargo_dot_toml)?;

    Ok(())
}

pub fn list_scripts(cfg: &config::Config) -> crate::Result<()> {
    let cargo_doc = CargoDotToml::read(&cfg.cargo_dot_toml)?;

    let Some(script_iter) = cargo_doc.list_scripts() else {
        if cfg.verbosity > config::Quiet {
            cprintln!(
                "<yellow>warning:</yellow> No [[bin]] entries found in {}",
                cfg.cargo_dot_toml.display()
            );
        }
        return Ok(());
    };

    if cfg.verbosity > config::Quiet {
        println!("Scripts in {}: \n", cfg.cargo_dot_toml.display());
    }

    for data::ScriptEntry {
        name,
        path,
        required_features,
    } in script_iter
    {
        if cfg.verbosity == config::Quiet {
            println!("{name}");
        } else {
            println!("{name}:\n  path: {path}\n  dependencies: {required_features:?}\n");
        }
    }
    Ok(())
}

pub fn inject_deps(
    bin_name: &str,
    deps: &[data::DepRequest],
    features: &[data::FeatureRequest],
    paths: &config::Config,
    cargo_add_args: &CargoAddArgs,
) -> crate::Result<()> {
    // PROBABLY: check whether deps already in manifest before running cargo add?

    // first: add the dependencies
    if !deps.is_empty() {
        // NOTE that cargo automatically adds a feature
        //  for every optional dependency, however this is not
        //  actually necessary - the dep name can be listed
        //  directly as a "required-feature" (without even
        //  the `dep:` qualifier, for whatever reason)
        let cargo_add_result = _run_cmd(
            process::Command::new(&paths.cargo_exe)
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

    // mutate it (in memory)
    cargo_doc.activate_features(bin_name, deps, features)?;

    // update the file on disk
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
    // Attempt to print the command before running it.
    // if any of these strings can't be represented in unicode,
    // this will panic (probably good tbh)
    let cmd_str = cmd.get_program().to_str().unwrap().to_owned();
    // TODO: probably this can just be the filename ...

    let mut arg_display = String::new();
    for arg in cmd.get_args().map(|s| s.to_str().unwrap()) {
        // This is a very, VERY bad "shell" "quoting" "algorithm".
        // But it's for display only, seems silly to
        // to bring in a whole-ass dependency for it.
        let wrap_in_quotes = arg.contains(' ');
        arg_display.push(' ');
        if wrap_in_quotes {
            arg_display.push('\'');
        }
        arg_display.push_str(arg);
        if wrap_in_quotes {
            arg_display.push('\'');
        }
    }
    cprintln!(
        "<cyan>running:</cyan>\n<blue>$</blue> <green>{}</green>{}",
        cmd_str,
        arg_display
    );
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

    // TODO: better output if cargo.toml didn't change
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
