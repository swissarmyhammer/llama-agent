# Llama Agent Examples and Documentation

This document provides a comprehensive overview of the examples and documentation created for the llama-agent system.

## 🎯 Project Overview

The llama-agent system is a Rust-based Agent API that provides text chat capabilities using llama-cpp-rs for model inference, with MCP (Model Context Protocol) client support. The system is designed around a single in-memory model instance accessed through a queue-based architecture to handle concurrent requests efficiently.

## 📁 Examples Structure

All examples are located in the `/examples` directory and demonstrate the complete functionality of the system:

```
examples/
├── README.md                   # Comprehensive examples guide
├── basic_usage.rs              # Core functionality (specification lines 605-709)
├── tool_workflow.rs            # Manual tool call handling
├── streaming.rs                # Real-time streaming responses
├── mcp_integration.rs          # Multiple MCP server integration
├── error_handling.rs           # Error scenarios and recovery
├── performance_examples.rs     # Performance optimization
├── integration_tests.rs        # Automated validation tests
└── cli_examples.md             # Command-line usage examples
```

## 🚀 Running Examples

Each example can be run independently:

```bash
# Core functionality demonstration
cargo run --example basic_usage

# Streaming response example
cargo run --example streaming

# MCP integration with multiple servers
cargo run --example mcp_integration

# Error handling patterns
cargo run --example error_handling

# Manual tool workflow
cargo run --example tool_workflow

# Performance optimization strategies
cargo run --example performance_examples

# Integration test suite
cargo run --example integration_tests
```

## 📖 Example Details

### 1. Basic Usage (`basic_usage.rs`)
- **Purpose**: Implements the exact example from specification lines 605-709
- **Features**: AgentConfig setup, session creation, tool discovery, tool execution
- **Learning**: Core system workflow and MCP integration

### 2. Tool Workflow (`tool_workflow.rs`)
- **Purpose**: Manual tool call handling demonstration
- **Features**: ChatTemplateEngine usage, manual tool execution, session management
- **Learning**: Deep understanding of tool call lifecycle

### 3. Streaming (`streaming.rs`)
- **Purpose**: Real-time token-by-token response generation
- **Features**: Stream handling, performance comparison, user experience optimization
- **Learning**: When and how to use streaming vs batch processing

### 4. MCP Integration (`mcp_integration.rs`)
- **Purpose**: Multiple MCP server integration patterns
- **Features**: Multiple server configuration, health monitoring, graceful degradation
- **Learning**: Production-ready MCP server management

### 5. Error Handling (`error_handling.rs`)
- **Purpose**: Comprehensive error scenarios and recovery strategies
- **Features**: Configuration validation, graceful degradation, retry patterns
- **Learning**: Building resilient applications

### 6. Performance Examples (`performance_examples.rs`)
- **Purpose**: Performance optimization techniques and benchmarking
- **Features**: Configuration tuning, memory optimization, concurrent processing
- **Learning**: Performance tuning for different workloads

### 7. Integration Tests (`integration_tests.rs`)
- **Purpose**: Automated validation of all example functionality
- **Features**: Comprehensive test coverage, validation patterns, error checking
- **Learning**: Testing strategies for distributed systems

### 8. CLI Examples (`cli_examples.md`)
- **Purpose**: Command-line interface usage patterns
- **Features**: All CLI combinations, performance tuning, deployment patterns
- **Learning**: Production CLI application usage

## 🏗️ Architecture Demonstrated

The examples showcase the complete system architecture:

### Core Components
- **AgentServer**: Main entry point and orchestration
- **ModelManager**: Model loading and lifecycle management
- **RequestQueue**: Thread-safe concurrent request handling
- **SessionManager**: Conversation context management
- **MCPClient**: External tool integration
- **ChatTemplateEngine**: Tool call parsing and rendering

### Data Flow
```
User Input → Session → Queue → Model → Response
                ↓
            Tool Discovery → MCP Servers → Tool Results
```

### Configuration Hierarchy
```
AgentConfig
├── ModelConfig (HuggingFace/Local, batch size, parameters)
├── QueueConfig (size, timeout, worker threads)
├── SessionConfig (max sessions, timeouts)
└── MCPServerConfig[] (name, command, args)
```

## 🛠️ Prerequisites

### Required
- Rust toolchain (1.70+)
- tokio async runtime
- Internet connection (for HuggingFace models)

### Optional (for full functionality)
- Node.js (for MCP servers)
- MCP servers:
  ```bash
  npm install -g @modelcontextprotocol/server-filesystem
  npm install -g @modelcontextprotocol/server-brave-search
  ```

## 📊 Validation and Testing

The examples serve as both documentation and integration tests:

### Automated Testing
- Configuration validation
- Error handling verification
- Type system correctness
- API contract validation

### Manual Testing
- Real model loading (requires models)
- MCP server integration (requires servers)
- Performance benchmarking
- User experience validation

## 🎯 Use Cases Demonstrated

### Interactive Applications
- Streaming chat interfaces
- Real-time response generation
- Progressive user experience

### API Services
- Batch processing
- RESTful endpoints
- Scalable architectures

### Tool Integration
- Filesystem operations
- Web search capabilities
- Custom tool development

### Production Deployment
- Error handling and recovery
- Performance optimization
- Health monitoring
- Graceful degradation

## 🔧 Configuration Patterns

### Development
```rust
AgentConfig {
    model: ModelSource::Local { /* fast local model */ },
    batch_size: 256,
    queue_config: QueueConfig { /* minimal settings */ },
    mcp_servers: vec![], // No external dependencies
}
```

### Production
```rust
AgentConfig {
    model: ModelSource::HuggingFace { /* quality model */ },
    batch_size: 1024,
    queue_config: QueueConfig { /* scaled settings */ },
    mcp_servers: vec![/* essential tools only */],
}
```

### High Performance
```rust
AgentConfig {
    model: ModelSource::Local { /* optimized model */ },
    batch_size: 1024,
    worker_threads: 1, // Single model instance
    max_queue_size: 1000, // High throughput
}
```

## 🚨 Error Handling Patterns

The examples demonstrate comprehensive error handling:

1. **Configuration Validation**: Early detection of invalid settings
2. **Graceful Degradation**: Continue operation with reduced functionality
3. **Retry Logic**: Handle transient failures
4. **Resource Management**: Proper cleanup and shutdown
5. **User Communication**: Clear error messages and recovery guidance

## 📈 Performance Considerations

### Memory Usage
- Model size selection (Small: ~100MB, Medium: ~1GB, Large: 4GB+)
- Batch size optimization
- Session management
- Tool result caching

### Throughput Optimization
- Single model instance sharing
- Queue-based request handling
- Parallel tool execution
- Streaming for perceived performance

### Latency Minimization
- Local model preference
- Minimal MCP servers
- Aggressive timeouts
- Pre-loaded models

## 🔮 Future Enhancements

The examples provide a foundation for:

1. **Multi-model Support**: Different models for different tasks
2. **Model Quantization**: Memory-efficient model formats
3. **GPU Acceleration**: Hardware-accelerated inference
4. **Distributed Deployment**: Multi-instance architectures
5. **Advanced Caching**: Intelligent response caching
6. **Custom MCP Servers**: Domain-specific tools

## 🎉 Success Criteria

All examples have been successfully implemented and validated:

✅ **Specification Compliance**: Exact implementation of specification example  
✅ **Compilation**: All examples compile without errors  
✅ **Documentation**: Comprehensive guides and inline documentation  
✅ **Error Handling**: Robust error scenarios and recovery  
✅ **Performance**: Multiple optimization strategies demonstrated  
✅ **Integration**: Real MCP server integration patterns  
✅ **Testing**: Automated validation and manual testing procedures  
✅ **CLI Usage**: Complete command-line interface examples  

## 📚 References

- [Core Specification](./specifications/index.md) - Complete system specification
- [llama-cpp-rs](https://docs.rs/llama-cpp-2/) - Rust bindings for llama.cpp
- [Model Context Protocol](https://modelcontextprotocol.io/) - MCP specification
- [Rust Async Book](https://rust-lang.github.io/async-book/) - Async programming guide

---

**The llama-agent examples provide a complete reference implementation demonstrating all aspects of the system, from basic usage to production deployment patterns. They serve as both documentation and integration tests, ensuring the system works as specified and providing clear guidance for developers building applications with llama-agent.**