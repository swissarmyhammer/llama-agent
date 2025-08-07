use anyhow::Result;
use clap::Parser;
use llama_agent::{
    model::ModelManager,
    queue::RequestQueue,
    session::SessionManager,
    types::{
        GenerationRequest, Message, MessageRole, ModelConfig, ModelSource, QueueConfig,
        SessionConfig,
    },
};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tracing::{error, info};

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
        Ok(response) => {
            println!("Response: {}", response);
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
            } else if error_msg.contains("Failed to load model") {
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
            batch_size: 512, // Default batch size
            use_hf_params: false,
        }
    } else {
        // Assume HuggingFace repo
        ModelConfig {
            source: ModelSource::HuggingFace {
                repo: args.model.clone(),
                filename: args.filename,
            },
            batch_size: 512, // Default batch size
            use_hf_params: true,
        }
    };

    // Create configurations
    let queue_config = QueueConfig {
        max_queue_size: 10,
        request_timeout: Duration::from_secs(30),
        worker_threads: 2, // Default worker threads
    };

    let session_config = SessionConfig {
        max_sessions: 100,
        session_timeout: Duration::from_secs(3600),
    };

    info!("Initializing model manager...");
    let model_manager = Arc::new(ModelManager::new(model_config)?);

    // Load the model
    if let Err(e) = model_manager.load_model().await {
        return Err(anyhow::anyhow!("Failed to load model: {}", e));
    }

    info!("Initializing request queue...");
    let queue = Arc::new(RequestQueue::new(model_manager.clone(), queue_config));

    info!("Initializing session manager...");
    let session_manager = Arc::new(SessionManager::new(session_config));

    // Create a new session
    let session = session_manager.create_session().await?;
    let session_id = session.id;

    // Add the user message
    let message = Message {
        role: MessageRole::User,
        content: args.prompt.clone(),
        tool_call_id: None,
        tool_name: None,
        timestamp: std::time::SystemTime::now(),
    };

    session_manager.add_message(&session_id, message).await?;

    // Get the updated session for generation
    let updated_session = session_manager
        .get_session(&session_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Session not found after adding message"))?;

    // Create generation request
    let request = GenerationRequest {
        session: updated_session,
        max_tokens: Some(args.limit),
        temperature: Some(0.7),
        top_p: Some(0.9),
        stop_tokens: vec![],
    };

    info!("Processing generation request...");
    match queue.submit_request(request).await {
        Ok(response) => {
            info!("Generation completed successfully");
            Ok(response.generated_text)
        }
        Err(e) => {
            error!("Generation failed: {}", e);
            Err(anyhow::anyhow!("Generation failed: {}", e))
        }
    }
}
