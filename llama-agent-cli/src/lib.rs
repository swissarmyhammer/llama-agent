use anyhow::Result;
use clap::Parser;
use futures::StreamExt;
use llama_agent::{
    types::{
        AgentAPI, AgentConfig, FinishReason, GenerationRequest, Message, MessageRole, ModelConfig,
        ModelSource, QueueConfig, SessionConfig,
    },
    AgentServer,
};
use std::{io::Write, path::PathBuf, time::Duration};
use tokio::signal;
use tracing::{error, info, warn};

const SEPARATOR_WIDTH: usize = 60;

#[derive(Parser, Clone)]
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

  # Use local specific file with custom settings
  llama-agent-cli --model ./models/llama2-7b --filename llama-2-7b.q4_k_m.gguf --prompt \"Write a haiku\" --temperature 0.8 --top-p 0.95"
)]
pub struct Args {
    /// Model source: HuggingFace repo name (e.g. 'microsoft/DialoGPT-medium') or local folder path
    #[arg(long)]
    pub model: String,

    /// Optional specific filename to use from the repo or folder
    /// If not provided, will auto-detect with BF16 preference
    #[arg(long)]
    pub filename: Option<String>,

    /// Prompt text to kick off generation
    #[arg(long)]
    pub prompt: String,

    /// Stop generation after this many tokens even without proper stop token
    #[arg(long, default_value = "512")]
    pub limit: u32,

    /// Model batch size for processing
    #[arg(long, default_value = "512")]
    pub batch_size: u32,

    /// Maximum queue size for pending requests
    #[arg(long, default_value = "10")]
    pub max_queue_size: usize,

    /// Request timeout in seconds
    #[arg(long, default_value = "120")]
    pub request_timeout: u64,

    /// Number of worker threads
    #[arg(long, default_value = "1")]
    pub worker_threads: usize,

    /// Maximum number of concurrent sessions
    #[arg(long, default_value = "10")]
    pub max_sessions: usize,

    /// Session timeout in seconds
    #[arg(long, default_value = "3600")]
    pub session_timeout: u64,

    /// Temperature for text generation (0.0 to 1.0)
    #[arg(long, default_value = "0.7")]
    pub temperature: f32,

    /// Top-p for nucleus sampling (0.0 to 1.0)
    #[arg(long, default_value = "0.9")]
    pub top_p: f32,

    /// Enable debug logging (shows verbose llama_cpp model loading output)
    #[arg(long, default_value = "false")]
    pub debug: bool,
}

pub fn validate_args(args: &Args) -> Result<()> {
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

    // Validate batch size
    if args.batch_size == 0 {
        return Err(anyhow::anyhow!("Batch size must be greater than 0"));
    }
    if args.batch_size > 2048 {
        return Err(anyhow::anyhow!(
            "Batch size is too large: {}. Maximum recommended is 2048",
            args.batch_size
        ));
    }

    // Validate queue configuration
    if args.max_queue_size == 0 {
        return Err(anyhow::anyhow!("Max queue size must be greater than 0"));
    }
    if args.request_timeout == 0 {
        return Err(anyhow::anyhow!(
            "Request timeout must be greater than 0 seconds"
        ));
    }
    if args.worker_threads == 0 {
        return Err(anyhow::anyhow!("Worker threads must be greater than 0"));
    }

    // Validate session configuration
    if args.max_sessions == 0 {
        return Err(anyhow::anyhow!("Max sessions must be greater than 0"));
    }
    if args.session_timeout == 0 {
        return Err(anyhow::anyhow!(
            "Session timeout must be greater than 0 seconds"
        ));
    }

    // Validate generation parameters
    if args.temperature < 0.0 || args.temperature > 2.0 {
        return Err(anyhow::anyhow!(
            "Temperature must be between 0.0 and 2.0, got: {}",
            args.temperature
        ));
    }
    if args.top_p < 0.0 || args.top_p > 1.0 {
        return Err(anyhow::anyhow!(
            "Top-p must be between 0.0 and 1.0, got: {}",
            args.top_p
        ));
    }

    Ok(())
}

pub async fn run_agent(args: Args) -> Result<String> {
    let debug_mode = args.debug;
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
            batch_size: args.batch_size,
            use_hf_params: false,
        }
    } else {
        // Assume HuggingFace repo
        ModelConfig {
            source: ModelSource::HuggingFace {
                repo: args.model.clone(),
                filename: args.filename,
            },
            batch_size: args.batch_size,
            use_hf_params: true,
        }
    };

    // Create agent configuration
    let agent_config = AgentConfig {
        model: model_config,
        queue_config: QueueConfig {
            max_queue_size: args.max_queue_size,
            request_timeout: Duration::from_secs(args.request_timeout),
            worker_threads: args.worker_threads,
        },
        session_config: SessionConfig {
            max_sessions: args.max_sessions,
            session_timeout: Duration::from_secs(args.session_timeout),
        },
        mcp_servers: vec![], // No MCP servers for basic CLI
    };

    if debug_mode {
        info!("Initializing AgentServer (this may take a while for model loading)...");
    }
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

    // Set up graceful shutdown handler using channels
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);

    // Spawn shutdown signal handler
    tokio::spawn(async move {
        match signal::ctrl_c().await {
            Ok(()) => {
                warn!("Interrupt signal received, shutting down gracefully...");
                println!("\n\nShutting down gracefully...");
                let _ = shutdown_tx.send(()).await;
            }
            Err(err) => {
                error!("Failed to listen for shutdown signal: {}", err);
            }
        }
    });

    // Store agent in an option so we can take ownership when needed
    let mut agent_option = Some(agent);

    // Check for shutdown signal before proceeding
    if shutdown_rx.try_recv().is_ok() {
        if let Some(agent) = agent_option.take() {
            if debug_mode {
                info!("Performing graceful shutdown...");
            }
            if let Err(e) = agent.shutdown().await {
                error!("Error during shutdown: {}", e);
            }
        }
        std::process::exit(0);
    }

    let agent = agent_option.take().unwrap();

    // Create a session
    let mut session = agent.create_session().await?;
    if debug_mode {
        info!("Created session: {}", session.id);
    }

    // Discover available tools (even though we have none configured)
    agent.discover_tools(&mut session).await?;

    if !session.available_tools.is_empty() && debug_mode {
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

    // Add message to session (this also updates the session timestamp)
    agent.add_message(&session.id, message).await?;

    // Create generation request
    let request = GenerationRequest {
        session_id: session.id.clone(),
        max_tokens: Some(args.limit),
        temperature: Some(args.temperature),
        top_p: Some(args.top_p),
        stop_tokens: vec![],
    };

    println!("\nGenerating response (streaming)...");
    println!("{}", "=".repeat(SEPARATOR_WIDTH));
    let start_time = std::time::Instant::now();

    // Use streaming generation for real-time token output
    match agent.generate_stream(request).await {
        Ok(mut stream) => {
            let mut token_count = 0;
            let mut full_response = String::new();
            let mut finish_reason = FinishReason::EndOfSequence; // Default finish reason

            // Process each chunk as it arrives
            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        // Print the new text immediately (real-time streaming)
                        print!("{}", chunk.text);
                        std::io::stdout().flush().unwrap_or_else(|e| {
                            warn!("Failed to flush stdout: {}", e);
                        });

                        // Accumulate for final statistics
                        full_response.push_str(&chunk.text);
                        token_count += chunk.token_count;

                        // Check if generation is complete
                        if chunk.is_complete {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Streaming error: {}", e);
                        finish_reason = FinishReason::Error(e.to_string());
                        break;
                    }
                }
            }

            let generation_time = start_time.elapsed();

            // Display generation statistics
            println!("\n{}", "=".repeat(SEPARATOR_WIDTH));
            println!("Generation Statistics:");
            println!("  Tokens generated: {}", token_count);
            println!("  Time taken: {:.2}s", generation_time.as_secs_f32());
            if token_count > 0 {
                println!(
                    "  Tokens per second: {:.1}",
                    token_count as f32 / generation_time.as_secs_f32()
                );
            }
            println!("  Finish reason: {:?}", finish_reason);
            println!("{}", "=".repeat(SEPARATOR_WIDTH));

            // Handle warnings based on finish reason or token count
            if token_count >= args.limit {
                println!(
                    "\n⚠️  Response may have been truncated due to token limit ({})",
                    args.limit
                );
            }

            // Check if the response looks like it contains tool calls
            if full_response.contains("```")
                && (full_response.contains("function_call") || full_response.contains("tool_call"))
            {
                println!("\n⚠️  Model wants to call tools, but basic CLI doesn't support tool execution yet.");
            }

            Ok(full_response)
        }
        Err(e) => {
            error!("Generation failed: {}", e);
            Err(anyhow::anyhow!("Generation failed: {}", e))
        }
    }
}
