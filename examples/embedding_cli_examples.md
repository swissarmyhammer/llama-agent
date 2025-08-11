# Embedding CLI Examples

This document provides comprehensive examples of using the `llama-cli embed` command for various text embedding scenarios.

## Basic Usage

### Simple Text Embedding
```bash
# Create a simple input file
echo -e "Hello world\nThis is a test\nAnother example" > sample.txt

# Generate embeddings with default settings
llama-cli embed \
  --model Qwen/Qwen3-Embedding-0.6B-GGUF \
  --input sample.txt \
  --output embeddings.parquet
```

### Normalized Embeddings for Similarity Search
```bash
# Generate L2-normalized embeddings suitable for similarity search
llama-cli embed \
  --model Qwen/Qwen3-Embedding-0.6B-GGUF \
  --input documents.txt \
  --output normalized_embeddings.parquet \
  --normalize
```

## Batch Processing Examples

### Small Batch (Memory Constrained)
```bash
# Process with small batches for limited memory
llama-cli embed \
  --model Qwen/Qwen3-Embedding-0.6B-GGUF \
  --input large_dataset.txt \
  --output embeddings_small.parquet \
  --batch-size 8 \
  --normalize
```

### Large Batch (High Performance)
```bash
# Process with large batches for maximum throughput
llama-cli embed \
  --model Qwen/Qwen3-Embedding-0.6B-GGUF \
  --input dataset.txt \
  --output embeddings_fast.parquet \
  --batch-size 128 \
  --normalize
```

### Text Length Optimization
```bash
# Truncate very long texts for consistent performance
llama-cli embed \
  --model Qwen/Qwen3-Embedding-0.6B-GGUF \
  --input mixed_length_texts.txt \
  --output truncated_embeddings.parquet \
  --max-length 512 \
  --batch-size 64 \
  --normalize
```

## Production Workflows

### Large Dataset Processing
```bash
#!/bin/bash
# Script for processing large datasets efficiently

INPUT_FILE="large_corpus.txt"
OUTPUT_FILE="corpus_embeddings.parquet"
MODEL="Qwen/Qwen3-Embedding-0.6B-GGUF"

# Check input file size
echo "Processing $(wc -l < $INPUT_FILE) texts..."

# Process with optimal settings
time llama-cli embed \
  --model $MODEL \
  --input $INPUT_FILE \
  --output $OUTPUT_FILE \
  --batch-size 64 \
  --max-length 1024 \
  --normalize \
  --debug

echo "Processing complete. Output: $OUTPUT_FILE"
echo "Output size: $(du -h $OUTPUT_FILE | cut -f1)"
```

### Multi-Model Comparison
```bash
#!/bin/bash
# Compare different embedding models on the same dataset

DATASET="comparison_texts.txt"
MODELS=("Qwen/Qwen3-Embedding-0.6B-GGUF" "sentence-transformers/all-MiniLM-L6-v2")

for model in "${MODELS[@]}"; do
    model_name=$(basename "$model")
    output_file="embeddings_${model_name}.parquet"
    
    echo "Processing with model: $model"
    time llama-cli embed \
        --model "$model" \
        --input "$DATASET" \
        --output "$output_file" \
        --batch-size 32 \
        --normalize
    
    echo "Completed: $output_file"
done
```

### Incremental Processing
```bash
#!/bin/bash
# Process files in chunks for very large datasets

INPUT_DIR="text_chunks"
OUTPUT_DIR="embedding_chunks"
MODEL="Qwen/Qwen3-Embedding-0.6B-GGUF"

mkdir -p "$OUTPUT_DIR"

for chunk_file in "$INPUT_DIR"/*.txt; do
    base_name=$(basename "$chunk_file" .txt)
    output_file="$OUTPUT_DIR/${base_name}_embeddings.parquet"
    
    echo "Processing chunk: $base_name"
    llama-cli embed \
        --model "$MODEL" \
        --input "$chunk_file" \
        --output "$output_file" \
        --batch-size 32 \
        --normalize
done

echo "All chunks processed in $OUTPUT_DIR"
```

## Performance Optimization

### Memory-Optimized Processing
```bash
# Optimal settings for systems with limited memory
llama-cli embed \
  --model Qwen/Qwen3-Embedding-0.6B-GGUF \
  --input large_file.txt \
  --output memory_opt.parquet \
  --batch-size 16 \
  --max-length 256 \
  --normalize
```

### Speed-Optimized Processing
```bash
# Optimal settings for maximum throughput
llama-cli embed \
  --model Qwen/Qwen3-Embedding-0.6B-GGUF \
  --input speed_test.txt \
  --output speed_opt.parquet \
  --batch-size 128 \
  --max-length 512 \
  --normalize
```

### Benchmark Different Settings
```bash
#!/bin/bash
# Benchmark script to find optimal settings

DATASET="benchmark.txt"
MODEL="Qwen/Qwen3-Embedding-0.6B-GGUF"
BATCH_SIZES=(16 32 64 128)

echo "Benchmarking batch sizes on $(wc -l < $DATASET) texts"
echo "Batch | Time | Throughput"
echo "------|------|----------"

for batch in "${BATCH_SIZES[@]}"; do
    output="bench_${batch}.parquet"
    
    start_time=$(date +%s)
    llama-cli embed \
        --model "$MODEL" \
        --input "$DATASET" \
        --output "$output" \
        --batch-size $batch \
        --normalize \
        >/dev/null 2>&1
    end_time=$(date +%s)
    
    duration=$((end_time - start_time))
    texts=$(wc -l < "$DATASET")
    throughput=$(echo "$texts / $duration" | bc -l)
    
    printf "%5d | %4ds | %8.1f t/s\n" $batch $duration $throughput
    
    rm "$output"
done
```

## Working with Output

### Reading Parquet Files

#### Python (Pandas)
```python
import pandas as pd
import numpy as np

# Load embeddings
df = pd.read_parquet("embeddings.parquet")
print(f"Loaded {len(df)} embeddings")
print(f"Embedding dimensions: {len(df['embedding'].iloc[0])}")

# Access data
texts = df['text'].tolist()
embeddings = np.array(df['embedding'].tolist())
text_hashes = df['text_hash'].tolist()

# If normalized
if 'embedding_norm' in df.columns:
    norms = df['embedding_norm'].tolist()
    print(f"Average norm: {np.mean(norms):.4f}")
```

#### Python (PyArrow)
```python
import pyarrow.parquet as pq
import numpy as np

# Load with PyArrow for better performance
table = pq.read_table("embeddings.parquet")
df = table.to_pandas()

# Convert embeddings to numpy array
embeddings = np.array([np.array(emb) for emb in df['embedding']])
print(f"Embeddings shape: {embeddings.shape}")
```

#### Rust (Polars)
```rust
use polars::prelude::*;

let df = LazyFrame::scan_parquet("embeddings.parquet", ScanArgsParquet::default())?
    .select([
        col("text"),
        col("text_hash"), 
        col("embedding"),
        col("embedding_norm")
    ])
    .collect()?;

println!("Loaded {} embeddings", df.height());
```

### Similarity Search Example
```python
import pandas as pd
import numpy as np
from sklearn.metrics.pairwise import cosine_similarity

# Load normalized embeddings
df = pd.read_parquet("normalized_embeddings.parquet")
texts = df['text'].tolist()
embeddings = np.array(df['embedding'].tolist())

# Query embedding (assuming first row)
query_embedding = embeddings[0:1]
query_text = texts[0]

# Compute similarities
similarities = cosine_similarity(query_embedding, embeddings)[0]

# Find most similar texts
similar_indices = np.argsort(similarities)[::-1][1:6]  # Top 5, excluding self

print(f"Query: {query_text}")
print("\nMost similar texts:")
for i, idx in enumerate(similar_indices):
    print(f"{i+1}. {texts[idx]} (similarity: {similarities[idx]:.4f})")
```

## Error Handling and Debugging

### Enable Debug Logging
```bash
# Show detailed processing information
RUST_LOG=debug llama-cli embed \
  --model Qwen/Qwen3-Embedding-0.6B-GGUF \
  --input debug_test.txt \
  --output debug_output.parquet \
  --batch-size 32
```

### Handle Large Files Safely
```bash
#!/bin/bash
# Safe processing with error handling

INPUT_FILE="large_dataset.txt"
OUTPUT_FILE="safe_embeddings.parquet"
TEMP_OUTPUT="${OUTPUT_FILE}.tmp"

# Check if input exists
if [[ ! -f "$INPUT_FILE" ]]; then
    echo "Error: Input file not found: $INPUT_FILE"
    exit 1
fi

# Check available disk space (need at least 2x input size)
input_size=$(stat -c%s "$INPUT_FILE" 2>/dev/null || stat -f%z "$INPUT_FILE")
available_space=$(df . | tail -1 | awk '{print $4}')

if [[ $((input_size * 2)) -gt $((available_space * 1024)) ]]; then
    echo "Warning: May not have enough disk space"
fi

# Process with temporary output
echo "Processing $(wc -l < "$INPUT_FILE") texts..."
if llama-cli embed \
    --model Qwen/Qwen3-Embedding-0.6B-GGUF \
    --input "$INPUT_FILE" \
    --output "$TEMP_OUTPUT" \
    --batch-size 32 \
    --normalize; then
    
    # Move to final location on success
    mv "$TEMP_OUTPUT" "$OUTPUT_FILE"
    echo "Success: $OUTPUT_FILE created"
else
    echo "Error: Processing failed"
    rm -f "$TEMP_OUTPUT"
    exit 1
fi
```

## Integration Examples

### Combined Generation and Embedding
```bash
#!/bin/bash
# Generate text and then create embeddings

# Step 1: Generate texts
echo "Generating sample texts..."
for i in {1..10}; do
    llama-cli generate \
        --model Qwen/Qwen2.5-7B-Instruct-GGUF \
        --prompt "Write a short sentence about topic number $i" \
        --max-tokens 50
done > generated_texts.txt

# Step 2: Create embeddings
echo "Creating embeddings for generated texts..."
llama-cli embed \
    --model Qwen/Qwen3-Embedding-0.6B-GGUF \
    --input generated_texts.txt \
    --output generated_embeddings.parquet \
    --batch-size 16 \
    --normalize

echo "Workflow complete!"
echo "Generated texts: generated_texts.txt"  
echo "Embeddings: generated_embeddings.parquet"
```

### Cache Optimization
```bash
# Warm up model cache first
echo "Warming up model cache..."
echo "test" | llama-cli embed \
    --model Qwen/Qwen3-Embedding-0.6B-GGUF \
    --input /dev/stdin \
    --output /tmp/warmup.parquet \
    --batch-size 1

# Now process main dataset (will use cached model)
echo "Processing main dataset..."
time llama-cli embed \
    --model Qwen/Qwen3-Embedding-0.6B-GGUF \
    --input main_dataset.txt \
    --output main_embeddings.parquet \
    --batch-size 64 \
    --normalize

rm /tmp/warmup.parquet
```

This comprehensive guide covers most real-world usage scenarios for the embedding CLI functionality.