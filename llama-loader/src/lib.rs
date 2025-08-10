//! # Llama Loader
//!
//! Shared model loading functionality for the llama-agent ecosystem.
//! This crate provides common types and interfaces for loading GGUF models
//! from HuggingFace and local sources.

pub mod error;
pub mod loader;
pub mod types;

// Re-export main types for convenience
pub use error::ModelError;
pub use loader::ModelLoader;
pub use types::{LoadedModel, ModelMetadata, ModelSource};
