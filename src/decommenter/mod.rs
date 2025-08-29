//! This module defines the language syntax structures and the database
//! for loading and querying them at runtime. This is inspired by the
//! architecture of `tokei`.

use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

pub mod logic;

// --- Data structures that mirror languages.toml ---

#[derive(Debug, Deserialize, Clone)]
struct LanguageDefinition {
    #[serde(default)]
    extensions: Vec<String>,
    #[serde(default, rename = "line_comment")]
    line_comments: Vec<String>,
    #[serde(default, rename = "multi_line_comments")]
    multi_line_comments: Vec<[String; 2]>,
    #[serde(default)]
    quotes: Vec<[String; 2]>,
    #[serde(default)]
    nested: bool,
}

#[derive(Debug, Deserialize)]
struct LanguagesFile {
    languages: BTreeMap<String, LanguageDefinition>,
}

// --- Public-facing, compiled Language struct ---

#[derive(Debug)]
pub struct Language {
    pub line_comments: Vec<String>,
    pub multi_line_comments: Vec<(String, String)>,
    pub quotes: Vec<(String, String)>,
    pub allows_nested: bool,
}

// --- Database to hold all loaded languages ---

#[derive(Debug)]
pub struct LanguageDB {
    languages: BTreeMap<String, Arc<Language>>,
    ext_map: HashMap<String, String>,
}

impl LanguageDB {
    pub fn new() -> Self {
        let toml_str = include_str!("../languages.toml");
        let languages_file: LanguagesFile =
            toml::from_str(toml_str).expect("Failed to parse languages.toml");

        let mut languages = BTreeMap::new();
        let mut ext_map = HashMap::new();

        for (name, def) in languages_file.languages {
            for ext in &def.extensions {
                ext_map.insert(ext.clone(), name.clone());
            }

            let lang = Language {
                line_comments: def.line_comments,
                multi_line_comments: def
                    .multi_line_comments
                    .into_iter()
                    .map(|[s, e]| (s, e))
                    .collect(),
                quotes: def.quotes.into_iter().map(|[s, e]| (s, e)).collect(),
                allows_nested: def.nested,
            };

            languages.insert(name, Arc::new(lang));
        }

        Self { languages, ext_map }
    }

    pub fn find_by_extension(&self, ext: &str) -> Option<Arc<Language>> {
        self.ext_map
            .get(ext)
            .and_then(|lang_name| self.languages.get(lang_name))
            .cloned()
    }
}

impl Default for LanguageDB {
    fn default() -> Self {
        Self::new()
    }
}
