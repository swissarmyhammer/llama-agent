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

    /// Batch size for model processing
    #[arg(long, default_value = "512")]
    batch_size: u32,

    /// Number of worker threads for queue processing
    #[arg(long, default_value = "2")]
    worker_threads: usize,
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
        }
        Err(e) => {
            error!("Failed to run agent: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

async fn run_agent(args: Args) -> Result<String> {
    // Create model configuration
    let model_config = if args.model.starts_with('/') || args.model.contains(['/', '\\']) {
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

    // Create configurations
    let queue_config = QueueConfig {
        max_queue_size: 10,
        request_timeout: Duration::from_secs(30),
        worker_threads: args.worker_threads,
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
