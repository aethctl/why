mod cli;
mod model;
mod ownership;
mod render;
mod resolver;
mod system;

use anyhow::{Result, bail};
use clap::Parser;

use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    let finding = match (&cli.command, &cli.subject) {
        (Some(Commands::Command { name }), _) => resolver::command::resolve(name)?,
        (Some(Commands::Service { name }), _) => resolver::service::resolve(name)?,
        (Some(Commands::Package { name }), _) => resolver::package::resolve(name)?,
        (Some(Commands::File { path }), _) => resolver::file::resolve(path)?,
        (Some(Commands::Process { pid }), _) => resolver::process::resolve(*pid)?,
        (Some(Commands::Port { port }), _) => resolver::port::resolve(*port)?,
        (None, Some(subject)) => resolver::auto::resolve(subject)?,
        (None, None) => bail!("tell me what to explain, for example: why git"),
    };

    render::print(&finding, cli.json)
}
