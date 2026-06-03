use std::env;

use camino::{Utf8Path, Utf8PathBuf};
use clap::CommandFactory;

use crate::cli;

pub(super) const SUBCMD_COMPLETE_VAR: &str = "COMPLETE_CARGO_LAB_SUBCMD";
pub(super) const DIRECT_COMPLETE_VAR: &str = "COMPLETE_CARGO_LAB_DIRECT";

/// Was this invoked with an argv of `["cargo-$X", "$X", ..args]`
/// or directly (with this exe as arg0?)
/// This struct (and its methods) helps to smooth over all the differences
/// between the two situations.
///
/// ## Notes
/// This does NOT detect usage as an xtask - xtasks
/// are just aliases for `cargo run`, and `cargo run` will
/// calls the executable directly (i.e., actually invoked executable
/// will be something like "./target/debug/cargo-lab")
#[derive(Debug)]
pub(super) enum InvocationType {
    CargoSubcmd {
        cargo_exe: Utf8PathBuf,
        cargo_subcmd: String,
        this_exe: Utf8PathBuf,
    },
    Direct(Utf8PathBuf),
}

impl InvocationType {
    pub(super) fn from_env() -> Self {
        let mut arg_iter = env::args();

        // figure out bin name
        let arg0 = arg_iter.next().expect("non-empty argv");
        let exe_path = Utf8Path::new(&arg0);

        // was it invoked as "cargo-subcmd subcmd [...]" or nah?
        // This is somewhat heuristic - there is not a way to know for sure
        // AFAICT, so it might have false positives
        // in certain (I think very rare) circumstances
        if let Ok(cargo_exe) = env::var("CARGO")
            && let Some(arg1) = arg_iter.next()
            && let Some(cmd_suffix) = crate::util::cargo_cmd_suffix(exe_path)
            && arg1 == cmd_suffix
        {
            InvocationType::CargoSubcmd {
                cargo_exe: Utf8PathBuf::from(cargo_exe),
                cargo_subcmd: arg1,
                this_exe: exe_path.to_owned(),
            }
        } else {
            InvocationType::Direct(exe_path.to_owned())
        }
    }

    // ───── Queries ────────────────────────────────────────────────── //
    /// Returns this executable's path
    pub(super) fn this_exe(&self) -> &Utf8Path {
        match &self {
            InvocationType::CargoSubcmd { this_exe, .. } => this_exe,
            InvocationType::Direct(its_me) => its_me,
        }
    }

    /// returns the exe that the user invoked (either this binary, or cargo itself)
    pub(super) fn invoked_exe(&self) -> &Utf8Path {
        match &self {
            InvocationType::CargoSubcmd { cargo_exe, .. } => cargo_exe,
            InvocationType::Direct(exe) => exe,
        }
    }

    /// returns the _command_ that the user invoked
    pub(super) fn invoked_cmd(&self) -> &str {
        self.invoked_exe().file_name().expect("exe has a filename")
    }

    /// env var to use the autocomplete system
    pub(super) fn completion_env_var(&self) -> &'static str {
        match self {
            InvocationType::CargoSubcmd { .. } => SUBCMD_COMPLETE_VAR,
            InvocationType::Direct(_) => DIRECT_COMPLETE_VAR,
        }
    }

    // ───── Argument handling ──────────────────────────────────────── //
    /// argv to in a form parsable by our [`cli::MainCli`] parser
    pub(super) fn normalized_argv(&self) -> Vec<String> {
        let mut args: Vec<String> = env::args().collect();
        if let InvocationType::CargoSubcmd { .. } = self {
            args.remove(1);
        }
        args
    }

    /// Build a runtime CLI parser. Note that this MUST
    /// be used with `parse_from(normalized_argv)` (TODO: perhaps
    /// this can just be exposed as `Invocation::parse_args`)?
    ///
    /// Note this CANNOT BE USED in autocomplete mode.
    pub(super) fn build_cli_cmd(&self) -> clap::Command {
        let cmd = cli::MainCli::command();

        if let InvocationType::CargoSubcmd {
            cargo_exe,
            cargo_subcmd,
            ..
        } = self
        {
            let name = format!(
                "{} {}",
                cargo_exe.file_name().expect("non-empty filename"),
                cargo_subcmd,
            );

            cmd.bin_name(&name)
        } else {
            cmd
        }
    }
}
