# CLI Usage Examples

This document provides comprehensive examples of using the llama-agent-cli for different scenarios.

## Basic Usage

### HuggingFace Model with Auto-Detection

```bash
# Use HuggingFace model with auto-detection (prefers BF16 files)
llama-agent-cli --model microsoft/DialoGPT-medium --prompt "Hello, how are you?"
```

### HuggingFace Model with Specific Filename

```bash
# Use specific filename from HuggingFace repo
llama-agent-cli --model microsoft/DialoGPT-medium --filename model-bf16.gguf --prompt "What is Rust?"
```

### Local Model Folder

```bash
# Use local model folder with auto-detection
llama-agent-cli --model ./models/llama2-7b --prompt "Explain quantum computing" --limit 200
```

### Local Specific File

```bash
# Use local specific file with custom settings
llama-agent-cli --model ./models/llama2-7b --filename llama-2-7b.q4_k_m.gguf --prompt "Write a haiku" --temperature 0.8 --top-p 0.95
```

## Advanced Configuration Examples

### Custom Generation Parameters

```bash
# Use custom temperature and top-p for more creative responses
llama-agent-cli \
  --model microsoft/DialoGPT-medium \
  --prompt "Write a creative short story about AI" \
  --temperature 0.9 \
  --top-p 0.95 \
  --limit 1000
```

### Performance Tuning

```bash
# Optimize for performance with larger batch sizes and multiple workers
llama-agent-cli \
  --model ./models/llama2-7b \
  --prompt "Summarize the key concepts of machine learning" \
  --batch-size 1024 \
  --worker-threads 2 \
  --max-queue-size 50 \
  --request-timeout 180
```

### Session Management

```bash
# Configure session handling for longer conversations
llama-agent-cli \
  --model microsoft/DialoGPT-medium \
  --prompt "Let's have a long conversation about philosophy" \
  --max-sessions 5 \
  --session-timeout 7200 \
  --limit 500
```

## Specific Model Examples

### Small/Fast Models

```bash
# For quick testing with smaller models
llama-agent-cli \
  --model microsoft/DialoGPT-small \
  --prompt "Quick test" \
  --limit 50 \
  --batch-size 256 \
  --temperature 0.5
```

### Large/Quality Models

```bash
# For high-quality responses with larger models
llama-agent-cli \
  --model microsoft/DialoGPT-large \
  --prompt "Write a detailed technical explanation" \
  --limit 2000 \
  --batch-size 1024 \
  --temperature 0.3 \
  --request-timeout 300
```

## Local Development Examples

### Using Development Models

```bash
# Test with local development model
llama-agent-cli \
  --model ./dev-models/custom-model \
  --filename custom-model-q4.gguf \
  --prompt "Test development features" \
  --limit 100
```

### Debug Mode with Verbose Output

```bash
# Enable verbose logging for debugging
RUST_LOG=debug llama-agent-cli \
  --model microsoft/DialoGPT-medium \
  --prompt "Debug this response generation" \
  --limit 200
```

## Production Examples

### Batch Processing

```bash
# Process multiple prompts efficiently
llama-agent-cli \
  --model microsoft/DialoGPT-medium \
  --prompt "First batch item" \
  --batch-size 512 \
  --worker-threads 4 \
  --max-queue-size 100
```

### Reliable Processing

```bash
# Configure for reliable processing with timeouts and error handling
llama-agent-cli \
  --model ./models/production-model \
  --filename stable-model.gguf \
  --prompt "Process important request" \
  --request-timeout 120 \
  --temperature 0.2 \
  --limit 1000
```

## Error Handling Examples

### Invalid Model Path

```bash
# This will show validation error (exit code 2)
llama-agent-cli --model nonexistent/model --prompt "test"
```

### Model Loading Error

```bash
# This will show model loading error (exit code 3)
llama-agent-cli --model ./invalid-model-path --prompt "test"
```

### Parameter Validation

```bash
# This will show parameter validation error (exit code 2)
llama-agent-cli --model microsoft/DialoGPT-medium --prompt "test" --temperature 2.5
```

## Exit Codes

- **0**: Success
- **1**: General runtime error
- **2**: Validation error (invalid parameters, paths, etc.)
- **3**: Model loading error

## Performance Tips

1. **Batch Size**: Larger batch sizes (512-1024) generally provide better throughput
2. **Worker Threads**: Use 1 worker thread unless you have a very powerful GPU
3. **Temperature**: Lower values (0.1-0.3) for deterministic responses, higher (0.7-0.9) for creative responses
4. **Request Timeout**: Increase for complex prompts or slower hardware
5. **Token Limit**: Set appropriate limits to prevent excessive generation costs

## Common Patterns

### Code Generation

```bash
llama-agent-cli \
  --model microsoft/DialoGPT-medium \
  --prompt "Write a Rust function to sort a vector" \
  --temperature 0.2 \
  --limit 300
```

### Creative Writing

```bash
llama-agent-cli \
  --model microsoft/DialoGPT-medium \
  --prompt "Write a poem about technology" \
  --temperature 0.8 \
  --top-p 0.9 \
  --limit 500
```

### Question Answering

```bash
llama-agent-cli \
  --model microsoft/DialoGPT-medium \
  --prompt "Explain how neural networks work" \
  --temperature 0.3 \
  --limit 800
```

### Summarization

```bash
llama-agent-cli \
  --model microsoft/DialoGPT-medium \
  --prompt "Summarize the key points of: [long text here]" \
  --temperature 0.1 \
  --limit 200
```