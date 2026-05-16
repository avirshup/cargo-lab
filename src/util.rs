use std::fmt::Write;
use std::path::Path;
use std::{fs, process};

use camino::Utf8Path;
use color_print::{ceprintln, cprint, cwrite};
use similar::ChangeTag;

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

// ───── Diffing ────────────────────────────────────────────────── //
/// Write a human-readable diff of 2 files to stdout
///
/// Basically https://github.com/mitsuhiko/similar/blob/main/examples/terminal-inline.rs
pub fn display_file_diff(old_content: &str, new_content: &str) {
    let diff = similar::TextDiff::from_lines(old_content, new_content);

    for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
        if idx > 0 {
            println!("{:-^1$}", "-", 80);
        }
        for op in group {
            for change in diff.iter_inline_changes(op) {
                // ───── Create the diff ─────
                // we'll build up the line in this buffer before writing it
                // (Note we can safely unwrap all the `write!`s to this,
                // writing to a buffer never returns an error)
                let mut diff_display = String::new();

                // format the actual diff for this line
                for (emphasized, value) in change.iter_strings_lossy() {
                    if emphasized {
                        cwrite!(&mut diff_display, "<underline>{value}</>")
                    } else {
                        write!(&mut diff_display, "{value}")
                    }
                    .unwrap();
                }
                if change.missing_newline() {
                    writeln!(&mut diff_display).unwrap();
                }

                // ───── Write to stdout ─────
                // write the line numbers for this line
                for lineno in [change.old_index(), change.new_index()] {
                    match lineno {
                        None => print!("{:<4}", ""),
                        Some(idx) => print!("{:<4}", idx + 1),
                    }
                }
                match change.tag() {
                    ChangeTag::Equal => cprint!("| <dim>{diff_display}</dim>"),
                    ChangeTag::Delete =>
                        cprint!("|<bold>-</><red>{diff_display}</>"),
                    ChangeTag::Insert =>
                        cprint!("|<bold>+</><green>{diff_display}</>"),
                }
            }
        }
    }
}
