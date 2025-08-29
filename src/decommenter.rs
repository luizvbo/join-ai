// FILE: ./src/decommenter.rs
//! This file contains code adapted from the `tokei` project (https://github.com/XAMPPRocky/tokei),
//! licensed under the MIT License.
//!
//! It dynamically loads language definitions from languages.toml.

use grep_searcher::LineStep;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

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
    nested: bool,
}

#[derive(Debug, Deserialize)]
struct LanguagesFile {
    languages: BTreeMap<String, LanguageDefinition>,
}

// --- Public-facing, compiled Language struct ---

#[derive(Debug)]
pub struct Language {
    line_comments: Vec<String>,
    multi_line_comments: Vec<[String; 2]>,
    any_multi_line_comments: Vec<(String, String)>,
    allows_nested: bool,
    // The unused `comment_matcher` has been removed.
}

// --- Database to hold all loaded languages ---

#[derive(Debug)]
pub struct LanguageDB {
    languages: BTreeMap<String, Arc<Language>>,
    ext_map: HashMap<String, String>,
}

impl LanguageDB {
    /// Loads and parses the languages.toml file.
    pub fn new() -> Self {
        let toml_str = include_str!("languages.toml");
        let languages_file: LanguagesFile =
            toml::from_str(toml_str).expect("Failed to parse languages.toml");

        let mut languages = BTreeMap::new();
        let mut ext_map = HashMap::new();

        for (name, def) in languages_file.languages {
            for ext in &def.extensions {
                ext_map.insert(ext.clone(), name.clone());
            }

            let any_multi_line: Vec<(String, String)> = def
                .multi_line_comments
                .iter()
                .map(|[start, end]| (start.clone(), end.clone()))
                .collect();

            let lang = Language {
                line_comments: def.line_comments,
                multi_line_comments: def.multi_line_comments,
                any_multi_line_comments: any_multi_line,
                allows_nested: def.nested,
            };

            languages.insert(name, Arc::new(lang));
        }

        Self { languages, ext_map }
    }

    /// Finds a compiled language definition by file extension.
    pub fn find_by_extension(&self, ext: &str) -> Option<Arc<Language>> {
        self.ext_map
            .get(ext)
            .and_then(|lang_name| self.languages.get(lang_name))
            .cloned()
    }
}

impl Default for LanguageDB {
    fn default() -> Self {
        LanguageDB::new()
    }
}

// --- Stripping Logic ---

trait SliceExt {
    fn trim(&self) -> &Self;
}

impl SliceExt for [u8] {
    fn trim(&self) -> &Self {
        fn is_whitespace(c: &u8) -> bool {
            *c == b' ' || (*c >= 0x09 && *c <= 0x0d)
        }
        let start = self.iter().position(|c| !is_whitespace(c)).unwrap_or(0);
        let end = self.iter().rposition(|c| !is_whitespace(c)).unwrap_or(0);
        if start > end { &[] } else { &self[start..=end] }
    }
}

#[derive(Clone, Debug)]
struct SyntaxCounter {
    lang: Arc<Language>,
    stack: Vec<String>,
}

impl SyntaxCounter {
    fn new(lang: Arc<Language>) -> Self {
        Self {
            lang,
            stack: Vec::with_capacity(1),
        }
    }

    fn line_is_comment(&self, line: &[u8], in_comment_block: bool) -> bool {
        let trimmed = line.trim();
        if in_comment_block {
            return true;
        }
        if self
            .lang
            .line_comments
            .iter()
            .any(|c| trimmed.starts_with(c.as_bytes()))
        {
            return true;
        }
        if self
            .lang
            .multi_line_comments
            .iter()
            .any(|[s, e]| trimmed.starts_with(s.as_bytes()) && trimmed.ends_with(e.as_bytes()))
        {
            return true;
        }
        false
    }
}

/// Strips comment lines from file content for a given language.
pub fn strip_comments(contents: &[u8], lang: Arc<Language>) -> Vec<u8> {
    let mut syntax = SyntaxCounter::new(lang);
    let mut output = Vec::with_capacity(contents.len());
    let mut stepper = LineStep::new(b'\n', 0, contents.len());

    while let Some((start, end)) = stepper.next(contents) {
        let line = &contents[start..end];
        let line_trimmed = line.trim();

        if line_trimmed.is_empty() {
            output.extend_from_slice(line);
            if end < contents.len() {
                output.push(b'\n');
            }
            continue;
        }

        let in_comment_block_before_line = !syntax.stack.is_empty();

        // Process this line for changes in multi-line comment state
        let mut i = 0;
        while i < line.len() {
            if let Some(last) = syntax.stack.last() {
                if line[i..].starts_with(last.as_bytes()) {
                    i += last.len();
                    syntax.stack.pop();
                    continue;
                }
            } else {
                for (s, e) in &syntax.lang.any_multi_line_comments {
                    if line[i..].starts_with(s.as_bytes()) {
                        if syntax.stack.is_empty() || syntax.lang.allows_nested {
                            syntax.stack.push(e.clone());
                        }
                        i += s.len();
                        continue;
                    }
                }
            }
            i += 1;
        }

        if !syntax.line_is_comment(line, in_comment_block_before_line) {
            output.extend_from_slice(line);
            if end < contents.len() {
                output.push(b'\n');
            }
        }
    }

    output
}

// --- Unit Tests ---
#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to make testing easier
    fn assert_stripped(ext: &str, input: &str, expected: &str) {
        let db = LanguageDB::new();
        let lang = db
            .find_by_extension(ext)
            .unwrap_or_else(|| panic!("Language for extension '{}' not found", ext));
        let stripped_bytes = strip_comments(input.as_bytes(), lang);
        let stripped_str = String::from_utf8(stripped_bytes).unwrap();
        assert_eq!(stripped_str.trim(), expected.trim());
    }

    #[test]
    fn strips_rust_single_line_comments() {
        let input = r#"
fn main() { // entry point
    println!("Hello"); // prints hello
}
"#;
        let expected = r#"
fn main() {
    println!("Hello");
}
"#;
        assert_stripped("rs", input, expected);
    }

    #[test]
    fn strips_c_style_multi_line_comments() {
        let input = r#"
int main() {
    /* This is a block comment
       spanning multiple lines. */
    return 0; /* trailing comment */
}
"#;
        let expected = r#"
int main() {
    return 0;
}
"#;
        assert_stripped("c", input, expected);
    }

    #[test]
    fn strips_python_comments() {
        let input = r#"
# Main function
def main():
    print("hello") # print statement
"#;
        let expected = r#"
def main():
    print("hello")
"#;
        assert_stripped("py", input, expected);
    }

    #[test]
    fn preserves_content_when_no_comments() {
        let input = r#"
fn main() {
    println!("Hello");
}
"#;
        assert_stripped("rs", input, input);
    }

    #[test]
    fn handles_empty_file() {
        assert_stripped("rs", "", "");
    }

    #[test]
    fn handles_file_with_only_comments() {
        let input = r#"
// All comments
// Nothing to see here
/*
And a block comment too
*/
"#;
        assert_stripped("rs", input, "");
    }

    #[test]
    fn handles_nested_comments_in_rust() {
        let input = r#"
/*
  outer comment
  /* inner comment */
  still outer
*/
fn test() {}
"#;
        let expected = r#"

fn test() {}
"#;
        assert_stripped("rs", input, expected);
    }

    #[test]
    fn limitation_comment_syntax_in_string() {
        // NOTE: This test demonstrates a known limitation. The current simplified
        // stripper does not track string literal state, so it will incorrectly
        // strip content that looks like a comment inside a string.
        let input = r#"
let url = "http://example.com"; // This is a URL
let path = "C://Users/test";
"#;
        // The `//` in the string is incorrectly identified as a comment.
        let expected = r#"
let url = "http://example.com";
let path = "C:";
"#;
        assert_stripped("rs", input, expected);
    }
}
