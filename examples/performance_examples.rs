//! Performance Optimization Examples
//!
//! This example demonstrates various performance optimization techniques
//! for the llama-agent system, including:
//!
//! - Model loading optimization
//! - Batch processing strategies
//! - Memory usage optimization
//! - Concurrent request handling
//! - Streaming vs batch performance
//! - Configuration tuning for different workloads

use llama_agent::{
    types::{
        AgentAPI, AgentConfig, GenerationRequest, Message, MessageRole, ModelConfig, ModelSource,
        QueueConfig, SessionConfig,
    },
    AgentServer,
};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::Semaphore;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting performance optimization examples");

    println!("Performance Optimization Examples");
    println!("{}", "=".repeat(60));

    // Example 1: Configuration optimization
    demonstrate_configuration_optimization().await?;

    // Example 2: Batch processing
    demonstrate_batch_processing().await?;

    // Example 3: Memory optimization
    demonstrate_memory_optimization().await?;

    // Example 4: Concurrent processing
    demonstrate_concurrent_processing().await?;

    // Example 5: Streaming performance
    demonstrate_streaming_performance().await?;

    // Example 6: Benchmark different configurations
    benchmark_configurations().await?;

    println!("\n✓ All performance examples completed");
    info!("Performance optimization examples completed");
    Ok(())
}

async fn demonstrate_configuration_optimization() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n1. Configuration Optimization");
    println!("{}", "-".repeat(40));

    println!("Optimal configurations for different use cases:");

    // High-throughput configuration
    println!("\n• High-throughput configuration:");
    let high_throughput_config = AgentConfig {
        model: ModelConfig {
            source: ModelSource::HuggingFace {
                repo: "microsoft/DialoGPT-medium".to_string(),
                filename: None,
            },
            batch_size: 1024, // Large batch for throughput
            use_hf_params: true,
        },
        queue_config: QueueConfig {
            max_queue_size: 1000,                      // Large queue
            request_timeout: Duration::from_secs(180), // Generous timeout
            worker_threads: 1,                         // Single worker for memory efficiency
        },
        mcp_servers: vec![], // Minimal MCP servers
        session_config: SessionConfig {
            max_sessions: 10000,                        // High session limit
            session_timeout: Duration::from_secs(1800), // 30 minutes
        },
    };

    print_config_summary("High Throughput", &high_throughput_config);

    // Low-latency configuration
    println!("\n• Low-latency configuration:");
    let low_latency_config = AgentConfig {
        model: ModelConfig {
            source: ModelSource::Local {
                folder: std::path::PathBuf::from("./models/fast"),
                filename: Some("small-model.gguf".to_string()),
            },
            batch_size: 256,      // Smaller batch for faster response
            use_hf_params: false, // Skip network calls
        },
        queue_config: QueueConfig {
            max_queue_size: 100,                      // Smaller queue
            request_timeout: Duration::from_secs(30), // Tight timeout
            worker_threads: 1,
        },
        mcp_servers: vec![], // No MCP for minimal latency
        session_config: SessionConfig {
            max_sessions: 1000,
            session_timeout: Duration::from_secs(600), // 10 minutes
        },
    };

    print_config_summary("Low Latency", &low_latency_config);

    // Memory-efficient configuration
    println!("\n• Memory-efficient configuration:");
    let memory_efficient_config = AgentConfig {
        model: ModelConfig {
            source: ModelSource::HuggingFace {
                repo: "microsoft/DialoGPT-small".to_string(), // Smaller model
                filename: None,
            },
            batch_size: 128, // Small batch size
            use_hf_params: true,
        },
        queue_config: QueueConfig {
            max_queue_size: 50, // Small queue
            request_timeout: Duration::from_secs(60),
            worker_threads: 1,
        },
        mcp_servers: vec![],
        session_config: SessionConfig {
            max_sessions: 100,                         // Low session count
            session_timeout: Duration::from_secs(300), // 5 minutes
        },
    };

    print_config_summary("Memory Efficient", &memory_efficient_config);

    Ok(())
}

async fn demonstrate_batch_processing() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n2. Batch Processing Strategies");
    println!("{}", "-".repeat(40));

    println!("Batch processing improves throughput by processing multiple");
    println!("requests together. Here are different strategies:");

    println!("\n• Sequential Processing:");
    println!("  - Process one request at a time");
    println!("  - Simple but slower");
    println!("  - Good for complex requests");

    println!("\n• Concurrent Processing:");
    println!("  - Multiple requests to same model");
    println!("  - Limited by single model instance");
    println!("  - Good for I/O bound operations");

    println!("\n• Request Batching:");
    println!("  - Group similar requests");
    println!("  - Process in single model call");
    println!("  - Best throughput");

    // Simulate batch processing performance
    let requests = vec![
        "Explain machine learning",
        "What is Rust programming?",
        "Describe neural networks",
        "How does tokenization work?",
        "What is transformer architecture?",
    ];

    println!("\nSimulated batch processing metrics:");
    println!("Requests: {}", requests.len());

    // Sequential timing
    let sequential_time = Duration::from_millis(requests.len() as u64 * 2000); // 2s per request
    println!(
        "Sequential processing: {:.1}s",
        sequential_time.as_secs_f32()
    );

    // Concurrent timing
    let concurrent_time = Duration::from_millis(2000); // All at once, limited by longest
    println!(
        "Concurrent processing: {:.1}s",
        concurrent_time.as_secs_f32()
    );

    // Batched timing
    let batched_time = Duration::from_millis(3000); // Batch overhead but efficient
    println!("Batched processing: {:.1}s", batched_time.as_secs_f32());

    println!(
        "Throughput improvement: {:.1}x",
        sequential_time.as_secs_f32() / batched_time.as_secs_f32()
    );

    Ok(())
}

async fn demonstrate_memory_optimization() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n3. Memory Optimization");
    println!("{}", "-".repeat(40));

    println!("Memory optimization strategies:");

    println!("\n• Model Size Selection:");
    println!("  - Small models: 100MB-1GB (fast, less capable)");
    println!("  - Medium models: 1GB-4GB (balanced)");
    println!("  - Large models: 4GB+ (high quality, slow)");

    println!("\n• Quantization:");
    println!("  - FP16: 50% memory reduction, minimal quality loss");
    println!("  - INT8: 75% memory reduction, slight quality loss");
    println!("  - INT4: 87% memory reduction, noticeable quality loss");

    println!("\n• Session Management:");
    println!("  - Limit max_sessions based on available memory");
    println!("  - Set appropriate session timeouts");
    println!("  - Clean up old sessions proactively");

    println!("\n• Batch Size Tuning:");
    println!("  - Larger batches: better throughput, more memory");
    println!("  - Smaller batches: less memory, more overhead");
    println!("  - Find sweet spot for your hardware");

    // Memory estimation example
    println!("\nMemory Usage Estimation:");
    let model_sizes = vec![
        ("DialoGPT-small", 117),
        ("DialoGPT-medium", 345),
        ("DialoGPT-large", 762),
    ];

    for (name, size_mb) in model_sizes {
        let batch_overhead = 50; // MB per batch
        let session_overhead = 10; // MB per 100 sessions

        println!(
            "  {}: ~{}MB + {}MB/batch + {}MB/100sessions",
            name, size_mb, batch_overhead, session_overhead
        );
    }

    println!("\nMemory optimization tips:");
    println!("  - Use local models to avoid download memory spikes");
    println!("  - Monitor memory usage with system tools");
    println!("  - Set swap space for memory pressure relief");
    println!("  - Consider model quantization for memory-constrained environments");

    Ok(())
}

async fn demonstrate_concurrent_processing() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n4. Concurrent Processing");
    println!("{}", "-".repeat(40));

    println!("Concurrent processing patterns for handling multiple requests:");

    // Simulate concurrent request handling
    println!("\nConcurrency patterns:");

    println!("\n• Semaphore-based limiting:");
    println!("  - Limit concurrent requests to prevent overload");
    println!("  - Good for protecting shared resources");

    let semaphore = Arc::new(Semaphore::new(3)); // Max 3 concurrent
    println!(
        "  Example: Max {} concurrent requests",
        semaphore.available_permits()
    );

    println!("\n• Queue-based processing:");
    println!("  - Built into llama-agent RequestQueue");
    println!("  - FIFO processing with configurable workers");
    println!("  - Automatic backpressure handling");

    println!("\n• Session-per-user:");
    println!("  - Each user gets persistent session");
    println!("  - Maintains conversation context");
    println!("  - Memory overhead per session");

    // Concurrency best practices
    println!("\nConcurrency Best Practices:");
    println!("  1. Use single AgentServer instance across threads");
    println!("  2. Create sessions per conversation, not per request");
    println!("  3. Set appropriate queue_config.max_queue_size");
    println!("  4. Monitor queue depth for capacity planning");
    println!("  5. Use timeouts to prevent resource starvation");

    // Demonstrate concurrency patterns (conceptual)
    println!("\nConcurrency Implementation Example:");
    println!("```rust");
    println!("// Shared agent across handlers");
    println!("let agent = Arc::new(AgentServer::initialize(config).await?);");
    println!();
    println!("// Concurrent request handler");
    println!("async fn handle_request(agent: Arc<AgentServer>, request: UserRequest) {{");
    println!("    let session = agent.create_session().await?;");
    println!("    let response = agent.generate(generation_request).await?;");
    println!("    // Handle response");
    println!("}}");
    println!();
    println!("// Spawn multiple handlers");
    println!("for request in requests {{");
    println!("    let agent = agent.clone();");
    println!("    tokio::spawn(async move {{");
    println!("        handle_request(agent, request).await");
    println!("    }});");
    println!("}}");
    println!("```");

    Ok(())
}

async fn demonstrate_streaming_performance() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n5. Streaming Performance");
    println!("{}", "-".repeat(40));

    println!("Streaming vs Batch performance characteristics:");

    println!("\n• Time to First Token (TTFT):");
    println!("  - Streaming: <100ms (immediate start)");
    println!("  - Batch: Full generation time (wait for complete)");

    println!("\n• Total Generation Time:");
    println!("  - Streaming: Same as batch (same computation)");
    println!("  - Batch: Same as streaming (same computation)");

    println!("\n• User Experience:");
    println!("  - Streaming: Progressive, responsive");
    println!("  - Batch: All-at-once, perceived as faster when complete");

    println!("\n• Resource Usage:");
    println!("  - Streaming: Constant memory, network buffering");
    println!("  - Batch: Peak memory for full response");

    println!("\n• Use Case Optimization:");
    println!("  Interactive Chat → Streaming");
    println!("  API Responses → Batch");
    println!("  Long Content → Streaming");
    println!("  Post-processing → Batch");

    // Performance comparison simulation
    let token_count = 500;
    let tokens_per_second = 25.0;
    let total_time = token_count as f32 / tokens_per_second;

    println!(
        "\nPerformance Simulation ({} tokens at {} tok/s):",
        token_count, tokens_per_second
    );
    println!("  Streaming TTFT: 0.1s");
    println!("  Streaming Total: {:.1}s", total_time);
    println!("  Batch TTFT: {:.1}s", total_time);
    println!("  Batch Total: {:.1}s", total_time);
    println!("  User Satisfaction: Streaming > Batch for long responses");

    Ok(())
}

async fn benchmark_configurations() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n6. Configuration Benchmarking");
    println!("{}", "-".repeat(40));

    println!("Benchmarking different configurations:");
    println!("(Note: These are simulated results for demonstration)");

    let configs = vec![
        ("Small/Fast", 128, 1, Duration::from_secs(30), 45.0),
        ("Medium/Balanced", 512, 1, Duration::from_secs(60), 28.0),
        ("Large/Quality", 1024, 1, Duration::from_secs(120), 15.0),
        ("Concurrent", 256, 2, Duration::from_secs(45), 35.0),
    ];

    println!(
        "\n{:<15} {:<10} {:<8} {:<12} {:<12}",
        "Config", "Batch", "Workers", "Timeout", "Tok/s"
    );
    println!("{}", "-".repeat(60));

    for (name, batch_size, workers, timeout, tokens_per_sec) in configs {
        println!(
            "{:<15} {:<10} {:<8} {:<12} {:<12.1}",
            name,
            batch_size,
            workers,
            format!("{}s", timeout.as_secs()),
            tokens_per_sec
        );
    }

    println!("\nBenchmarking Methodology:");
    println!("  1. Use consistent hardware and model");
    println!("  2. Test with realistic workloads");
    println!("  3. Measure multiple metrics:");
    println!("     - Tokens per second");
    println!("     - Memory usage");
    println!("     - Latency percentiles");
    println!("     - Error rates");
    println!("  4. Test under different loads");
    println!("  5. Monitor system resources");

    println!("\nKey Performance Metrics to Track:");
    println!("  • Throughput: tokens/second, requests/second");
    println!("  • Latency: TTFT, total response time");
    println!("  • Resource Usage: CPU, memory, GPU utilization");
    println!("  • Reliability: error rates, timeout rates");
    println!("  • Scalability: performance vs. load");

    Ok(())
}

fn print_config_summary(name: &str, config: &AgentConfig) {
    println!("  {}: ", name);
    println!("    Batch size: {}", config.model.batch_size);
    println!("    Queue size: {}", config.queue_config.max_queue_size);
    println!("    Worker threads: {}", config.queue_config.worker_threads);
    println!(
        "    Request timeout: {}s",
        config.queue_config.request_timeout.as_secs()
    );
    println!("    Max sessions: {}", config.session_config.max_sessions);
    println!("    MCP servers: {}", config.mcp_servers.len());
}

#[allow(dead_code)]
async fn benchmark_real_performance(
    agent: &AgentServer,
    test_prompts: Vec<String>,
    iterations: usize,
) -> Result<BenchmarkResults, Box<dyn std::error::Error>> {
    let mut results = BenchmarkResults::default();

    for i in 0..iterations {
        let start = Instant::now();

        for prompt in &test_prompts {
            let mut session = agent.create_session().await?;
            session.messages.push(Message {
                role: MessageRole::User,
                content: prompt.clone(),
                tool_call_id: None,
                tool_name: None,
                timestamp: SystemTime::now(),
            });

            let request = GenerationRequest {
                session_id: session.id.clone(),
                max_tokens: Some(100),
                temperature: Some(0.7),
                top_p: Some(0.9),
                stop_tokens: vec![],
            };

            let response = agent.generate(request).await?;
            results.total_tokens += response.tokens_generated;
        }

        results.total_time += start.elapsed();
        results.iterations = i + 1;
    }

    results.calculate_averages();
    Ok(results)
}

#[derive(Default)]
struct BenchmarkResults {
    total_tokens: u32,
    total_time: Duration,
    iterations: usize,
    tokens_per_second: f32,
    average_time_per_request: Duration,
}

impl BenchmarkResults {
    fn calculate_averages(&mut self) {
        if self.iterations > 0 {
            self.tokens_per_second = self.total_tokens as f32 / self.total_time.as_secs_f32();
            self.average_time_per_request = self.total_time / self.iterations as u32;
        }
    }
}
