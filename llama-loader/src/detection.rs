use crate::error::ModelError;
use crate::multipart::detect_multi_part_base;
use tracing::info;

/// Auto-detects the best model file from a HuggingFace repository
pub async fn auto_detect_hf_model_file(
    repo_api: &hf_hub::api::tokio::ApiRepo,
) -> Result<String, ModelError> {
    // List files in the repository
    match repo_api.info().await {
        Ok(repo_info) => {
            let mut gguf_files = Vec::new();
            let mut bf16_files = Vec::new();

            // Look for GGUF files in the repository
            for sibling in repo_info.siblings {
                if sibling.rfilename.ends_with(".gguf") {
                    let filename = sibling.rfilename.to_lowercase();
                    if filename.contains("bf16") {
                        bf16_files.push(sibling.rfilename);
                    } else {
                        gguf_files.push(sibling.rfilename);
                    }
                }
            }

            // Prioritize BF16 files - check for multi-part files first
            if !bf16_files.is_empty() {
                // Sort to ensure consistent ordering
                bf16_files.sort();

                // Check if this is a multi-part file
                if let Some(base_filename) = detect_multi_part_base(&bf16_files[0]) {
                    info!("Found multi-part BF16 model file: {}", base_filename);
                    return Ok(base_filename);
                } else {
                    info!("Found BF16 model file: {}", bf16_files[0]);
                    return Ok(bf16_files[0].clone());
                }
            }

            // Fallback to first GGUF file
            if !gguf_files.is_empty() {
                gguf_files.sort();
                if let Some(base_filename) = detect_multi_part_base(&gguf_files[0]) {
                    info!("Found multi-part GGUF model file: {}", base_filename);
                    return Ok(base_filename);
                } else {
                    info!("Found GGUF model file: {}", gguf_files[0]);
                    return Ok(gguf_files[0].clone());
                }
            }

            Err(ModelError::NotFound(
                "No .gguf model files found in HuggingFace repository".to_string(),
            ))
        }
        Err(e) => Err(ModelError::LoadingFailed(format!(
            "Failed to get repository info: {}",
            e
        ))),
    }
}

#[cfg(test)]
mod tests {

    // Note: These would be integration tests that require actual HuggingFace API access
    // For unit testing, we'd need to mock the ApiRepo and repo_info structures

    #[test]
    fn test_module_exists() {
        // Basic test to ensure the module compiles correctly
        // If this test runs, the module definition is valid
    }
}
