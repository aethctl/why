use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "why", version, about = "Ask your Linux system why")]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: bool,

    #[arg(long, global = true)]
    pub plain: bool,

    #[arg(long, global = true)]
    pub deep: bool,

    #[arg(long, global = true)]
    pub report: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(value_name = "THING")]
    pub subject: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(alias = "cmd")]
    Command {
        name: String,
    },

    #[command(alias = "svc")]
    Service {
        name: String,
    },

    #[command(alias = "pkg")]
    Package {
        name: String,
    },

    File {
        path: String,
    },

    #[command(alias = "proc")]
    Process {
        pid: u32,
    },

    Port {
        port: u16,
    },

    Env {
        name: String,
    },

    Shell {
        name: String,
    },
}
