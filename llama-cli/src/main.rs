use anyhow::Result;
use clap::{Parser, Subcommand};
use llama_cli::{
    embed::EmbedArgs,
    generate::{run_generate, GenerateArgs},
};
use tracing::info;

#[derive(Parser)]
#[command(name = "llama-cli")]
#[command(about = "Unified Llama CLI for generation and embeddings")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate text using a language model (existing functionality)
    Generate(GenerateArgs),
    /// Generate embeddings for input texts
    Embed(EmbedArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate(args) => {
            // Configure logging level based on debug flag
            if args.debug {
                tracing_subscriber::fmt()
                    .with_max_level(tracing::Level::DEBUG)
                    .init();
            } else {
                tracing_subscriber::fmt()
                    .with_max_level(tracing::Level::WARN)
                    .init();
            }

            if args.debug {
                info!("Starting llama-cli generate");
                info!("Model: {}", args.model);
                info!("Filename: {:?}", args.filename);
                info!("Prompt: {}", args.prompt);
                info!("Limit: {}", args.limit);
            }

            // Initialize agent components and process request
            match run_generate(args).await {
                Ok(_response) => {
                    std::process::exit(0);
                }
                Err(e) => {
                    // Check if it's a validation error or runtime error for appropriate exit codes
                    let error_msg = e.to_string();
                    if error_msg.contains("does not exist")
                        || error_msg.contains("Invalid HuggingFace")
                        || error_msg.contains("Token limit")
                        || error_msg.contains("cannot be empty")
                        || error_msg.contains("HuggingFace model repo must be")
                    {
                        // Validation error - exit code 2
                        eprintln!("Error: {}", e);
                        std::process::exit(2);
                    } else if error_msg.contains("Failed to load model")
                        || error_msg.contains("Failed to initialize agent")
                    {
                        // Model loading error - exit code 3
                        eprintln!("Model Error: {}", e);
                        std::process::exit(3);
                    } else {
                        // General runtime error - exit code 1
                        eprintln!("Runtime Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
        Commands::Embed(args) => {
            // Configure logging level based on debug flag
            if args.debug {
                tracing_subscriber::fmt()
                    .with_max_level(tracing::Level::DEBUG)
                    .init();
            } else {
                tracing_subscriber::fmt()
                    .with_max_level(tracing::Level::WARN)
                    .init();
            }

            if args.debug {
                info!("Starting llama-cli embed");
                info!("Model: {}", args.model);
                info!("Input: {:?}", args.input);
                info!("Output: {:?}", args.output);
            }

            // Run embed command implementation
            match llama_cli::embed::run_embed_command(args).await {
                Ok(_) => {
                    std::process::exit(0);
                }
                Err(e) => {
                    // Check error type for appropriate exit codes
                    let error_msg = e.to_string();
                    if error_msg.contains("does not exist")
                        || error_msg.contains("Invalid")
                        || error_msg.contains("cannot be empty")
                        || error_msg.contains("Batch size")
                        || error_msg.contains("Max length")
                    {
                        // Validation error - exit code 2
                        eprintln!("Error: {}", e);
                        std::process::exit(2);
                    } else if error_msg.contains("Failed to load model")
                        || error_msg.contains("Failed to initialize")
                        || error_msg.contains("Model")
                    {
                        // Model loading error - exit code 3
                        eprintln!("Model Error: {}", e);
                        std::process::exit(3);
                    } else {
                        // General runtime error - exit code 1
                        eprintln!("Runtime Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
    }
}
