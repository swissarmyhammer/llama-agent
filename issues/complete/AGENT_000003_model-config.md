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

## Proposed Solution

After analyzing the existing `types.rs` file, I can see that most of the core configuration types are already implemented:

✅ **Already Implemented:**
- `ModelConfig` with `ModelSource`, `batch_size`, `use_hf_params`
- `ModelSource` enum with `HuggingFace` and `Local` variants
- `QueueConfig` with max size, timeout, worker threads
- `SessionConfig` with relevant session settings  
- `AgentConfig` as main configuration container
- `MCPServerConfig` with name, command, args
- All core types have proper ULID integration
- Comprehensive error types defined
- Extensive test coverage exists

❌ **Missing Items:**
1. **Serde Support**: Config types need `Serialize` and `Deserialize` derives
2. **Validation Methods**: Missing validation for configurations
3. **Default Implementations**: Missing `Default` for some config types
4. **Path Validation**: Need to validate paths exist for Local sources
5. **HF Repo Format Validation**: Need to validate HuggingFace repo format

## Implementation Steps:
1. Add serde derives to all config types
2. Add Default implementations for ModelConfig, QueueConfig, AgentConfig
3. Implement validation methods for each config type
4. Add validation that checks paths exist for Local model sources
5. Add HuggingFace repo format validation
6. Add tests for new functionality

## Benefits:
- Proper serialization/deserialization for config files
- Early validation catches configuration errors
- Sensible defaults for development and testing
- Clear error reporting for invalid configurations