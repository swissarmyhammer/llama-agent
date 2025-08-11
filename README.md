# Llama Agent

A high-performance, async Rust agent framework for LLaMA models with embedding support.

## Features

### Text Generation
- High-performance LLaMA model integration
- Async streaming support  
- MCP (Model Context Protocol) integration
- Session management and validation
- Configurable stopping criteria

### Text Embedding (New!)
- Batch text embedding with configurable batch sizes
- Apache Parquet output format
- Shared model caching between generation and embedding
- Support for various embedding models (Qwen, etc.)
- Streaming processing for large datasets

## Installation

```bash
cargo install llama-cli
```

## Usage

### Text Generation
```bash
llama-cli generate --model Qwen/Qwen2.5-7B-Instruct-GGUF --prompt "Hello world"
```

### Text Embedding
```bash
llama-cli embed --model Qwen/Qwen3-Embedding-0.6B-GGUF --input texts.txt --output embeddings.parquet
```

## Architecture

- **llama-agent**: Core agent framework and generation logic
- **llama-loader**: Shared model loading with caching (HuggingFace + local)
- **llama-embedding**: Batch text embedding library
- **llama-cli**: Unified CLI for both generation and embedding

## Development

### Building from Source
```bash
git clone https://github.com/your-org/llama-agent.git
cd llama-agent
cargo build --release
```

### Running Tests
```bash
cargo test
```

### Running Examples
```bash
cargo run --example basic_usage
cargo run --example embedding_usage
```

## Performance

- Generation: High-performance streaming with configurable stopping
- Embedding: 20-50 texts/second (model and hardware dependent)
- Memory: Efficient batch processing with memory scaling based on batch size
- Caching: Shared model cache across generation and embedding operations

## License

Licensed under either of

* Apache License, Version 2.0
* MIT license

at your option.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.