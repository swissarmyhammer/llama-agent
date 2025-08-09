use super::Stopper;
use crate::types::FinishReason;
use llama_cpp_2::{
    context::LlamaContext,
    llama_batch::LlamaBatch,
};

/// Detects End-of-Sequence (EOS) tokens in the generated output
pub struct EosStopper {
    eos_token_id: u32,
}

impl EosStopper {
    pub fn new(eos_token_id: u32) -> Self {
        Self { eos_token_id }
    }
}

impl Stopper for EosStopper {
    fn should_stop(&mut self, _context: &LlamaContext, _batch: &LlamaBatch) -> Option<FinishReason> {
        // Implementation will be added in later issue
        None
    }
}