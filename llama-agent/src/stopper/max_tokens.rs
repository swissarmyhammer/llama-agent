use super::Stopper;
use crate::types::FinishReason;
use llama_cpp_2::{
    context::LlamaContext,
    llama_batch::LlamaBatch,
};

/// Tracks total tokens generated and stops when a configured maximum is reached
pub struct MaxTokensStopper {
    max_tokens: usize,
    current_tokens: usize,
}

impl MaxTokensStopper {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            current_tokens: 0,
        }
    }
}

impl Stopper for MaxTokensStopper {
    fn should_stop(&mut self, _context: &LlamaContext, _batch: &LlamaBatch) -> Option<FinishReason> {
        // Implementation will be added in later issue
        None
    }
}