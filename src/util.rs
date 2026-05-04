use std::path::Path;
use std::{fs, process};

use camino::Utf8Path;
use color_print::ceprintln;

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

/// Why not built-in?
pub fn join_str_iter<'a, 'b>(
    mut iter: impl Iterator<Item = &'a str>,
    sep: &'b str,
) -> String {
    let Some(mut result) = iter.next().map(str::to_owned) else {
        return String::new();
    };

    for item in iter {
        result.push_str(sep);
        result.push_str(item);
    }

    result
}

// ───── String rectification ───────────────────────────────────── //

/// `cmd_suffix("/path/to/cargo-thing.exe")` produces `Some("thing")`
///
/// Q: what if it's called "cargo-playground.old.exe" huh? what then hotshot?
/// A: In this case, cargo won't run it as a subocmmand
/// (`cargo playground.old` doesn't work), so we don't have to handle
/// this case.
pub fn cargo_cmd_suffix(exe_path: &Utf8Path) -> Option<&str> {
    exe_path
        .file_stem()
        .and_then(|stem| stem.strip_prefix("cargo-"))
}

/// Canonicalize a name for matching purposes
/// (i.e., 2 names "match" if they both canonicalize to the same string)
pub fn canonicalize_crate_name(s: &str) -> String {
    s.to_lowercase().replace('-', "_")
}
//
// /// Ensure that a requested filename for a script is of the form
// /// "src/[filename].rs"
// ///
// /// It will correct for the following input issues:
// ///  - if ".rs" is not present it will add it
// ///  - the leading "src/" will be added if not present
// pub fn normalize_script_path(
//     input_path: &Utf8Path,
// ) -> crate::Result<Utf8PathBuf> {
//     // extract the filename from paths of the form "src/filename", if necessary
//     // TODO: this function has hardcoded "src" thus it is business logic, not a util
//     let filename: &str = {
//         let mut path_part_iter = input_path.components().map(|p| p.as_str());
//         match [
//             path_part_iter.next(),
//             path_part_iter.next(),
//             path_part_iter.next(),
//         ] {
//             [Some("src"), Some(fname), None] => Ok(fname),
//             [Some(fname), None, None] => Ok(fname),
//             _otherwise =>
//                 Err(crate::Error::InvalidScriptFilename(input_path.to_owned())),
//         }
//     }?;
//
//     // strip trailing ".rs" if present
//     let stem = filename.strip_suffix(".rs").unwrap_or(filename);
//
//     if !stem
//         .chars()
//         .all(|c| c.is_ascii_digit() || c.is_ascii_alphabetic() || c == '_')
//     {
//         return Err(crate::Error::InvalidScriptFilename(input_path.to_owned()));
//     }
//
//     Ok(Utf8Path::new("src").join(stem).with_extension("rs"))
// }

// ───── Wrappers around stdlib with our own error handling ─────── //
// we can't just use a From trait for these because afaict io::Error doesn't provide
// enough context to build a good error message (like, e.g., the name of the file)

/// rename a file, overwriting destination if it exists.
///
/// nb, there does not seem to a general purpose API that will prevent
/// overwriting atomically by failing if the destination exists, as it's
/// not available even on all posix systems.
pub fn rename_file(src: &Utf8Path, dest: &Utf8Path) -> crate::Result<()> {
    fs::rename(src, dest).map_err(|ioerr| {
        crate::ioerr!(ioerr, "Failed to rename '{src}' to '{dest}'")
    })
}

pub fn write_file(dest: &Utf8Path, content: &str) -> crate::Result<()> {
    fs::write(dest, content)
        .map_err(|ioerr| crate::ioerr!(ioerr, "Failed to write to {dest}"))
}

pub fn read_file(src: &Utf8Path) -> crate::Result<String> {
    fs::read_to_string(src)
        .map_err(|ioerr| crate::ioerr!(ioerr, "Failed to read '{src}'"))
}

pub fn copy_file(src: &Utf8Path, dest: &Utf8Path) -> crate::Result<()> {
    fs::copy(src, dest).map_err(|e| crate::Error::CopyFailed {
        src: src.to_owned(),
        dest: dest.to_owned(),
        err: e.to_string(),
    })?;

    Ok(())
}

pub fn run_subproc(
    mut cmd: process::Command,
) -> crate::Result<process::ExitStatus> {
    let mut child = cmd.spawn().map_err(|ioerr| {
        crate::ioerr!(ioerr, "Failed to spawn process '{cmd:?}'")
    })?;

    child.wait().map_err(|ioerr| {
        crate::ioerr!(ioerr, "Wait failed for '{cmd:?}' (pid={})", child.id())
    })
}

pub fn create_dir(path: &Utf8Path) -> crate::Result<()> {
    fs::create_dir(path).map_err(|ioerr| {
        crate::ioerr!(ioerr, "Failed to create directory at '{path}'",)
    })?;
    Ok(())
}
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     fn test_normalize_script_path() {
//         for (input, expected) in [
//             ("hi", "src/hi.rs"),
//             ("bye.rs", "src/bye.rs"),
//             ("src/sigh", "src/sigh.rs"),
//             ("src/die.rs", "src/die.rs"),
//         ] {
//             assert_eq!(
//                 normalize_script_path(input.into()).unwrap(),
//                 expected.to_owned()
//             );
//         }
//     }
// }
