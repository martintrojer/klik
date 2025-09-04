use rand::seq::SliceRandom;
use rand::Rng;

/// Trait for text formatting strategies
pub trait TextFormatter {
    /// Format a list of words into a final string
    fn format(&self, words: Vec<String>) -> String;
}

/// Basic formatter that just joins words with spaces
pub struct BasicFormatter;

impl TextFormatter for BasicFormatter {
    fn format(&self, words: Vec<String>) -> String {
        words.join(" ")
    }
}

/// Formatter that adds capitalization and punctuation
pub struct CapitalizationFormatter;

impl TextFormatter for CapitalizationFormatter {
    fn format(&self, words: Vec<String>) -> String {
        if words.is_empty() {
            return String::new();
        }

        let rng = &mut rand::thread_rng();
        let mut result = Vec::new();

        for (i, word) in words.iter().enumerate() {
            let mut formatted_word = word.clone();

            // Capitalize first word and randomly capitalize others (20% chance)
            if i == 0 || rng.gen_bool(0.2) {
                formatted_word = capitalize_first_letter(&formatted_word);
            }

            result.push(formatted_word);

            // Add commas between words (15% chance)
            if i < words.len() - 1 && rng.gen_bool(0.15) {
                result.push(",".to_string());
            }
        }

        // Add final punctuation
        let final_punct = match rng.gen_range(0..100) {
            0..=79 => ".",
            80..=94 => "!",
            _ => "?",
        };
        result.push(final_punct.to_string());

        // Clean up spacing around punctuation
        let mut text = result.join(" ");
        text = text.replace(" ,", ",");
        text = text.replace(" .", ".");
        text = text.replace(" !", "!");
        text = text.replace(" ?", "?");
        text
    }
}

/// Formatter that adds symbols and special characters
pub struct SymbolFormatter;

impl TextFormatter for SymbolFormatter {
    fn format(&self, words: Vec<String>) -> String {
        if words.is_empty() {
            return String::new();
        }

        let rng = &mut rand::thread_rng();
        let mut result = Vec::new();

        // Define symbol sets for different contexts
        let mathematical = ["+", "-", "*", "/", "=", "<", ">"];
        let programming = ["@", "#", "$", "%", "^", "&", "|", "\\", "~", "`"];
        let punctuation_symbols = [":", ";", "\"", "'"];

        for (i, word) in words.iter().enumerate() {
            let mut formatted_word = word.clone();

            // Add symbols around words (25% chance)
            if rng.gen_bool(0.25) {
                let symbol_type = rng.gen_range(0..4);
                match symbol_type {
                    0 => {
                        // Brackets - always paired
                        let bracket_pair = rng.gen_range(0..3);
                        match bracket_pair {
                            0 => formatted_word = format!("({formatted_word})"),
                            1 => formatted_word = format!("[{formatted_word}]"),
                            _ => formatted_word = format!("{{{formatted_word}}}"),
                        }
                    }
                    1 => {
                        // Mathematical symbols - prefix or suffix
                        let symbol = mathematical.choose(rng).unwrap();
                        if rng.gen_bool(0.5) {
                            formatted_word = format!("{symbol}{formatted_word}");
                        } else {
                            formatted_word = format!("{formatted_word}{symbol}");
                        }
                    }
                    2 => {
                        // Programming symbols - usually prefix
                        let symbol = programming.choose(rng).unwrap();
                        formatted_word = format!("{symbol}{formatted_word}");
                    }
                    _ => {
                        // Punctuation symbols - usually suffix
                        let symbol = punctuation_symbols.choose(rng).unwrap();
                        formatted_word = format!("{formatted_word}{symbol}");
                    }
                }
            }

            result.push(formatted_word);

            // Add special separators between words (20% chance)
            if i < words.len() - 1 {
                let separator_choice = rng.gen_range(0..10);
                match separator_choice {
                    0 => result.push(",".to_string()),
                    1 => result.push(";".to_string()),
                    _ => {} // Just space
                }
            }
        }

        // Add final punctuation with more variety
        let final_punct = match rng.gen_range(0..100) {
            0..=50 => ".",
            51..=65 => "!",
            66..=75 => "?",
            76..=85 => ";",
            86..=92 => ":",
            _ => "...",
        };
        result.push(final_punct.to_string());

        // Clean up spacing around punctuation
        let mut text = result.join(" ");
        text = text.replace(" ,", ",");
        text = text.replace(" .", ".");
        text = text.replace(" !", "!");
        text = text.replace(" ?", "?");
        text = text.replace(" ;", ";");
        text = text.replace(" :", ":");
        text
    }
}

/// Composite formatter that combines multiple formatters
pub struct CompositeFormatter {
    formatters: Vec<Box<dyn TextFormatter>>,
}

impl Default for CompositeFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl CompositeFormatter {
    pub fn new() -> Self {
        Self {
            formatters: Vec::new(),
        }
    }

    pub fn add_formatter(mut self, formatter: Box<dyn TextFormatter>) -> Self {
        self.formatters.push(formatter);
        self
    }

    pub fn build_from_flags(
        include_capitalize: bool,
        include_symbols: bool,
    ) -> Box<dyn TextFormatter> {
        if !include_capitalize && !include_symbols {
            return Box::new(BasicFormatter);
        }

        let mut composite = CompositeFormatter::new();

        if include_capitalize && include_symbols {
            // For combined formatting, use the original advanced formatting logic
            composite = composite.add_formatter(Box::new(CombinedFormatter));
        } else if include_capitalize {
            composite = composite.add_formatter(Box::new(CapitalizationFormatter));
        } else if include_symbols {
            composite = composite.add_formatter(Box::new(SymbolFormatter));
        }

        Box::new(composite)
    }
}

impl TextFormatter for CompositeFormatter {
    fn format(&self, words: Vec<String>) -> String {
        // Apply formatters in sequence
        self.formatters
            .iter()
            .fold(words, |current_words, formatter| {
                // For sequential application, we need to split the formatted text back to words
                // This is a simplification - in practice, you might want more sophisticated chaining
                let formatted = formatter.format(current_words);
                vec![formatted]
            })
            .into_iter()
            .next()
            .unwrap_or_default()
    }
}

/// Combined formatter that handles both capitalization and symbols together
/// (preserves the original advanced formatting behavior)
pub struct CombinedFormatter;

impl TextFormatter for CombinedFormatter {
    fn format(&self, words: Vec<String>) -> String {
        if words.is_empty() {
            return String::new();
        }

        let rng = &mut rand::thread_rng();
        let mut result = Vec::new();

        // Define symbol sets for different contexts
        let mathematical = ["+", "-", "*", "/", "=", "<", ">"];
        let programming = ["@", "#", "$", "%", "^", "&", "|", "\\", "~", "`"];
        let punctuation_symbols = [":", ";", "\"", "'"];

        for (i, word) in words.iter().enumerate() {
            let mut formatted_word = word.clone();

            // Capitalize first word and randomly capitalize others (20% chance)
            if i == 0 || rng.gen_bool(0.2) {
                formatted_word = capitalize_first_letter(&formatted_word);
            }

            // Add symbols around words (25% chance)
            if rng.gen_bool(0.25) {
                let symbol_type = rng.gen_range(0..4);
                match symbol_type {
                    0 => {
                        // Brackets - always paired
                        let bracket_pair = rng.gen_range(0..3);
                        match bracket_pair {
                            0 => formatted_word = format!("({formatted_word})"),
                            1 => formatted_word = format!("[{formatted_word}]"),
                            _ => formatted_word = format!("{{{formatted_word}}}"),
                        }
                    }
                    1 => {
                        // Mathematical symbols - prefix or suffix
                        let symbol = mathematical.choose(rng).unwrap();
                        if rng.gen_bool(0.5) {
                            formatted_word = format!("{symbol}{formatted_word}");
                        } else {
                            formatted_word = format!("{formatted_word}{symbol}");
                        }
                    }
                    2 => {
                        // Programming symbols - usually prefix
                        let symbol = programming.choose(rng).unwrap();
                        formatted_word = format!("{symbol}{formatted_word}");
                    }
                    _ => {
                        // Punctuation symbols - usually suffix
                        let symbol = punctuation_symbols.choose(rng).unwrap();
                        formatted_word = format!("{formatted_word}{symbol}");
                    }
                }
            }

            result.push(formatted_word);

            // Add punctuation between words
            if i < words.len() - 1 {
                // With symbols enabled, more variety in separators (20% chance for special separator)
                let separator_choice = rng.gen_range(0..10);
                match separator_choice {
                    0 => result.push(",".to_string()),
                    1 => result.push(";".to_string()),
                    _ => {} // Just space
                }
            }
        }

        // Add final punctuation with more variety
        let final_punct = match rng.gen_range(0..100) {
            0..=50 => ".",
            51..=65 => "!",
            66..=75 => "?",
            76..=85 => ";",
            86..=92 => ":",
            _ => "...",
        };
        result.push(final_punct.to_string());

        // Clean up spacing around punctuation
        let mut text = result.join(" ");
        text = text.replace(" ,", ",");
        text = text.replace(" .", ".");
        text = text.replace(" !", "!");
        text = text.replace(" ?", "?");
        text = text.replace(" ;", ";");
        text = text.replace(" :", ":");

        // Ensure the first alphabetic character is capitalized
        // This is a safety net to handle edge cases in the complex formatting logic
        if let Some(first_alpha_pos) = text.chars().position(|c| c.is_alphabetic()) {
            let mut chars: Vec<char> = text.chars().collect();
            if let Some(first_alpha_char) = chars.get(first_alpha_pos) {
                if first_alpha_char.is_lowercase() {
                    chars[first_alpha_pos] = first_alpha_char
                        .to_uppercase()
                        .next()
                        .unwrap_or(*first_alpha_char);
                    text = chars.into_iter().collect();
                }
            }
        }

        text
    }
}

/// Helper function to capitalize the first letter of a word
fn capitalize_first_letter(word: &str) -> String {
    let mut chars: Vec<char> = word.chars().collect();
    if !chars.is_empty() && chars[0].is_alphabetic() {
        chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
    }
    chars.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_formatter() {
        let formatter = BasicFormatter;
        let words = vec!["hello".to_string(), "world".to_string()];

        let result = formatter.format(words);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_capitalization_formatter() {
        let formatter = CapitalizationFormatter;
        let words = vec!["hello".to_string(), "world".to_string()];

        let result = formatter.format(words);

        // Should start with capital letter
        assert!(result.chars().next().unwrap().is_uppercase());
        // Should end with punctuation
        assert!(result.ends_with('.') || result.ends_with('!') || result.ends_with('?'));
    }

    #[test]
    fn test_symbol_formatter() {
        let formatter = SymbolFormatter;
        let words = vec!["hello".to_string(), "world".to_string()];

        let result = formatter.format(words);

        // Should end with punctuation
        assert!(
            result.ends_with('.')
                || result.ends_with('!')
                || result.ends_with('?')
                || result.ends_with(';')
                || result.ends_with(':')
                || result.ends_with("...")
        );
        // Should contain the original words
        let lowercase_result = result.to_lowercase();
        assert!(lowercase_result.contains("hello"));
        assert!(lowercase_result.contains("world"));
    }

    #[test]
    fn test_composite_formatter_build_from_flags() {
        let words = vec!["hello".to_string(), "world".to_string()];

        // Test basic (no flags)
        let basic_formatter = CompositeFormatter::build_from_flags(false, false);
        let basic_result = basic_formatter.format(words.clone());
        assert_eq!(basic_result, "hello world");

        // Test capitalization only
        let cap_formatter = CompositeFormatter::build_from_flags(true, false);
        let cap_result = cap_formatter.format(words.clone());
        assert!(cap_result.chars().next().unwrap().is_uppercase());

        // Test symbols only
        let sym_formatter = CompositeFormatter::build_from_flags(false, true);
        let sym_result = sym_formatter.format(words.clone());
        assert!(!sym_result.is_empty());

        // Test combined
        let combined_formatter = CompositeFormatter::build_from_flags(true, true);
        let combined_result = combined_formatter.format(words);
        // Should have capitalization (first alphabetic character should be uppercase)
        let first_alpha_char = combined_result.chars().find(|c| c.is_alphabetic());
        if let Some(first_char) = first_alpha_char {
            assert!(first_char.is_uppercase());
        }
    }

    #[test]
    fn test_capitalize_first_letter() {
        assert_eq!(capitalize_first_letter("hello"), "Hello");
        assert_eq!(capitalize_first_letter("WORLD"), "WORLD");
        assert_eq!(capitalize_first_letter("test123"), "Test123");
        assert_eq!(capitalize_first_letter(""), "");
        assert_eq!(capitalize_first_letter("123abc"), "123abc");
    }

    #[test]
    fn test_formatters_with_empty_input() {
        let empty_words = vec![];

        assert_eq!(BasicFormatter.format(empty_words.clone()), "");
        assert_eq!(CapitalizationFormatter.format(empty_words.clone()), "");
        assert_eq!(SymbolFormatter.format(empty_words.clone()), "");
    }

    #[test]
    fn test_formatters_with_single_word() {
        let single_word = vec!["test".to_string()];

        let basic_result = BasicFormatter.format(single_word.clone());
        assert_eq!(basic_result, "test");

        let cap_result = CapitalizationFormatter.format(single_word.clone());
        assert!(cap_result.starts_with("Test"));
        assert!(
            cap_result.ends_with('.') || cap_result.ends_with('!') || cap_result.ends_with('?')
        );

        let sym_result = SymbolFormatter.format(single_word);
        assert!(!sym_result.is_empty());
    }

    #[test]
    fn test_composite_formatter_new_and_add() {
        let composite = CompositeFormatter::new().add_formatter(Box::new(BasicFormatter));

        let words = vec!["hello".to_string(), "world".to_string()];
        let result = composite.format(words);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_combined_formatter_empty_input() {
        let combined = CombinedFormatter;
        let empty_words = vec![];

        let result = combined.format(empty_words);
        assert_eq!(result, "");
    }

    #[test]
    fn test_combined_formatter_functionality() {
        let combined = CombinedFormatter;
        let words = vec!["hello".to_string(), "world".to_string()];

        let result = combined.format(words);

        // Should have capitalization (first alphabetic character should be uppercase)
        let first_alpha_char = result.chars().find(|c| c.is_alphabetic());
        if let Some(first_char) = first_alpha_char {
            assert!(first_char.is_uppercase());
        }
        // Should end with punctuation
        assert!(
            result.ends_with('.')
                || result.ends_with('!')
                || result.ends_with('?')
                || result.ends_with(';')
                || result.ends_with(':')
                || result.ends_with("...")
        );
    }

    #[test]
    fn test_combined_formatter_capitalization_guaranteed() {
        // Test that the first word is ALWAYS capitalized in CombinedFormatter
        let combined = CombinedFormatter;
        let words = vec!["test".to_string(), "word".to_string()];

        // Test multiple times to ensure capitalization is consistent
        for attempt in 0..100 {
            let result = combined.format(words.clone());

            let first_alpha_char = result.chars().find(|c| c.is_alphabetic());
            if let Some(first_char) = first_alpha_char {
                assert!(
                    first_char.is_uppercase(),
                    "First alphabetic character should ALWAYS be uppercase on attempt {attempt}. Generated result: '{result}'",
                );
            }
        }
    }

    #[test]
    fn test_combined_formatter_debug_comma_issue() {
        // Try to reproduce the comma-at-beginning issue
        let combined = CombinedFormatter;
        let words = vec![
            "ve".to_string(),
            "nt".to_string(),
            "ask".to_string(),
            "yany".to_string(),
            "i".to_string(),
        ];

        for attempt in 0..100 {
            let result = combined.format(words.clone());

            // Check for comma at beginning
            if result.starts_with(',') {
                panic!("Found comma at beginning on attempt {attempt}: '{result}'");
            }

            // Check first alphabetic character
            let first_alpha_char = result.chars().find(|c| c.is_alphabetic());
            if let Some(first_char) = first_alpha_char {
                assert!(
                    first_char.is_uppercase(),
                    "First alphabetic character should be uppercase on attempt {attempt}. Generated result: '{result}'",
                );
            }
        }
    }
}
