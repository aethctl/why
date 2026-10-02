mod cli;
mod model;
mod render;
mod resolver;
mod system;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    let finding = match &cli.command {
        Commands::Command { name } => resolver::command::resolve(name)?,
        Commands::Service { name } => resolver::service::resolve(name)?,
        Commands::Package { name } => resolver::package::resolve(name)?,
    };

    render::print(&finding, cli.json)
}
