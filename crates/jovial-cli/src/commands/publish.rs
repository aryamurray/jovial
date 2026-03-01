use clap::Parser;

/// Arguments for the publish command.
#[derive(Parser)]
pub struct Args {
    /// Path to the plugin directory
    #[arg(default_value = ".")]
    pub path: String,
}

/// Run the publish command.
pub fn run(_args: Args) -> anyhow::Result<()> {
    todo!()
}
