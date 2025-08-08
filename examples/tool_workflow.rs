//! Tool workflow example demonstrating manual tool call handling
//!
//! This example shows the detailed tool call workflow from the specification:
//! - Extract tool calls from generated text
//! - Execute tool calls individually  
//! - Add tool results to session
//! - Generate final response with tool results

use llama_agent::{
    chat_template::ChatTemplateEngine,
    types::{
        AgentAPI, AgentConfig, FinishReason, GenerationRequest, MCPServerConfig, Message,
        MessageRole, ModelConfig, ModelSource, QueueConfig, RetryConfig, SessionConfig,
    },
    AgentServer,
};
use std::time::{Duration, SystemTime};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting tool workflow example");

    let config = AgentConfig {
        model: ModelConfig {
            source: ModelSource::HuggingFace {
                repo: "microsoft/DialoGPT-medium".to_string(),
                filename: None,
            },
            batch_size: 512,
            use_hf_params: true,
            retry_config: RetryConfig::default(),
            debug: false,
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

    let agent = AgentServer::initialize(config).await?;
    let mut session = agent.create_session().await?;

    // Configure MCP servers for the session
    session.mcp_servers = vec![MCPServerConfig {
        name: "filesystem".to_string(),
        command: "npx".to_string(),
        args: vec![
            "-y".to_string(),
            "@modelcontextprotocol/server-filesystem".to_string(),
        ],
        timeout_secs: None,
    }];

    // Discover available tools
    agent.discover_tools(&mut session).await?;
    println!("Available tools:");
    for tool in &session.available_tools {
        println!("  - {}: {}", tool.name, tool.description);
    }

    // Add user message that should trigger tool use
    session.messages.push(Message {
        role: MessageRole::User,
        content: "Please list the files in the current directory and tell me about any Rust files you find.".to_string(),
        tool_call_id: None,
        tool_name: None,
        timestamp: SystemTime::now(),
    });

    // Generate initial response that might contain tool calls
    let request = GenerationRequest {
        session_id: session.id.clone(),
        max_tokens: Some(200),
        temperature: Some(0.7),
        top_p: Some(0.9),
        stop_tokens: vec![],
    };

    println!("\nGenerating initial response...");
    let response = agent.generate(request).await?;

    // Handle the response based on finish reason
    match response.finish_reason {
        FinishReason::ToolCall => {
            println!("Model wants to call tools!");
            println!(
                "Generated text with tool calls:\n{}",
                response.generated_text
            );

            // Create a chat template engine to extract tool calls manually
            let chat_engine = ChatTemplateEngine::new();

            // Extract tool calls from the generated text
            match chat_engine.extract_tool_calls(&response.generated_text) {
                Ok(tool_calls) => {
                    println!("\nExtracted {} tool calls:", tool_calls.len());

                    for (i, tool_call) in tool_calls.iter().enumerate() {
                        println!("  {}. Tool: {}", i + 1, tool_call.name);
                        println!("     ID: {}", tool_call.id);
                        println!("     Arguments: {}", tool_call.arguments);
                    }

                    // Add assistant's message with tool calls to session
                    session.messages.push(Message {
                        role: MessageRole::Assistant,
                        content: response.generated_text.clone(),
                        tool_call_id: None,
                        tool_name: None,
                        timestamp: SystemTime::now(),
                    });

                    // Execute each tool call
                    for tool_call in tool_calls {
                        println!("\nExecuting tool call: {}", tool_call.name);

                        match agent.execute_tool(tool_call.clone(), &session).await {
                            Ok(tool_result) => {
                                println!("Tool result for '{}': Success", tool_call.name);
                                if let Some(error) = &tool_result.error {
                                    println!("  Error: {}", error);
                                } else {
                                    println!("  Result: {}", tool_result.result);
                                }

                                // Add tool result to session
                                let tool_content = if let Some(error) = &tool_result.error {
                                    format!("Error: {}", error)
                                } else {
                                    serde_json::to_string(&tool_result.result)?
                                };

                                session.messages.push(Message {
                                    role: MessageRole::Tool,
                                    content: tool_content,
                                    tool_call_id: Some(tool_call.id),
                                    tool_name: Some(tool_call.name.clone()),
                                    timestamp: SystemTime::now(),
                                });
                            }
                            Err(e) => {
                                warn!("Tool call execution failed: {}", e);

                                // Add error result to session
                                session.messages.push(Message {
                                    role: MessageRole::Tool,
                                    content: format!("Error executing tool: {}", e),
                                    tool_call_id: Some(tool_call.id),
                                    tool_name: Some(tool_call.name.clone()),
                                    timestamp: SystemTime::now(),
                                });
                            }
                        }
                    }

                    // Generate final response incorporating tool results
                    let final_request = GenerationRequest {
                        session_id: session.id.clone(),
                        max_tokens: Some(200),
                        temperature: Some(0.7),
                        top_p: Some(0.9),
                        stop_tokens: vec![],
                    };

                    println!("\nGenerating final response with tool results...");
                    let final_response = agent.generate(final_request).await?;
                    println!("Final response:");
                    println!("{}", final_response.generated_text);

                    println!("\nFinal Statistics:");
                    println!(
                        "  Total tokens: {}",
                        response.tokens_generated + final_response.tokens_generated
                    );
                    println!("  Messages in session: {}", session.messages.len());
                }
                Err(e) => {
                    warn!("Failed to extract tool calls: {}", e);
                    println!("Could not extract tool calls from response: {}", e);
                    println!("Raw response: {}", response.generated_text);
                }
            }
        }
        _ => {
            println!("Response (no tool calls):");
            println!("{}", response.generated_text);
        }
    }

    println!("\nGeneration Statistics:");
    println!("  Tokens generated: {}", response.tokens_generated);
    println!("  Time taken: {:?}", response.generation_time);
    println!("  Finish reason: {:?}", response.finish_reason);

    info!("Tool workflow example completed");
    Ok(())
}
