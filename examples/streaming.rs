//! Streaming response example
//!
//! This example demonstrates how to use the streaming API to get real-time token-by-token
//! responses from the model, which is useful for interactive applications and better user experience.

use futures::StreamExt;
use llama_agent::{
    types::{
        AgentAPI, AgentConfig, GenerationRequest, Message, MessageRole, ModelConfig, ModelSource,
        QueueConfig, SessionConfig,
    },
    AgentServer,
};
use std::io::{self, Write};
use std::time::{Duration, SystemTime};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting streaming response example");

    // Create agent configuration for streaming
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
            request_timeout: Duration::from_secs(60), // Longer timeout for streaming
            worker_threads: 1,
        },
        mcp_servers: vec![], // No MCP servers for this example
        session_config: SessionConfig::default(),
    };

    println!("Initializing agent for streaming...");
    let agent = AgentServer::initialize(config).await?;

    // Create a session
    let mut session = agent.create_session().await?;
    info!("Created session: {}", session.id);

    // Add user message
    session.messages.push(Message {
        role: MessageRole::User,
        content: "Please write a detailed explanation of how machine learning works, including key concepts and examples.".to_string(),
        tool_call_id: None,
        tool_name: None,
        timestamp: SystemTime::now(),
    });

    // Create generation request for streaming
    let request = GenerationRequest {
        session: session.clone(),
        max_tokens: Some(500),
        temperature: Some(0.7),
        top_p: Some(0.9),
        stop_tokens: vec![],
    };

    println!("\nStarting streaming generation...");
    println!("Response (streaming):");
    println!("{}", "=".repeat(60));

    // Get streaming response
    let mut stream = agent.generate_stream(request).await?;

    let mut token_count = 0;
    let mut full_response = String::new();
    let start_time = std::time::Instant::now();

    // Process each chunk as it arrives
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                // Print the new text immediately (real-time streaming)
                print!("{}", chunk.text);
                io::stdout().flush()?; // Ensure immediate output

                // Accumulate for final statistics
                full_response.push_str(&chunk.text);
                token_count += chunk.token_count;

                // Check if generation is complete
                if chunk.is_complete {
                    println!("\n{}", "=".repeat(60));
                    println!("Streaming completed!");
                    break;
                }
            }
            Err(e) => {
                println!("\n❌ Streaming error: {}", e);
                break;
            }
        }
    }

    let elapsed = start_time.elapsed();

    // Display final statistics
    println!("\nStreaming Statistics:");
    println!("  Total tokens: {}", token_count);
    println!("  Total time: {:.2}s", elapsed.as_secs_f32());
    if token_count > 0 {
        println!(
            "  Tokens per second: {:.1}",
            token_count as f32 / elapsed.as_secs_f32()
        );
    }
    println!("  Response length: {} characters", full_response.len());

    // Demonstrate the difference between streaming and batch generation
    println!("\n{}", "=".repeat(60));
    println!("Comparing with batch generation for the same prompt...");

    // Create a new session with the same message
    let mut batch_session = agent.create_session().await?;
    batch_session.messages.push(Message {
        role: MessageRole::User,
        content: "Please write a detailed explanation of how machine learning works, including key concepts and examples.".to_string(),
        tool_call_id: None,
        tool_name: None,
        timestamp: SystemTime::now(),
    });

    let batch_request = GenerationRequest {
        session: batch_session,
        max_tokens: Some(500),
        temperature: Some(0.7),
        top_p: Some(0.9),
        stop_tokens: vec![],
    };

    let batch_start = std::time::Instant::now();
    let batch_response = agent.generate(batch_request).await?;
    let batch_elapsed = batch_start.elapsed();

    println!("\nBatch response received all at once:");
    println!("{}", "=".repeat(60));
    println!("{}", batch_response.generated_text);
    println!("{}", "=".repeat(60));

    // Compare performance characteristics
    println!("\nPerformance Comparison:");
    println!("Streaming:");
    println!("  Tokens: {}", token_count);
    println!("  Time: {:.2}s", elapsed.as_secs_f32());
    println!("  Time to first token: <1s (immediate)");
    println!("  User experience: Real-time, progressive");

    println!("\nBatch:");
    println!("  Tokens: {}", batch_response.tokens_generated);
    println!("  Time: {:.2}s", batch_elapsed.as_secs_f32());
    println!(
        "  Time to first token: {:.2}s (wait for complete)",
        batch_elapsed.as_secs_f32()
    );
    println!("  User experience: All-at-once, wait then complete");

    // Show use case recommendations
    println!("\nUse Case Recommendations:");
    println!("Streaming is better for:");
    println!("  - Interactive chat applications");
    println!("  - Long-form content generation");
    println!("  - Real-time user feedback");
    println!("  - Better perceived performance");

    println!("\nBatch is better for:");
    println!("  - API endpoints with complete responses");
    println!("  - Post-processing of complete text");
    println!("  - Simpler implementation");
    println!("  - When you need the complete response before proceeding");

    info!("Streaming response example completed");
    Ok(())
}
