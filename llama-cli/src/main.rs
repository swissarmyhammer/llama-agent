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

    let result = match cli.command {
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
            run_generate(args).await.map(|_| ())
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
            llama_cli::embed::run_embed_command(args).await.map(|_| ())
        }
    };

    // Handle errors and set appropriate exit codes after all cleanup has occurred
    match result {
        Ok(_) => {
            // Success - normal exit (code 0)
            Ok(())
        }
        Err(e) => {
            let error_msg = e.to_string();
            
            // Check error type for appropriate exit codes
            let exit_code = if error_msg.contains("does not exist")
                || error_msg.contains("Invalid HuggingFace")
                || error_msg.contains("Token limit")
                || error_msg.contains("cannot be empty")
                || error_msg.contains("HuggingFace model repo must be")
                || error_msg.contains("Invalid")
                || error_msg.contains("Batch size")
                || error_msg.contains("Max length")
            {
                eprintln!("Error: {}", e);
                2 // Validation error
            } else if error_msg.contains("Failed to load model")
                || error_msg.contains("Failed to initialize agent")
                || error_msg.contains("Failed to initialize")
                || error_msg.contains("Model loading failed")
            {
                eprintln!("Model Error: {}", e);
                3 // Model loading error
            } else {
                eprintln!("Runtime Error: {}", e);
                1 // General runtime error
            };

            std::process::exit(exit_code);
        }
    }
}
