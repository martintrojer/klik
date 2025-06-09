use super::{core::Language, difficulty::CharacterDifficulty};
use rand::seq::SliceRandom;
use rand::Rng;
use std::collections::HashMap;

/// Trait for different word selection strategies
pub trait WordSelector {
    /// Select words from the language based on the strategy
    fn select_words(
        &self,
        language: &Language,
        count: usize,
        char_stats: &HashMap<char, CharacterDifficulty>,
    ) -> Vec<String>;
}

/// Random word selection (legacy behavior)
pub struct RandomSelector;

impl WordSelector for RandomSelector {
    fn select_words(
        &self,
        language: &Language,
        count: usize,
        _char_stats: &HashMap<char, CharacterDifficulty>,
    ) -> Vec<String> {
        let mut rng = &mut rand::thread_rng();
        language
            .words
            .choose_multiple(&mut rng, count)
            .cloned()
            .collect()
    }
}

/// Intelligent word selection based on character difficulty
pub struct IntelligentSelector;

impl WordSelector for IntelligentSelector {
    fn select_words(
        &self,
        language: &Language,
        count: usize,
        char_stats: &HashMap<char, CharacterDifficulty>,
    ) -> Vec<String> {
        if char_stats.is_empty() {
            // Fall back to random selection if no statistics available
            return RandomSelector.select_words(language, count, char_stats);
        }

        // Score each word based on the difficulty of characters it contains
        let mut word_scores: Vec<(String, f64)> = language
            .words
            .iter()
            .map(|word| {
                let score = calculate_word_difficulty_score(word, char_stats);
                (word.clone(), score)
            })
            .collect();

        // Sort by score (highest difficulty first for more practice)
        word_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Select from top 30% of difficult words to avoid repetition while still targeting weak areas
        let selection_pool_size = (word_scores.len() as f64 * 0.3)
            .max(count as f64)
            .min(word_scores.len() as f64) as usize;
        let selection_pool = &word_scores[0..selection_pool_size];

        // Randomly select from the high-difficulty pool
        let mut rng = &mut rand::thread_rng();
        selection_pool
            .choose_multiple(&mut rng, count)
            .map(|(word, _score)| word.clone())
            .collect()
    }
}

/// Character substitution selector
pub struct SubstitutionSelector;

impl WordSelector for SubstitutionSelector {
    fn select_words(
        &self,
        language: &Language,
        count: usize,
        char_stats: &HashMap<char, CharacterDifficulty>,
    ) -> Vec<String> {
        if char_stats.is_empty() {
            // Fall back to random selection if no statistics available
            return RandomSelector.select_words(language, count, char_stats);
        }

        // Get regular words first
        let base_words = RandomSelector.select_words(language, count, char_stats);

        // Find the most difficult characters to practice
        let weak_chars = get_weakest_characters(char_stats, 10);

        // For each word, substitute some characters with weak ones
        base_words
            .into_iter()
            .map(|word| substitute_characters_in_word(&word, &weak_chars))
            .collect()
    }
}

/// Calculate difficulty score for a word based on character statistics
fn calculate_word_difficulty_score(
    word: &str,
    char_stats: &HashMap<char, CharacterDifficulty>,
) -> f64 {
    let chars: Vec<char> = word.chars().collect();
    if chars.is_empty() {
        return 0.0;
    }

    let mut total_score = 0.0;
    let mut char_count = 0;

    for ch in chars {
        let base_char = ch.to_lowercase().next().unwrap_or(ch);
        let is_uppercase = ch != base_char;

        if let Some(difficulty) = char_stats.get(&base_char) {
            // Base difficulty calculation
            let miss_penalty = difficulty.miss_rate * 2.0; // Miss rate has higher weight
            let timing_penalty = if difficulty.avg_time_ms > 200.0 {
                (difficulty.avg_time_ms - 200.0) / 100.0 // Normalize timing penalty
            } else {
                0.0
            };

            let mut char_score = miss_penalty + timing_penalty;

            // Apply uppercase penalty if applicable
            if is_uppercase && ch.is_alphabetic() {
                let uppercase_multiplier = 1.0 + difficulty.uppercase_penalty;
                char_score *= uppercase_multiplier;

                // Additional penalty based on uppercase-specific performance
                if difficulty.uppercase_attempts > 0 {
                    let uppercase_miss_penalty = difficulty.uppercase_miss_rate * 1.5;
                    let uppercase_timing_penalty = if difficulty.uppercase_avg_time > 200.0 {
                        (difficulty.uppercase_avg_time - 200.0) / 100.0
                    } else {
                        0.0
                    };
                    char_score += (uppercase_miss_penalty + uppercase_timing_penalty) * 0.5;
                }
            }

            total_score += char_score;
            char_count += 1;
        } else if ch.is_alphabetic() {
            // Unknown alphabetic characters get higher priority if uppercase
            let base_score = 5.0;
            total_score += if is_uppercase {
                base_score * 1.5
            } else {
                base_score
            };
            char_count += 1;
        } else {
            // Punctuation gets medium difficulty score
            total_score += 3.0;
            char_count += 1;
        }
    }

    if char_count == 0 {
        0.0
    } else {
        total_score / char_count as f64
    }
}

/// Get the weakest characters (those that need most practice) from character statistics
fn get_weakest_characters(
    char_stats: &HashMap<char, CharacterDifficulty>,
    count: usize,
) -> Vec<char> {
    let mut char_difficulties: Vec<(char, f64)> = char_stats
        .iter()
        .map(|(ch, difficulty)| {
            // Calculate combined difficulty score (higher = more practice needed)
            let miss_penalty = difficulty.miss_rate * 2.0;
            let timing_penalty = if difficulty.avg_time_ms > 200.0 {
                (difficulty.avg_time_ms - 200.0) / 100.0
            } else {
                0.0
            };
            let combined_difficulty = miss_penalty + timing_penalty;
            (*ch, combined_difficulty)
        })
        .collect();

    // Sort by difficulty (highest first)
    char_difficulties.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Return the weakest characters, limited by count
    char_difficulties
        .into_iter()
        .take(count)
        .map(|(ch, _)| ch)
        .collect()
}

/// Substitute some characters in a word with weaker characters for practice
fn substitute_characters_in_word(word: &str, weak_chars: &[char]) -> String {
    if weak_chars.is_empty() || word.is_empty() {
        return word.to_string();
    }

    let rng = &mut rand::thread_rng();
    let chars: Vec<char> = word.chars().collect();
    let mut result: Vec<char> = Vec::with_capacity(chars.len());

    for ch in chars {
        // Only substitute alphabetic characters, preserve punctuation/spaces
        if ch.is_alphabetic() && rng.gen_bool(0.3) {
            // 30% chance to substitute each character
            if let Some(&weak_char) = weak_chars.choose(rng) {
                // Preserve case: if original was uppercase, make weak char uppercase too
                if ch.is_uppercase() {
                    result.push(weak_char.to_uppercase().next().unwrap_or(weak_char));
                } else {
                    result.push(weak_char);
                }
            } else {
                result.push(ch);
            }
        } else {
            result.push(ch);
        }
    }

    result.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_language() -> Language {
        Language {
            name: "test".to_string(),
            size: 4,
            words: vec![
                "easy".to_string(),
                "hard".to_string(),
                "test".to_string(),
                "zap".to_string(),
            ],
        }
    }

    fn create_test_char_stats() -> HashMap<char, CharacterDifficulty> {
        let mut char_stats = HashMap::new();
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
    fn test_random_selector() {
        let selector = RandomSelector;
        let language = create_test_language();
        let char_stats = HashMap::new();

        let words = selector.select_words(&language, 2, &char_stats);
        assert_eq!(words.len(), 2);

        for word in &words {
            assert!(language.words.contains(word));
        }
    }

    #[test]
    fn test_intelligent_selector() {
        let selector = IntelligentSelector;
        let language = create_test_language();
        let char_stats = create_test_char_stats();

        let words = selector.select_words(&language, 2, &char_stats);
        assert_eq!(words.len(), 2);

        for word in &words {
            assert!(language.words.contains(word));
        }
    }

    #[test]
    fn test_substitution_selector() {
        let selector = SubstitutionSelector;
        let language = create_test_language();
        let char_stats = create_test_char_stats();

        let words = selector.select_words(&language, 2, &char_stats);
        assert_eq!(words.len(), 2);

        // Words should be same length as originals but potentially modified
        for word in &words {
            assert!(!word.is_empty());
        }
    }

    #[test]
    fn test_selector_fallback_to_random() {
        let selectors: Vec<Box<dyn WordSelector>> = vec![
            Box::new(IntelligentSelector),
            Box::new(SubstitutionSelector),
        ];

        let language = create_test_language();
        let empty_stats = HashMap::new();

        for selector in selectors {
            let words = selector.select_words(&language, 2, &empty_stats);
            assert_eq!(words.len(), 2);

            for word in &words {
                assert!(language.words.contains(word));
            }
        }
    }

    #[test]
    fn test_calculate_word_difficulty_score() {
        let char_stats = create_test_char_stats();

        let score1 = calculate_word_difficulty_score("zap", &char_stats);
        let score2 = calculate_word_difficulty_score("easy", &char_stats);

        // Word with difficult character should score higher
        assert!(score1 > score2);
    }

    #[test]
    fn test_calculate_word_difficulty_score_edge_cases() {
        let char_stats = create_test_char_stats();

        // Empty word
        let empty_score = calculate_word_difficulty_score("", &char_stats);
        assert_eq!(empty_score, 0.0);

        // Word with unknown characters
        let unknown_score = calculate_word_difficulty_score("xyz", &HashMap::new());
        assert!(unknown_score > 0.0);

        // Word with mixed known and unknown characters
        let mixed_score = calculate_word_difficulty_score("zxy", &char_stats);
        assert!(mixed_score > 0.0);
    }

    #[test]
    fn test_get_weakest_characters() {
        let char_stats = create_test_char_stats();

        let weak_chars = get_weakest_characters(&char_stats, 1);
        assert_eq!(weak_chars.len(), 1);
        assert_eq!(weak_chars[0], 'z'); // 'z' should be the weakest based on our test data

        // Test with more characters than available
        let all_weak_chars = get_weakest_characters(&char_stats, 10);
        assert_eq!(all_weak_chars.len(), 1); // Only one character in test data
    }

    #[test]
    fn test_substitute_characters_in_word() {
        let weak_chars = vec!['x', 'y', 'z'];

        // Test with empty word
        let empty_result = substitute_characters_in_word("", &weak_chars);
        assert_eq!(empty_result, "");

        // Test with empty weak chars
        let no_substitution = substitute_characters_in_word("hello", &[]);
        assert_eq!(no_substitution, "hello");

        // Test with actual substitution - should preserve word length
        let substituted = substitute_characters_in_word("hello", &weak_chars);
        assert_eq!(substituted.len(), 5);

        // Test case preservation
        let uppercase_result = substitute_characters_in_word("HELLO", &weak_chars);
        assert_eq!(uppercase_result.len(), 5);
        // At least the first character should remain uppercase if substituted
        if !uppercase_result.starts_with('H') {
            assert!(uppercase_result.chars().next().unwrap().is_uppercase());
        }
    }

    #[test]
    fn test_word_selectors_with_different_counts() {
        let language = create_test_language();
        let char_stats = create_test_char_stats();

        let selectors: Vec<Box<dyn WordSelector>> = vec![
            Box::new(RandomSelector),
            Box::new(IntelligentSelector),
            Box::new(SubstitutionSelector),
        ];

        for selector in selectors {
            // Test with count 0
            let zero_words = selector.select_words(&language, 0, &char_stats);
            assert_eq!(zero_words.len(), 0);

            // Test with count 1
            let one_word = selector.select_words(&language, 1, &char_stats);
            assert_eq!(one_word.len(), 1);

            // Test with count larger than available words
            let many_words = selector.select_words(&language, 100, &char_stats);
            assert!(many_words.len() <= language.words.len());
        }
    }

    #[test]
    fn test_calculate_word_difficulty_score_with_uppercase() {
        let mut char_stats = HashMap::new();
        char_stats.insert(
            'a',
            CharacterDifficulty {
                miss_rate: 10.0,
                avg_time_ms: 200.0,
                total_attempts: 20,
                uppercase_miss_rate: 25.0,
                uppercase_avg_time: 350.0,
                uppercase_attempts: 5,
                uppercase_penalty: 0.8,
            },
        );

        let lowercase_score = calculate_word_difficulty_score("aaa", &char_stats);
        let uppercase_score = calculate_word_difficulty_score("AAA", &char_stats);

        // Uppercase should score higher due to penalty
        assert!(uppercase_score > lowercase_score);
    }

    #[test]
    fn test_intelligent_selector_with_limited_words() {
        let small_language = Language {
            name: "small".to_string(),
            size: 2,
            words: vec!["a".to_string(), "z".to_string()],
        };

        let char_stats = create_test_char_stats();
        let selector = IntelligentSelector;

        let words = selector.select_words(&small_language, 2, &char_stats);
        assert_eq!(words.len(), 2);
    }
}
