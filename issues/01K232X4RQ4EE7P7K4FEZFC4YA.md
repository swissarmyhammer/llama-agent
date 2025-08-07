Could not load a model that I know exists (https://huggingface.co/unsloth/Qwen3-0.6B-GGUF)

I found this forbidden placeholder 

```
        // For now, HuggingFace integration is not available in llama-cpp-2
        // We'll treat the repo as a local path as fallback with performance optimization
        info!(
            "HuggingFace integration not available, treating repo as local path: {}",
            repo
        );
```

You can clearly see how to use the huggingface API with https://github.com/utilityai/llama-cpp-rs/blob/main/examples/simple/src/main.rs.

 cargo run --package llama-agent-cli -- --model unsloth/Qwen3-0.6B-GGUF --prompt "What is an apple?"
    Finished `dev` profile [optimized + debuginfo] target(s) in 0.06s
     Running `target/debug/llama-agent-cli --model unsloth/Qwen3-0.6B-GGUF --prompt 'What is an apple?'`
2025-08-07T20:17:06.083012Z  INFO llama_agent_cli: Starting llama-agent-cli
2025-08-07T20:17:06.083073Z  INFO llama_agent_cli: Model: unsloth/Qwen3-0.6B-GGUF
2025-08-07T20:17:06.083077Z  INFO llama_agent_cli: Filename: None
2025-08-07T20:17:06.083080Z  INFO llama_agent_cli: Prompt: What is an apple?
2025-08-07T20:17:06.083083Z  INFO llama_agent_cli: Limit: 512
2025-08-07T20:17:06.083088Z  INFO llama_agent_cli: Initializing AgentServer (this may take a while for model loading)...
Loading model from unsloth/Qwen3-0.6B-GGUF...
2025-08-07T20:17:06.083138Z  INFO llama_agent::agent: Initializing AgentServer with config: AgentConfig { model: ModelConfig { source: HuggingFace { repo: "unsloth/Qwen3-0.6B-GGUF", filename: None }, batch_size: 512, use_hf_params: true }, queue_config: QueueConfig { max_queue_size: 10, request_timeout: 120s, worker_threads: 1 }, mcp_servers: [], session_config: SessionConfig { max_sessions: 10, session_timeout: 3600s } }
2025-08-07T20:17:06.083411Z  INFO llama_agent::model: 🚀 Starting model loading process...
2025-08-07T20:17:06.083439Z  INFO llama_agent::model: Model configuration: ModelConfig { source: HuggingFace { repo: "unsloth/Qwen3-0.6B-GGUF", filename: None }, batch_size: 512, use_hf_params: true }
2025-08-07T20:17:06.083443Z  INFO llama_agent::model: 📊 Memory usage during model loading start: Process=0MB, Estimated Model=100MB
2025-08-07T20:17:06.083447Z  INFO llama_agent::model: 📋 Validating model configuration...
2025-08-07T20:17:06.083450Z  INFO llama_agent::model: ✅ Configuration validation completed
2025-08-07T20:17:06.083454Z  INFO llama_agent::model: Starting HuggingFace model download/loading for: unsloth/Qwen3-0.6B-GGUF
2025-08-07T20:17:06.083480Z  INFO llama_agent::model: HuggingFace integration not available, treating repo as local path: unsloth/Qwen3-0.6B-GGUF
Error: Failed to initialize agent: Model error: Model not found: HuggingFace repo path does not exist: unsloth/Qwen3-0.6B-GGUF

💡 Please check:
• Model file path exists and is readable
• Filename matches exactly (case-sensitive)
• File permissions allow read access (chmod 644)
• For HuggingFace repos: verify repo name exists and model file is present
• Use absolute paths to avoid relative path issues
💡 Check model file exists, is valid GGUF format, and sufficient memory is available

 󰀵 wballard   …/llama-agent    main [$]  


## Proposed Solution

The issue is that the current code contains a placeholder implementation that treats HuggingFace repository paths as local filesystem paths, causing models to fail to load. The solution involves implementing proper HuggingFace model downloading using the `hf-hub` crate.

### Implementation Steps:

1. **Add hf-hub dependency**: Add the `hf-hub` crate to Cargo.toml with appropriate features for async functionality
2. **Implement proper HuggingFace integration**: Replace the placeholder code in `load_huggingface_model` method in `llama-agent/src/model.rs:216-234` 
3. **Use HuggingFace API**: Utilize the `Api::model().get()` pattern to download models from HuggingFace Hub
4. **Handle filename resolution**: When no filename is specified, implement auto-detection of GGUF files in the repository
5. **Maintain progress indication**: Use the progress feature of hf-hub to provide user feedback during downloads
6. **Error handling**: Provide meaningful error messages for network failures, missing repos, etc.

### Key Changes:
- Remove the fallback code treating repos as local paths
- Implement actual HuggingFace API calls using `hf_hub::api::tokio::Api`
- Cache downloaded models locally (hf-hub handles this automatically)
- Support both explicit filenames and auto-detection patterns

This will allow commands like `cargo run --package llama-agent-cli -- --model unsloth/Qwen3-0.6B-GGUF --prompt "What is an apple?"` to work properly by downloading the model from HuggingFace Hub.