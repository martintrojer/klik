use include_dir::{include_dir, Dir};
use serde::Deserialize;
use serde_json::from_str;

static LANG_DIR: Dir = include_dir!("src/lang");

#[derive(Deserialize, Clone, Debug)]
pub struct Language {
    pub name: String,
    pub size: u32,
    pub words: Vec<String>,
}

impl Language {
    pub fn new(file_name: String) -> Self {
        let file_name = format!("{file_name}.json");
        let file = LANG_DIR
            .get_file(&file_name)
            .unwrap_or_else(|| panic!("Language file not found: {file_name}"));

        let file_as_str = file
            .contents_utf8()
            .unwrap_or_else(|| panic!("Unable to interpret {file_name} as a string"));

        from_str(file_as_str).unwrap_or_else(|e| panic!("Unable to deserialize {file_name}: {e}"))
    }
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
    #[should_panic(expected = "Language file not found")]
    fn test_read_nonexistent_language_file() {
        Language::new("nonexistent".to_string());
    }
}
