use super::Stopper;
use crate::types::FinishReason;
use llama_cpp_2::{context::LlamaContext, llama_batch::LlamaBatch};
use std::collections::VecDeque;

/// Configuration for repetition detection
#[derive(Debug, Clone)]
pub struct RepetitionConfig {
    pub min_pattern_length: usize,
    pub max_pattern_length: usize,
    pub min_repetitions: usize,
    pub window_size: usize,
}

impl Default for RepetitionConfig {
    fn default() -> Self {
        Self {
            min_pattern_length: 10,
            max_pattern_length: 100,
            min_repetitions: 3,
            window_size: 1000,
        }
    }
}

/// Stopper that detects repetitive patterns in generated text
pub struct RepetitionStopper {
    config: RepetitionConfig,
    text_window: VecDeque<String>,
    current_window_size: usize,
}

impl RepetitionStopper {
    /// Create a new repetition stopper
    pub fn new(config: RepetitionConfig) -> Self {
        Self {
            config,
            text_window: VecDeque::new(),
            current_window_size: 0,
        }
    }

    /// Add newly generated token text to the sliding window
    pub fn add_token_text(&mut self, token_text: String) {
        let text_len = token_text.len();
        self.text_window.push_back(token_text);
        self.current_window_size += text_len;

        // Maintain window size bounds by removing old text from the front
        while self.current_window_size > self.config.window_size {
            if let Some(old_text) = self.text_window.pop_front() {
                self.current_window_size -= old_text.len();
            } else {
                break;
            }
        }
    }

    /// Get the current text window as a single string
    fn get_window_text(&self) -> String {
        self.text_window.iter().cloned().collect()
    }

    /// Check for repetitive patterns in the current window
    fn detect_repetition(&self) -> Option<(String, usize)> {
        let window_text = self.get_window_text();
        let chars: Vec<char> = window_text.chars().collect();

        if chars.len() < self.config.min_pattern_length {
            return None;
        }

        // Check patterns from max length down to min length
        // This prioritizes detecting longer patterns first
        if self.config.min_pattern_length > self.config.max_pattern_length {
            return None; // Invalid configuration
        }

        for pattern_length in
            (self.config.min_pattern_length..=self.config.max_pattern_length).rev()
        {
            if pattern_length == 0 {
                continue; // Skip zero-length patterns
            }
            if pattern_length > chars.len() {
                continue;
            }

            // Extract the most recent pattern of this length
            let pattern_start = chars.len() - pattern_length;
            let pattern: String = chars[pattern_start..].iter().collect();

            // Count consecutive occurrences of this pattern working backwards
            let mut count = 0;
            let mut pos = chars.len();

            while pos >= pattern_length {
                let check_start = pos - pattern_length;
                let check_slice: String = chars[check_start..pos].iter().collect();

                if check_slice == pattern {
                    count += 1;
                    pos = check_start;
                } else {
                    break;
                }
            }

            // If we found enough repetitions, return the pattern and count
            if count >= self.config.min_repetitions {
                return Some((pattern, count));
            }
        }

        None
    }
}

impl Stopper for RepetitionStopper {
    fn should_stop(
        &mut self,
        _context: &LlamaContext,
        _batch: &LlamaBatch,
    ) -> Option<FinishReason> {
        // For now, this implementation assumes that the batch contains the tokens
        // that were just processed. In practice, this stopper needs to be integrated
        // differently - it should receive the actual generated tokens.
        //
        // This is a preliminary implementation that establishes the pattern detection
        // logic. The integration with actual token flow will be handled in the
        // queue integration phase (STOPPING_000007).

        // We can't easily extract individual tokens from the batch in the current
        // llama_cpp_2 API design. The stoppers are called after batch processing,
        // but before token sampling. For repetition detection, we need the actual
        // generated token text.

        // For now, return None to maintain interface compatibility
        // The actual implementation will be integrated in queue.rs where
        // tokens are available after sampling.

        if self.text_window.is_empty() {
            return None;
        }

        // Check for repetitive patterns
        if let Some((pattern, count)) = self.detect_repetition() {
            let message = format!(
                "Repetition detected: '{}' repeated {} times",
                pattern.chars().take(50).collect::<String>(),
                count
            );
            Some(FinishReason::Stopped(message))
        } else {
            None
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repetition_config_default() {
        let config = RepetitionConfig::default();
        assert_eq!(config.min_pattern_length, 10);
        assert_eq!(config.max_pattern_length, 100);
        assert_eq!(config.min_repetitions, 3);
        assert_eq!(config.window_size, 1000);
    }

    #[test]
    fn test_repetition_stopper_creation() {
        let config = RepetitionConfig::default();
        let stopper = RepetitionStopper::new(config.clone());

        assert_eq!(stopper.config.min_pattern_length, config.min_pattern_length);
        assert_eq!(stopper.config.max_pattern_length, config.max_pattern_length);
        assert_eq!(stopper.config.min_repetitions, config.min_repetitions);
        assert_eq!(stopper.config.window_size, config.window_size);
        assert!(stopper.text_window.is_empty());
        assert_eq!(stopper.current_window_size, 0);
    }

    #[test]
    fn test_add_token_text() {
        let config = RepetitionConfig::default();
        let mut stopper = RepetitionStopper::new(config);

        stopper.add_token_text("Hello".to_string());
        assert_eq!(stopper.text_window.len(), 1);
        assert_eq!(stopper.current_window_size, 5);

        stopper.add_token_text(" World".to_string());
        assert_eq!(stopper.text_window.len(), 2);
        assert_eq!(stopper.current_window_size, 11);
    }

    #[test]
    fn test_window_text_concatenation() {
        let config = RepetitionConfig::default();
        let mut stopper = RepetitionStopper::new(config);

        stopper.add_token_text("Hello".to_string());
        stopper.add_token_text(" ".to_string());
        stopper.add_token_text("World".to_string());

        let window_text = stopper.get_window_text();
        assert_eq!(window_text, "Hello World");
    }

    #[test]
    fn test_window_size_enforcement() {
        let config = RepetitionConfig {
            min_pattern_length: 5,
            max_pattern_length: 20,
            min_repetitions: 2,
            window_size: 10, // Very small window for testing
        };
        let mut stopper = RepetitionStopper::new(config);

        // Add text that exceeds window size
        stopper.add_token_text("Hello".to_string()); // 5 chars
        stopper.add_token_text(" ".to_string()); // 1 char
        stopper.add_token_text("World".to_string()); // 5 chars = 11 total, exceeds limit

        // First token should be removed to stay under limit
        assert!(stopper.current_window_size <= 10);
        let window_text = stopper.get_window_text();
        assert!(!window_text.starts_with("Hello"));
    }

    #[test]
    fn test_repetition_detection_simple_pattern() {
        let config = RepetitionConfig {
            min_pattern_length: 3,
            max_pattern_length: 10,
            min_repetitions: 3,
            window_size: 100,
        };
        let mut stopper = RepetitionStopper::new(config);

        // Add a pattern that repeats exactly 3 times
        stopper.add_token_text("abc".to_string());
        stopper.add_token_text("abc".to_string());
        stopper.add_token_text("abc".to_string());

        let result = stopper.detect_repetition();
        assert!(result.is_some());

        let (pattern, count) = result.unwrap();
        assert_eq!(pattern, "abc");
        assert_eq!(count, 3);
    }

    #[test]
    fn test_repetition_detection_no_repetition() {
        let config = RepetitionConfig {
            min_pattern_length: 3,
            max_pattern_length: 10,
            min_repetitions: 3,
            window_size: 100,
        };
        let mut stopper = RepetitionStopper::new(config);

        stopper.add_token_text("Hello".to_string());
        stopper.add_token_text(" ".to_string());
        stopper.add_token_text("World".to_string());

        let result = stopper.detect_repetition();
        assert!(result.is_none());
    }

    #[test]
    fn test_repetition_detection_insufficient_repetitions() {
        let config = RepetitionConfig {
            min_pattern_length: 3,
            max_pattern_length: 10,
            min_repetitions: 3,
            window_size: 100,
        };
        let mut stopper = RepetitionStopper::new(config);

        // Only 2 repetitions, but we need 3
        stopper.add_token_text("abc".to_string());
        stopper.add_token_text("abc".to_string());

        let result = stopper.detect_repetition();
        assert!(result.is_none());
    }

    #[test]
    fn test_repetition_detection_longer_patterns() {
        let config = RepetitionConfig {
            min_pattern_length: 5,
            max_pattern_length: 20,
            min_repetitions: 2,
            window_size: 100,
        };
        let mut stopper = RepetitionStopper::new(config);

        // Add a longer pattern that repeats
        let pattern = "hello world ";
        stopper.add_token_text(pattern.to_string());
        stopper.add_token_text(pattern.to_string());

        let result = stopper.detect_repetition();
        assert!(result.is_some());

        let (detected_pattern, count) = result.unwrap();
        assert_eq!(detected_pattern, pattern);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_repetition_detection_prioritizes_longer_patterns() {
        let config = RepetitionConfig {
            min_pattern_length: 2,
            max_pattern_length: 10,
            min_repetitions: 2,
            window_size: 100,
        };
        let mut stopper = RepetitionStopper::new(config);

        // "ababab" could be detected as "ab" repeated 3 times or "abab" repeated 1.5 times
        // But with our algorithm, we should detect the longest valid pattern
        stopper.add_token_text("ab".to_string());
        stopper.add_token_text("ab".to_string());
        stopper.add_token_text("ab".to_string());

        let result = stopper.detect_repetition();
        assert!(result.is_some());

        let (pattern, count) = result.unwrap();
        // Should detect "ab" repeated 3 times (since that meets the min_repetitions)
        assert_eq!(pattern, "ab");
        assert_eq!(count, 3);
    }

    #[test]
    fn test_repetition_detection_pattern_too_short() {
        let config = RepetitionConfig {
            min_pattern_length: 10,
            max_pattern_length: 20,
            min_repetitions: 2,
            window_size: 100,
        };
        let mut stopper = RepetitionStopper::new(config);

        // Pattern is shorter than minimum length
        stopper.add_token_text("abc".to_string());
        stopper.add_token_text("abc".to_string());

        let result = stopper.detect_repetition();
        assert!(result.is_none());
    }

    #[test]
    fn test_repetition_detection_window_too_small() {
        let config = RepetitionConfig {
            min_pattern_length: 5,
            max_pattern_length: 20,
            min_repetitions: 2,
            window_size: 100,
        };
        let mut stopper = RepetitionStopper::new(config);

        // Window text is too small for minimum pattern length
        stopper.add_token_text("hi".to_string());

        let result = stopper.detect_repetition();
        assert!(result.is_none());
    }

    #[test]
    fn test_repetition_detection_mixed_content() {
        let config = RepetitionConfig {
            min_pattern_length: 4,
            max_pattern_length: 15,
            min_repetitions: 3,
            window_size: 200,
        };
        let mut stopper = RepetitionStopper::new(config);

        // Add some normal content followed by repetition
        stopper.add_token_text("The quick brown fox ".to_string());
        stopper.add_token_text("test ".to_string());
        stopper.add_token_text("test ".to_string());
        stopper.add_token_text("test ".to_string());

        let result = stopper.detect_repetition();
        assert!(result.is_some());

        let (pattern, count) = result.unwrap();
        assert_eq!(pattern, "test ");
        assert_eq!(count, 3);
    }

    #[test]
    fn test_repetition_detection_partial_pattern_at_end() {
        let config = RepetitionConfig {
            min_pattern_length: 4,
            max_pattern_length: 15,
            min_repetitions: 3,
            window_size: 100,
        };
        let mut stopper = RepetitionStopper::new(config);

        // Pattern repeated 3 times, then partial pattern
        stopper.add_token_text("test".to_string());
        stopper.add_token_text("test".to_string());
        stopper.add_token_text("test".to_string());
        stopper.add_token_text("te".to_string()); // Partial pattern

        let result = stopper.detect_repetition();
        assert!(result.is_some());

        // Should still detect the complete pattern repetitions
        let (_pattern, count) = result.unwrap();
        // The algorithm looks for the most recent complete pattern
        // Since "te" is at the end, it might affect detection depending on implementation
        // but we should still detect repetition in the complete patterns
        assert!(count >= 3);
    }

    #[test]
    fn test_stopper_trait_compliance() {
        let config = RepetitionConfig::default();
        let stopper = RepetitionStopper::new(config);

        // Verify it can be stored as a trait object
        let _boxed: Box<dyn Stopper> = Box::new(stopper);
    }

    #[test]
    fn test_thread_safety() {
        // Test that RepetitionStopper implements Send (but not Sync due to interior mutability)
        fn assert_send<T: Send>() {}
        assert_send::<RepetitionStopper>();

        // RepetitionStopper is not Sync because it has interior mutability
        // This is expected and correct for stoppers that maintain state
    }

    #[test]
    fn test_edge_case_empty_tokens() {
        let config = RepetitionConfig::default();
        let mut stopper = RepetitionStopper::new(config);

        // Adding empty token text should not break the stopper
        stopper.add_token_text("".to_string());
        stopper.add_token_text("test".to_string());
        stopper.add_token_text("".to_string());

        let window_text = stopper.get_window_text();
        assert_eq!(window_text, "test");
    }

    #[test]
    fn test_large_window_size() {
        let config = RepetitionConfig {
            min_pattern_length: 3,
            max_pattern_length: 10,
            min_repetitions: 2,
            window_size: 10000,
        };
        let mut stopper = RepetitionStopper::new(config);

        // Add a lot of content to test memory management
        for i in 0..1000 {
            stopper.add_token_text(format!("token{} ", i));
        }

        // Should not exceed window size limit
        assert!(stopper.current_window_size <= 10000);
    }

    #[test]
    fn test_config_edge_cases() {
        // Test with zero values (edge case)
        let config = RepetitionConfig {
            min_pattern_length: 0,
            max_pattern_length: 0,
            min_repetitions: 0,
            window_size: 0,
        };
        let mut stopper = RepetitionStopper::new(config);

        stopper.add_token_text("test".to_string());

        // Should not crash with zero configurations
        let _result = stopper.detect_repetition();
        // With min_repetitions = 0, any pattern should be detected
        // But with pattern lengths = 0, no patterns can be formed
        // Result depends on implementation details
    }

    #[test]
    fn test_unicode_support() {
        let config = RepetitionConfig {
            min_pattern_length: 1, // Each emoji is one character
            max_pattern_length: 10,
            min_repetitions: 2,
            window_size: 100,
        };
        let mut stopper = RepetitionStopper::new(config);

        // Test with unicode characters
        stopper.add_token_text("🔥".to_string());
        stopper.add_token_text("🔥".to_string());

        let result = stopper.detect_repetition();
        assert!(result.is_some());

        let (pattern, count) = result.unwrap();
        assert_eq!(pattern, "🔥");
        assert_eq!(count, 2);
    }
}
