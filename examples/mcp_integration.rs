//! MCP (Model Context Protocol) Integration Examples
//!
//! This example demonstrates how to integrate multiple MCP servers to provide
//! various tools and capabilities to the language model. It shows:
//!
//! - Configuring multiple MCP servers
//! - Tool discovery from different servers
//! - Using different types of tools (filesystem, web, calculations, etc.)
//! - Error handling for MCP operations

use llama_agent::{
    types::{
        AgentAPI, AgentConfig, FinishReason, GenerationRequest, MCPServerConfig, Message,
        MessageRole, ModelConfig, ModelSource, QueueConfig, SessionConfig,
    },
    AgentServer,
};
use std::time::{Duration, SystemTime};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting MCP integration example");

    // Create configuration with multiple MCP servers
    let config = AgentConfig {
        model: ModelConfig {
            source: ModelSource::HuggingFace {
                repo: "microsoft/DialoGPT-medium".to_string(),
                filename: None,
            },
            batch_size: 512,
            use_hf_params: true,
        },
        queue_config: QueueConfig {
            max_queue_size: 100,
            request_timeout: Duration::from_secs(45),
            worker_threads: 1,
        },
        mcp_servers: vec![
            // Filesystem server for file operations
            MCPServerConfig {
                name: "filesystem".to_string(),
                command: "npx".to_string(),
                args: vec![
                    "-y".to_string(),
                    "@modelcontextprotocol/server-filesystem".to_string(),
                ],
                timeout_secs: Some(30),
            },
            // Web search server (if available)
            MCPServerConfig {
                name: "brave-search".to_string(),
                command: "npx".to_string(),
                args: vec![
                    "-y".to_string(),
                    "@modelcontextprotocol/server-brave-search".to_string(),
                ],
                timeout_secs: Some(60),
            },
            // Memory server for persistent data
            MCPServerConfig {
                name: "memory".to_string(),
                command: "npx".to_string(),
                args: vec![
                    "-y".to_string(),
                    "@modelcontextprotocol/server-memory".to_string(),
                ],
                timeout_secs: Some(30),
            },
        ],
        session_config: SessionConfig::default(),
    };

    println!("Initializing AgentServer with multiple MCP servers...");
    println!("This may take a while as each MCP server is started...");

    let agent = match AgentServer::initialize(config).await {
        Ok(agent) => {
            println!("✓ AgentServer initialized successfully with MCP servers");
            agent
        }
        Err(e) => {
            warn!("Failed to initialize some MCP servers: {}", e);
            println!("⚠ Some MCP servers may not be available, continuing with available ones...");

            // Create a fallback configuration with only filesystem server
            let fallback_config = AgentConfig {
                model: ModelConfig {
                    source: ModelSource::HuggingFace {
                        repo: "microsoft/DialoGPT-medium".to_string(),
                        filename: None,
                    },
                    batch_size: 512,
                    use_hf_params: true,
                },
                queue_config: QueueConfig {
                    max_queue_size: 100,
                    request_timeout: Duration::from_secs(45),
                    worker_threads: 1,
                },
                mcp_servers: vec![MCPServerConfig {
                    name: "filesystem".to_string(),
                    command: "npx".to_string(),
                    args: vec![
                        "-y".to_string(),
                        "@modelcontextprotocol/server-filesystem".to_string(),
                    ],
                    timeout_secs: Some(30),
                }],
                session_config: SessionConfig::default(),
            };

            AgentServer::initialize(fallback_config).await?
        }
    };

    // Create session and discover tools
    let mut session = agent.create_session().await?;
    info!("Created session: {}", session.id);

    // Configure the session with the same MCP servers
    session.mcp_servers = vec![MCPServerConfig {
        name: "filesystem".to_string(),
        command: "npx".to_string(),
        args: vec![
            "-y".to_string(),
            "@modelcontextprotocol/server-filesystem".to_string(),
        ],
        timeout_secs: Some(30),
    }];

    // Discover available tools from all configured MCP servers
    println!("\nDiscovering tools from MCP servers...");
    agent.discover_tools(&mut session).await?;

    if session.available_tools.is_empty() {
        println!("⚠ No tools discovered. This could be because:");
        println!("  - MCP servers are not installed (try: npm install -g @modelcontextprotocol/server-filesystem)");
        println!("  - MCP servers failed to start");
        println!("  - Network connectivity issues");
        return Ok(());
    }

    println!("✓ Discovered {} tools:", session.available_tools.len());
    for tool in &session.available_tools {
        println!(
            "  • {}: {} (from server: {})",
            tool.name, tool.description, tool.server_name
        );
    }

    // Example 1: File system operations
    println!("\n{}", "=".repeat(60));
    println!("Example 1: File System Operations");
    println!("{}", "=".repeat(60));

    session.messages.push(Message {
        role: MessageRole::User,
        content: "Please list the files in the current directory and tell me about any README files you find.".to_string(),
        tool_call_id: None,
        tool_name: None,
        timestamp: SystemTime::now(),
    });

    let request1 = GenerationRequest {
        session: session.clone(),
        max_tokens: Some(300),
        temperature: Some(0.3),
        top_p: Some(0.9),
        stop_tokens: vec![],
    };

    match agent.generate(request1).await {
        Ok(response) => {
            println!("Response: {}", response.generated_text);
            println!(
                "Tokens: {}, Finish: {:?}",
                response.tokens_generated, response.finish_reason
            );

            // Add response to session for context
            session.messages.push(Message {
                role: MessageRole::Assistant,
                content: response.generated_text,
                tool_call_id: None,
                tool_name: None,
                timestamp: SystemTime::now(),
            });
        }
        Err(e) => {
            warn!("Example 1 failed: {}", e);
            println!("❌ Example 1 failed: {}", e);
        }
    }

    // Example 2: Multi-step operations
    println!("\n{}", "=".repeat(60));
    println!("Example 2: Multi-step File Operations");
    println!("{}", "=".repeat(60));

    session.messages.push(Message {
        role: MessageRole::User,
        content: "Can you find all Rust source files (.rs) in the project and give me a summary of the main modules?".to_string(),
        tool_call_id: None,
        tool_name: None,
        timestamp: SystemTime::now(),
    });

    let request2 = GenerationRequest {
        session: session.clone(),
        max_tokens: Some(400),
        temperature: Some(0.3),
        top_p: Some(0.9),
        stop_tokens: vec![],
    };

    match agent.generate(request2).await {
        Ok(response) => {
            println!("Response: {}", response.generated_text);
            println!(
                "Tokens: {}, Finish: {:?}",
                response.tokens_generated, response.finish_reason
            );

            session.messages.push(Message {
                role: MessageRole::Assistant,
                content: response.generated_text,
                tool_call_id: None,
                tool_name: None,
                timestamp: SystemTime::now(),
            });
        }
        Err(e) => {
            warn!("Example 2 failed: {}", e);
            println!("❌ Example 2 failed: {}", e);
        }
    }

    // Example 3: Error handling and recovery
    println!("\n{}", "=".repeat(60));
    println!("Example 3: Error Handling");
    println!("{}", "=".repeat(60));

    session.messages.push(Message {
        role: MessageRole::User,
        content: "Please try to read a file that doesn't exist: /nonexistent/file.txt".to_string(),
        tool_call_id: None,
        tool_name: None,
        timestamp: SystemTime::now(),
    });

    let request3 = GenerationRequest {
        session: session.clone(),
        max_tokens: Some(200),
        temperature: Some(0.3),
        top_p: Some(0.9),
        stop_tokens: vec![],
    };

    match agent.generate(request3).await {
        Ok(response) => {
            println!("Response: {}", response.generated_text);

            match response.finish_reason {
                FinishReason::Error(error) => {
                    println!("✓ Error was handled gracefully: {}", error);
                }
                _ => {
                    println!("✓ Model handled the error and continued generating");
                }
            }
        }
        Err(e) => {
            println!("❌ Unexpected error: {}", e);
        }
    }

    // Check agent and MCP server health
    println!("\n{}", "=".repeat(60));
    println!("Health Check");
    println!("{}", "=".repeat(60));

    match agent.health().await {
        Ok(health) => {
            println!("Agent Health Status: {}", health.status);
            println!("  Model loaded: {}", health.model_loaded);
            println!("  Queue size: {}", health.queue_size);
            println!("  Active sessions: {}", health.active_sessions);
            println!("  Uptime: {:?}", health.uptime);
        }
        Err(e) => {
            warn!("Health check failed: {}", e);
        }
    }

    // MCP-specific health check
    let mcp_client = agent.mcp_client();
    println!("\nMCP Server Health:");
    let health_map = mcp_client.health_check_all().await;
    for (server_name, health_status) in health_map {
        println!("  {}: {:?}", server_name, health_status);
    }

    // Display final session statistics
    println!("\n{}", "=".repeat(60));
    println!("Session Summary");
    println!("{}", "=".repeat(60));
    println!("Session ID: {}", session.id);
    println!("Total messages: {}", session.messages.len());
    println!("Available tools: {}", session.available_tools.len());
    println!("MCP servers configured: {}", session.mcp_servers.len());

    // Show message history
    println!("\nMessage History:");
    for (i, message) in session.messages.iter().enumerate() {
        println!(
            "  {}. {} ({}): {}",
            i + 1,
            message.role.as_str(),
            message.timestamp.elapsed().unwrap_or_default().as_secs(),
            if message.content.len() > 100 {
                format!("{}...", &message.content[..100])
            } else {
                message.content.clone()
            }
        );
    }

    println!("\n✓ MCP integration example completed successfully");
    info!("MCP integration example completed");
    Ok(())
}

#[allow(dead_code)]
async fn demonstrate_custom_mcp_server() -> Result<(), Box<dyn std::error::Error>> {
    println!("Custom MCP Server Integration Example");
    println!("This would show how to create and integrate a custom MCP server");

    // This is a conceptual example of how you might configure a custom MCP server
    let _custom_config = MCPServerConfig {
        name: "custom-calculator".to_string(),
        command: "python3".to_string(),
        args: vec!["custom_mcp_server.py".to_string()],
        timeout_secs: Some(30),
    };

    println!("Custom MCP server would provide specialized tools for your domain");
    println!("Examples: database queries, API calls, custom computations, etc.");

    Ok(())
}
