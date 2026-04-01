mod cli;
mod commands;
mod config;
mod data;
mod errors;
mod manifest_editor;
mod passthrough_arg_macro;

use all_the_errors::CollectAllTheErrors;

use crate::cli::{ArgParser, SubCmd};
use crate::errors::{Error, Result};
use clap::Parser;
use config::Config;

fn main() -> Result<()> {
    let args = ArgParser::parse();

    let verbosity = match (args.general.quiet, args.general.verbose) {
        (true, _) => config::Quiet,
        (false, 0) => config::Normal,
        (false, 1) => config::Verbose,
        (false, _more_than_1) => config::Debug,
    };
    let cfg = Config::from_env(verbosity);

    if cfg.verbosity >= config::Verbose {
        println!("{cfg:#?}");
    }

    match args.cmd {
        SubCmd::RunScript { bin_name, args } => {
            commands::run_script(&bin_name, &args, &cfg)?;
        }
        SubCmd::NewScript { bin_name, template } => {
            commands::new_script(&bin_name, &template, &cfg)?;
        }
        SubCmd::ListScripts {} => {
            commands::list_scripts(&cfg)?;
        }
        SubCmd::InjectDeps {
            bin_name,
            deps: input_deps,
            features: mut input_features,
            cargo_add_args,
        } => {
            // ───── Extra parsing for features ─────────────────────────────── //

            // insert implicit feature dependency qualifiers
            // i.e., `inject depname -F feature` => `inject depname -F depname/feature`
            // but only if there is exactly one dependency listed
            if input_deps.len() == 1 {
                let implicit_depname = &input_deps.first().unwrap().depname;
                for input_feat in &mut input_features {
                    if input_feat.dep_qualifier.is_none() {
                        input_feat.dep_qualifier = Some(implicit_depname.to_owned());
                    }
                }
            }

            // ensure all requested features have a dependency
            let features: Vec<data::FeatureRequest> = input_features
                .into_iter()
                .map(cli::FeatureCliArg::into_feature_req)
                .collect_oks_or_iter_errs()
                .map_err(Error::from_nonempty_iter)?;

            commands::inject_deps(&bin_name, &input_deps, &features, &cfg, &cargo_add_args)?;
        }
        SubCmd::DoNothing {} => (),
    };

    Ok(())
}
