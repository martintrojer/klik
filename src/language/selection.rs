use super::{
    core::Language,
    difficulty::CharacterDifficulty,
    selector::{IntelligentSelector, RandomSelector, SubstitutionSelector, WordSelector},
};
// Delegates selection to selector module; no direct RNG use here
use std::collections::HashMap;

impl Language {
    /// Get random words from the language
    pub fn get_random(&self, num: usize) -> Vec<String> {
        // Delegate to the unified selector implementation
        let empty: std::collections::HashMap<char, CharacterDifficulty> = Default::default();
        RandomSelector.select_words(self, num, &empty)
    }

    /// Get words with character substitution: replace some characters with ones that need most practice
    pub fn get_substituted(
        &self,
        num: usize,
        char_stats: &HashMap<char, CharacterDifficulty>,
    ) -> Vec<String> {
        SubstitutionSelector.select_words(self, num, char_stats)
    }

    /// Get words intelligently selected based on character statistics
    /// Words containing characters that need more practice are prioritized
    pub fn get_intelligent(
        &self,
        num: usize,
        char_stats: &HashMap<char, CharacterDifficulty>,
    ) -> Vec<String> {
        IntelligentSelector.select_words(self, num, char_stats)
    }

    /// Calculate difficulty score for a word based on character statistics
    #[allow(dead_code)]
    fn calculate_word_difficulty_score(
        &self,
        _word: &str,
        _char_stats: &HashMap<char, CharacterDifficulty>,
    ) -> f64 {
        // Deprecated: logic moved to selector module; kept for compatibility
        0.0
    }

    /// Get the weakest characters (those that need most practice) from character statistics
    #[allow(dead_code)]
    fn get_weakest_characters(
        &self,
        _char_stats: &HashMap<char, CharacterDifficulty>,
        _count: usize,
    ) -> Vec<char> {
        // Deprecated: logic moved to selector module; kept for compatibility
        Vec::new()
    }

    /// Substitute some characters in a word with weaker characters for practice
    #[allow(dead_code)]
    fn substitute_characters_in_word(&self, word: &str, _weak_chars: &[char]) -> String {
        // Deprecated: logic moved to selector module; kept for compatibility
        word.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_language() -> Language {
        Language {
            name: "test".to_string(),
            size: 4,
            words: vec![
                "easy".to_string(), // Contains 'e' (easy)
                "hard".to_string(), // Contains 'h' (hard)
                "test".to_string(), // Contains 't' (medium)
                "zap".to_string(),  // Contains 'z' (very hard)
            ],
        }
    }

    fn create_test_char_stats() -> HashMap<char, CharacterDifficulty> {
        let mut char_stats = HashMap::new();
        char_stats.insert(
            'e',
            CharacterDifficulty {
                miss_rate: 2.0,
                avg_time_ms: 120.0,
                total_attempts: 50,
                uppercase_miss_rate: 5.0,
                uppercase_avg_time: 140.0,
                uppercase_attempts: 10,
                uppercase_penalty: 0.2,
            },
        );
        char_stats.insert(
            'h',
            CharacterDifficulty {
                miss_rate: 15.0,
                avg_time_ms: 250.0,
                total_attempts: 30,
                uppercase_miss_rate: 25.0,
                uppercase_avg_time: 350.0,
                uppercase_attempts: 8,
                uppercase_penalty: 0.6,
            },
        );
        char_stats.insert(
            't',
            CharacterDifficulty {
                miss_rate: 8.0,
                avg_time_ms: 180.0,
                total_attempts: 40,
                uppercase_miss_rate: 12.0,
                uppercase_avg_time: 220.0,
                uppercase_attempts: 15,
                uppercase_penalty: 0.3,
            },
        );
        char_stats.insert(
            'z',
            CharacterDifficulty {
                miss_rate: 25.0,
                avg_time_ms: 400.0,
                total_attempts: 10,
                uppercase_miss_rate: 40.0,
                uppercase_avg_time: 600.0,
                uppercase_attempts: 3,
                uppercase_penalty: 0.8,
            },
        );
        char_stats
    }

    #[test]
    fn test_get_random_words() {
        let lang = Language::new("english".to_string());

        let words = lang.get_random(5);
        assert_eq!(words.len(), 5);

        for word in &words {
            assert!(lang.words.contains(word));
        }
    }

    #[test]
    fn test_get_random_single_word() {
        let lang = Language::new("english".to_string());

        let words = lang.get_random(1);
        assert_eq!(words.len(), 1);
        assert!(lang.words.contains(&words[0]));
    }

    #[test]
    fn test_get_random_zero_words() {
        let lang = Language::new("english".to_string());

        let words = lang.get_random(0);
        assert_eq!(words.len(), 0);
    }

    #[test]
    fn test_intelligent_selection_prioritizes_difficult_characters() {
        let lang = create_test_language();
        let char_stats = create_test_char_stats();

        // Test multiple selections to check statistical preference
        let mut hard_count = 0;
        let mut zap_count = 0;
        let trials = 100;

        for _ in 0..trials {
            let words = lang.get_intelligent(2, &char_stats);
            if words.contains(&"hard".to_string()) {
                hard_count += 1;
            }
            if words.contains(&"zap".to_string()) {
                zap_count += 1;
            }
        }

        // "zap" and "hard" should be selected more often than "easy"
        // due to higher difficulty scores
        assert!(
            zap_count > trials / 4,
            "zap should be selected frequently (got {zap_count} out of {trials})",
        );
        assert!(
            hard_count > trials / 4,
            "hard should be selected frequently (got {hard_count} out of {trials})",
        );
    }

    #[test]
    fn test_get_substituted_returns_correct_count() {
        let lang = Language::new("english".to_string());

        let mut char_stats = HashMap::new();
        char_stats.insert(
            'x',
            CharacterDifficulty {
                miss_rate: 20.0,
                avg_time_ms: 300.0,
                total_attempts: 10,
                uppercase_miss_rate: 25.0,
                uppercase_avg_time: 400.0,
                uppercase_attempts: 3,
                uppercase_penalty: 0.5,
            },
        );

        for count in [1, 5, 10] {
            let words = lang.get_substituted(count, &char_stats);
            assert_eq!(words.len(), count, "Should return exactly {count} words");
        }
    }
}
