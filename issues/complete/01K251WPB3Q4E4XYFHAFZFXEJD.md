You broke something in the last three issues of fixing.

 cargo run --package llama-agent-cli -- --model unsloth/Qwen3-30B-A3B-GGUF --filename Qwen3-30B-A3B-UD-Q8_K_XL.gguf  --prompt "What is an apple?" --limit 64
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on build directory
   Compiling ring v0.17.14
   Compiling rustls v0.23.31
   Compiling rustls-webpki v0.103.4
   Compiling ureq v2.12.1
   Compiling hf-hub v0.3.2
   Compiling llama-agent v0.1.0 (/Users/wballard/github/llama-agent/llama-agent)
   Compiling llama-agent-cli v0.1.0 (/Users/wballard/github/llama-agent/llama-agent-cli)
    Finished `dev` profile [optimized + debuginfo] target(s) in 5.61s
     Running `target/debug/llama-agent-cli --model unsloth/Qwen3-30B-A3B-GGUF --filename Qwen3-30B-A3B-UD-Q8_K_XL.gguf --prompt 'What is an apple?' --limit 64`
Loading model from unsloth/Qwen3-30B-A3B-GGUF...
Error: Failed to initialize agent: Model error: Model not found: Model file does not exist: unsloth/Qwen3-30B-A3B-GGUF/Qwen3-30B-A3B-UD-Q8_K_XL.gguf
📁 Verify file path is correct, file exists and is readable. For HuggingFace: check repo name and filename
💡 Check model file exists, is valid GGUF format, and sufficient memory is available

 󰀵 wballard   …/llama-agent    main [$⇡]  
 cargo run --package llama-agent-cli -- --model unsloth/Qwen3-30B-A3B-GGUF   --prompt "What is an apple?" --limit 64
   Compiling ring v0.17.14
   Compiling rustls v0.23.31
   Compiling rustls-webpki v0.103.4
   Compiling ureq v2.12.1
   Compiling hf-hub v0.3.2
^[OP   Compiling llama-agent v0.1.0 (/Users/wballard/github/llama-agent/llama-agent)
   Compiling llama-agent-cli v0.1.0 (/Users/wballard/github/llama-agent/llama-agent-cli)
    Finished `dev` profile [optimized + debuginfo] target(s) in 5.39s
     Running `target/debug/llama-agent-cli --model unsloth/Qwen3-30B-A3B-GGUF --prompt 'What is an apple?' --limit 64`
Loading model from unsloth/Qwen3-30B-A3B-GGUF...
Model Error: Failed to initialize agent: Model error: Model loading failed: Cannot read directory unsloth/Qwen3-30B-A3B-GGUF: No such file or directory (os error 2)

## Proposed Solution

After analyzing the error, I found that the issue is in the HuggingFace model loading implementation. The CLI correctly identifies `unsloth/Qwen3-30B-A3B-GGUF` as a HuggingFace repo, but the `load_huggingface_model` method in model.rs simply treats it as a local path, which fails.

The error occurs because:
1. CLI parses `unsloth/Qwen3-30B-A3B-GGUF` as HuggingFace repo (correct)
2. ModelManager calls `load_huggingface_model` with this repo name
3. `load_huggingface_model` has a fallback that treats the repo as a local path
4. This fails because `unsloth/Qwen3-30B-A3B-GGUF` directory doesn't exist locally

### Implementation Steps:
1. Check if hf-hub dependency is available and can be used for downloading
2. If not available, implement proper HuggingFace downloading using the `hf-hub` crate (which is already in dependencies)
3. Cache downloaded models to avoid re-downloading
4. Provide better error messages that explain how to download models manually

The fix will implement proper HuggingFace model downloading instead of the current fallback to local paths.