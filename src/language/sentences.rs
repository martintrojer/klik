use super::core::Language;
use cgisf_lib::cgisf;
use rand::Rng;

impl Language {
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
