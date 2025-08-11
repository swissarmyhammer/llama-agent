# Performance Guide

This guide provides optimization strategies and performance characteristics for the llama-agent framework.

## Text Generation Performance

### Model Loading
- **First Load**: 5-30 seconds (downloads model if not cached)
- **Subsequent Loads**: 2-10 seconds (loads from cache)
- **Memory Usage**: 4-20GB depending on model size

### Generation Speed
- **Small Models (1-3B)**: 20-50 tokens/second
- **Medium Models (7B)**: 10-25 tokens/second  
- **Large Models (13B+)**: 5-15 tokens/second

*Speeds vary significantly based on hardware (CPU, GPU, memory bandwidth)*

### Optimization Tips

#### Hardware Acceleration
```bash
# Enable Metal acceleration on macOS (default)
export GGML_METAL=1

# Enable CUDA on Linux/Windows with compatible GPU
export GGML_CUDA=1
```

#### CPU Optimization
```bash
# Use all CPU cores (default behavior)
export OMP_NUM_THREADS=8

# For better single-request performance on high-core systems
export OMP_NUM_THREADS=4
```

#### Memory Management
```bash
# Increase model cache size (default: 50GB)
export LLAMA_CACHE_MAX_SIZE=107374182400  # 100GB

# Use SSD for cache directory
export LLAMA_CACHE_DIR=/path/to/fast/ssd/cache
```

## Text Embedding Performance

### Throughput Characteristics
- **Typical Range**: 20-50 texts/second
- **Batch Size Impact**: Larger batches = higher throughput
- **Hardware Dependent**: GPU acceleration provides significant speedup

### Batch Size Guidelines

| Batch Size | Memory Usage | Throughput | Use Case |
|------------|--------------|------------|----------|
| 8-16       | Low (2-4GB)  | Moderate   | Resource-constrained |
| 32-64      | Medium (4-8GB) | High     | Recommended |
| 128+       | High (8GB+)  | Very High  | High-memory systems |

### Performance Benchmarks

Based on Qwen/Qwen3-Embedding-0.6B-GGUF model:

| Dataset Size | Batch Size | Time | Throughput |
|--------------|------------|------|------------|
| 1,000 texts  | 32         | 25s  | 40 texts/s |
| 10,000 texts | 64         | 4min | 42 texts/s |
| 100,000 texts| 128        | 38min| 44 texts/s |

### Memory Scaling

Memory usage scales with **batch size**, not dataset size:
- Batch 16: ~3GB peak memory
- Batch 64: ~6GB peak memory
- Batch 128: ~12GB peak memory

Large datasets (100k+ texts) use constant memory through streaming.

### Optimization Strategies

#### Batch Size Tuning
```bash
# Start with recommended size
llama-cli embed --batch-size 32 --input data.txt --output out.parquet

# Increase if you have memory (better throughput)
llama-cli embed --batch-size 64 --input data.txt --output out.parquet

# Decrease if you hit memory limits  
llama-cli embed --batch-size 16 --input data.txt --output out.parquet
```

#### Text Length Management
```bash
# Truncate very long texts for consistent performance
llama-cli embed --max-length 512 --input data.txt --output out.parquet
```

#### Progress Monitoring
```bash
# Enable progress tracking for large datasets
RUST_LOG=info llama-cli embed --input large_data.txt --output out.parquet
```

## Model Caching Performance

### Cache Architecture
- **Shared Cache**: Models cached once, used by both generation and embedding
- **LRU Eviction**: Least recently used models removed when cache full
- **Integrity Checks**: MD5 validation ensures cache consistency

### Cache Hit Benefits
- **First Load**: Full download + validation (slow)
- **Cache Hit**: Direct file loading (fast)
- **Speed Improvement**: 10-20x faster model loading

### Cache Management
```bash
# Check cache status (requires custom implementation)
ls -la $(llama-cli cache-dir 2>/dev/null || echo ~/.cache/llama-loader/)

# Clear cache if needed
rm -rf ~/.cache/llama-loader/

# Monitor cache size
du -sh ~/.cache/llama-loader/
```

## System Requirements

### Minimum Requirements
- **RAM**: 8GB (for small models)
- **Storage**: 10GB free space
- **CPU**: Modern x86_64 or ARM64 processor

### Recommended Configuration
- **RAM**: 16GB+ (for medium models and embedding)
- **Storage**: 100GB+ SSD for model cache
- **CPU**: 8+ cores with AVX2 support
- **GPU**: Metal (macOS) or CUDA compatible (optional)

### High-Performance Setup
- **RAM**: 32GB+ (for large models and high-throughput embedding)
- **Storage**: NVMe SSD with high IOPS
- **CPU**: High-core count (16+) with AVX-512
- **GPU**: Dedicated GPU with 8GB+ VRAM

## Monitoring and Profiling

### Performance Logging
```bash
# Enable detailed timing logs
RUST_LOG=debug llama-cli generate --model model --prompt "test"

# Monitor system resources during processing
htop  # or similar system monitor
```

### Benchmarking Commands

#### Generation Benchmarks
```bash
# Measure generation speed
time llama-cli generate \
  --model Qwen/Qwen2.5-7B-Instruct-GGUF \
  --prompt "Write a detailed explanation of quantum computing" \
  --max-tokens 500
```

#### Embedding Benchmarks  
```bash
# Create test dataset
seq 1 1000 | xargs -I {} echo "Test sentence number {}" > test_1k.txt

# Benchmark different batch sizes
for batch in 16 32 64 128; do
  echo "Testing batch size: $batch"
  time llama-cli embed \
    --model Qwen/Qwen3-Embedding-0.6B-GGUF \
    --input test_1k.txt \
    --output "test_batch_${batch}.parquet" \
    --batch-size $batch
done
```

## Troubleshooting Performance Issues

### Slow Model Loading
- **Check**: Internet connection for first download
- **Solution**: Use wired connection, or download manually
- **Monitor**: Check cache directory for partial downloads

### High Memory Usage
- **Embedding**: Reduce batch size
- **Generation**: Use smaller model
- **Cache**: Clear old cached models

### Low Throughput
- **CPU**: Verify OMP_NUM_THREADS is appropriate
- **Memory**: Ensure sufficient RAM available
- **Storage**: Use SSD for model cache
- **Batch**: Increase batch size for embedding

### Inconsistent Performance
- **Thermal**: Check CPU/GPU throttling
- **Background**: Close unnecessary applications
- **System**: Monitor system load and memory pressure

## Production Deployment Tips

### Container Optimization
```dockerfile
# Pre-warm model cache in Docker build
RUN llama-cli generate --model YOUR_MODEL --prompt "test" --max-tokens 1

# Use multi-stage build to optimize image size
FROM rust:alpine AS builder
# ... build steps ...
FROM alpine:latest
COPY --from=builder /usr/local/bin/llama-cli /usr/local/bin/
```

### Load Balancing
- Models are CPU/Memory intensive, not network bound
- Scale horizontally with multiple instances
- Use shared model cache via network storage if needed

### Monitoring in Production
- Track model loading times
- Monitor memory usage patterns
- Set up alerts for cache disk usage
- Profile batch sizes for optimal throughput

### Cost Optimization
- Use spot instances for batch processing
- Scale down during low-usage periods
- Share model cache across instances
- Monitor egress costs for model downloads