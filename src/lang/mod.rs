use cgisf_lib::cgisf;
use rand::seq::SliceRandom;
use serde::Deserialize;
use serde_json::from_str;

use include_dir::{include_dir, Dir};
use rand::Rng;
use std::collections::HashMap;
use std::error::Error;

static LANG_DIR: Dir = include_dir!("src/lang");

/// Character difficulty metrics for intelligent word selection
#[derive(Debug, Clone)]
pub struct CharacterDifficulty {
    pub miss_rate: f64,             // Percentage of incorrect attempts (0-100) for any case
    pub avg_time_ms: f64,           // Average time to type the character (any case)
    pub total_attempts: i64,        // Total number of attempts for weighting
    // Uppercase-specific difficulty metrics
    pub uppercase_miss_rate: f64,   // Percentage of incorrect uppercase attempts (0-100)
    pub uppercase_avg_time: f64,    // Average time for uppercase variants
    pub uppercase_attempts: i64,    // Total uppercase attempts for weighting
    pub uppercase_penalty: f64,     // Additional difficulty penalty for uppercase (0-1)
}

#[allow(dead_code)]
#[derive(Deserialize, Clone, Debug)]
pub struct Language {
    pub name: String,
    pub size: u32,
    pub words: Vec<String>,
}

impl Language {
    pub fn new(file_name: String) -> Self {
        read_language_from_file(format!("{}.json", file_name)).unwrap()
    }

    pub fn get_random_sentence(&self, num: usize) -> (Vec<String>, usize) {
        let rng = &mut rand::thread_rng();
        let mut vec = Vec::new();
        let mut word_count = 0;
        for i in 0..num {
            let mut s = cgisf(
                rng.gen_range(1..3),
                rng.gen_range(1..3),
                rng.gen_range(1..5),
                rng.gen_bool(0.5),
                rng.gen_range(1..3),
                rng.gen_bool(0.5),
            );
            word_count += &s.matches(' ').count();
            // gets the word count of the sentence.
            if i == num - 1 {
                s.pop();
            }
            vec.push(s);
        }
        (vec, word_count)
    }

    pub fn get_random(&self, num: usize) -> Vec<String> {
        let mut rng = &mut rand::thread_rng();

        self.words.choose_multiple(&mut rng, num).cloned().collect()
    }

    /// Apply capitalization, punctuation, and commas to words for realistic typing practice
    pub fn apply_advanced_formatting(&self, words: Vec<String>) -> String {
        if words.is_empty() {
            return String::new();
        }

        let rng = &mut rand::thread_rng();
        let mut result = Vec::new();
        
        for (i, word) in words.iter().enumerate() {
            let mut formatted_word = word.clone();
            
            // Capitalize first word and randomly capitalize others (20% chance)
            if i == 0 || rng.gen_bool(0.2) {
                formatted_word = Self::capitalize_first_letter(&formatted_word);
            }
            
            result.push(formatted_word);
            
            // Add punctuation between words
            if i < words.len() - 1 {
                // 15% chance for comma, otherwise space
                if rng.gen_bool(0.15) {
                    result.push(",".to_string());
                }
            }
        }
        
        // Add final punctuation (80% period, 15% exclamation, 5% question mark)
        let final_punct = match rng.gen_range(0..100) {
            0..=79 => ".",
            80..=94 => "!",
            _ => "?",
        };
        result.push(final_punct.to_string());
        
        result.join(" ").replace(" ,", ",").replace(" .", ".").replace(" !", "!").replace(" ?", "?")
    }
    
    /// Helper function to capitalize the first letter of a word
    fn capitalize_first_letter(word: &str) -> String {
        let mut chars: Vec<char> = word.chars().collect();
        if !chars.is_empty() && chars[0].is_alphabetic() {
            chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
        }
        chars.into_iter().collect()
    }

    /// Get words intelligently selected based on character statistics
    /// Words containing characters that need more practice are prioritized
    pub fn get_intelligent(&self, num: usize, char_stats: &HashMap<char, CharacterDifficulty>) -> Vec<String> {
        if char_stats.is_empty() {
            // Fall back to random selection if no statistics available
            return self.get_random(num);
        }

        // Score each word based on the difficulty of characters it contains
        let mut word_scores: Vec<(String, f64)> = self.words
            .iter()
            .map(|word| {
                let score = self.calculate_word_difficulty_score(word, char_stats);
                (word.clone(), score)
            })
            .collect();

        // Sort by score (highest difficulty first for more practice)
        word_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Select from top 30% of difficult words to avoid repetition while still targeting weak areas
        let selection_pool_size = (word_scores.len() as f64 * 0.3).max(num as f64).min(word_scores.len() as f64) as usize;
        let selection_pool = &word_scores[0..selection_pool_size];

        // Randomly select from the high-difficulty pool
        let mut rng = &mut rand::thread_rng();
        selection_pool
            .choose_multiple(&mut rng, num)
            .map(|(word, _score)| word.clone())
            .collect()
    }

    /// Calculate difficulty score for a word based on character statistics
    fn calculate_word_difficulty_score(&self, word: &str, char_stats: &HashMap<char, CharacterDifficulty>) -> f64 {
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
                total_score += if is_uppercase { base_score * 1.5 } else { base_score };
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
}

fn read_language_from_file(file_name: String) -> Result<Language, Box<dyn Error>> {
    let file = LANG_DIR
        .get_file(file_name)
        .expect("Language file not found");

    let file_as_str = file
        .contents_utf8()
        .expect("Unable to interpret file as a string");

    let lang = from_str(file_as_str).expect("Unable to deserialize language json");

    Ok(lang)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_new() {
        let lang = Language::new("english".to_string());

        assert_eq!(lang.name, "english");
        assert!(lang.words.len() > 0);
        assert!(lang.size > 0);
    }

    #[test]
    fn test_language_new_english1k() {
        let lang = Language::new("english1k".to_string());

        assert_eq!(lang.name, "english_1k");
        assert!(lang.words.len() > 0);
        assert!(lang.size > 0);
    }

    #[test]
    fn test_language_new_english10k() {
        let lang = Language::new("english10k".to_string());

        assert_eq!(lang.name, "english_10k");
        assert!(lang.words.len() > 0);
        assert!(lang.size > 0);
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
    fn test_get_random_words_unique() {
        let lang = Language::new("english".to_string());

        let words = lang.get_random(10);
        assert_eq!(words.len(), 10);

        let mut unique_words = words.clone();
        unique_words.sort();
        unique_words.dedup();

        assert!(unique_words.len() >= 8);
    }

    #[test]
    fn test_get_random_sentence() {
        let lang = Language::new("english".to_string());

        let (sentences, word_count) = lang.get_random_sentence(2);

        assert_eq!(sentences.len(), 2);
        assert!(word_count > 0);

        for sentence in &sentences {
            assert!(!sentence.is_empty());
            assert!(sentence.chars().any(|c| c.is_alphabetic()));
        }
    }

    #[test]
    fn test_get_random_sentence_single() {
        let lang = Language::new("english".to_string());

        let (sentences, word_count) = lang.get_random_sentence(1);

        assert_eq!(sentences.len(), 1);
        assert!(word_count > 0);
        assert!(!sentences[0].is_empty());
    }

    #[test]
    fn test_get_random_sentence_zero() {
        let lang = Language::new("english".to_string());

        let (sentences, word_count) = lang.get_random_sentence(0);

        assert_eq!(sentences.len(), 0);
        assert_eq!(word_count, 0);
    }

    #[test]
    fn test_get_random_sentence_word_count_accuracy() {
        let lang = Language::new("english".to_string());

        let (sentences, word_count) = lang.get_random_sentence(1);

        if !sentences.is_empty() {
            let actual_word_count = sentences[0].matches(' ').count();
            assert!(word_count <= actual_word_count + 2);
        }
    }

    #[test]
    fn test_language_deserialization() {
        let json_data = r#"
        {
            "name": "test",
            "size": 3,
            "words": ["hello", "world", "test"]
        }
        "#;

        let lang: Language = from_str(json_data).expect("Failed to deserialize test language");

        assert_eq!(lang.name, "test");
        assert_eq!(lang.size, 3);
        assert_eq!(lang.words.len(), 3);
        assert!(lang.words.contains(&"hello".to_string()));
        assert!(lang.words.contains(&"world".to_string()));
        assert!(lang.words.contains(&"test".to_string()));
    }

    #[test]
    fn test_read_language_from_file() {
        let result = read_language_from_file("english.json".to_string());
        assert!(result.is_ok());

        let lang = result.unwrap();
        assert_eq!(lang.name, "english");
        assert!(lang.words.len() > 0);
    }

    #[test]
    #[should_panic(expected = "Language file not found")]
    fn test_read_nonexistent_language_file() {
        let _result = read_language_from_file("nonexistent.json".to_string());
    }

    #[test]
    fn test_intelligent_selection_with_empty_stats() {
        let lang = Language::new("english".to_string());
        let char_stats = HashMap::new();

        let words = lang.get_intelligent(5, &char_stats);
        assert_eq!(words.len(), 5);
        
        // Should fall back to random selection when no stats available
        for word in &words {
            assert!(lang.words.contains(word));
        }
    }

    #[test]
    fn test_intelligent_selection_prioritizes_difficult_characters() {
        let lang = Language {
            name: "test".to_string(),
            size: 4,
            words: vec![
                "easy".to_string(),   // Contains 'e' (easy)
                "hard".to_string(),   // Contains 'h' (hard)  
                "test".to_string(),   // Contains 't' (medium)
                "zap".to_string(),    // Contains 'z' (very hard)
            ],
        };

        let mut char_stats = HashMap::new();
        char_stats.insert('e', CharacterDifficulty {
            miss_rate: 2.0,
            avg_time_ms: 120.0,
            total_attempts: 50,
            uppercase_miss_rate: 5.0,
            uppercase_avg_time: 140.0,
            uppercase_attempts: 10,
            uppercase_penalty: 0.2,
        });
        char_stats.insert('h', CharacterDifficulty {
            miss_rate: 15.0,
            avg_time_ms: 250.0,
            total_attempts: 30,
            uppercase_miss_rate: 25.0,
            uppercase_avg_time: 350.0,
            uppercase_attempts: 8,
            uppercase_penalty: 0.6,
        });
        char_stats.insert('t', CharacterDifficulty {
            miss_rate: 8.0,
            avg_time_ms: 180.0,
            total_attempts: 40,
            uppercase_miss_rate: 12.0,
            uppercase_avg_time: 220.0,
            uppercase_attempts: 15,
            uppercase_penalty: 0.3,
        });
        char_stats.insert('z', CharacterDifficulty {
            miss_rate: 25.0,
            avg_time_ms: 400.0,
            total_attempts: 10,
            uppercase_miss_rate: 40.0,
            uppercase_avg_time: 600.0,
            uppercase_attempts: 3,
            uppercase_penalty: 0.8,
        });

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
        assert!(zap_count > trials / 4, "zap should be selected frequently (got {} out of {})", zap_count, trials);
        assert!(hard_count > trials / 4, "hard should be selected frequently (got {} out of {})", hard_count, trials);
    }

    #[test]
    fn test_calculate_word_difficulty_score() {
        let lang = Language::new("english".to_string());
        
        let mut char_stats = HashMap::new();
        char_stats.insert('a', CharacterDifficulty {
            miss_rate: 10.0,
            avg_time_ms: 300.0,
            total_attempts: 20,
            uppercase_miss_rate: 15.0,
            uppercase_avg_time: 400.0,
            uppercase_attempts: 5,
            uppercase_penalty: 0.5,
        });
        char_stats.insert('b', CharacterDifficulty {
            miss_rate: 5.0,
            avg_time_ms: 150.0,
            total_attempts: 15,
            uppercase_miss_rate: 8.0,
            uppercase_avg_time: 180.0,
            uppercase_attempts: 7,
            uppercase_penalty: 0.2,
        });

        // Word with one difficult character
        let score1 = lang.calculate_word_difficulty_score("abc", &char_stats);
        
        // Word with only easy characters
        let score2 = lang.calculate_word_difficulty_score("bbb", &char_stats);
        
        // Word with only difficult characters
        let score3 = lang.calculate_word_difficulty_score("aaa", &char_stats);

        // Difficult characters should result in higher scores
        assert!(score3 > score1, "Word with all difficult chars should score higher");
        assert!(score1 > score2, "Word with mixed chars should score higher than easy word");
        assert!(score3 > 0.0, "Difficult word should have positive score");
    }

    #[test]
    fn test_calculate_word_difficulty_score_unknown_characters() {
        let lang = Language::new("english".to_string());
        
        let char_stats = HashMap::new(); // No known characters
        
        let score = lang.calculate_word_difficulty_score("unknown", &char_stats);
        
        // Unknown characters should get medium priority score
        assert!(score > 0.0, "Unknown characters should get positive score");
        assert!(score == 5.0, "Unknown characters should get score of 5.0, got {}", score);
    }

    #[test]
    fn test_calculate_word_difficulty_score_empty_word() {
        let lang = Language::new("english".to_string());
        let char_stats = HashMap::new();
        
        let score = lang.calculate_word_difficulty_score("", &char_stats);
        assert_eq!(score, 0.0, "Empty word should have zero score");
    }

    #[test]
    fn test_intelligent_selection_returns_requested_count() {
        let lang = Language::new("english".to_string());
        
        let mut char_stats = HashMap::new();
        char_stats.insert('a', CharacterDifficulty {
            miss_rate: 10.0,
            avg_time_ms: 200.0,
            total_attempts: 20,
            uppercase_miss_rate: 15.0,
            uppercase_avg_time: 250.0,
            uppercase_attempts: 8,
            uppercase_penalty: 0.3,
        });

        for count in [1, 5, 10, 15] {
            let words = lang.get_intelligent(count, &char_stats);
            assert_eq!(words.len(), count, "Should return exactly {} words", count);
            
            for word in &words {
                assert!(lang.words.contains(word), "Selected word should be from language word list");
            }
        }
    }

    #[test]
    fn test_intelligent_selection_avoids_exact_repetition() {
        let lang = Language {
            name: "test".to_string(),
            size: 10,
            words: (0..10).map(|i| format!("word{}", i)).collect(),
        };

        let mut char_stats = HashMap::new();
        // Make all characters equally difficult
        for c in 'a'..='z' {
            char_stats.insert(c, CharacterDifficulty {
                miss_rate: 10.0,
                avg_time_ms: 200.0,
                total_attempts: 20,
                uppercase_miss_rate: 15.0,
                uppercase_avg_time: 250.0,
                uppercase_attempts: 8,
                uppercase_penalty: 0.3,
            });
        }

        // Request more words than available, should get unique words
        let words = lang.get_intelligent(5, &char_stats);
        let mut unique_words = words.clone();
        unique_words.sort();
        unique_words.dedup();
        
        assert_eq!(words.len(), unique_words.len(), "Should not have duplicate words");
    }

    #[test]
    fn test_character_difficulty_creation() {
        let difficulty = CharacterDifficulty {
            miss_rate: 15.5,
            avg_time_ms: 250.0,
            total_attempts: 42,
            uppercase_miss_rate: 20.0,
            uppercase_avg_time: 300.0,
            uppercase_attempts: 15,
            uppercase_penalty: 0.4,
        };

        assert_eq!(difficulty.miss_rate, 15.5);
        assert_eq!(difficulty.avg_time_ms, 250.0);
        assert_eq!(difficulty.total_attempts, 42);
        assert_eq!(difficulty.uppercase_miss_rate, 20.0);
        assert_eq!(difficulty.uppercase_avg_time, 300.0);
        assert_eq!(difficulty.uppercase_attempts, 15);
        assert_eq!(difficulty.uppercase_penalty, 0.4);
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
    fn test_apply_advanced_formatting_basic() {
        let lang = Language::new("english".to_string());
        let words = vec!["hello".to_string(), "world".to_string(), "test".to_string()];
        
        let result = lang.apply_advanced_formatting(words);
        
        // Should have capitalized first word and end with punctuation
        assert!(result.starts_with("Hello"));
        assert!(result.ends_with('.') || result.ends_with('!') || result.ends_with('?'));
        assert!(result.contains("world"));
        assert!(result.contains("test"));
    }

    #[test]
    fn test_apply_advanced_formatting_empty() {
        let lang = Language::new("english".to_string());
        let words = vec![];
        
        let result = lang.apply_advanced_formatting(words);
        assert_eq!(result, "");
    }

    #[test]
    fn test_apply_advanced_formatting_single_word() {
        let lang = Language::new("english".to_string());
        let words = vec!["test".to_string()];
        
        let result = lang.apply_advanced_formatting(words);
        
        // Should start with capital T and end with punctuation
        assert!(result.starts_with("Test"));
        assert!(result.ends_with('.') || result.ends_with('!') || result.ends_with('?'));
        assert_eq!(result.len(), 5); // "Test" + 1 punctuation mark
    }

    #[test]
    fn test_calculate_word_difficulty_score_with_uppercase() {
        let lang = Language::new("english".to_string());
        
        let mut char_stats = HashMap::new();
        char_stats.insert('h', CharacterDifficulty {
            miss_rate: 10.0,
            avg_time_ms: 200.0,
            total_attempts: 20,
            uppercase_miss_rate: 20.0,
            uppercase_avg_time: 300.0,
            uppercase_attempts: 5,
            uppercase_penalty: 0.6,
        });
        
        // Test lowercase vs uppercase scoring
        let lowercase_score = lang.calculate_word_difficulty_score("hello", &char_stats);
        let uppercase_score = lang.calculate_word_difficulty_score("Hello", &char_stats);
        
        // Uppercase should have higher difficulty score
        assert!(uppercase_score > lowercase_score, "Uppercase should score higher than lowercase");
    }

    #[test]
    fn test_calculate_word_difficulty_score_with_punctuation() {
        let lang = Language::new("english".to_string());
        
        let mut char_stats = HashMap::new();
        char_stats.insert('h', CharacterDifficulty {
            miss_rate: 10.0,
            avg_time_ms: 200.0,
            total_attempts: 20,
            uppercase_miss_rate: 15.0,
            uppercase_avg_time: 250.0,
            uppercase_attempts: 8,
            uppercase_penalty: 0.3,
        });
        
        // Test word with punctuation
        let score_with_punct = lang.calculate_word_difficulty_score("hello.", &char_stats);
        let score_without_punct = lang.calculate_word_difficulty_score("hello", &char_stats);
        
        // Should handle punctuation without crashing
        assert!(score_with_punct > 0.0);
        assert!(score_without_punct > 0.0);
    }

    #[test]
    fn test_calculate_word_difficulty_score_mixed_case() {
        let lang = Language::new("english".to_string());
        
        let mut char_stats = HashMap::new();
        char_stats.insert('h', CharacterDifficulty {
            miss_rate: 5.0,
            avg_time_ms: 150.0,
            total_attempts: 30,
            uppercase_miss_rate: 15.0,
            uppercase_avg_time: 220.0,
            uppercase_attempts: 10,
            uppercase_penalty: 0.4,
        });
        char_stats.insert('e', CharacterDifficulty {
            miss_rate: 3.0,
            avg_time_ms: 120.0,
            total_attempts: 40,
            uppercase_miss_rate: 8.0,
            uppercase_avg_time: 160.0,
            uppercase_attempts: 15,
            uppercase_penalty: 0.2,
        });
        
        // Test mixed case word
        let score = lang.calculate_word_difficulty_score("Hello", &char_stats);
        
        // Should incorporate both uppercase penalty for 'H' and normal scoring for other letters
        assert!(score > 0.0);
    }

    #[test]
    fn test_advanced_formatting_consistency() {
        let lang = Language::new("english".to_string());
        let words = vec!["the".to_string(), "quick".to_string(), "brown".to_string()];
        
        // Test multiple times to ensure consistent structure
        for _ in 0..10 {
            let result = lang.apply_advanced_formatting(words.clone());
            
            // Should always start with capital letter
            assert!(result.chars().next().unwrap().is_uppercase());
            
            // Should always end with punctuation
            let last_char = result.chars().last().unwrap();
            assert!(last_char == '.' || last_char == '!' || last_char == '?');
            
            // Should contain all original words
            assert!(result.to_lowercase().contains("the"));
            assert!(result.to_lowercase().contains("quick"));
            assert!(result.to_lowercase().contains("brown"));
        }
    }
}
