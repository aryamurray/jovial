use clap::Parser;

/// Arguments for the test-plugin command.
#[derive(Parser)]
pub struct Args {
    /// Path to the plugin directory
    #[arg(default_value = ".")]
    pub path: String,
}

/// Run the test-plugin command.
pub fn run(_args: Args) -> anyhow::Result<()> {
    todo!()
}
