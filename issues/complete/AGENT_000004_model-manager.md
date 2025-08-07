# Model Manager Implementation

Refer to ./specifications/index.md

## Objective
Implement ModelManager for loading models from HuggingFace and local sources with auto-detection capabilities.

## Tasks
- [ ] Create `model.rs` module with ModelManager struct
- [ ] Implement model loading from HuggingFace repos using llama-cpp-2
- [ ] Implement model loading from local folders/files
- [ ] Add BF16 preference auto-detection logic for both sources
- [ ] Integrate HuggingFace generation_config.json loading
- [ ] Add proper error handling for model loading failures
- [ ] Create ModelManager::load_model async method
- [ ] Add model validation and compatibility checks

## Model Selection Strategy
When filename is None, auto-detect with priority:
1. Files containing "BF16" or "bf16" in the name
2. First available .gguf file in the location  
3. Error if no compatible model files found

## Key Methods
- `load_model(config: ModelConfig) -> Result<LlamaModel, ModelError>`
- Auto-detection helpers for both HF and local sources
- HuggingFace generation config integration
- Model validation and capability reporting

## Error Handling
- ModelError variants for different failure modes
- Clear error messages for missing files/repos
- Network error handling for HuggingFace downloads
- Model format validation errors

## Acceptance Criteria
- Successfully loads models from HuggingFace repos
- Successfully loads models from local folders
- BF16 preference works for both sources
- Auto-detection fails gracefully with clear errors
- HuggingFace generation config integration works
- All model loading paths are tested

## Proposed Solution

I will implement the ModelManager by replacing the current mock implementation with real llama-cpp-2 integration:

### Implementation Steps

1. **Replace MockModel with real LlamaModel from llama-cpp-2**:
   - Import necessary types from llama-cpp-2: `LlamaModel`, `LlamaParams`, `LoadProgress`
   - Replace MockModel/MockContext with actual model and context structs

2. **Implement HuggingFace model loading**:
   - Use llama-cpp-2's built-in HuggingFace support via `LlamaModel::load_from_hf_*` methods
   - Implement auto-detection logic for BF16 preference
   - Add generation_config.json integration when use_hf_params is enabled

3. **Enhance local model loading**:
   - Replace mock local path resolution with actual file loading
   - Keep existing BF16 preference auto-detection logic
   - Use `LlamaModel::load_from_file` for direct file loading

4. **Add proper error handling**:
   - Convert llama-cpp-2 errors to ModelError variants
   - Add network error handling for HuggingFace downloads
   - Improve validation and compatibility checks

5. **Update method signatures**:
   - Change `load_model()` to return `Result<(), ModelError>`
   - Update getters to return `Option<Arc<LlamaModel>>` and `Option<Arc<LlamaContext>>`
   - Add model validation and capability reporting methods

6. **Add HuggingFace generation config integration**:
   - Fetch and parse generation_config.json from HF repos
   - Apply parameters when use_hf_params is true
   - Handle missing or invalid config files gracefully

The implementation will maintain the existing test coverage while providing real model loading capabilities, following the specifications exactly as defined.