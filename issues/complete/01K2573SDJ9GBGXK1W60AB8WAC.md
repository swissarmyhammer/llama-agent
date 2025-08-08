 cargo run --package llama-agent-cli -- --model unsloth/Qwen3-30B-A3B-GGUF  --prompt "What is an apple?" --limit 64
    Finished `dev` profile [optimized + debuginfo] target(s) in 0.09s
     Running `target/debug/llama-agent-cli --model unsloth/Qwen3-30B-A3B-GGUF --prompt 'What is an apple?' --limit 64`
Loading model from unsloth/Qwen3-30B-A3B-GGUF...
Model Error: Failed to initialize agent: Model error: Model loading failed: Failed to download model file 'BF16/Qwen3-30B-A3B-BF16-00001-of-00002.gguf' from repository 'unsloth/Qwen3-30B-A3B-GGUF': request error: HTTP status server error (500 Internal Server Error) for url (<https://cas-bridge.xethub.hf.co/xet-bridge-us/680f87393952fce74921f3d9/6c4fcb075e3a8d1fcc10d5e2ea002a809aa01db0a47040cdf64b77fd5599a650?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Content-Sha256=UNSIGNED-PAYLOAD&X-Amz-Credential=cas%2F20250808%2Fus-east-1%2Fs3%2Faws4_request&X-Amz-Date=20250808T152228Z&X-Amz-Expires=3600&X-Amz-Signature=609e2791c3950e3ec262b1271f62b0f3566798edef6035a4f0ecc7265f4c71b8&X-Amz-SignedHeaders=host&X-Xet-Cas-Uid=6463f0187572c66a8e63984a&response-content-disposition=inline%3B+filename*%3DUTF-8%27%27Qwen3-30B-A3B-BF16-00001-of-00002.gguf%3B+filename%3D%22Qwen3-30B-A3B-BF16-00001-of-00002.gguf%22%3B&x-id=GetObject&Expires=1754670148&Policy=eyJTdGF0ZW1lbnQiOlt7IkNvbmRpdGlvbiI6eyJEYXRlTGVzc1RoYW4iOnsiQVdTOkVwb2NoVGltZSI6MTc1NDY3MDE0OH19LCJSZXNvdXJjZSI6Imh0dHBzOi8vY2FzLWJyaWRnZS54ZXRodWIuaGYuY28veGV0LWJyaWRnZS11cy82ODBmODczOTM5NTJmY2U3NDkyMWYzZDkvNmM0ZmNiMDc1ZTNhOGQxZmNjMTBkNWUyZWEwMDJhODA5YWEwMWRiMGE0NzA0MGNkZjY0Yjc3ZmQ1NTk5YTY1MCoifV19&Signature=jNzATUJsrcrkcH3uRiLBDeIGVs4VYStd6ofFjBBhAteMF8koj4qkrotiE6mtbzrSb-IavazxQbcDYRYfJm1jNeaYCxrzptUJxH%7EY6-I3k6f%7EFL8BfzJ7OklViUih1-w%7Es-GGa8hjxvhH8gIrCkEMpLhKsihaBgnxaNlxWx9Gnn-1lbC4hKGZu3eyNgUL7m4CAelXhrBlkzly8LEMswReGA4l9qEv0tka8EeN4DdRxd7%7EUw05LycX9F73x%7EVZXf5Ce40KwknqR4HjaqCjIMGF7D5nE7eDVc9N7plg7bMGi-Ek6v%7EaqYYCrciNkSZT2zRo9jXv7HUGf7hOmNLk8Po6iQ__&Key-Pair-Id=K2L8F4GPSG1IFC>).
📁 Verify file path is correct, file exists and is readable. For HuggingFace: check repo name and filename
💡 Check model file exists, is valid GGUF format, and sufficient memory is available
🔧 Check available memory (4-8GB needed), verify GGUF file integrity, ensure compatible llama.cpp version
💡 Check model file exists, is valid GGUF format, and sufficient memory is available


## Proposed Solution

After analyzing the issue, the HTTP 500 error is occurring during HuggingFace model download. The current error handling provides basic feedback but lacks retry mechanisms and more specific guidance for transient network issues.

### Root Cause
The error is happening in `/Users/wballard/github/llama-agent/llama-agent/src/model.rs:194-202` where `repo_api.get(&target_filename).await` fails with an HTTP 500 server error. This is likely a transient server issue on HuggingFace's infrastructure.

### Implementation Plan
1. **Add retry logic with exponential backoff** for HTTP download failures
2. **Improve error messaging** to distinguish between client errors (404, authentication) and server errors (500, 503)
3. **Add configurable retry parameters** to ModelConfig
4. **Implement better fallback mechanisms** when downloads fail
5. **Add download progress indication** for large model files

### Changes Required
1. Update `ModelConfig` to include retry configuration
2. Enhance `load_huggingface_model` method with retry logic 
3. Improve error messages to be more actionable for users
4. Add unit tests for retry scenarios

This will make the CLI more resilient to transient network issues and provide better user experience when model downloads fail.