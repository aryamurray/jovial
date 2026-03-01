#![allow(dead_code)]

mod commands;
mod config;
mod discovery;
mod extractor;
mod loader;

use clap::Parser;

/// Jovial — Java-to-Go transpiler with plugin system
#[derive(Parser)]
#[command(name = "jovial", version, about)]
enum Cli {
    /// Transpile a Java project to Go
    Transpile(commands::transpile::Args),
    /// Explain what transformations would be applied to a file
    Explain(commands::explain::Args),
    /// Install a community plugin
    Install(commands::install::Args),
    /// Publish a plugin to the registry
    Publish(commands::publish::Args),
    /// Scaffold a new plugin project
    InitPlugin(commands::init_plugin::Args),
    /// Test a plugin against its testdata
    TestPlugin(commands::test_plugin::Args),
    /// Search the plugin registry
    Search(commands::search::Args),
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli {
        Cli::Transpile(args) => commands::transpile::run(args),
        Cli::Explain(args) => commands::explain::run(args),
        Cli::Install(args) => commands::install::run(args),
        Cli::Publish(args) => commands::publish::run(args),
        Cli::InitPlugin(args) => commands::init_plugin::run(args),
        Cli::TestPlugin(args) => commands::test_plugin::run(args),
        Cli::Search(args) => commands::search::run(args),
    }
}
