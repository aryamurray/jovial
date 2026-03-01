use clap::Parser;

/// Arguments for the explain command.
#[derive(Parser)]
pub struct Args {
    /// File and optional line to explain (e.g., "Foo.java:42")
    pub target: String,

    /// Path to jovial.yaml config file
    #[arg(short, long, default_value = "jovial.yaml")]
    pub config: String,
}

/// Run the explain command.
pub fn run(_args: Args) -> anyhow::Result<()> {
    todo!()
}
