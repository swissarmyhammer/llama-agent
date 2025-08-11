# llama-cli

Unified command-line interface for LLaMA text generation and embedding.

## Installation

```bash
cargo install llama-cli
```

Or build from source:
```bash
git clone https://github.com/your-org/llama-agent.git
cd llama-agent
cargo build --release --bin llama-cli
```

## Commands

### generate
Generate text using language models:

```bash
llama-cli generate --model Qwen/Qwen2.5-7B-Instruct-GGUF --prompt "Hello world"
```

**Options:**
- `--model, -m`: Model identifier (HuggingFace repo or local path)
- `--prompt, -p`: Input prompt for generation
- `--max-tokens`: Maximum number of tokens to generate (default: 100)
- `--temperature`: Sampling temperature (default: 0.7)
- `--top-p`: Top-p sampling parameter (default: 0.9)
- `--stream`: Enable streaming output
- `--system`: System message for chat models

### embed  
Generate embeddings for text files:

```bash
llama-cli embed \
  --model Qwen/Qwen3-Embedding-0.6B-GGUF \
  --input texts.txt \
  --output embeddings.parquet \
  --batch-size 32 \
  --normalize
```

**Options:**
- `--model, -m`: Embedding model identifier
- `--input, -i`: Input text file (one text per line)
- `--output, -o`: Output Parquet file path
- `--batch-size`: Processing batch size (default: 32)
- `--normalize`: L2 normalize embeddings
- `--max-length`: Maximum sequence length

## Usage Examples

### Text Generation

#### Simple Generation
```bash
llama-cli generate \
  --model Qwen/Qwen2.5-7B-Instruct-GGUF \
  --prompt "Explain quantum computing in simple terms"
```

#### Chat with System Message
```bash
llama-cli generate \
  --model Qwen/Qwen2.5-7B-Instruct-GGUF \
  --system "You are a helpful coding assistant" \
  --prompt "Write a function to reverse a string in Python"
```

#### Streaming Output
```bash
llama-cli generate \
  --model Qwen/Qwen2.5-7B-Instruct-GGUF \
  --prompt "Tell me a story about AI" \
  --stream \
  --max-tokens 200
```

### Text Embedding

#### Basic Embedding
```bash
# Create input file
echo -e "Hello world\nThis is a test\nAnother line" > texts.txt

# Generate embeddings
llama-cli embed \
  --model Qwen/Qwen3-Embedding-0.6B-GGUF \
  --input texts.txt \
  --output embeddings.parquet
```

#### Optimized Batch Processing
```bash
llama-cli embed \
  --model Qwen/Qwen3-Embedding-0.6B-GGUF \
  --input large_texts.txt \
  --output large_embeddings.parquet \
  --batch-size 64 \
  --normalize \
  --max-length 512
```

#### Processing with Progress
```bash
llama-cli embed \
  --model Qwen/Qwen3-Embedding-0.6B-GGUF \
  --input dataset.txt \
  --output dataset_embeddings.parquet \
  --batch-size 32 \
  --debug  # Shows progress information
```

## Configuration

### Environment Variables

- `LLAMA_CACHE_DIR`: Custom model cache directory
- `LLAMA_CACHE_MAX_SIZE`: Maximum cache size in bytes
- `RUST_LOG`: Logging level (debug, info, warn, error)

### Model Sources

#### HuggingFace Models
```bash
llama-cli generate --model "Qwen/Qwen2.5-7B-Instruct-GGUF"
llama-cli embed --model "sentence-transformers/all-MiniLM-L6-v2"
```

#### Local Models
```bash
llama-cli generate --model "/path/to/local/model.gguf"
llama-cli embed --model "./models/embedding-model.gguf"
```

## Output Formats

### Generation Output
Text generation outputs to stdout by default. Use shell redirection for files:
```bash
llama-cli generate -m model -p "prompt" > output.txt
```

### Embedding Output
Embeddings are saved in Apache Parquet format, readable with:

#### Python (Pandas)
```python
import pandas as pd
df = pd.read_parquet("embeddings.parquet")
print(df.head())
```

#### Rust (Polars)
```rust
use polars::prelude::*;
let df = LazyFrame::scan_parquet("embeddings.parquet", ScanArgsParquet::default())?;
```

## Performance Tips

### Generation
- Use streaming (`--stream`) for interactive feel
- Adjust `--temperature` and `--top-p` for creativity vs consistency
- Use `--max-tokens` to control output length

### Embedding
- **Batch Size**: Start with 32, increase for better throughput if memory allows
- **Normalization**: Use `--normalize` for similarity search applications
- **Max Length**: Use `--max-length` to truncate very long texts
- **Caching**: First model load downloads, subsequent runs use cache

## Error Handling

Common errors and solutions:

- **Model not found**: Check model name spelling and network connectivity
- **Out of memory**: Reduce batch size or use smaller model
- **File not found**: Verify input file path exists
- **Permission denied**: Check write permissions for output directory

## Integration

The CLI uses the same model cache as the `llama-agent` library, enabling:
- Shared cache between generation and embedding
- Efficient model reuse across tools
- Consistent model management

## Development

### Running from Source
```bash
cargo run --bin llama-cli -- generate --model model-name --prompt "test"
cargo run --bin llama-cli -- embed --model model-name --input test.txt --output out.parquet
```

### Testing
```bash
cargo test --bin llama-cli
```