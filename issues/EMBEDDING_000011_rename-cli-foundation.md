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