use clap::Parser;

/// Arguments for the init-plugin command.
#[derive(Parser)]
pub struct Args {
    /// Name for the new plugin
    #[arg(short, long)]
    pub name: String,

    /// Output directory
    #[arg(short, long, default_value = ".")]
    pub output: String,
}

/// Run the init-plugin command.
pub fn run(_args: Args) -> anyhow::Result<()> {
    todo!()
}
