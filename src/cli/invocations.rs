use std::env;

use camino::{Utf8Path, Utf8PathBuf};
use clap::CommandFactory;

use crate::cli;

pub(super) const SUBCMD_COMPLETE_VAR: &str = "COMPLETE_CARGO_PG_SUBCMD";
pub(super) const DIRECT_COMPLETE_VAR: &str = "COMPLETE_CARGO_PG_DIRECT";

/// Was this invoked with an argv of `["cargo-$X", "$X", ..args]`
/// or directly (with this exe as arg0?)
///
/// Note that this does NOT detect usage as an xtask - xtasks
/// are just aliases for `cargo run`, and `cargo run` will
/// calls the executable directly (i.e., actually invoked executable
/// will be something like "./target/debug/cargo-playground")
#[derive(Debug)]
pub(super) enum InvocationType {
    CargoSubcmd {
        cargo_exe: Utf8PathBuf,
        cargo_subcmd: String,
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
                cargo_exe: cargo_exe.into(),
                cargo_subcmd: arg1,
            }
        } else {
            InvocationType::Direct(exe_path.to_owned())
        }
    }

    pub(super) fn env_var_name(&self) -> &'static str {
        match self {
            InvocationType::CargoSubcmd { .. } => SUBCMD_COMPLETE_VAR,
            InvocationType::Direct(_) => DIRECT_COMPLETE_VAR,
        }
    }

    pub(super) fn normalized_argv(&self) -> Vec<String> {
        let mut args: Vec<String> = env::args().collect();
        if let InvocationType::CargoSubcmd {
            cargo_exe: _,
            cargo_subcmd: _,
        } = self
        {
            args.remove(1);
        }
        args
    }

    /// Build a runtime CLI parser. Note that this MUST
    /// be used with `parse_from(normalized_argv)` (TODO: perhaps
    /// this can just be exposed as `Invocatin::parse_args`)?
    ///
    /// Note this CANNOT BE USED in autocomplete mode.
    pub(super) fn build_cli_cmd(&self) -> clap::Command {
        let cmd = cli::MainCli::command();

        if let InvocationType::CargoSubcmd {
            cargo_exe,
            cargo_subcmd,
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
