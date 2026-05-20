use std::io;

use camino::{Utf8Path, Utf8PathBuf};
use clap_complete::env::Shells;

use super::InvocationType;
use crate::cli::invocations::InvocationType::{CargoSubcmd, Direct};

/// Print the completion script for the requested shell to stdout
///
/// Clap's autocomplete scripts dispatch based on arg0 only (and
/// in general this might be how shell autocompletion works too?)
/// so we need to need to generate autocompletions for the `cargo`
/// command itself when being invoked as `cargo playground`.
///
/// # Notes
///
/// This doesn't need to know anything about our CLI itself.
///
/// All it needs to know is:
/// A) what is the command we are generating completions for?
/// B) what executable needs to be called to generate those completions?
pub fn print_completion_script(
    shell_name: &str,
    mut dest: impl io::Write,
) -> crate::Result<()> {
    // ───── Part 1: gather all the different parameters we need ─────
    let invocation = InvocationType::from_env();

    // path to this specific executable, which
    // will _generate_ the completions
    let this_exe = _cmd_or_canonical_path(invocation.this_exe());
    let completion_generator_cmd = this_exe.as_str();

    // the command to generate completions _for_
    // this _might_ be cargo itself, or it might be this binary.
    let cmd = invocation.invoked_cmd();

    // unique prefix for functions in the generated script
    // to prevent collisions with other scripts
    let name = match &invocation {
        CargoSubcmd { cargo_subcmd, .. } => {
            format!("{cmd}_pgsubcmd_{cargo_subcmd}")
        },
        Direct(_) => cmd.to_owned(),
    };

    // ───── Part 2: build completer, add customizations, and print─────
    let builtins = Shells::builtins();
    let completer = builtins
        .completer(shell_name)
        .ok_or_else(|| crate::Error::UnknownShell(shell_name.to_owned()))?;

    // this is an IIFE so I can use the try operator
    || -> io::Result<()> {
        // have clap build the default completion script for us
        let completion_script = {
            let mut buffer: Vec<u8> = Vec::new();
            completer.write_registration(
                invocation.completion_env_var(),
                &name,
                cmd,
                completion_generator_cmd,
                &mut buffer,
            )?;
            String::from_utf8(buffer).expect("completion script is text")
        };

        // potentially customize the completion script and write the result to stdout
        match (completer.name(), &invocation) {
            ("fish", CargoSubcmd { .. }) => _write_fish_cargosubcmd_script(
                cmd,
                &completion_script,
                &mut dest,
            ),

            ("bash", CargoSubcmd { .. }) => _write_bash_autocomplete_script(
                &name,
                cmd,
                &completion_script,
                &mut dest,
            ),

            // TODO: a zsh

            // all other
            _everything_else => completer.write_registration(
                invocation.completion_env_var(),
                &name,
                cmd,
                completion_generator_cmd,
                &mut dest,
            ),
        }
    }()
    .map_err(|ioerr| {
        crate::ioerr!(
            ioerr,
            "Failed to write completion script for '{shell_name}'"
        )
    })
}

/// Determine if `path` is a command on the $PATH or a filesystem path
/// and canonicalize it if it's a filesystem path.
///
/// Panics if given a non-utf-8 path  and empty strings.
fn _cmd_or_canonical_path(path: &Utf8Path) -> Utf8PathBuf {
    match path.components().count() {
        0 => panic!("argv[0] is empty???"),
        1 => path.to_owned(),
        _2_or_more => path.canonicalize_utf8().expect("Path is valid utf-8"),
    }
}

// ───── Script customization ───────────────────────────────────── //

/// For fish, add a call to ensure that cargo's autocompletion is also loaded
fn _write_fish_cargosubcmd_script(
    cmd: &str,
    clap_completion_script: &str,
    mut dest: impl io::Write,
) -> io::Result<()> {
    writeln!(
        &mut dest,
        "{}",
        minijinja::render!(
            include_str!("autocomplete_cargo_shim.fish"),
            cmd,
            clap_completion_script
        )
    )
}

/// for bash, we create a new autocomplete function that
/// merges the results from cargo's autocomplete function (if it exists)
/// and our own autocomplete function.
/// Hacky as all hell unfortunately as we're using a regex to inject
/// our extra function into clap's original script
fn _write_bash_autocomplete_script(
    name: &str,
    cmd: &str,
    clap_completion_script: &str,
    mut dest: impl io::Write,
) -> io::Result<()> {
    writeln!(
        &mut dest,
        "{}",
        minijinja::render!(
            include_str!("autocomplete_cargo_shim.bash"),
            cmd,
            name,
            clap_completion_script
        )
    )
}
