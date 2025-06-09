pub mod core;
pub mod difficulty;
pub mod formatter;
pub mod formatting;
pub mod selection;
pub mod selector;
pub mod sentences;

// Re-export the main types for convenience
pub use core::Language;
pub use difficulty::CharacterDifficulty;
pub use formatter::{TextFormatter, BasicFormatter, CapitalizationFormatter, SymbolFormatter, CompositeFormatter};
pub use selector::{WordSelector, RandomSelector, IntelligentSelector, SubstitutionSelector};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_integrated_functionality() {
        let lang = Language::new("english".to_string());
        
        // Test that all functionality works together
        let words = lang.get_random(5);
        assert_eq!(words.len(), 5);
        
        let formatted = lang.apply_advanced_formatting(words, true, false);
        assert!(!formatted.is_empty());
        assert!(formatted.chars().next().unwrap().is_uppercase());
    }

    #[test] 
    fn test_substitution_with_formatting() {
        let lang = Language::new("english".to_string());
        
        let mut char_stats = HashMap::new();
        char_stats.insert('x', CharacterDifficulty {
            miss_rate: 20.0,
            avg_time_ms: 300.0,
            total_attempts: 10,
            uppercase_miss_rate: 25.0,
            uppercase_avg_time: 400.0,
            uppercase_attempts: 3,
            uppercase_penalty: 0.5,
        });
        
        let substituted_words = lang.get_substituted(3, &char_stats);
        let formatted = lang.apply_advanced_formatting(substituted_words, true, true);
        
        assert!(!formatted.is_empty());
        // Should have capitalization
        assert!(formatted.chars().next().unwrap().is_uppercase());
    }
}