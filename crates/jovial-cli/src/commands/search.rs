use clap::Parser;

/// Arguments for the search command.
#[derive(Parser)]
pub struct Args {
    /// Search query
    pub query: String,
}

/// Run the search command.
pub fn run(_args: Args) -> anyhow::Result<()> {
    todo!()
}
