//! Tools for determining whether this was invoked as a cargo subcommand
//! or not and then parsing argv accordingly
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use clap::CommandFactory;

use crate::cli;

/// Build a CLI parser appropriate for whether or not
/// we are running as a cargo subcommand or not
///
pub(super) fn build_cli_command() -> clap::Command {
    match InvocationType::from_env() {
        InvocationType::CargoSubcmd(_cargo_path, subcmd_name) => {
            clap::Command::new("cargo").subcommand(
                cli::MainCli::command() //
                    .name(
                        subcmd_name //
                            .into_string()
                            .unwrap(),
                    ),
            )
        },
        InvocationType::Direct(bin_path) => {
            cli::MainCli::command() //
                .name(
                    bin_path
                        .file_name()
                        .unwrap()
                        .to_owned()
                        .into_string()
                        .unwrap(),
                )
        },
    }
}

/// Was this invoked with an argv of `["cargo-$X", "$X", ..args]`
/// or directly (with this exe as arg0?)
///
/// Note that this does NOT detect usage as an xtask - xtasks
/// are just aliases for `cargo run`, and `cargo run` will
/// calls the executable directly (i.e., actually invoked executable
/// will be something like "./target/debug/cargo-playground")
pub(super) enum InvocationType {
    CargoSubcmd(PathBuf, OsString),
    Direct(PathBuf),
}

impl InvocationType {
    pub(super) fn from_env() -> Self {
        let mut arg_iter = std::env::args_os();

        let arg0 = arg_iter.next().expect("Empty argv?");
        let bin_path = Path::new(&arg0).to_owned();
        let bin_name = bin_path.file_stem().and_then(OsStr::to_str);

        // was it invoked as "cargo-subcmd subcmd [...]" or nah?
        if let Some(arg1) = arg_iter.next()
            && let Some(cargo_subcmd) =
                bin_name.and_then(|s| s.strip_prefix("cargo-"))
            && arg1 == cargo_subcmd
        {
            InvocationType::CargoSubcmd(bin_path.to_owned(), arg1)
        } else {
            InvocationType::Direct(bin_path.to_owned())
        }
    }
}
