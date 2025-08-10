//! # Llama Loader
//!
//! Shared model loading functionality for the llama-agent ecosystem.
//! This crate provides common types and interfaces for loading GGUF models
//! from HuggingFace and local sources.

pub mod cache;
pub mod detection;
pub mod error;
pub mod huggingface;
pub mod loader;
pub mod multipart;
pub mod retry;
pub mod types;

// Re-export main types for convenience
pub use cache::{CacheManager, FileMetadata};
pub use error::ModelError;
pub use huggingface::{load_huggingface_model, load_huggingface_model_with_path};
pub use loader::ModelLoader;
pub use types::{LoadedModel, ModelConfig, ModelMetadata, ModelSource, RetryConfig};
