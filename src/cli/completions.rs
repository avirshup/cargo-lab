use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::{env, io};

use clap_complete::env::Shells;
use clap_complete::{
    ArgValueCandidates, ArgValueCompleter, CompletionCandidate, PathCompleter,
};

use super::cargo_subcmd_shim::build_cli_command;
use crate::global_ctx::{GlobalCtx, Verbosity};

const COMPLETE_VAR: &str = "COMPLETE";

// ──────────────────────────────────────────────────────────────────────── //
// ───── Dynamic completers                                          ───── //
// ──────────────────────────────────────────────────────────────────────── //

// Callbacks to generate dynamic autocomplete options.
// These get invoked through the [`maybe_exec_dynamic_automcomplete`] entrypoint,
// and shouldn't emit any STDOUT (anything written to stdout becomes
// an autocomplete option), and STDERR only in pathological siutations (as autocomplete
// should generally not have user-visible side effects)
// TODO: set up proper logging so this can be debugged when it all goes wrong

pub fn manifest_path_completer() -> ArgValueCompleter {
    ArgValueCompleter::new(PathCompleter::any().filter(|path: &Path| {
        path.file_name() == Some(OsStr::new("Cargo.toml"))
    }))
}

pub fn script_name_completer() -> ArgValueCandidates {
    ArgValueCandidates::new(|| {
        let ctx = _completion_ctx();
        if let Ok(manifest) = ctx.manifest_data() {
            manifest
                .iter_script_entries()
                .map(|script| {
                    CompletionCandidate::new(script.name)
                        .help(Some(script.path.to_string().into()))
                    // TODO: add .display_order() based on most recenty used?
                    //     Or based on creation order (would be reverse of order
                    //    from cargo.toml if it hasn't been modified)?
                })
                .collect()
        } else {
            vec![]
        }
    })
}

pub fn template_name_completer() -> ArgValueCandidates {
    ArgValueCandidates::new(|| {
        let ctx = _completion_ctx();

        if let Ok(paths) = ctx.project_paths()
            && let Ok(template_iter) = paths.iter_templates()
        {
            template_iter
                .map(|template| CompletionCandidate::new(template.name))
                .collect()
        } else {
            vec![]
        }
    })
}

/// Build a context for dynamic autocomplete
fn _completion_ctx() -> GlobalCtx {
    // if manifest path already specified in CLI args, use it
    // but only if it actually exists

    GlobalCtx::new(
        Verbosity::NearlySilent,
        _maybe_get_manifest_path_from_argv(),
    )
}

// this must match `cli::parser::GlobalArgs`
const MANIFEST_PATH_FLAG: &str = "--manifest-path";

/// Get the value of the `--manifest-path` option if it
/// was specified on the CLI
/// shouldn't complain upon any errors.
///
/// This just checks the CLI arguments directly rather than using clap.
/// Although I suppose it makes some strong assumptions about the CLI args
/// are formatted that may not be true in, like, powershell or something?
///
fn _maybe_get_manifest_path_from_argv() -> Option<String> {
    let mut arg_iter = env::args_os();

    while let Some(osarg) = arg_iter.next() {
        let arg = osarg.to_string_lossy();

        if let Some(rest) = arg.strip_prefix(MANIFEST_PATH_FLAG) {
            return if rest.is_empty() {
                // "--manifest-path [path]"
                arg_iter.next().map(|osstr| osstr.to_string_lossy().into())
            } else if let Some(val) = rest.strip_prefix('=')
                && !val.is_empty()
            {
                // "--manifest-path [path]"
                Some(val.into())
            } else {
                // of the form "--manifest-path[unexpected characters]"
                None
            };
        }
    }

    None
}

//
// #[derive(Parser, Debug)]
// struct JustGlobalArgs {
//     #[command(flatten)]
//     pub global_args: GlobalArgs,
// }
//
// /// This _attempts_ to parse a few options from argv that
// /// are relevant to autocomplete (e.g., `--manifest-path`), but
// /// shouldn't complain upon any errors.
// ///
// /// (Alternatively, if all we need is `--manifest-path`, maybe just
// /// iterate through `argv` ourselves? Probably much faster, but
// /// would need to stay in sync w/ the main parser)
// fn _maybe_get_manifest_path_from_argv() -> Option<PathBuf> {
//     JustGlobalArgs::command()
//         .ignore_errors(true)
//         .try_get_matches()
//         .ok()
//         .and_then(|matches| {
//             matches
//                 .try_get_one::<&String>("manifest_path")
//                 .unwrap_or(None)
//                 .cloned()
//         })
//         .map(PathBuf::from)
// }

// ───── The entrypoint ─────────────────────────────────────────── //
/// "HEY! I'M ~~WALKIN~~ *USING UNSTABLE CLAP_COMPLETE FEATURES* HERE!"
///
/// To use dynamic autocomplete with clap_complete's "unstable_dynamic" feature,
/// this should probably be the very first thing called from main.
/// Things will go very badly for you if there's any STDOUT
/// before this gets called.
///
/// If the `COMPLETE={shell}` env var is *not* set, this is a no-op.
/// Othewrise it acts as the entrypoint for "autocomplete" mode
/// and it will terminate this process upon completion (i.e., the function won't return).
///
/// If the `COMPLETE` env var is set:
/// - if the program is called without any arguments, it outputs the shell
///   completion script. This is a completely _different_ script
///   than the one generated by `clap_complete::generate`, which does not include
///   any support for the dynamic options.
/// - The above dynamic completion script includes instructions for the shell
///   to callback with various arguments in order to generate argument lists.
pub fn maybe_exec_dynamic_automcomplete() {
    clap_complete::CompleteEnv::with_factory(build_cli_command)
        .var(COMPLETE_VAR)
        .complete();
}

/// Print the completion script for the requested shell to stdout
///
/// # Notes
/// This is sort of a pain and explicitly uses unstable APIs
/// (since the entire feature is called "unstable_dynamic").
///
/// ## Easier alternatives?
/// Could also just spawn `COMPLETE=shellname cargo playground` ...
/// but that's a fork bomb if something goes wrong. Could do it
/// as `exec` which upon $bug would be "just" be a process
/// exec'ing itself forever (and doesn't work on all platforms anyway).
///
/// Could we just set `COMPLETE=shell` for _this process_ (which
/// is unsafe but fine here I think) and call `CompleteEnv.complete()`
/// to trick it into thinking we're in autocomplete mode ... oh and
/// also modify argv ... yeah no.
pub fn print_completion_script(
    shell_name: &str,
    mut dest: impl io::Write,
) -> crate::Result<()> {
    // FIXME: does this work as a cargo subcommand? Check

    let shells = Shells::builtins();
    let completer = shells
        .completer(shell_name)
        .ok_or_else(|| crate::Error::UnknownShell(shell_name.to_owned()))?;

    let cli_cmd = build_cli_command();

    // these are all _nearly_ but not quite the same thing I think?
    let my_name = cli_cmd.get_name();
    let my_bin_name = cli_cmd.get_bin_name().unwrap_or(my_name);
    let my_cmd_or_exe_path = _get_my_cmd_or_exe_abspath();

    completer
        .write_registration(
            COMPLETE_VAR,
            my_name,
            my_bin_name,
            &my_cmd_or_exe_path,
            &mut dest,
        )
        .map_err(|ioerr| {
            crate::ioerr!(
                ioerr,
                "Failed to write completion script for shell '{shell_name}'"
            )
        })
}

/// Get the `completer: &str` argument, attempting to do it the same way that
/// [`clap_complete::env::CompleteEnv::write_registration`]  does it.
///
/// This will panic if the path can't be represented as a string, CWD
/// can't be determined, or `argv[0]` is a path can't be canonicalized.
///
/// HOWEVER, that function's flow is completely unreadable to me
/// (there are 4 different things all named `completer`, and none of them
/// actually complete anything?) ...
/// [might be a (me) skill issue?](https://github.com/clap-rs/clap/blob/c774a892ba8bb703a9e77a16e6ebc6ff1c551868/clap_complete/src/env/mod.rs#L292-L302)
///
/// The flow seems to be:
/// 1. If the "completer" was manually set when constructing the `CompleteEnv`,
///    it uses that.
/// 2. Otherwise, the caller ([`CompleteEnv::try_complete_`])
///    sets `completer` argument to `argv[0]` (and then removes
///    it, along with everything else up to the first "--" in argv)
/// 3. If the completer has *more* than one component (i.e., it's not in
///    the path then it is joined to the current dir (which I think is a no-op
///    if the completer is already an absolute path)
fn _get_my_cmd_or_exe_abspath() -> String {
    let my_arg0 = PathBuf::from(env::args().next().unwrap());

    let my_cmd = if my_arg0.is_absolute() || my_arg0.components().count() == 1 {
        my_arg0
    } else {
        my_arg0.canonicalize().unwrap()
    };

    my_cmd.to_str().unwrap().to_owned()
}

// /// Print the completion script for the requested shell to stdout
// /// BUT: this script only works for *static* completions, the dynamic
// /// completions are separate and possibly not even compatible with this
// pub fn write_static_completion_script<Stream>(
//     shell: Shell,
//     mut dest: Stream,
// ) -> crate::Result<()>
// where
//     Stream: io::Write,
// {
//     // the completions we generate depend upon exactly how this is invoked
//     let mut cmd = build_cli_command();
//
//     let cmd_name = cmd.get_name().to_owned();
//     generate(shell, &mut cmd, cmd_name, &mut dest);
//
//     Ok(())
// }
