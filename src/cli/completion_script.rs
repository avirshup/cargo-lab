use std::{env, io};

use camino::Utf8PathBuf;
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
    // ───── Part 1: figure out what paths to use in the script ─────
    let invocation = InvocationType::from_env();

    // path to this specific executable, which
    // will _generate_ the completions
    // here, this is always _this binary_ (not ever cargo itself)
    let arg0 = Utf8PathBuf::from(env::args().next().expect("argv[0]"));
    let this_exe = arg0
        .canonicalize_utf8()
        .expect("exe has well-defined utf-8 path");
    let completion_generator = this_exe.as_str();

    // an identifier for the command
    // (only used for namespacing in the generated shell script,
    // does not affect anything else AFAICT)
    let name = arg0.file_name().expect("arg0 is a file path");

    // the command to generate completions _for_
    // this _might_ be cargo itself, or it might be this binary.
    let cmd = match &invocation {
        InvocationType::CargoSubcmd { cargo_exe, .. } => cargo_exe,
        InvocationType::Direct() => &arg0,
    }
    .file_name()
    .expect("cmd has recognizeable filename");

    // ───── Part 2: ask clap to please print out the script now ─────
    Shells::builtins()
        .completer(shell_name)
        .ok_or_else(|| crate::Error::UnknownShell(shell_name.to_owned()))?
        .write_registration(
            invocation.env_var_name(),
            name,
            cmd,
            completion_generator,
            &mut dest,
        )
        .map_err(|ioerr| {
            crate::ioerr!(
                ioerr,
                "Failed to write completion script for shell '{shell_name}'"
            )
        })
}
