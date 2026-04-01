use crate::{cli, util};
use clap::CommandFactory;
use clap_complete::{Generator, Shell, generate};
use std::ffi::OsString;
use std::io::Write;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::{fs, io, process};
// FIXME: use the right error types here, not just InputError for everything

/// Given a shell, print its completions to stdout
pub fn generate_completions(shell: Shell, install: bool) -> crate::Result<()> {
    let mut cmd = cli::PlaygroundCli::command();
    let cmd_name = cmd.get_name().to_owned();

    let mut writer: Box<dyn Write> = if install {
        let path = _shell_autocomplete_target_path(&shell)?;
        eprintln!("writing completions to {}", path.display());
        Box::new(fs::OpenOptions::new().write(true).open(path)?)
    } else {
        Box::new(io::stdout())
    };

    generate(shell, &mut cmd, cmd_name, &mut writer);

    Ok(())
}

/// Try to discover the path that we should write the completions script to.
///
/// Implementing this for fish was *relatively* straightforward, but not sure any
/// other shells work this way (where you install autocompletion by writing a script
/// to a config dir then you're done).
fn _shell_autocomplete_target_path(shell: &Shell) -> crate::Result<PathBuf> {
    match shell {
        // TODO: error message should instruct user to pipe results to desired file
        Shell::Fish => _get_fish_autocomplete_path().map_err(|_| crate::Error::AutocompleteFail {
            shell: shell.to_string(),
            reason: "directory discovery failed",
            guidance: "Please manually redirect the results of this command\
to a file in your fish autocompletion directory (typically `~/.config/fish/completions`)",
        }),
        _other => Err(crate::Error::AutocompleteFail {
            shell: shell.to_string(),
            reason: "shell not yet supported.",
            guidance: "Please see [the docs i guess]",
        }),
    }
}

/// Right this doesn't bother with a helpful error - either we could find the
/// right directory or we couldn't.
fn _get_fish_autocomplete_path() -> Result<PathBuf, ()> {
    // TODO: not this
    let mut cmd = process::Command::new("pkg-config");
    cmd.args(["--variable", "completionsdir", "fish"]);
    util::show_invocation(&cmd);
    let pkg_conf_result = cmd.output().map_err(|_| ())?;

    let mut autocomplete_path = if pkg_conf_result.status.success() {
        // TODO: figure out how to make this happy (stdout is a raw `Vec<u8>` here)
        // TODO: this is unix-only (but it's not like fish runs on windows anyway)
        PathBuf::from(OsString::from_vec(pkg_conf_result.stdout))
    } else {
        // fallback - this will probably work like 99% of the time
        let mut dir = std::env::home_dir().ok_or(())?;
        dir.extend([".config", "fish", "completions"]);
        dir
    };

    // note that this _will_ deref symlinks, which is good
    if autocomplete_path.is_dir() {
        Err(())
    } else {
        autocomplete_path.push("cargo-playground.fish");
        Ok(autocomplete_path)
    }
}
