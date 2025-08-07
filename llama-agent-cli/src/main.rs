use anyhow::Result;
use clap::Parser;
use tracing::info;

#[derive(Parser)]
#[command(name = "llama-agent-cli")]
#[command(about = "A CLI for testing the llama-agent library")]
struct Args {
    /// Model source: HuggingFace repo name (e.g. 'microsoft/DialoGPT-medium') or local folder path
    #[arg(long)]
    model: String,

    /// Optional specific filename to use from the repo or folder
    /// If not provided, will auto-detect with BF16 preference
    #[arg(long)]
    filename: Option<String>,

    /// Prompt text to kick off generation
    #[arg(long)]
    prompt: String,

    /// Stop generation after this many tokens even without proper stop token
    #[arg(long, default_value = "512")]
    limit: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    info!("Starting llama-agent-cli");
    info!("Model: {}", args.model);
    info!("Filename: {:?}", args.filename);
    info!("Prompt: {}", args.prompt);
    info!("Limit: {}", args.limit);

    // TODO: Initialize agent and process request
    // This is a placeholder until the full agent implementation is available

    println!("Agent CLI initialized successfully!");
    println!("Model: {}", args.model);
    println!("Prompt: {}", args.prompt);
    println!("Token limit: {}", args.limit);

    Ok(())
}
