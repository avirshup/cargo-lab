use color_print::ceprintln;
use std::path::Path;
use std::{fs, process};

pub fn copy_file(src: &Path, dest: &Path) -> crate::Result<()> {
    fs::copy(src, dest).map_err(|e| crate::Error::CopyFailed {
        src: src.display().to_string(),
        dest: dest.display().to_string(),
        err: e,
    })?;

    Ok(())
}

/// A rough attempt to print the subprocess invocations we're about to run.
/// TODO: this should be controlled via verbosity level
pub fn show_invocation(cmd: &process::Command) {
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
    ceprintln!(
        "<cyan>running:</>\n<blue>$</> <green>{}</>{}",
        cmd_str,
        arg_display
    );
}
