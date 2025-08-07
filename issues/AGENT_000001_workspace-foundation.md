# Workspace Foundation Setup

Refer to ./specifications/index.md

## Objective
Set up the basic Rust workspace structure with proper dependency management and initial crate layout.

## Tasks
- [ ] Create workspace root Cargo.toml with members and shared dependencies
- [ ] Create llama-agent library crate with Cargo.toml  
- [ ] Create llama-agent-cli binary crate with Cargo.toml
- [ ] Set up basic lib.rs and main.rs files with placeholder content
- [ ] Configure .gitignore for Rust projects (include mcp.log and semantic.db)
- [ ] Verify workspace builds with `cargo build`

## Structure to Create
```
llama-agent/
├── Cargo.toml          # Workspace root
├── llama-agent/        # Core library crate
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs
└── llama-agent-cli/    # Command-line interface
    ├── Cargo.toml
    └── src/
        └── main.rs
```

## Dependencies to Configure
Configure shared workspace dependencies as specified. Use the latest versions from crates.io.

- llama-cpp-2
- rmcp
- tokio with full features
- serde with derive feature
- clap with derive feature
- Error handling crates (thiserror, anyhow)
- tracing crates
- uuid with v4 feature (note: should migrate to ULID later per coding standards)
- async-trait

## Acceptance Criteria
- Workspace builds successfully
- Both crates are recognized as workspace members
- Dependencies are properly shared via workspace configuration
- Basic module structure is in place for future development