//! "HEY! I'M ~~WALKIN~~ *USING UNSTABLE CLAP_COMPLETE FEATURES* HERE!"
use std::env;
use std::ffi::OsStr;
use std::path::Path;

use clap_complete::{
    ArgValueCandidates, ArgValueCompleter, CompletionCandidate, PathCompleter,
};

use crate::global_ctx::{GlobalCtx, Verbosity};

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
