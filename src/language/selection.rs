use super::{core::Language, difficulty::CharacterDifficulty};
use rand::seq::SliceRandom;
use rand::Rng;
use std::collections::HashMap;

impl Language {
    /// Get random words from the language
    pub fn get_random(&self, num: usize) -> Vec<String> {
        let mut rng = &mut rand::thread_rng();
        self.words.choose_multiple(&mut rng, num).cloned().collect()
    }

    /// Get words with character substitution: replace some characters with ones that need most practice
    pub fn get_substituted(
        &self,
        num: usize,
        char_stats: &HashMap<char, CharacterDifficulty>,
    ) -> Vec<String> {
        if char_stats.is_empty() {
            // Fall back to random selection if no statistics available
            return self.get_random(num);
        }

        // Get regular words first
        let base_words = self.get_random(num);

        // Find the most difficult characters to practice
        let weak_chars = self.get_weakest_characters(char_stats, 10);

        // For each word, substitute some characters with weak ones
        base_words
            .into_iter()
            .map(|word| self.substitute_characters_in_word(&word, &weak_chars))
            .collect()
    }

    /// Get words intelligently selected based on character statistics
    /// Words containing characters that need more practice are prioritized
    pub fn get_intelligent(
        &self,
        num: usize,
        char_stats: &HashMap<char, CharacterDifficulty>,
    ) -> Vec<String> {
        if char_stats.is_empty() {
            // Fall back to random selection if no statistics available
            return self.get_random(num);
        }

        // Score each word based on the difficulty of characters it contains
        let mut word_scores: Vec<(String, f64)> = self
            .words
            .iter()
            .map(|word| {
                let score = self.calculate_word_difficulty_score(word, char_stats);
                (word.clone(), score)
            })
            .collect();

        // Sort by score (highest difficulty first for more practice)
        word_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Select from top 30% of difficult words to avoid repetition while still targeting weak areas
        let selection_pool_size = (word_scores.len() as f64 * 0.3)
            .max(num as f64)
            .min(word_scores.len() as f64) as usize;
        let selection_pool = &word_scores[0..selection_pool_size];

        // Randomly select from the high-difficulty pool
        let mut rng = &mut rand::thread_rng();
        selection_pool
            .choose_multiple(&mut rng, num)
            .map(|(word, _score)| word.clone())
            .collect()
    }

    /// Calculate difficulty score for a word based on character statistics
    fn calculate_word_difficulty_score(
        &self,
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
        &self,
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
        char_difficulties
            .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Return the weakest characters, limited by count
        char_difficulties
            .into_iter()
            .take(count)
            .map(|(ch, _)| ch)
            .collect()
    }

    /// Substitute some characters in a word with weaker characters for practice
    fn substitute_characters_in_word(&self, word: &str, weak_chars: &[char]) -> String {
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
            "zap should be selected frequently (got {} out of {})",
            zap_count,
            trials
        );
        assert!(
            hard_count > trials / 4,
            "hard should be selected frequently (got {} out of {})",
            hard_count,
            trials
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
            assert_eq!(words.len(), count, "Should return exactly {} words", count);
        }
    }
}
