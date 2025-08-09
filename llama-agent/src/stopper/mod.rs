use llama_cpp_2::{
    context::LlamaContext,
    llama_batch::LlamaBatch,
};

use crate::types::FinishReason;

/// Core trait for determining when to stop generation
pub trait Stopper {
    fn should_stop(&mut self, context: &LlamaContext, batch: &LlamaBatch) -> Option<FinishReason>;
}

// Re-export stopper implementations
pub mod eos;
pub mod max_tokens; 
pub mod repetition;

pub use eos::EosStopper;
pub use max_tokens::MaxTokensStopper;
pub use repetition::{RepetitionConfig, RepetitionStopper};