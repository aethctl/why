mod cli;
mod deep;
mod model;
mod ownership;
mod render;
mod report;
mod resolver;
mod system;

use anyhow::{Result, bail};
use clap::Parser;

use cli::{Cli, Commands};

fn main() {
    if let Err(error) = run() {
        eprintln!("why could not explain that\n\n{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    let mut finding = match (&cli.command, &cli.subject) {
        (Some(Commands::Command { name }), _) => resolver::command::resolve(name)?,
        (Some(Commands::Service { name }), _) => resolver::service::resolve(name)?,
        (Some(Commands::Package { name }), _) => resolver::package::resolve(name)?,
        (Some(Commands::File { path }), _) => resolver::file::resolve(path)?,
        (Some(Commands::Process { pid }), _) => resolver::process::resolve(*pid)?,
        (Some(Commands::Port { port }), _) => resolver::port::resolve(*port)?,
        (Some(Commands::Env { name }), _) => resolver::env::resolve(name)?,
        (Some(Commands::Shell { name }), _) => resolver::shell::resolve(name)?,
        (None, Some(subject)) => resolver::auto::resolve(subject)?,
        (None, None) => bail!("tell me what to explain, for example: why git"),
    };

    if cli.deep {
        deep::enrich(&mut finding)?;
    }

    if cli.report {
        report::print(&finding);
        Ok(())
    } else {
        render::print(&finding, cli.json, cli.plain)
    }
}
