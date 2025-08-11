 cargo run --package llama-cli embed \
  --model Qwen/Qwen3-Embedding-0.6B-GGUF \
  --input texts.txt \
  --output embeddings.parquet \
  --batch-size 32 \
  --normalize
    Finished `dev` profile [optimized + debuginfo] target(s) in 0.14s
     Running `target/debug/llama-cli embed --model Qwen/Qwen3-Embedding-0.6B-GGUF --input texts.txt --output embeddings.parquet --batch-size 32 --normalize`
Error: Output directory does not exist: