//! # llama-embedding
//!
//! A library for batch text embedding using llama-cpp-2, providing efficient
//! processing of multiple texts with configurable batching and normalization.
//!
//! ## Features
//!
//! - Batch processing for efficient embedding generation
//! - Configurable normalization and sequence length limits
//! - Integrates with llama-loader for model management
//! - Streaming support for large datasets
//! - MD5 hash generation for text deduplication
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use llama_embedding::{EmbeddingModel, EmbeddingConfig};
//! use llama_loader::ModelSource;
//!
//! # async fn example() -> Result<(), llama_embedding::EmbeddingError> {
//! let config = EmbeddingConfig {
//!     model_source: ModelSource::HuggingFace {
//!         repo: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
//!         filename: None,
//!     },
//!     batch_size: 32,
//!     normalize_embeddings: true,
//!     ..Default::default()
//! };
//!
//! let mut model = EmbeddingModel::new(config).await?;
//! model.load_model().await?;
//!
//! let result = model.embed_text("Hello, world!").await?;
//! println!("Embedding dimension: {}", result.embedding.len());
//! # Ok(())
//! # }
//! ```

pub mod batch;
pub mod error;
pub mod model;
pub mod types;

// Re-export public API
pub use batch::BatchProcessor;
pub use error::{EmbeddingError, EmbeddingResult as Result};
pub use model::EmbeddingModel;
pub use types::{EmbeddingConfig, EmbeddingResult};
