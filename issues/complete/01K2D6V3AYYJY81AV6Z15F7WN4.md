 cargo run --package llama-cli embed \
  --model Qwen/Qwen3-Embedding-0.6B-GGUF \
  --input texts.txt \
  --output embeddings.parquet \
  --batch-size 32 \
  --normalize
    Finished `dev` profile [optimized + debuginfo] target(s) in 0.14s
     Running `target/debug/llama-cli embed --model Qwen/Qwen3-Embedding-0.6B-GGUF --input texts.txt --output embeddings.parquet --batch-size 32 --normalize`
Error: Output directory does not exist:

## Proposed Solution

The issue is in the `validate_embed_args` function in `llama-cli/src/embed.rs` at lines 61-68. The validation code checks if the parent directory of the output file exists, but:

1. When the output path is just a filename like `embeddings.parquet`, the parent is the current directory which should always exist
2. The code should create missing parent directories instead of just failing
3. The error message is incomplete (missing the path)

**Fix steps:**
1. Improve the directory validation logic to handle current directory case
2. Create missing parent directories automatically using `std::fs::create_dir_all`
3. Fix the incomplete error message
4. Add proper error handling for directory creation
5. Write tests to ensure the fix works correctly

This follows the principle of "be helpful" - instead of failing when a directory doesn't exist, we should create it.