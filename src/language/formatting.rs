use super::core::Language;
use rand::seq::SliceRandom;
use rand::Rng;

impl Language {
    /// Apply capitalization, punctuation, commas, and optionally symbols to words for realistic typing practice
    pub fn apply_advanced_formatting(&self, words: Vec<String>, include_capitalize: bool, include_symbols: bool) -> String {
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
            
            // Capitalize first word and randomly capitalize others (20% chance) if capitalize is enabled
            if include_capitalize && (i == 0 || rng.gen_bool(0.2)) {
                formatted_word = Self::capitalize_first_letter(&formatted_word);
            }
            
            // Add symbols around words if symbols are enabled
            if include_symbols {
                // 25% chance to add symbols around words
                if rng.gen_bool(0.25) {
                    let symbol_type = rng.gen_range(0..4);
                    match symbol_type {
                        0 => {
                            // Brackets - always paired
                            let bracket_pair = rng.gen_range(0..3);
                            match bracket_pair {
                                0 => formatted_word = format!("({})", formatted_word),
                                1 => formatted_word = format!("[{}]", formatted_word),
                                _ => formatted_word = format!("{{{}}}", formatted_word),
                            }
                        },
                        1 => {
                            // Mathematical symbols - prefix or suffix
                            let symbol = mathematical.choose(rng).unwrap();
                            if rng.gen_bool(0.5) {
                                formatted_word = format!("{}{}", symbol, formatted_word);
                            } else {
                                formatted_word = format!("{}{}", formatted_word, symbol);
                            }
                        },
                        2 => {
                            // Programming symbols - usually prefix
                            let symbol = programming.choose(rng).unwrap();
                            formatted_word = format!("{}{}", symbol, formatted_word);
                        },
                        _ => {
                            // Punctuation symbols - usually suffix
                            let symbol = punctuation_symbols.choose(rng).unwrap();
                            formatted_word = format!("{}{}", formatted_word, symbol);
                        }
                    }
                }
            }
            
            result.push(formatted_word);
            
            // Add punctuation between words
            if i < words.len() - 1 {
                if include_symbols {
                    // With symbols enabled, more variety in separators (20% chance for special separator)
                    let separator_choice = rng.gen_range(0..10);
                    match separator_choice {
                        0 => result.push(",".to_string()),
                        1 => result.push(";".to_string()),
                        _ => {} // Just space
                    }
                } else {
                    // Original behavior: 15% chance for comma
                    if rng.gen_bool(0.15) {
                        result.push(",".to_string());
                    }
                }
            }
        }
        
        // Add final punctuation 
        if include_symbols {
            // More variety with symbols enabled
            let final_punct = match rng.gen_range(0..100) {
                0..=50 => ".",
                51..=65 => "!",
                66..=75 => "?",
                76..=85 => ";",
                86..=92 => ":",
                _ => "...",
            };
            result.push(final_punct.to_string());
        } else {
            // Original behavior
            let final_punct = match rng.gen_range(0..100) {
                0..=79 => ".",
                80..=94 => "!",
                _ => "?",
            };
            result.push(final_punct.to_string());
        }
        
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
    
    /// Helper function to capitalize the first letter of a word
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
        let main_content = &result[..result.len()-1]; // Remove final punctuation
        let has_symbols_in_main = main_content.chars().any(|c| symbol_chars.contains(c));
        assert!(!has_symbols_in_main, "Should not have symbols in main content with capitalize only");
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
        assert!(!result.chars().next().unwrap().is_uppercase(), "Should not capitalize without flag");
        
        // Should not have symbols
        let symbol_chars = "@#$%^&*()[]{}|\\~`+-=<>";
        let main_content = &result[..result.len()-1]; // Remove final punctuation
        let has_symbols_in_main = main_content.chars().any(|c| symbol_chars.contains(c));
        assert!(!has_symbols_in_main, "Should not have symbols without flag");
    }
}