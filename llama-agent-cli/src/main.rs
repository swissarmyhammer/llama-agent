use anyhow::Result;
use clap::Parser;
use llama_agent_cli::{run_agent, Args};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

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
        info!("Starting llama-agent-cli");
        info!("Model: {}", args.model);
        info!("Filename: {:?}", args.filename);
        info!("Prompt: {}", args.prompt);
        info!("Limit: {}", args.limit);
    }

    // Initialize agent components and process request
    match run_agent(args).await {
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
