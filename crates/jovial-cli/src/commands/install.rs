use clap::Parser;

/// Arguments for the install command.
#[derive(Parser)]
pub struct Args {
    /// Plugin name or git URL to install
    pub plugin: String,

    /// Specific version to install
    #[arg(short, long)]
    pub version: Option<String>,
}

/// Run the install command.
pub fn run(_args: Args) -> anyhow::Result<()> {
    todo!()
}
