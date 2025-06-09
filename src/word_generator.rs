use crate::{
    language::{Language, WordSelector, RandomSelector, IntelligentSelector, SubstitutionSelector, CompositeFormatter}, 
    stats::StatsDb, 
    SupportedLanguage
};
use std::collections::HashMap;

/// Configuration for word generation
#[derive(Debug, Clone)]
pub struct WordGenConfig {
    pub number_of_words: usize,
    pub number_of_sentences: Option<usize>,
    pub custom_prompt: Option<String>,
    pub language: SupportedLanguage,
    pub random_words: bool,
    pub substitute: bool,
    pub capitalize: bool,
    pub symbols: bool,
}

/// Handles all word and prompt generation logic
pub struct WordGenerator {
    config: WordGenConfig,
}

impl WordGenerator {
    pub fn new(config: WordGenConfig) -> Self {
        Self { config }
    }

    /// Generate a complete prompt based on the configuration
    pub fn generate_prompt(&self) -> (String, usize) {
        if let Some(ref custom_prompt) = self.config.custom_prompt {
            return (custom_prompt.clone(), self.config.number_of_words);
        }

        if let Some(sentence_count) = self.config.number_of_sentences {
            return self.generate_sentences(sentence_count);
        }

        self.generate_words()
    }

    /// Generate sentences using cgisf
    fn generate_sentences(&self, count: usize) -> (String, usize) {
        let language = self.config.language.as_lang();
        let (sentences, word_count) = language.get_random_sentence(count);
        (sentences.join(""), word_count)
    }

    /// Generate words based on selection strategy and apply formatting
    fn generate_words(&self) -> (String, usize) {
        let language = self.config.language.as_lang();
        
        // Step 1: Select words based on strategy
        let words = self.select_words(&language);
        
        // Step 2: Apply formatting using the new formatter system
        let formatter = CompositeFormatter::build_from_flags(self.config.capitalize, self.config.symbols);
        let formatted_text = formatter.format(words);

        (formatted_text, self.config.number_of_words)
    }

    /// Select words based on the configured strategy
    fn select_words(&self, language: &Language) -> Vec<String> {
        // Load character statistics for intelligent/substitution modes
        let char_difficulties = match StatsDb::new() {
            Ok(stats_db) => stats_db.get_character_difficulties().unwrap_or_default(),
            Err(_) => HashMap::new(),
        };

        // Choose the appropriate selector based on configuration
        let selector: Box<dyn WordSelector> = if self.config.random_words {
            Box::new(RandomSelector)
        } else if self.config.substitute {
            Box::new(SubstitutionSelector)
        } else {
            Box::new(IntelligentSelector)
        };

        selector.select_words(language, self.config.number_of_words, &char_difficulties)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> WordGenConfig {
        WordGenConfig {
            number_of_words: 5,
            number_of_sentences: None,
            custom_prompt: None,
            language: SupportedLanguage::English,
            random_words: false,
            substitute: false,
            capitalize: false,
            symbols: false,
        }
    }

    #[test]
    fn test_custom_prompt() {
        let mut config = create_test_config();
        config.custom_prompt = Some("custom test prompt".to_string());
        
        let generator = WordGenerator::new(config);
        let (prompt, word_count) = generator.generate_prompt();
        
        assert_eq!(prompt, "custom test prompt");
        assert_eq!(word_count, 5);
    }

    #[test]
    fn test_sentence_generation() {
        let mut config = create_test_config();
        config.number_of_sentences = Some(2);
        
        let generator = WordGenerator::new(config);
        let (prompt, word_count) = generator.generate_prompt();
        
        assert!(!prompt.is_empty());
        assert!(word_count > 0);
    }

    #[test]
    fn test_word_generation_random() {
        let mut config = create_test_config();
        config.random_words = true;
        
        let generator = WordGenerator::new(config);
        let (prompt, word_count) = generator.generate_prompt();
        
        assert!(!prompt.is_empty());
        assert_eq!(word_count, 5);
        // Should be space-separated words
        assert!(prompt.contains(' '));
    }

    #[test]
    fn test_word_generation_with_capitalization() {
        let mut config = create_test_config();
        config.capitalize = true;
        
        let generator = WordGenerator::new(config);
        let (prompt, _) = generator.generate_prompt();
        
        assert!(!prompt.is_empty());
        // Should start with capital letter
        assert!(prompt.chars().next().unwrap().is_uppercase());
    }

    #[test]
    fn test_word_generation_with_substitution() {
        let mut config = create_test_config();
        config.substitute = true;
        
        let generator = WordGenerator::new(config);
        let (prompt, word_count) = generator.generate_prompt();
        
        assert!(!prompt.is_empty());
        assert_eq!(word_count, 5);
    }

    #[test]
    fn test_word_generation_combined_flags() {
        let mut config = create_test_config();
        config.substitute = true;
        config.capitalize = true;
        config.symbols = true;
        
        let generator = WordGenerator::new(config);
        let (prompt, word_count) = generator.generate_prompt();
        
        assert!(!prompt.is_empty());
        assert_eq!(word_count, 5);
        // Should have capitalization (first alphabetic character should be uppercase)
        let first_alpha_char = prompt.chars().find(|c| c.is_alphabetic());
        if let Some(first_char) = first_alpha_char {
            assert!(first_char.is_uppercase());
        }
    }

    #[test]
    fn test_word_generation_intelligent_selection() {
        let mut config = create_test_config();
        config.random_words = false; // Use intelligent selection
        
        let generator = WordGenerator::new(config);
        let (prompt, word_count) = generator.generate_prompt();
        
        assert!(!prompt.is_empty());
        assert_eq!(word_count, 5);
    }

    #[test]
    fn test_word_generation_with_symbols_only() {
        let mut config = create_test_config();
        config.symbols = true;
        
        let generator = WordGenerator::new(config);
        let (prompt, word_count) = generator.generate_prompt();
        
        assert!(!prompt.is_empty());
        assert_eq!(word_count, 5);
        // Should end with punctuation
        assert!(prompt.ends_with('.') || prompt.ends_with('!') || prompt.ends_with('?') || 
                prompt.ends_with(';') || prompt.ends_with(':') || prompt.ends_with("..."));
    }

    #[test]
    fn test_config_conversion() {
        let config = create_test_config();
        
        assert_eq!(config.number_of_words, 5);
        assert_eq!(config.number_of_sentences, None);
        assert_eq!(config.custom_prompt, None);
        assert!(!config.random_words);
        assert!(!config.substitute);
        assert!(!config.capitalize);
        assert!(!config.symbols);
    }

    #[test]
    fn test_generate_prompt_respects_custom_prompt() {
        let mut config = create_test_config();
        config.custom_prompt = Some("test custom prompt".to_string());
        
        let generator = WordGenerator::new(config.clone());
        let (prompt, word_count) = generator.generate_prompt();
        
        assert_eq!(prompt, "test custom prompt");
        assert_eq!(word_count, config.number_of_words);
    }

    #[test]
    fn test_priority_order_custom_prompt_over_sentences() {
        let mut config = create_test_config();
        config.number_of_sentences = Some(1);
        config.custom_prompt = Some("custom takes priority".to_string());
        
        let expected_word_count = config.number_of_words;
        let generator = WordGenerator::new(config);
        let (prompt, word_count) = generator.generate_prompt();
        
        // Custom prompt should take priority over sentences
        assert_eq!(prompt, "custom takes priority");
        assert_eq!(word_count, expected_word_count);
    }

    #[test]
    fn test_sentences_when_no_custom_prompt() {
        let mut config = create_test_config();
        config.number_of_sentences = Some(1);
        config.custom_prompt = None; // No custom prompt
        
        let generator = WordGenerator::new(config);
        let (prompt, word_count) = generator.generate_prompt();
        
        // Should generate sentences when no custom prompt is provided
        assert!(!prompt.is_empty());
        assert!(word_count > 0);
        // Should be sentence-generated content
        assert_ne!(prompt, "custom takes priority");
    }
}