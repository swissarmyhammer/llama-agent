use crate::detection::auto_detect_hf_model_file;
use crate::error::ModelError;
use crate::multipart::download_multi_part_model;
use crate::retry::download_with_retry;
use crate::types::RetryConfig;
use hf_hub::api::tokio::ApiBuilder;
use llama_cpp_2::{
    llama_backend::LlamaBackend,
    model::{params::LlamaModelParams, LlamaModel},
};
use std::path::PathBuf;
use tracing::{info, warn};

/// Loads a model from HuggingFace and returns path info for caching
pub async fn load_huggingface_model_with_path(
    repo: &str,
    filename: Option<&str>,
    retry_config: &RetryConfig,
) -> Result<(PathBuf, String), ModelError> {
    info!("Loading HuggingFace model: {}", repo);

    // Create HuggingFace API client
    let api = match ApiBuilder::new().build() {
        Ok(api) => api,
        Err(e) => {
            return Err(ModelError::Network(format!(
                "Failed to create HuggingFace API client for {}: {}. Use ModelSource::Local to load from local path instead.",
                repo, e
            )));
        }
    };

    let repo_api = api.model(repo.to_string());

    // Determine which file to download
    let target_filename = if let Some(filename) = filename {
        filename.to_string()
    } else {
        // Auto-detect the model file by listing repository files
        match auto_detect_hf_model_file(&repo_api).await {
            Ok(detected_filename) => detected_filename,
            Err(e) => {
                warn!("Failed to auto-detect model file: {}", e);
                return Err(ModelError::NotFound(format!(
                    "Could not auto-detect model file in repository: {}. Please specify --filename",
                    repo
                )));
            }
        }
    };

    info!("Downloading model file: {}", target_filename);

    // Download the model file(s) with retry logic
    let model_path = if let Some(parts) = get_all_parts(&target_filename) {
        info!("Downloading multi-part model with {} parts", parts.len());
        download_multi_part_model(&repo_api, &parts, repo, retry_config).await?
    } else {
        download_with_retry(&repo_api, &target_filename, repo, retry_config).await?
    };

    info!("Model downloaded to: {}", model_path.display());

    Ok((model_path, target_filename))
}

/// Loads a model from HuggingFace (original function for backward compatibility)
pub async fn load_huggingface_model(
    backend: &LlamaBackend,
    repo: &str,
    filename: Option<&str>,
    retry_config: &RetryConfig,
) -> Result<LlamaModel, ModelError> {
    // Use the new function to get the path, then load the model
    let (model_path, _) = load_huggingface_model_with_path(repo, filename, retry_config).await?;

    // Load the downloaded model
    let model_params = LlamaModelParams::default();
    let model = LlamaModel::load_from_file(backend, &model_path, &model_params).map_err(|e| {
        ModelError::LoadingFailed(format!(
            "Failed to load downloaded model from {}: {}",
            model_path.display(),
            e
        ))
    })?;

    Ok(model)
}

/// Gets all parts of a multi-part GGUF file
pub fn get_all_parts(base_filename: &str) -> Option<Vec<String>> {
    use regex::Regex;
    let re = Regex::new(r"^(.+)-00001-of-(\d{5})\.gguf$").ok()?;

    if let Some(captures) = re.captures(base_filename) {
        let base_name = captures.get(1)?.as_str();
        let total_parts_str = captures.get(2)?.as_str();
        let total_parts: u32 = total_parts_str.parse().ok()?;

        let mut parts = Vec::new();
        for part_num in 1..=total_parts {
            parts.push(format!(
                "{}-{:05}-of-{}.gguf",
                base_name, part_num, total_parts_str
            ));
        }

        Some(parts)
    } else {
        None
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_parts_valid() {
        let parts = get_all_parts("model-00001-of-00003.gguf").unwrap();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "model-00001-of-00003.gguf");
        assert_eq!(parts[1], "model-00002-of-00003.gguf");
        assert_eq!(parts[2], "model-00003-of-00003.gguf");
    }

    #[test]
    fn test_get_all_parts_single_file() {
        let parts = get_all_parts("model.gguf");
        assert!(parts.is_none());
    }

    #[test]
    fn test_get_all_parts_invalid_format() {
        let parts = get_all_parts("model-part1-of-3.gguf");
        assert!(parts.is_none());
    }
}
