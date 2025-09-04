use include_dir::{include_dir, Dir};
use serde::Deserialize;
use serde_json::from_str;
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
        read_language_from_file(format!("{file_name}.json")).unwrap()
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
        assert!(!lang.words.is_empty());
        assert!(lang.size > 0);
    }

    #[test]
    fn test_language_new_english1k() {
        let lang = Language::new("english1k".to_string());

        assert_eq!(lang.name, "english_1k");
        assert!(!lang.words.is_empty());
        assert!(lang.size > 0);
    }

    #[test]
    fn test_language_new_english10k() {
        let lang = Language::new("english10k".to_string());

        assert_eq!(lang.name, "english_10k");
        assert!(!lang.words.is_empty());
        assert!(lang.size > 0);
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
        assert!(!lang.words.is_empty());
    }

    #[test]
    #[should_panic(expected = "Language file not found")]
    fn test_read_nonexistent_language_file() {
        let _result = read_language_from_file("nonexistent.json".to_string());
    }
}
