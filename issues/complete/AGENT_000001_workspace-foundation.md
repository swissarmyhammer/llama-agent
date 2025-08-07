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

## Proposed Solution

I will implement the Rust workspace foundation setup using Test Driven Development (TDD) approach:

### Implementation Steps:

1. **Create workspace root Cargo.toml** - Set up the workspace configuration with the exact dependencies specified in the specification (index.md), using workspace.dependencies for shared dependency management.

2. **Create directory structure** - Establish the proper crate layout:
   - `llama-agent/` (library crate)
   - `llama-agent-cli/` (binary crate)

3. **Set up individual crate Cargo.toml files** - Each crate will reference the workspace dependencies appropriately.

4. **Create basic source files** - Implement minimal lib.rs and main.rs with proper module structure for future development.

5. **Configure .gitignore** - Add Rust-specific ignores plus the required mcp.log and semantic.db entries per coding standards.

6. **Verification** - Use `cargo build` to ensure the workspace compiles successfully.

### Key Technical Decisions:

- Use exact dependency versions from the specification for consistency
- Structure workspace to support both library and CLI development
- Include proper error handling imports (thiserror, anyhow) from the start
- Set up tracing infrastructure for logging
- Configure tokio with full features for async support
- Include uuid (to be migrated to ULID later per coding standards)

The implementation will follow the Rust coding standards: proper formatting with `cargo fmt`, linting with `cargo clippy`, and testing with `cargo nextest` once available.