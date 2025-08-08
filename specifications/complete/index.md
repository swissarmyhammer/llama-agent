# Agent API Specification

## Overview

This specification defines a Rust-based Agent API that provides text chat capabilities using llama-cpp-rs for model inference, with MCP (Model Context Protocol) client support. The system is designed around a single in-memory model instance accessed through a queue-based architecture to handle concurrent requests efficiently.

## Project Structure

This project is organized as a Rust workspace with two main crates:

```
llama-agent/
├── Cargo.toml          # Workspace root
├── llama-agent/        # Core library crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── agent.rs    # Main AgentServer implementation
│       ├── model.rs    # ModelManager and loading
│       ├── queue.rs    # RequestQueue implementation
│       ├── session.rs  # Session management
│       ├── mcp.rs      # MCP client integration
│       └── types.rs    # Core types and traits
└── llama-agent-cli/    # Command-line interface
    ├── Cargo.toml
    └── src/
        └── main.rs
```

**Workspace Cargo.toml:**
```toml
[workspace]
members = [
    "llama-agent",
    "llama-agent-cli",
]
resolver = "2"

[workspace.dependencies]
# Core inference
llama-cpp-2 = "0.1.109"

# MCP integration  
rmcp = "0.4.0"

# Async runtime
tokio = { version = "1.0", features = ["full"] }
tokio-stream = "0.1"

# CLI
clap = { version = "4.0", features = ["derive"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Utilities
uuid = { version = "1.0", features = ["v4"] }
tracing = "0.1"
tracing-subscriber = "0.3"

# Async traits
async-trait = "0.1"
```

### Crate Responsibilities

- **llama-agent**: Core library with all agent functionality, model management, MCP integration, and request handling
- **llama-agent-cli**: Simple command-line interface for testing and user interaction

## CLI Interface

The CLI provides a simple way to test and interact with the agent:

```bash
# Use HuggingFace model with auto-detection
llama-agent-cli --model microsoft/DialoGPT-medium --prompt "Hello, how are you?"

# Use specific filename from HuggingFace repo
llama-agent-cli --model microsoft/DialoGPT-medium --filename model-bf16.gguf --prompt "What is Rust?"

# Use local model folder
llama-agent-cli --model ./models/llama2-7b --prompt "Explain quantum computing" --limit 200

# Use local specific file
llama-agent-cli --model ./models/llama2-7b --filename llama-2-7b.q4_k_m.gguf --prompt "Write a haiku"
```

### CLI Arguments

```rust
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
```

## Architecture

### Core Components

1. **AgentServer** - Main server managing the model instance and request queue
2. **ModelManager** - Handles model loading, initialization, and lifecycle
3. **RequestQueue** - Thread-safe queue for managing concurrent inference requests
4. **Session** - Represents a chat session containing a sequence of messages
5. **MCPClient** - Integration with Model Context Protocol servers
6. **ChatTemplateEngine** - Leverages llama-cpp-rs chat template support

### Design Principles

- **Single Model Instance**: One loaded model shared across all requests
- **Queue-based Concurrency**: Serialize access to the model through a request queue
- **Session-based Chat**: Group messages by session for context management
- **Streaming & Batch Support**: Both real-time streaming and batch processing
- **Memory Efficiency**: In-memory model with efficient resource management

## API Interface

### Core Types

```rust
#[derive(Debug, Clone)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub tool_call_id: Option<String>,
    pub tool_name: Option<String>,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub messages: Vec<Message>,
    pub mcp_servers: Vec<MCPServerConfig>,
    pub available_tools: Vec<ToolDefinition>,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
}

#[derive(Debug)]
pub struct GenerationRequest {
    pub session_id: SessionId,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub stop_tokens: Vec<String>,
}

#[derive(Debug)]
pub struct GenerationResponse {
    pub generated_text: String,
    pub tokens_generated: u32,
    pub generation_time: Duration,
    pub finish_reason: FinishReason,
}

#[derive(Debug)]
pub enum FinishReason {
    MaxTokens,
    StopToken,
    EndOfSequence,
    ToolCall,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub server_name: String,
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub call_id: String,
    pub result: serde_json::Value,
    pub error: Option<String>,
}
```

### Main API

```rust
#[async_trait]
pub trait AgentAPI {
    /// Initialize the agent with a model
    async fn initialize(config: AgentConfig) -> Result<Self, AgentError>;
    
    /// Generate text completion for a session (batch mode)
    async fn generate(&self, request: GenerationRequest) -> Result<GenerationResponse, AgentError>;
    
    /// Generate streaming text completion
    async fn generate_stream(
        &self, 
        request: GenerationRequest
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, AgentError>>>>, AgentError>;
    
    /// Create a new chat session
    async fn create_session(&self) -> Result<Session, AgentError>;
    
    /// Get existing session
    async fn get_session(&self, session_id: &str) -> Result<Option<Session>, AgentError>;
    
    /// Update session with new messages
    async fn update_session(&self, session: Session) -> Result<(), AgentError>;
    
    /// Discover available tools from MCP servers and update session
    async fn discover_tools(&self, session: &mut Session) -> Result<(), AgentError>;
    
    /// Execute a tool call via MCP client
    async fn execute_tool(&self, tool_call: ToolCall, session: &Session) -> Result<ToolResult, AgentError>;
    
    /// Health check
    async fn health(&self) -> Result<HealthStatus, AgentError>;
    
    /// Get MCP client for external tool integration
    fn mcp_client(&self) -> &MCPClient;
}
```

### Configuration

```rust
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Model configuration
    pub model: ModelConfig,
    
    /// Queue configuration
    pub queue_config: QueueConfig,
    
    /// MCP server configurations
    pub mcp_servers: Vec<MCPServerConfig>,
    
    /// Session management
    pub session_config: SessionConfig,
}

#[derive(Debug, Clone)]
pub struct ModelConfig {
    /// Model source (HuggingFace repo or local path)
    pub source: ModelSource,
    
    /// Batch size for processing
    pub batch_size: u32,
    
    /// Use generation parameters from HuggingFace generation_config.json
    pub use_hf_params: bool,
}

#[derive(Debug, Clone)]
pub enum ModelSource {
    HuggingFace { repo: String, filename: Option<String> },
    Local { folder: PathBuf, filename: Option<String> },
}

#[derive(Debug, Clone)]
pub struct QueueConfig {
    /// Maximum queue size
    pub max_queue_size: usize,
    
    /// Request timeout
    pub request_timeout: Duration,
    
    /// Number of worker threads
    pub worker_threads: usize,
}
```

## Implementation Details

### Model Loading and Management

The `ModelManager` component handles:

1. **Model Download**: Using llama-cpp-rs HuggingFace integration for automatic model downloading
2. **Model Loading**: Initialize llama.cpp model with specified parameters
3. **Context Management**: Manage model context and state
4. **Resource Cleanup**: Proper cleanup on shutdown

**Model Selection Strategy:**
- If `filename` is specified, use that exact file
- If `filename` is `None`, auto-detect with this priority:
  1. Files containing "BF16" or "bf16" in the name
  2. First available `.gguf` file in the location
  3. Error if no compatible model files found
- This strategy applies to both HuggingFace repos and local folders

```rust
impl ModelManager {
    pub async fn load_model(config: ModelConfig) -> Result<LlamaModel, ModelError> {
        match config.source {
            ModelSource::HuggingFace { repo, filename } => {
                // Download repo and use specified filename or auto-detect
                // Priority: specified filename > BF16/bf16 > first .gguf
                let mut model = match filename {
                    Some(f) => LlamaModel::load_from_hf(repo, f)?,
                    None => LlamaModel::load_from_hf_auto(repo)?, // Auto-detect with BF16 preference
                };
                
                // Load generation parameters from HuggingFace generation_config.json if requested
                if config.use_hf_params {
                    model.load_generation_config_from_hf(repo).await?;
                }
                
                Ok(model)
            },
            ModelSource::Local { folder, filename } => {
                // Load from local folder with optional filename specification
                let model = match filename {
                    Some(f) => LlamaModel::load_from_file(folder.join(f))?,
                    None => LlamaModel::load_from_folder_auto(folder)?, // Auto-detect with BF16 preference
                };
                Ok(model)
            }
        }
    }
}
```

### Queue-based Request Handling

The request queue ensures thread-safe access to the single model instance:

```rust
pub struct RequestQueue {
    sender: mpsc::Sender<QueuedRequest>,
    receiver: Arc<Mutex<mpsc::Receiver<QueuedRequest>>>,
    worker_handles: Vec<JoinHandle<()>>,
}

struct QueuedRequest {
    request: GenerationRequest,
    response_sender: oneshot::Sender<Result<GenerationResponse, AgentError>>,
    stream_sender: Option<mpsc::Sender<Result<StreamChunk, AgentError>>>,
}

impl RequestQueue {
    pub fn new(model: Arc<LlamaModel>, config: QueueConfig) -> Self {
        // Initialize queue with worker threads
        // Workers process requests sequentially against the single model
    }
    
    pub async fn submit_request(
        &self, 
        request: GenerationRequest,
        session: &Session
    ) -> Result<GenerationResponse, AgentError> {
        // Submit request to queue and await response
    }
    
    pub async fn submit_streaming_request(
        &self, 
        request: GenerationRequest,
        session: &Session
    ) -> Result<impl Stream<Item = Result<StreamChunk, AgentError>>, AgentError> {
        // Submit streaming request to queue
    }
}
```

### Chat Template Integration

Leverage llama-cpp-rs built-in chat template support:

```rust
impl ChatTemplateEngine {
    pub fn render_session(
        &self, 
        session: &Session, 
        model: &LlamaModel
    ) -> Result<String, TemplateError> {
        // Use llama-cpp-rs chat template functionality
        // Convert Session messages to the format expected by the model
        let chat_messages = session.messages.iter()
            .map(|msg| (msg.role.as_str(), msg.content.as_str()))
            .collect::<Vec<_>>();
        
        // Include available tools in the template context
        let tools_json = if !session.available_tools.is_empty() {
            Some(serde_json::to_value(&session.available_tools)?)
        } else {
            None
        };
            
        model.apply_chat_template_with_tools(chat_messages, tools_json)
    }
    
    pub fn extract_tool_calls(&self, generated_text: &str) -> Result<Vec<ToolCall>, TemplateError> {
        // Parse generated text for tool call syntax (model-dependent format)
        // This would need to be implemented based on the specific chat template format
        // Common formats include JSON function calls or special tokens
        self.parse_tool_calls_from_text(generated_text)
    }
}
```

### MCP Client Integration

Integrate MCP client for external tool capabilities:

```rust
pub struct MCPClient {
    servers: HashMap<String, Arc<dyn MCPServer>>,
    runtime: tokio::runtime::Handle,
}

impl MCPClient {
    pub async fn initialize(configs: Vec<MCPServerConfig>) -> Result<Self, MCPError> {
        // Initialize MCP servers based on configuration
        // Use rmcp crate for server connections
    }
    
    pub async fn discover_tools(&self) -> Result<Vec<ToolDefinition>, MCPError> {
        // Discover all available tools from all configured MCP servers
        let mut all_tools = Vec::new();
        
        for (server_name, server) in &self.servers {
            let tools = server.list_tools().await?;
            for tool in tools {
                all_tools.push(ToolDefinition {
                    name: tool.name,
                    description: tool.description,
                    parameters: tool.input_schema,
                    server_name: server_name.clone(),
                });
            }
        }
        
        Ok(all_tools)
    }
    
    pub async fn call_tool(
        &self, 
        server_name: &str, 
        tool_name: &str, 
        args: serde_json::Value
    ) -> Result<serde_json::Value, MCPError> {
        // Call tool on specified MCP server
        let server = self.servers.get(server_name)
            .ok_or_else(|| MCPError::ServerNotFound(server_name.to_string()))?;
            
        server.call_tool(tool_name, args).await
    }
}
```

### Session Management

Sessions maintain conversation context:

```rust
impl SessionManager {
    pub async fn create_session(&self) -> Result<Session, SessionError> {
        let session = Session {
            id: Uuid::new_v4().to_string(),
            messages: Vec::new(),
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
        };
        
        // Store in memory (could be extended to persistent storage)
        self.sessions.insert(session.id.clone(), session.clone()).await;
        Ok(session)
    }
    
    pub async fn add_message(
        &self, 
        session_id: &str, 
        message: Message
    ) -> Result<(), SessionError> {
        // Add message to session and update timestamp
    }
}
```

## Streaming Implementation

Support both batch and streaming responses:

```rust
#[derive(Debug)]
pub struct StreamChunk {
    pub text: String,
    pub is_complete: bool,
    pub token_count: u32,
}

impl AgentServer {
    pub async fn generate_stream(
        &self, 
        request: GenerationRequest
    ) -> Result<impl Stream<Item = Result<StreamChunk, AgentError>>, AgentError> {
        let (tx, rx) = mpsc::channel(100);
        
        // Submit to queue with streaming callback
        self.queue.submit_streaming_request(request, tx).await?;
        
        Ok(UnboundedReceiverStream::new(rx))
    }
}
```

## Error Handling

Comprehensive error types for different failure modes:

```rust
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("Model error: {0}")]
    Model(#[from] ModelError),
    
    #[error("Queue error: {0}")]
    Queue(#[from] QueueError),
    
    #[error("Session error: {0}")]
    Session(#[from] SessionError),
    
    #[error("MCP error: {0}")]
    MCP(#[from] MCPError),
    
    #[error("Template error: {0}")]
    Template(#[from] TemplateError),
    
    #[error("Timeout: request took longer than {timeout:?}")]
    Timeout { timeout: Duration },
    
    #[error("Queue full: maximum capacity {capacity} exceeded")]
    QueueFull { capacity: usize },
}
```

## Dependencies

Required Cargo.toml dependencies:

```toml
[dependencies]
# Core inference
llama-cpp-2 = "0.1.114"

# MCP integration
rmcp = "0.4.0"

# Async runtime
tokio = { version = "1.0", features = ["full"] }
tokio-stream = "0.1"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Utilities
uuid = { version = "1.0", features = ["v4"] }
tracing = "0.1"
tracing-subscriber = "0.3"

# Async traits
async-trait = "0.1"
```

## Usage Example

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AgentConfig {
        model: ModelConfig {
            source: ModelSource::HuggingFace { 
                repo: "microsoft/DialoGPT-medium".to_string(),
                filename: None, // Auto-detect with BF16 preference
            },
            batch_size: 512,
            use_hf_params: true, // Use HuggingFace generation_config.json
        },
        queue_config: QueueConfig {
            max_queue_size: 100,
            request_timeout: Duration::from_secs(30),
            worker_threads: 1,
        },
        mcp_servers: vec![
            MCPServerConfig {
                name: "filesystem".to_string(),
                command: "npx".to_string(),
                args: vec!["-y".to_string(), "@modelcontextprotocol/server-filesystem".to_string()],
            }
        ],
        session_config: SessionConfig::default(),
    };
    
    let agent = AgentServer::initialize(config).await?;
    
    // Create a session with MCP servers
    let mut session = agent.create_session().await?;
    session.mcp_servers = vec![
        MCPServerConfig {
            name: "filesystem".to_string(),
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "@modelcontextprotocol/server-filesystem".to_string()],
        }
    ];
    
    // Discover available tools from MCP servers
    agent.discover_tools(&mut session).await?;
    println!("Available tools: {:?}", session.available_tools);
    
    // Add a message that might trigger tool use
    session.messages.push(Message {
        role: MessageRole::User,
        content: "Can you list the files in the current directory?".to_string(),
        tool_call_id: None,
        tool_name: None,
        timestamp: SystemTime::now(),
    });
    
    // Generate response
    let request = GenerationRequest {
        session_id: session.id.clone(),
        max_tokens: Some(100),
        temperature: Some(0.7),
        top_p: Some(0.9),
        stop_tokens: vec![],
    };
    
    let response = agent.generate(request).await?;
    
    // Check if the response includes tool calls
    match response.finish_reason {
        FinishReason::ToolCall => {
            println!("Model wants to call tools!");
            // Extract tool calls from the generated text
            let chat_engine = ChatTemplateEngine::new();
            let tool_calls = chat_engine.extract_tool_calls(&response.generated_text)?;
            
            for tool_call in tool_calls {
                // Execute the tool call
                let tool_result = agent.execute_tool(tool_call.clone(), &session).await?;
                
                // Add tool result to session
                session.messages.push(Message {
                    role: MessageRole::Tool,
                    content: serde_json::to_string(&tool_result.result)?,
                    tool_call_id: Some(tool_call.id),
                    tool_name: Some(tool_call.name),
                    timestamp: SystemTime::now(),
                });
            }
            
            // Generate final response with tool results
            let final_request = GenerationRequest {
                session_id: session.id.clone(),
                max_tokens: Some(100),
                temperature: Some(0.7),
                top_p: Some(0.9),
                stop_tokens: vec![],
            };
            
            let final_response = agent.generate(final_request).await?;
            println!("Final response: {}", final_response.generated_text);
        }
        _ => {
            println!("Response: {}", response.generated_text);
        }
    }
    
    Ok(())
}
```

## References

### Key Dependencies
- **llama-cpp-2**: Rust bindings for llama.cpp - [crates.io](https://crates.io/crates/llama-cpp-2) | [GitHub](https://github.com/utilityai/llama-cpp-rs)
- **rmcp**: Official Rust SDK for Model Context Protocol - [crates.io](https://crates.io/crates/rmcp) | [GitHub](https://github.com/modelcontextprotocol/rust-sdk)

### Related Projects
- **mistral.rs**: Reference implementation for Rust AI inference APIs - [GitHub](https://github.com/EricLBuehler/mistral.rs)
- **llama.cpp**: Core C++ library for LLM inference - [GitHub](https://github.com/ggml-org/llama.cpp)
- **Model Context Protocol**: Specification for tool integration - [GitHub](https://github.com/modelcontextprotocol)