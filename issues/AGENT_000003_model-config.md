# Model Configuration and Sources

Refer to ./specifications/index.md

## Objective
Implement configuration types for model loading from various sources with proper validation.

## Tasks
- [ ] Create ModelConfig struct with source, batch_size, use_hf_params
- [ ] Implement ModelSource enum for HuggingFace and Local variants
- [ ] Add QueueConfig for queue management settings
- [ ] Create SessionConfig for session management
- [ ] Implement AgentConfig as main configuration container
- [ ] Add MCPServerConfig for MCP server configuration
- [ ] Add validation methods for configurations
- [ ] Add default implementations where appropriate

## Configuration Types
- ModelConfig with ModelSource, batch size, HF params flag
- ModelSource enum:
  - HuggingFace { repo: String, filename: Option<String> }
  - Local { folder: PathBuf, filename: Option<String> }
- QueueConfig with max size, timeout, worker threads
- SessionConfig with relevant session settings
- AgentConfig combining all configuration types
- MCPServerConfig with name, command, args

## Validation
- Validate that paths exist for Local sources
- Validate HuggingFace repo format
- Ensure reasonable defaults for queue and session configs
- Proper error reporting for invalid configurations

## Acceptance Criteria
- All config types serialize/deserialize properly
- Validation catches common configuration errors
- Default values are sensible for development and testing
- Configuration supports both simple and advanced use cases
- Clear documentation for all configuration options