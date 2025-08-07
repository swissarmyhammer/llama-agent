# CLI Foundation Implementation

Refer to ./specifications/index.md

## Objective
Create the basic CLI structure with argument parsing and configuration setup.

## Tasks
- [ ] Set up `llama-agent-cli/src/main.rs` with clap argument parsing
- [ ] Implement Args struct matching the specification
- [ ] Add basic CLI command structure and help text
- [ ] Create configuration conversion from CLI args to AgentConfig
- [ ] Add basic error handling and user-friendly error messages
- [ ] Implement logging setup with tracing
- [ ] Add version and about information
- [ ] Create CLI validation for arguments

## CLI Arguments (matching specification)
- `--model`: HuggingFace repo name or local folder path
- `--filename`: Optional specific filename to use
- `--prompt`: Prompt text for generation
- `--limit`: Token limit (default: 512)

## Argument Validation
- Validate model path format for both HF repos and local paths
- Check local paths exist when using local models
- Validate reasonable token limits
- Provide helpful error messages for invalid arguments

## Configuration Mapping
- Convert CLI args to ModelConfig with appropriate ModelSource
- Set up reasonable defaults for QueueConfig and SessionConfig
- Handle optional MCP server configuration (future extension)
- Create AgentConfig from CLI arguments

## Error Handling
- User-friendly error messages for invalid arguments
- Clear guidance on correct usage patterns
- Proper exit codes for different error conditions
- Help text showing examples

## Acceptance Criteria
- CLI accepts all required arguments correctly
- Help text is clear and includes examples
- Arguments are validated with helpful error messages
- Configuration conversion works for both HF and local models
- Error handling provides actionable feedback
- CLI follows standard Unix conventions