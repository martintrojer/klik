use cgisf_lib::cgisf;
use rand::seq::SliceRandom;
use serde::Deserialize;
use serde_json::from_str;

use include_dir::{include_dir, Dir};
use rand::Rng;
use std::error::Error;

static LANG_DIR: Dir = include_dir!("src/lang");

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
}
