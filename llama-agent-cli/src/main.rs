use anyhow::Result;
use clap::Parser;
use llama_agent::{
    types::{
        AgentAPI, AgentConfig, FinishReason, GenerationRequest, Message, MessageRole, ModelConfig,
        ModelSource, QueueConfig, SessionConfig,
    },
    AgentServer,
};
use std::{path::PathBuf, time::Duration};
use tokio::signal;
use tracing::{error, info, warn};

#[derive(Parser)]
#[command(name = "llama-agent-cli")]
#[command(about = "A CLI for testing the llama-agent library")]
#[command(version)]
#[command(
    long_about = "A command-line interface for testing the llama-agent library.

Examples:
  # Use HuggingFace model with auto-detection
  llama-agent-cli --model microsoft/DialoGPT-medium --prompt \"Hello, how are you?\"

  # Use specific filename from HuggingFace repo
  llama-agent-cli --model microsoft/DialoGPT-medium --filename model-bf16.gguf --prompt \"What is Rust?\"

  # Use local model folder
  llama-agent-cli --model ./models/llama2-7b --prompt \"Explain quantum computing\" --limit 200

  # Use local specific file
  llama-agent-cli --model ./models/llama2-7b --filename llama-2-7b.q4_k_m.gguf --prompt \"Write a haiku\""
)]
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

fn validate_args(args: &Args) -> Result<()> {
    // Validate model path
    if args.model.is_empty() {
        return Err(anyhow::anyhow!("Model path cannot be empty"));
    }

    // Check if local path exists (starts with / or ./ or contains \)
    if args.model.starts_with('/')
        || args.model.starts_with("./")
        || args.model.starts_with("../")
        || args.model.contains('\\')
    {
        let path = PathBuf::from(&args.model);
        if !path.exists() {
            return Err(anyhow::anyhow!(
                "Local model path does not exist: {}. Please check that the path is correct.",
                args.model
            ));
        }
        if !path.is_dir() {
            return Err(anyhow::anyhow!(
                "Local model path must be a directory: {}. Please provide a folder containing model files.",
                args.model
            ));
        }
    } else {
        // Validate HuggingFace repo format
        if !args.model.contains('/') {
            return Err(anyhow::anyhow!(
                "HuggingFace model repo must be in format 'organization/model': {}. Example: microsoft/DialoGPT-medium",
                args.model
            ));
        }
        if args.model.split('/').count() != 2 {
            return Err(anyhow::anyhow!(
                "Invalid HuggingFace repo format: {}. Must be exactly 'organization/model'",
                args.model
            ));
        }
    }

    // Validate token limit
    if args.limit == 0 {
        return Err(anyhow::anyhow!("Token limit must be greater than 0"));
    }
    if args.limit > 8192 {
        return Err(anyhow::anyhow!(
            "Token limit is too large: {}. Maximum recommended limit is 8192 tokens.",
            args.limit
        ));
    }

    // Validate prompt is not empty
    if args.prompt.trim().is_empty() {
        return Err(anyhow::anyhow!("Prompt cannot be empty"));
    }

    Ok(())
}

async fn run_agent(args: Args) -> Result<String> {
    // Validate arguments
    validate_args(&args)?;

    // Create model configuration
    let model_config = if args.model.starts_with('/')
        || args.model.starts_with("./")
        || args.model.starts_with("../")
        || args.model.contains('\\')
    {
        // Local path
        ModelConfig {
            source: ModelSource::Local {
                folder: PathBuf::from(&args.model),
                filename: args.filename,
            },
            batch_size: 512,
            use_hf_params: false,
        }
    } else {
        // Assume HuggingFace repo
        ModelConfig {
            source: ModelSource::HuggingFace {
                repo: args.model.clone(),
                filename: args.filename,
            },
            batch_size: 512,
            use_hf_params: true,
        }
    };

    // Create agent configuration
    let agent_config = AgentConfig {
        model: model_config,
        queue_config: QueueConfig {
            max_queue_size: 10,
            request_timeout: Duration::from_secs(120), // Longer timeout for model loading
            worker_threads: 1,                         // Single worker thread for simplicity
        },
        session_config: SessionConfig {
            max_sessions: 10, // Lower limit for CLI
            session_timeout: Duration::from_secs(3600),
        },
        mcp_servers: vec![], // No MCP servers for basic CLI
    };

    info!("Initializing AgentServer (this may take a while for model loading)...");
    println!("Loading model from {}...", args.model);

    // Initialize agent server with progress indication
    let agent = match AgentServer::initialize(agent_config).await {
        Ok(agent) => {
            println!("✓ Model loaded successfully!");
            agent
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to initialize agent: {}", e));
        }
    };

    // Set up graceful shutdown handler
    let agent_for_shutdown = std::sync::Arc::new(agent);
    let _agent_clone = agent_for_shutdown.clone();

    // Spawn shutdown handler
    tokio::spawn(async move {
        match signal::ctrl_c().await {
            Ok(()) => {
                warn!("Interrupt signal received, shutting down gracefully...");
                println!("\n\nShutting down gracefully...");
                // Note: We can't call shutdown here because we'd need to move the agent
                // For now, just let the process exit naturally
                std::process::exit(0);
            }
            Err(err) => {
                error!("Failed to listen for shutdown signal: {}", err);
            }
        }
    });

    // Create a session
    let mut session = agent_for_shutdown.create_session().await?;
    info!("Created session: {}", session.id);

    // Discover available tools (even though we have none configured)
    agent_for_shutdown.discover_tools(&mut session).await?;

    if !session.available_tools.is_empty() {
        info!("Discovered {} tools", session.available_tools.len());
        for tool in &session.available_tools {
            println!("  - {}: {}", tool.name, tool.description);
        }
    }

    // Add the user message
    let message = Message {
        role: MessageRole::User,
        content: args.prompt.clone(),
        tool_call_id: None,
        tool_name: None,
        timestamp: std::time::SystemTime::now(),
    };
    session.messages.push(message);
    session.updated_at = std::time::SystemTime::now();

    // Update session
    agent_for_shutdown.update_session(session.clone()).await?;

    // Create generation request
    let request = GenerationRequest {
        session: session.clone(),
        max_tokens: Some(args.limit),
        temperature: Some(0.7),
        top_p: Some(0.9),
        stop_tokens: vec![],
    };

    println!("\nGenerating response...");
    let start_time = std::time::Instant::now();

    match agent_for_shutdown.generate(request).await {
        Ok(response) => {
            let generation_time = start_time.elapsed();

            // Display generation statistics
            println!("\n{}", "=".repeat(60));
            println!(
                "Response ({} tokens, {:.2}s):",
                response.tokens_generated,
                generation_time.as_secs_f32()
            );
            println!("{}", "=".repeat(60));

            // Handle different finish reasons
            match response.finish_reason {
                FinishReason::ToolCall => {
                    println!("\n⚠️  Model wants to call tools, but basic CLI doesn't support tool execution yet.");
                    println!("Generated text with tool calls:");
                    println!("{}", response.generated_text);
                }
                FinishReason::MaxTokens => {
                    println!("{}", response.generated_text);
                    println!(
                        "\n⚠️  Response truncated due to token limit ({})",
                        args.limit
                    );
                }
                FinishReason::StopToken => {
                    println!("{}", response.generated_text);
                }
                FinishReason::EndOfSequence => {
                    println!("{}", response.generated_text);
                }
                FinishReason::Error(ref err) => {
                    println!("{}", response.generated_text);
                    println!("\n❌ Generation completed with error: {}", err);
                }
            }

            println!("{}", "=".repeat(60));
            println!("Generation Statistics:");
            println!("  Tokens generated: {}", response.tokens_generated);
            println!("  Time taken: {:.2}s", generation_time.as_secs_f32());
            if response.tokens_generated > 0 {
                println!(
                    "  Tokens per second: {:.1}",
                    response.tokens_generated as f32 / generation_time.as_secs_f32()
                );
            }
            println!("  Finish reason: {:?}", response.finish_reason);
            println!("{}", "=".repeat(60));

            Ok(response.generated_text)
        }
        Err(e) => {
            error!("Generation failed: {}", e);
            Err(anyhow::anyhow!("Generation failed: {}", e))
        }
    }
}
