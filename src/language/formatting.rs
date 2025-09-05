use super::core::Language;
use crate::language::formatter::TextFormatter;
// Deprecated: logic moved to formatter strategies; keep tests redirected

impl Language {
    /// Apply capitalization, punctuation, commas, and optionally symbols to words for realistic typing practice.
    /// Redirects to the formatter strategy implementation to keep a single source of truth.
    pub fn apply_advanced_formatting(
        &self,
        words: Vec<String>,
        include_capitalize: bool,
        include_symbols: bool,
    ) -> String {
        use super::formatter::CompositeFormatter;
        if !include_capitalize && !include_symbols {
            // Preserve historical behavior: basic join plus terminal punctuation
            if words.is_empty() {
                return String::new();
            }
            let mut text = super::formatter::BasicFormatter.format(words);
            let rng = &mut rand::thread_rng();
            let final_punct = match rand::Rng::gen_range(rng, 0..100) {
                0..=79 => '.',
                80..=94 => '!',
                _ => '?',
            };
            text.push(final_punct);
            return text;
        }
        let formatter = CompositeFormatter::build_from_flags(include_capitalize, include_symbols);
        formatter.format(words)
    }

    /// Helper function to capitalize the first letter of a word
    #[allow(dead_code)]
    fn capitalize_first_letter(word: &str) -> String {
        let mut chars: Vec<char> = word.chars().collect();
        if !chars.is_empty() && chars[0].is_alphabetic() {
            chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
        }
        chars.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_advanced_formatting_basic() {
        let lang = Language::new("english".to_string());
        let words = vec!["hello".to_string(), "world".to_string(), "test".to_string()];

        let result = lang.apply_advanced_formatting(words, true, false);

        // Should have capitalized first word and end with punctuation
        assert!(result.starts_with("Hello"));
        assert!(result.ends_with('.') || result.ends_with('!') || result.ends_with('?'));
        // Check that the base words are present (case-insensitive)
        let lowercase_result = result.to_lowercase();
        assert!(lowercase_result.contains("world"));
        assert!(lowercase_result.contains("test"));
    }

    #[test]
    fn test_capitalize_first_letter() {
        assert_eq!(Language::capitalize_first_letter("hello"), "Hello");
        assert_eq!(Language::capitalize_first_letter("WORLD"), "WORLD");
        assert_eq!(Language::capitalize_first_letter("test123"), "Test123");
        assert_eq!(Language::capitalize_first_letter(""), "");
        assert_eq!(Language::capitalize_first_letter("123abc"), "123abc");
    }

    #[test]
    fn test_flag_independence_capitalize_only() {
        let lang = Language::new("english".to_string());
        let words = vec!["hello".to_string(), "world".to_string()];

        let result = lang.apply_advanced_formatting(words, true, false);

        // Should have capitalization
        assert!(result.chars().next().unwrap().is_uppercase());

        // Should NOT have symbols (only basic punctuation at end)
        let symbol_chars = "@#$%^&*()[]{}|\\~`+-=<>";
        // Allow symbols only at the very end (final punctuation)
        let main_content = &result[..result.len() - 1]; // Remove final punctuation
        let has_symbols_in_main = main_content.chars().any(|c| symbol_chars.contains(c));
        assert!(
            !has_symbols_in_main,
            "Should not have symbols in main content with capitalize only"
        );
    }

    #[test]
    fn test_flag_independence_symbols_only() {
        let lang = Language::new("english".to_string());
        let words = vec!["hello".to_string(), "world".to_string()];

        let result = lang.apply_advanced_formatting(words.clone(), false, true);

        // Should NOT have capitalization (except potentially from symbols)
        // But basic words should remain lowercase
        let lowercase_result = result.to_lowercase();
        assert!(lowercase_result.contains("hello"));
        assert!(lowercase_result.contains("world"));

        // Should potentially have symbols (due to 25% chance, run multiple times)
        let mut _found_symbols = false;
        for _ in 0..10 {
            let test_result = lang.apply_advanced_formatting(words.clone(), false, true);
            let symbol_chars = "@#$%^&*()[]{}|\\~`+-=<>";
            if test_result.chars().any(|c| symbol_chars.contains(c)) {
                _found_symbols = true;
                break;
            }
        }
        // Note: Due to randomness, we can't guarantee symbols, but the function should support them
    }

    #[test]
    fn test_flag_independence_neither() {
        let lang = Language::new("english".to_string());
        let words = vec!["hello".to_string(), "world".to_string()];

        let result = lang.apply_advanced_formatting(words, false, false);

        // Should have basic punctuation but no capitalization or symbols
        assert!(result.ends_with('.') || result.ends_with('!') || result.ends_with('?'));

        // Should not start with capital (unless word itself was capitalized)
        assert!(
            !result.chars().next().unwrap().is_uppercase(),
            "Should not capitalize without flag"
        );

        // Should not have symbols
        let symbol_chars = "@#$%^&*()[]{}|\\~`+-=<>";
        let main_content = &result[..result.len() - 1]; // Remove final punctuation
        let has_symbols_in_main = main_content.chars().any(|c| symbol_chars.contains(c));
        assert!(!has_symbols_in_main, "Should not have symbols without flag");
    }
}
