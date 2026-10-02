use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "why", version, about = "Ask your Linux system why")]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(alias = "cmd")]
    Command { name: String },

    #[command(alias = "svc")]
    Service { name: String },

    #[command(alias = "pkg")]
    Package { name: String },
}
