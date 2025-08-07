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