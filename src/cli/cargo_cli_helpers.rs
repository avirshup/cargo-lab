//! Tools for determining whether this was invoked as a cargo subcommand
//! or not and then parsing argv accordingly
use std::ffi::{OsStr, OsString};
use std::fmt::Write;
use std::path::{Path, PathBuf};

use clap::Parser;

use crate::cli;

/// Was this invoked with an argv of `["cargo-$X", "$X", ..args]`
/// or directly (with this exe as arg0?)
///
/// Note that this does NOT detect usage as an xtask - xtasks
/// are just aliases for `cargo run`, and `cargo run` will
/// calls the executable directly.
pub enum InvocationType {
    CargoSubcmd(PathBuf, OsString),
    Direct(PathBuf),
}

impl InvocationType {
    pub fn from_argv(argv: &[OsString]) -> Self {
        let mut arg_iter = argv.iter();

        let arg0 = arg_iter.next().expect("Empty argv?");
        let maybe_arg1 = arg_iter.next();

        let bin_path = Path::new(&arg0).to_owned();
        let bin_name = bin_path.file_stem().and_then(OsStr::to_str);

        // was it invoked as "cargo-subcmd subcmd [...]" or nah?
        if let Some(cargo_subcmd) =
            bin_name.and_then(|s| s.strip_prefix("cargo-"))
            && let Some(arg1) = maybe_arg1
            && arg1 == cargo_subcmd
        {
            InvocationType::CargoSubcmd(bin_path.to_owned(), arg1.clone())
        } else {
            InvocationType::Direct(bin_path.to_owned())
        }
    }
}

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
pub fn parse_argv() -> cli::PlaygroundCli {
    let mut arg_vec: Vec<OsString> = std::env::args_os().collect();

    let argv_slice: &[OsString] = match InvocationType::from_argv(&arg_vec) {
        InvocationType::Direct(_) => &arg_vec,

        InvocationType::CargoSubcmd(_cargo_path, subcmd) => {
            // lie to clap, tell it that arg0 is the string "cargo [subcmd]"
            // MAYBE: are we supposed to do this with ".bin_name" instead?
            let clap_argv = &mut arg_vec[1..];
            let clap_arg0 = clap_argv.first_mut().unwrap();
            clap_arg0.clear();
            clap_arg0.write_str("cargo ").unwrap();
            clap_arg0.push(&subcmd);

            clap_argv
        },
    };

    cli::PlaygroundCli::parse_from(argv_slice)
}
