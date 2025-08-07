//! Basic usage example from the specification
//!
//! This example demonstrates the complete system functionality as outlined in
//! specifications/index.md lines 605-709. It shows:
//!
//! - AgentConfig setup with HuggingFace model loading
//! - Session creation with MCP server configuration
//! - Tool discovery and integration
//! - User message processing with tool calls
//! - Tool execution and result integration
//! - Follow-up generation with tool results

use llama_agent::{
    types::{
        AgentAPI, AgentConfig, FinishReason, GenerationRequest, MCPServerConfig, Message,
        MessageRole, ModelConfig, ModelSource, QueueConfig, SessionConfig,
    },
    AgentServer,
};
use std::time::{Duration, SystemTime};
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting basic usage example");

    // Create agent configuration exactly as shown in the specification
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
        mcp_servers: vec![MCPServerConfig {
            name: "filesystem".to_string(),
            command: "npx".to_string(),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-filesystem".to_string(),
            ],
            timeout_secs: None,
        }],
        session_config: SessionConfig::default(),
    };

    info!("Initializing AgentServer (this may take a while for model loading)...");
    let agent = AgentServer::initialize(config).await?;
    info!("AgentServer initialized successfully");

    // Create a session with MCP servers
    let mut session = agent.create_session().await?;
    session.mcp_servers = vec![MCPServerConfig {
        name: "filesystem".to_string(),
        command: "npx".to_string(),
        args: vec![
            "-y".to_string(),
            "@modelcontextprotocol/server-filesystem".to_string(),
        ],
        timeout_secs: None,
    }];

    info!("Created session: {}", session.id);

    // Discover available tools from MCP servers
    agent.discover_tools(&mut session).await?;
    info!("Available tools: {:#?}", session.available_tools);

    for tool in &session.available_tools {
        println!("  - {}: {}", tool.name, tool.description);
    }

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
        session: session.clone(),
        max_tokens: Some(100),
        temperature: Some(0.7),
        top_p: Some(0.9),
        stop_tokens: vec![],
    };

    info!("Generating response...");
    let response = agent.generate(request).await?;

    // Check if the response includes tool calls
    match response.finish_reason {
        FinishReason::ToolCall => {
            info!("Model wants to call tools!");
            println!("Model wants to call tools!");

            // Extract tool calls from the generated text
            // Note: The ChatTemplateEngine is used internally by AgentServer.generate()
            // The tool call extraction and execution is handled automatically in the generate() method
            // This example shows the conceptual flow, but the actual implementation is handled internally

            println!("Generated text with tool calls:");
            println!("{}", response.generated_text);

            // The tool calls have already been processed by the generate() method
            // and the response includes the final result after tool execution
        }
        FinishReason::MaxTokens => {
            println!("Response (truncated due to token limit):");
            println!("{}", response.generated_text);
        }
        FinishReason::StopToken => {
            println!("Response:");
            println!("{}", response.generated_text);
        }
        FinishReason::EndOfSequence => {
            println!("Response:");
            println!("{}", response.generated_text);
        }
        FinishReason::Error(ref err) => {
            error!("Generation completed with error: {}", err);
            println!("Response with error:");
            println!("{}", response.generated_text);
            println!("Error: {}", err);
        }
    }

    // Display generation statistics
    println!("\nGeneration Statistics:");
    println!("  Tokens generated: {}", response.tokens_generated);
    println!("  Time taken: {:?}", response.generation_time);
    println!("  Finish reason: {:?}", response.finish_reason);

    info!("Basic usage example completed successfully");
    Ok(())
}
