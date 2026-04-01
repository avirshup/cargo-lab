use std::ffi::OsString;
use std::fmt::Write;

use clap::Parser;

use super::cargo_subcmd_shim::InvocationType;
use crate::cli;

/// Extra handling before letting clap parse the CLI args.
///
/// On POSIX, when you run `cargo playground [..args]` (or similar)
/// cargo will exec `["cargo-playground", "playground", ..args].
/// This isn't how this program excects to receive its input!
///
/// So, _if_ it is called like this: `["cargo-subcmdname", "subcmdname" ..args]`,
/// we send `clap` a modified argv: `["cargo subcmdname", ..args]`.
///
/// This is loosely based on how "cargo tauri" does it:
/// https://github.com/tauri-apps/tauri/blob/36eee37/crates/tauri-cli/src/main.rs
pub fn parse_argv() -> cli::MainCli {
    let mut arg_vec: Vec<OsString> = std::env::args_os().collect();

    match InvocationType::from_env() {
        InvocationType::Direct(_) => cli::MainCli::parse(),

        InvocationType::CargoSubcmd(_cargo_path, subcmd) => {
            // lie to clap, tell it that arg0 is the string "cargo [subcmd]"
            // MAYBE: are we supposed to do this with ".bin_name" instead?
            let clap_argv = &mut arg_vec[1..];
            let clap_arg0 = clap_argv.first_mut().unwrap();
            clap_arg0.clear();
            clap_arg0.write_str("cargo ").unwrap();
            clap_arg0.push(&subcmd);

            cli::MainCli::parse_from(&*clap_argv)
        },
    }
}
