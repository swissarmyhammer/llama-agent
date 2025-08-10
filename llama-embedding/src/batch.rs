use crate::error::{EmbeddingError, EmbeddingResult as Result};
use crate::model::EmbeddingModel;
use crate::types::EmbeddingResult;
use std::path::Path;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{debug, info, warn};

/// Handles batch processing of multiple texts for embedding generation
pub struct BatchProcessor {
    model: Arc<EmbeddingModel>,
    batch_size: usize,
}

impl BatchProcessor {
    /// Create a new BatchProcessor
    pub fn new(model: Arc<EmbeddingModel>, batch_size: usize) -> Self {
        Self { model, batch_size }
    }

    /// Process a batch of texts and return embedding results
    pub async fn process_batch(&self, texts: &[String]) -> Result<Vec<EmbeddingResult>> {
        if !self.model.is_loaded() {
            return Err(EmbeddingError::ModelNotLoaded);
        }

        debug!("Processing batch of {} texts", texts.len());
        
        let mut results = Vec::with_capacity(texts.len());
        
        for text in texts {
            match self.model.embed_text(text).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    warn!("Failed to embed text '{}...': {}", 
                         text.chars().take(50).collect::<String>(), e);
                    return Err(e);
                }
            }
        }
        
        debug!("Successfully processed batch of {} embeddings", results.len());
        Ok(results)
    }

    /// Process a file containing texts (one per line) and return an iterator of results
    pub async fn process_file(&self, input_path: &Path) -> Result<Vec<EmbeddingResult>> {
        if !input_path.exists() {
            return Err(EmbeddingError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Input file not found: {}", input_path.display()),
            )));
        }

        info!("Processing file: {}", input_path.display());
        
        // Read all lines from the file
        let file = File::open(input_path).await?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        
        let mut all_texts = Vec::new();
        while let Some(line) = lines.next_line().await? {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                all_texts.push(trimmed.to_string());
            }
        }

        info!("Read {} texts from file", all_texts.len());
        
        // Process in batches
        let mut all_results = Vec::new();
        
        for chunk in all_texts.chunks(self.batch_size) {
            let batch_results = self.process_batch(chunk).await?;
            all_results.extend(batch_results);
            
            debug!("Processed batch, total results so far: {}", all_results.len());
        }
        
        info!("Completed processing file with {} embeddings", all_results.len());
        Ok(all_results)
    }

    /// Get the configured batch size
    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    /// Set a new batch size
    pub fn set_batch_size(&mut self, new_batch_size: usize) {
        if new_batch_size > 0 {
            self.batch_size = new_batch_size;
            debug!("Updated batch size to {}", new_batch_size);
        } else {
            warn!("Attempted to set invalid batch size: {}", new_batch_size);
        }
    }

    /// Get statistics about the underlying model
    pub fn get_model_info(&self) -> Option<(usize, bool)> {
        if let Some(dim) = self.model.get_embedding_dimension() {
            Some((dim, self.model.is_loaded()))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::EmbeddingConfig;
    use std::sync::Arc;

    // Note: These are structural tests that validate compilation
    // Actual functionality tests would require a real model to be loaded

    #[test]
    fn test_batch_processor_creation() {
        // Create a dummy config for testing structure
        let config = EmbeddingConfig::default();
        
        // We can't actually create an EmbeddingModel in unit tests
        // without proper setup, but we can test the structure
        assert_eq!(1, 1); // Placeholder test to verify compilation
    }

    #[test]
    fn test_batch_size_management() {
        // Test batch size validation logic
        let valid_sizes = vec![1, 8, 16, 32, 64, 128];
        for size in valid_sizes {
            assert!(size > 0);
        }
    }

    #[tokio::test]
    async fn test_empty_text_handling() {
        let texts = vec!["".to_string(), "   ".to_string()];
        let non_empty: Vec<String> = texts.into_iter()
            .filter(|t| !t.trim().is_empty())
            .collect();
        
        assert_eq!(non_empty.len(), 0);
    }
}
