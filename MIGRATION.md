# Migration Guide

## Migration from llama-agent-cli to llama-cli

The CLI has been renamed and restructured with subcommands to support both text generation and embedding functionality.

### CLI Name Change

**Before:**
```bash
llama-agent-cli --model Qwen/Qwen2.5-7B-Instruct-GGUF --prompt "Hello"
```

**After:**  
```bash
llama-cli generate --model Qwen/Qwen2.5-7B-Instruct-GGUF --prompt "Hello"
```

### Key Changes

1. **Binary Name**: `llama-agent-cli` → `llama-cli`
2. **Subcommands**: All generation functionality now under the `generate` subcommand
3. **New Features**: Added `embed` subcommand for text embedding

### Migration Steps

#### 1. Update Installation
```bash
# Uninstall old version
cargo uninstall llama-agent-cli

# Install new version
cargo install llama-cli
```

#### 2. Update Scripts and Aliases

**Shell Scripts:**
```bash
# Before
llama-agent-cli "$@"

# After
llama-cli generate "$@"
```

**Bash Aliases:**
```bash
# Before
alias llama='llama-agent-cli'

# After
alias llama='llama-cli generate'
```

#### 3. Update Docker/Container Usage

**Dockerfile:**
```dockerfile
# Before
RUN cargo install llama-agent-cli
ENTRYPOINT ["llama-agent-cli"]

# After
RUN cargo install llama-cli
ENTRYPOINT ["llama-cli", "generate"]
```

### Command Mapping

All existing options remain the same, just prepend `generate`:

| Old Command | New Command |
|-------------|-------------|
| `llama-agent-cli --model M --prompt P` | `llama-cli generate --model M --prompt P` |
| `llama-agent-cli --model M --prompt P --stream` | `llama-cli generate --model M --prompt P --stream` |
| `llama-agent-cli --model M --prompt P --max-tokens 100` | `llama-cli generate --model M --prompt P --max-tokens 100` |
| `llama-agent-cli --help` | `llama-cli generate --help` |

### New Embedding Functionality

The new `embed` subcommand provides text embedding capabilities:

```bash
# Basic embedding
llama-cli embed \
  --model Qwen/Qwen3-Embedding-0.6B-GGUF \
  --input texts.txt \
  --output embeddings.parquet

# Batch processing with normalization
llama-cli embed \
  --model Qwen/Qwen3-Embedding-0.6B-GGUF \
  --input large_dataset.txt \
  --output embeddings.parquet \
  --batch-size 64 \
  --normalize
```

### Environment Variables

All environment variables remain the same:

- `LLAMA_CACHE_DIR`: Custom model cache directory
- `LLAMA_CACHE_MAX_SIZE`: Maximum cache size in bytes
- `RUST_LOG`: Logging configuration

### Model Cache Compatibility

Your existing model cache remains fully compatible. Models cached by the old CLI will be automatically used by the new CLI, and vice versa. The cache is also shared with the new embedding functionality.

### Configuration Files

If you have any configuration files or scripts that reference:
- Binary path: Update from `llama-agent-cli` to `llama-cli`
- Commands: Add `generate` subcommand

### Automated Migration

For projects with many scripts, you can use this sed command to automatically update:

```bash
# Update all shell scripts in a directory
find . -name "*.sh" -type f -exec sed -i 's/llama-agent-cli/llama-cli generate/g' {} \;

# Update specific script
sed -i 's/llama-agent-cli/llama-cli generate/g' your-script.sh
```

### Validation

After migration, verify your setup:

```bash
# Test generation (should work as before)
llama-cli generate --model Qwen/Qwen2.5-7B-Instruct-GGUF --prompt "Hello world"

# Test new embedding functionality  
echo "Test text" > test.txt
llama-cli embed --model Qwen/Qwen3-Embedding-0.6B-GGUF --input test.txt --output test.parquet
```

### Troubleshooting

#### Command Not Found
- Ensure you uninstalled the old binary: `cargo uninstall llama-agent-cli`
- Install the new binary: `cargo install llama-cli`
- Restart your shell or update PATH

#### Missing Subcommand
- Remember to use `generate` subcommand for text generation
- Use `embed` subcommand for text embedding

#### Model Cache Issues
- The cache location and structure remain the same
- If you encounter issues, you can clear the cache and let it re-download

### Breaking Changes

**None!** This is a backward-compatible change. All existing command-line options, model formats, and behavior remain identical. The only change is the addition of the `generate` subcommand prefix.

### Future Compatibility

The `llama-cli` tool is designed to be extended with additional subcommands in the future while maintaining backward compatibility. Current plans include:
- Enhanced model management commands
- Batch processing utilities
- Performance profiling tools

All existing `generate` and `embed` functionality will remain stable across future versions.