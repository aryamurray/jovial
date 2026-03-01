use clap::Parser;

/// Arguments for the transpile command.
#[derive(Parser)]
pub struct Args {
    /// Path to the Java project root
    #[arg(short, long)]
    pub input: String,

    /// Output directory for generated Go code
    #[arg(short, long, default_value = "./generated")]
    pub output: String,

    /// Path to jovial.yaml config file
    #[arg(short, long, default_value = "jovial.yaml")]
    pub config: String,

    /// Path to pre-extracted manifest.json
    #[arg(short, long)]
    pub manifest: Option<String>,

    /// Go module path for the generated project
    #[arg(long)]
    pub go_module: Option<String>,
}

/// Run the transpile command.
///
/// This is where walker + codegen are orchestrated:
/// 1. Parse Java sources → Java AST
/// 2. Walk Java AST with plugins → Go AST
/// 3. Use manifest + Go AST to scaffold output project via codegen
pub fn run(_args: Args) -> anyhow::Result<()> {
    todo!()
}
