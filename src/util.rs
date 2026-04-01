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

pub fn run_subproc(mut cmd: process::Command) -> crate::Result<process::ExitStatus> {
    let mut child = cmd.spawn().map_err(|ioerr| {
        crate::Error::IoFail(format!("Failed to spawn process '{cmd:?}'"), ioerr)
    })?;

    child.wait().map_err(|ioerr| {
        crate::Error::IoFail(
            format!("Wait failed for '{cmd:?}' (pid={})", child.id()),
            ioerr,
        )
    })
}

/// A rough attempt to print the subprocess invocations we're about to run.
/// TODO: this should be controlled via verbosity level
pub fn show_invocation(cmd: &process::Command) {
    // Attempt to print the command before running it.
    // if any of these strings can't be represented in unicode,
    // this will panic (probably good tbh)
    let cmd_path: &Path = cmd.get_program().as_ref();
    let cmd_name = cmd_path
        .file_name()
        .unwrap_or(cmd.get_program())
        .to_str()
        .unwrap();

    let mut arg_display = String::new();
    for arg in cmd.get_args().map(|s| s.to_str().unwrap()) {
        // TODO: This is a very, VERY bad "shell" "quoting" "algorithm"
        //  that absolutely DOES NOT handle escaping correctly.
        //  BUT It is for display only.

        // Conservative (but probably wrong somehow)
        // list of characters that don't need escapes or quoting
        arg_display.push(' ');
        if arg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-+/=".contains(c))
        {
            arg_display.push_str(arg);
        } else {
            // TODO: this is wrong
            arg_display.push_str(&format!("'{}'", arg.replace('\'', "\\'")));
        }
    }
    ceprintln!(
        "<cyan>running:</>\n<blue>$</> <green>{}</>{}",
        cmd_name,
        arg_display
    );
}
