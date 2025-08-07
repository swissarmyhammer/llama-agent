# Llama Agent Examples

This directory contains comprehensive examples demonstrating all aspects of the llama-agent system. These examples serve both as documentation and as integration tests to validate the functionality.

## Overview

The llama-agent system provides a Rust-based API for text chat using llama-cpp-rs with Model Context Protocol (MCP) support. It features:

- **Single Model Instance**: Shared across all requests for efficiency
- **Queue-based Concurrency**: Thread-safe request handling
- **Session-based Chat**: Context management for conversations
- **MCP Integration**: External tool capabilities
- **Streaming Support**: Real-time token generation

## Examples Directory Structure

```
examples/
├── README.md                   # This file - comprehensive guide
├── basic_usage.rs              # Core functionality example from specification
├── tool_workflow.rs            # Manual tool call handling
├── streaming.rs                # Real-time streaming responses
├── mcp_integration.rs          # Multiple MCP server integration
├── error_handling.rs           # Error scenarios and recovery
├── cli_examples.md             # Command-line usage examples
├── performance_examples.rs     # Performance optimization examples
└── integration_tests.rs        # Automated validation tests
```

## Quick Start

### Prerequisites

1. **Rust Toolchain**: Install via [rustup.rs](https://rustup.rs/)
2. **Node.js** (for MCP servers): Install via [nodejs.org](https://nodejs.org/)
3. **MCP Servers** (optional): Install filesystem server:
   ```bash
   npm install -g @modelcontextprotocol/server-filesystem
   ```

### Running Examples

Each example is a standalone Rust binary that can be run with:

```bash
# Basic usage example
cargo run --example basic_usage

# Streaming example
cargo run --example streaming

# MCP integration example
cargo run --example mcp_integration

# Error handling example
cargo run --example error_handling

# Tool workflow example
cargo run --example tool_workflow
```

## Detailed Example Descriptions

### 1. Basic Usage (`basic_usage.rs`)

**Purpose**: Implements the exact usage example from the specification (lines 605-709)

**Demonstrates**:
- AgentConfig setup with HuggingFace model loading
- Session creation with MCP server configuration
- Tool discovery and integration
- User message processing with tool calls
- Tool execution and result integration
- Follow-up generation with tool results

**Key Learning Points**:
- Configuration of ModelSource::HuggingFace
- MCP server setup and tool discovery
- Basic request/response cycle
- Automatic tool call handling

### 2. Tool Workflow (`tool_workflow.rs`)

**Purpose**: Shows detailed manual tool call handling

**Demonstrates**:
- ChatTemplateEngine usage for tool call extraction
- Manual tool execution with error handling
- Session message management
- Multi-step tool workflows

**Key Learning Points**:
- Understanding the tool call lifecycle
- Manual vs automatic tool handling
- Session state management
- Tool result integration

### 3. Streaming (`streaming.rs`)

**Purpose**: Real-time streaming response generation

**Demonstrates**:
- Stream-based token generation
- Performance comparison with batch generation
- Real-time user experience
- Stream error handling

**Key Learning Points**:
- When to use streaming vs batch
- Performance characteristics
- User experience considerations
- Stream lifecycle management

### 4. MCP Integration (`mcp_integration.rs`)

**Purpose**: Multiple MCP server integration

**Demonstrates**:
- Multiple MCP server configuration
- Tool discovery from different servers
- Error handling for MCP failures
- Health monitoring of MCP servers
- Graceful degradation strategies

**Key Learning Points**:
- MCP server lifecycle management
- Tool organization and discovery
- Fault tolerance in distributed systems
- Health monitoring patterns

### 5. Error Handling (`error_handling.rs`)

**Purpose**: Comprehensive error scenarios and recovery

**Demonstrates**:
- Model loading failures
- Invalid configuration handling
- MCP server failures
- Tool execution errors
- Timeout management
- Graceful degradation patterns

**Key Learning Points**:
- Defensive programming practices
- Error categorization and handling
- Recovery strategies
- System resilience patterns

### 6. CLI Examples (`cli_examples.md`)

**Purpose**: Command-line interface usage patterns

**Demonstrates**:
- All CLI argument combinations
- Performance tuning options
- Error scenarios and exit codes
- Production deployment patterns

**Key Learning Points**:
- CLI best practices
- Performance configuration
- Operational considerations
- User experience design

## Architecture Overview

### Core Components

```rust
// Main entry point
AgentServer::initialize(config).await?

// Session management
let session = agent.create_session().await?
agent.discover_tools(&mut session).await?

// Generation
let response = agent.generate(request).await?
let stream = agent.generate_stream(request).await?

// Tool execution
let result = agent.execute_tool(tool_call, &session).await?
```

### Configuration Hierarchy

```
AgentConfig
├── ModelConfig (model source, batch size, HF params)
├── QueueConfig (queue size, timeout, worker threads)
├── SessionConfig (max sessions, session timeout)
└── MCPServerConfig[] (name, command, args, timeout)
```

### Data Flow

```
User Input → Session → Queue → Model → Response
                ↓
            Tool Discovery → MCP Servers → Tool Results
```

## Best Practices

### Configuration

1. **Model Selection**: Use BF16 models for best quality/performance balance
2. **Batch Size**: Start with 512, adjust based on available memory
3. **Queue Size**: Set based on expected concurrent load
4. **Timeouts**: Be generous for model loading, conservative for requests

### Error Handling

1. **Validation**: Always validate configuration before initialization
2. **Graceful Degradation**: Continue operating with reduced functionality
3. **Logging**: Use tracing for observability
4. **Recovery**: Implement retry logic for transient failures

### Performance

1. **Single Model**: Reuse the same AgentServer instance
2. **Session Reuse**: Keep sessions alive for conversation context
3. **Streaming**: Use for interactive applications
4. **Tool Parallelism**: Let the system handle tool execution concurrency

### Security

1. **Input Validation**: Validate all user inputs
2. **MCP Security**: Only use trusted MCP servers
3. **Resource Limits**: Set appropriate token and time limits
4. **Network Security**: Use HTTPS where applicable

## Integration Patterns

### Web Applications

```rust
// Share AgentServer across HTTP handlers
let agent = Arc::new(AgentServer::initialize(config).await?);

// Per-request session management
async fn handle_chat(agent: Arc<AgentServer>, message: String) {
    let session = agent.create_session().await?;
    // ... handle request
}
```

### CLI Applications

```rust
// Single-use pattern
let agent = AgentServer::initialize(config).await?;
let response = agent.generate(request).await?;
println!("{}", response.generated_text);
```

### Long-running Services

```rust
// Service with health monitoring
let agent = AgentServer::initialize(config).await?;
let health = agent.health().await?;

// Graceful shutdown
tokio::select! {
    _ = signal::ctrl_c() => {
        agent.shutdown().await?;
    }
}
```

## Troubleshooting

### Common Issues

1. **Model Not Found**: Check model path/repo name, network connectivity
2. **MCP Server Failed**: Verify server installation, check logs
3. **Out of Memory**: Reduce batch_size, limit concurrent requests
4. **Slow Performance**: Check worker_threads, batch_size, hardware

### Debug Logging

Enable detailed logging:
```bash
RUST_LOG=debug cargo run --example basic_usage
```

Log levels:
- `error`: Critical failures
- `warn`: Recoverable issues
- `info`: General operations
- `debug`: Detailed diagnostics
- `trace`: Very verbose (development only)

### Performance Profiling

Monitor key metrics:
- Tokens per second
- Memory usage
- Queue depth
- Session count
- MCP server response times

## Advanced Topics

### Custom MCP Servers

Create domain-specific tools by implementing MCP servers:

```python
# custom_mcp_server.py
from mcp.server import Server
from mcp.types import Tool

server = Server()

@server.list_tools()
async def list_tools():
    return [Tool(name="custom_tool", description="Custom functionality")]

@server.call_tool()
async def call_tool(name: str, arguments: dict):
    # Implement custom logic
    return {"result": "custom response"}
```

### Model Optimization

Optimize model performance:
1. Use quantized models (Q4, Q8) for memory efficiency
2. Enable GPU acceleration if available
3. Tune batch sizes for your hardware
4. Consider model parallelism for very large models

### Scaling Considerations

For production deployment:
1. **Horizontal Scaling**: Multiple AgentServer instances
2. **Load Balancing**: Distribute requests across instances
3. **Caching**: Cache model loading and common responses
4. **Monitoring**: Comprehensive metrics and alerting

## Contributing

When adding new examples:

1. **Documentation**: Include comprehensive comments
2. **Error Handling**: Demonstrate proper error management
3. **Testing**: Ensure examples can run independently
4. **Integration**: Add to the test suite
5. **Performance**: Consider resource usage

## Resources

- [Specification](../specifications/index.md) - Complete system specification
- [llama-cpp-rs Documentation](https://docs.rs/llama-cpp-2/) - Model inference library
- [MCP Specification](https://modelcontextprotocol.io/) - Model Context Protocol
- [Rust Async Book](https://rust-lang.github.io/async-book/) - Async programming in Rust

## Support

For issues and questions:
1. Check the examples in this directory
2. Review the specification document
3. Enable debug logging for diagnostics
4. Check the integration tests for expected behavior

---

**Next Steps**: Run the basic_usage example to get started, then explore the other examples based on your specific needs.