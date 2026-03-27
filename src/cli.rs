use clap::{Parser, Subcommand};

/// Manage scripts and dependencies in a playground project
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct ArgParser {
    #[command(subcommand)]
    pub cmd: SubCmd,
}

#[derive(Clone, Subcommand)]
pub enum SubCmd {
    /// Create a new script
    #[command(name = "new")]
    NewScript {
        bin_name: String,

        #[arg(short, long, default_value = "bare")]
        template: String,
    },

    /// List the scripts declared in `Cargo.toml`
    #[command(name = "list")]
    ListScripts {
        #[arg(long, short, help = "Only print script names, nothing else")]
        quiet: bool,
    },

    /// Add a dependency to a script
    #[command(name = "inject")]
    InjectDep {
        #[arg(help = "name of the script to add dependencies to")]
        bin_name: String,

        #[arg(help = "(depname)[@version], e.g., `clap` or `clap@0.1.2`")]
        dep_name: String,

        #[arg(help = "features of the dependency to activate")]
        dep_features: Vec<String>,
    },
}
