# EMBEDDING_000011: Rename and Restructure CLI Foundation

## Overview
Rename `llama-agent-cli` to `llama-cli` and restructure it with clap subcommands to support both `generate` (existing) and `embed` (new) functionality.

Refer to ./specification/embedding.md

## Tasks

### 1. Rename Crate
- Rename `llama-agent-cli/` directory to `llama-cli/`
- Update `Cargo.toml` name from `llama-agent-cli` to `llama-cli`
- Update binary name to `llama-cli`
- Update workspace members list

### 2. Add Required Dependencies
```toml
# llama-cli/Cargo.toml
[dependencies]
llama-loader = { workspace = true }
llama-agent = { workspace = true }     # For generate command
llama-embedding = { workspace = true } # For embed command
clap = { workspace = true, features = ["derive"] }
tokio = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
```

### 3. Create Subcommand Structure
```rust
// llama-cli/src/main.rs
#[derive(Parser)]
#[command(name = "llama-cli")]
#[command(about = "Unified Llama CLI for generation and embeddings")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate text using a language model (existing functionality)
    Generate(GenerateArgs),
    /// Generate embeddings for input texts
    Embed(EmbedArgs),
}
```

### 4. Move Existing Logic
- Move existing `main.rs` logic to `generate.rs`
- Create `GenerateArgs` struct with all existing CLI arguments
- Preserve all existing command-line options and behavior
- Ensure backward compatibility with existing usage patterns

### 5. Create Embed Args Structure
```rust
#[derive(Args)]
pub struct EmbedArgs {
    /// Model source (HuggingFace repo or local path)
    #[arg(long, short)]
    model: String,
    
    /// Optional model filename
    #[arg(long)]
    filename: Option<String>,
    
    /// Input text file (one text per line)
    #[arg(long, short)]
    input: PathBuf,
    
    /// Output Parquet file path
    #[arg(long, short)]
    output: PathBuf,
    
    /// Batch size for processing
    #[arg(long, default_value = "32")]
    batch_size: usize,
    
    /// Normalize embeddings
    #[arg(long)]
    normalize: bool,
    
    /// Maximum sequence length
    #[arg(long)]
    max_length: Option<usize>,
    
    /// Enable debug output
    #[arg(long)]
    debug: bool,
}
```

### 6. Module Structure
```
llama-cli/src/
├── main.rs           # Main CLI with subcommands
├── lib.rs            # Shared utilities
├── generate.rs       # Generate command (existing logic)
└── embed.rs          # Embed command (placeholder)
```

## Success Criteria
- [ ] CLI successfully renamed from llama-agent-cli to llama-cli
- [ ] Subcommand structure works: `llama-cli generate` and `llama-cli embed`
- [ ] All existing generation functionality preserved
- [ ] Generate command works identically to old CLI
- [ ] Embed command structure ready for implementation
- [ ] Help messages and documentation updated
- [ ] No breaking changes to existing generate usage

## Backward Compatibility
- Ensure existing users of llama-agent-cli can migrate easily
- Provide clear migration documentation
- Consider maintaining old binary name as alias (optional)
- All existing command-line arguments preserved

## Integration Notes
- This step restructures without adding embed functionality yet
- Focus on clean subcommand architecture
- Prepare foundation for embed command implementation
- Maintain all existing functionality exactly

## Proposed Solution

I will implement this restructuring in the following steps:

1. **Directory and Workspace Rename**: Rename the `llama-agent-cli` directory to `llama-cli` and update the workspace configuration
2. **Subcommand Architecture**: Create a clean clap-based subcommand structure with `generate` and `embed` commands
3. **Code Migration**: Move all existing CLI logic into a `generate.rs` module without losing any functionality
4. **Module Structure**: Create the foundation modules (`generate.rs`, `embed.rs`, `lib.rs`, `main.rs`)
5. **Testing**: Ensure backward compatibility for all existing generate functionality

The key design will be:
- `main.rs`: Entry point with clap subcommand parsing
- `generate.rs`: All existing functionality migrated from current `lib.rs`
- `embed.rs`: Placeholder structure ready for implementation
- `lib.rs`: Shared utilities and module exports

This preserves all existing functionality while setting up the foundation for the embedding command implementation.
## Implementation Complete ✅

Successfully completed all requirements:

### ✅ **CLI Renamed and Restructured**
- Renamed `llama-agent-cli` to `llama-cli` (directory and binary)
- Updated workspace configuration
- Updated all dependencies and exports

### ✅ **Subcommand Architecture Implemented**
```bash
# Main CLI help
llama-cli --help  # Shows generate/embed subcommands

# Generate command (preserved all functionality)
llama-cli generate --model <MODEL> --prompt <PROMPT> [OPTIONS]

# Embed command (placeholder ready)
llama-cli embed --model <MODEL> --input <INPUT> --output <OUTPUT> [OPTIONS]
```

### ✅ **Module Structure Created**
```
llama-cli/src/
├── main.rs           # Entry point with subcommand routing
├── lib.rs            # Module exports
├── generate.rs       # All existing generation functionality
└── embed.rs          # Placeholder embed structure
```

### ✅ **Backward Compatibility Maintained**
- All existing generation functionality preserved
- Same command-line arguments and validation
- Identical error handling and exit codes
- Same streaming output behavior

### ✅ **Foundation Ready**
- `EmbedArgs` struct with all required fields from specification
- Validation framework in place
- Proper error messages referencing EMBEDDING_000013
- Clean clap integration

### ✅ **Quality Checks Passed**
- ✅ Code compiles without warnings
- ✅ All clippy lints pass
- ✅ Code properly formatted
- ✅ Help messages work correctly
- ✅ Placeholder behavior functions as expected

The CLI foundation is ready for the embedding command implementation in EMBEDDING_000013.