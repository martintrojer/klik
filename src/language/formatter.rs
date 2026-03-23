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

        clean_punctuation_spacing(result.join(" "))
    }
}

// --- Shared symbol formatting logic ---

const MATHEMATICAL: [&str; 7] = ["+", "-", "*", "/", "=", "<", ">"];
const PROGRAMMING: [&str; 10] = ["@", "#", "$", "%", "^", "&", "|", "\\", "~", "`"];
const PUNCTUATION_SYMBOLS: [&str; 4] = [":", ";", "\"", "'"];

fn add_symbol_to_word(word: &str, rng: &mut impl Rng) -> String {
    let symbol_type = rng.gen_range(0..4);
    match symbol_type {
        0 => {
            let bracket_pair = rng.gen_range(0..3);
            match bracket_pair {
                0 => format!("({word})"),
                1 => format!("[{word}]"),
                _ => format!("{{{word}}}"),
            }
        }
        1 => {
            let symbol = MATHEMATICAL.choose(rng).unwrap();
            if rng.gen_bool(0.5) {
                format!("{symbol}{word}")
            } else {
                format!("{word}{symbol}")
            }
        }
        2 => {
            let symbol = PROGRAMMING.choose(rng).unwrap();
            format!("{symbol}{word}")
        }
        _ => {
            let symbol = PUNCTUATION_SYMBOLS.choose(rng).unwrap();
            format!("{word}{symbol}")
        }
    }
}

fn pick_extended_final_punct(rng: &mut impl Rng) -> &'static str {
    match rng.gen_range(0..100) {
        0..=50 => ".",
        51..=65 => "!",
        66..=75 => "?",
        76..=85 => ";",
        86..=92 => ":",
        _ => "...",
    }
}

fn maybe_add_separator(result: &mut Vec<String>, rng: &mut impl Rng) {
    match rng.gen_range(0..10) {
        0 => result.push(",".to_string()),
        1 => result.push(";".to_string()),
        _ => {}
    }
}

fn clean_punctuation_spacing(text: String) -> String {
    text.replace(" ,", ",")
        .replace(" .", ".")
        .replace(" !", "!")
        .replace(" ?", "?")
        .replace(" ;", ";")
        .replace(" :", ":")
}

/// Format words with symbol decorations (shared by SymbolFormatter and CombinedFormatter)
fn format_with_symbols(words: &[String], capitalize: bool, rng: &mut impl Rng) -> String {
    let mut result = Vec::new();

    for (i, word) in words.iter().enumerate() {
        let mut formatted_word = word.clone();

        if capitalize && (i == 0 || rng.gen_bool(0.2)) {
            formatted_word = capitalize_first_letter(&formatted_word);
        }

        if rng.gen_bool(0.25) {
            formatted_word = add_symbol_to_word(&formatted_word, rng);
        }

        result.push(formatted_word);

        if i < words.len() - 1 {
            maybe_add_separator(&mut result, rng);
        }
    }

    result.push(pick_extended_final_punct(rng).to_string());

    let mut text = clean_punctuation_spacing(result.join(" "));

    // Safety net: ensure the first alphabetic character is capitalized
    if capitalize {
        if let Some(pos) = text.chars().position(|c| c.is_alphabetic()) {
            let mut chars: Vec<char> = text.chars().collect();
            if chars[pos].is_lowercase() {
                chars[pos] = chars[pos].to_uppercase().next().unwrap_or(chars[pos]);
                text = chars.into_iter().collect();
            }
        }
    }

    text
}

/// Formatter that adds symbols and special characters
pub struct SymbolFormatter;

impl TextFormatter for SymbolFormatter {
    fn format(&self, words: Vec<String>) -> String {
        if words.is_empty() {
            return String::new();
        }
        format_with_symbols(&words, false, &mut rand::thread_rng())
    }
}

/// Combined formatter that handles both capitalization and symbols together
pub struct CombinedFormatter;

impl TextFormatter for CombinedFormatter {
    fn format(&self, words: Vec<String>) -> String {
        if words.is_empty() {
            return String::new();
        }
        format_with_symbols(&words, true, &mut rand::thread_rng())
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
        match (include_capitalize, include_symbols) {
            (false, false) => Box::new(BasicFormatter),
            (true, false) => Box::new(CapitalizationFormatter),
            (false, true) => Box::new(SymbolFormatter),
            (true, true) => Box::new(CombinedFormatter),
        }
    }
}

impl TextFormatter for CompositeFormatter {
    fn format(&self, words: Vec<String>) -> String {
        self.formatters
            .iter()
            .fold(words, |current_words, formatter| {
                let formatted = formatter.format(current_words);
                vec![formatted]
            })
            .into_iter()
            .next()
            .unwrap_or_default()
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

        assert!(result.chars().next().unwrap().is_uppercase());
        assert!(result.ends_with('.') || result.ends_with('!') || result.ends_with('?'));
    }

    #[test]
    fn test_symbol_formatter() {
        let formatter = SymbolFormatter;
        let words = vec!["hello".to_string(), "world".to_string()];

        let result = formatter.format(words);

        assert!(
            result.ends_with('.')
                || result.ends_with('!')
                || result.ends_with('?')
                || result.ends_with(';')
                || result.ends_with(':')
                || result.ends_with("...")
        );
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

        let first_alpha_char = result.chars().find(|c| c.is_alphabetic());
        if let Some(first_char) = first_alpha_char {
            assert!(first_char.is_uppercase());
        }
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
        let combined = CombinedFormatter;
        let words = vec!["test".to_string(), "word".to_string()];

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

            if result.starts_with(',') {
                panic!("Found comma at beginning on attempt {attempt}: '{result}'");
            }

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
