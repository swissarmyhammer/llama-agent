cargo run --package llama-agent-cli -- --model unsloth/Qwen3-0.6B-GGUF --prompt "What is an apple?"

This generates a lot of useless logging from llama_cpp

```
llama_model_load_from_file_impl: using device Metal (Apple M2 Ultra) - 147455 MiB free
llama_model_loader: loaded meta data with 28 key-value pairs and 310 tensors from /Users/wballard/.cache/huggingface/hub/models--unsloth--Qwen3-0.6B-GGUF/snapshots/50968a4468ef4233ed78cd7c3de230dd1d61a56b/Qwen3-0.6B-BF16.gguf (version GGUF V3 (latest))
llama_model_loader: Dumping metadata keys/values. Note: KV overrides do not apply in this output.
llama_model_loader: - kv   0:                       general.architecture str              = qwen3
llama_model_loader: - kv   1:                               general.type str              = model
llama_model_loader: - kv   2:                               general.name str              = Qwen3-0.6B
llama_model_loader: - kv   3:                           general.basename str              = Qwen3-0.6B
llama_model_loader: - kv   4:                       general.quantized_by str              = Unsloth
llama_model_loader: - kv   5:                         general.size_label str              = 0.6B
llama_model_loader: - kv   6:                           general.repo_url str              = https://huggingface.co/unsloth
llama_model_loader: - kv   7:                          qwen3.block_count u32              = 28
llama_model_loader: - kv   8:                       qwen3.context_length u32              = 40960
llama_model_loader: - kv   9:                     qwen3.embedding_length u32              = 1024
llama_model_loader: - kv  10:                  qwen3.feed_forward_length u32              = 3072
llama_model_loader: - kv  11:                 qwen3.attention.head_count u32              = 16
llama_model_loader: - kv  12:              qwen3.attention.head_count_kv u32              = 8
llama_model_loader: - kv  13:                       qwen3.rope.freq_base f32              = 1000000.000000
llama_model_loader: - kv  14:     qwen3.attention.layer_norm_rms_epsilon f32              = 0.000001
llama_model_loader: - kv  15:                 qwen3.attention.key_length u32              = 128
llama_model_loader: - kv  16:               qwen3.attention.value_length u32              = 128
llama_model_loader: - kv  17:                          general.file_type u32              = 32
llama_model_loader: - kv  18:               general.quantization_version u32              = 2
llama_model_loader: - kv  19:                       tokenizer.ggml.model str              = gpt2
llama_model_loader: - kv  20:                         tokenizer.ggml.pre str              = qwen2
```


All this should only show up with a --debug switch that powers a debug option boolean on the session.
