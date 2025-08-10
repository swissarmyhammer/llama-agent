use llama_embedding::{EmbeddingConfig, EmbeddingResult, EmbeddingError};
use llama_loader::ModelSource;

#[test]
fn test_embedding_config_default() {
    let config = EmbeddingConfig::default();
    assert_eq!(config.batch_size, 32);
    assert!(!config.normalize_embeddings);
    assert!(!config.debug);
    assert!(config.max_sequence_length.is_none());
    
    match &config.model_source {
        ModelSource::HuggingFace { repo, filename } => {
            assert_eq!(repo, "sentence-transformers/all-MiniLM-L6-v2");
            assert!(filename.is_none());
        }
        _ => panic!("Expected HuggingFace model source"),
    }
}

#[test]
fn test_embedding_result_creation() {
    let text = "Hello, world!".to_string();
    let embedding = vec![0.1, 0.2, 0.3, 0.4];
    let sequence_length = 4;
    let processing_time_ms = 100;
    
    let result = EmbeddingResult::new(
        text.clone(),
        embedding.clone(),
        sequence_length,
        processing_time_ms,
    );
    
    assert_eq!(result.text, text);
    assert_eq!(result.embedding, embedding);
    assert_eq!(result.sequence_length, sequence_length);
    assert_eq!(result.processing_time_ms, processing_time_ms);
    assert!(!result.text_hash.is_empty());
    
    // Verify MD5 hash is consistent
    let expected_hash = format!("{:x}", md5::compute(&text));
    assert_eq!(result.text_hash, expected_hash);
}

#[test]
fn test_error_types() {
    let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let embedding_error = EmbeddingError::Io(io_error);
    
    match embedding_error {
        EmbeddingError::Io(_) => {}, // Expected
        _ => panic!("Expected IO error variant"),
    }
    
    let batch_error = EmbeddingError::BatchProcessing("test error".to_string());
    assert!(batch_error.to_string().contains("Batch processing error"));
    
    let encoding_error = EmbeddingError::TextEncoding("invalid utf-8".to_string());
    assert!(encoding_error.to_string().contains("Text encoding error"));
}