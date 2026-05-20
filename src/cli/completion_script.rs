use std::io;

use camino::{Utf8Path, Utf8PathBuf};
use clap_complete::env::Shells;

use super::InvocationType;

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
    // ───── Part 1: gather all the different paramters we need ─────
    let invocation = InvocationType::from_env();

    // path to this specific executable, which
    // will _generate_ the completions
    let this_exe = _cmd_or_canonical_path(invocation.this_exe());
    let completion_generator_cmd = this_exe.as_str();

    // the command to generate completions _for_
    // this _might_ be cargo itself, or it might be this binary.
    let cmd = invocation.invoked_cmd();

    // an identifier for the command (rn just the same as the command)
    // (only used for namespacing in the generated shell script,
    // does not affect anything else AFAICT)
    let name = cmd;

    // ───── Part 2: build completer, add customizations, and print─────
    let builtins = Shells::builtins();
    let completer = builtins
        .completer(shell_name)
        .ok_or_else(|| crate::Error::UnknownShell(shell_name.to_owned()))?;

    // this is an IIFE so I can use the try operator
    || -> io::Result<()> {
        // additions to autocomplete scripts to handle use as a cargo subcmd
        if let InvocationType::CargoSubcmd { .. } = &invocation {
            // TODO: bash / zsh ... if it's even possible

            // FOR FISH: special pre-script to ensure that the normal cargo
            // completions get loaded too, if available.
            // Why? Because when _lazy-loading_ autocompletions, fish stops
            // looking after the first one it finds, so if it finds *our* "cargo.fish"
            // then it won't also load the built-in `cargo.fish`.
            if completer.name() == "fish" {
                writeln!(dest, include_str!("autocomplete_helper.fish"))?;
                writeln!(dest, "__load_lazy_completions \"{cmd}\"\n")?;
            }
        }

        // ───── Part 3: ask clap to please print out the script now ─────
        completer.write_registration(
            invocation.completion_env_var(),
            name,
            cmd,
            completion_generator_cmd,
            &mut dest,
        )
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
