//! This file contains code adapted from the `tokei` project (https://github.com/XAMPPRocky/tokei),
//! licensed under the MIT License.
//!
//! It dynamically loads language definitions from languages.toml.

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
    line_comments: Vec<String>,
    multi_line_comments: Vec<(String, String)>,
    quotes: Vec<(String, String)>,
    allows_nested: bool,
}

// --- Database to hold all loaded languages ---

#[derive(Debug)]
pub struct LanguageDB {
    languages: BTreeMap<String, Arc<Language>>,
    ext_map: HashMap<String, String>,
}

impl LanguageDB {
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

// --- NEW, ROBUST Stripping Logic ---

/// Strips comment lines from file content for a given language.
pub fn strip_comments(contents: &[u8], lang: Arc<Language>) -> Vec<u8> {
    let mut output = Vec::with_capacity(contents.len());
    let mut cursor = 0;
    let mut comment_stack: Vec<String> = Vec::new();
    let mut string_delimiter: Option<String> = None;

    while cursor < contents.len() {
        let remaining = &contents[cursor..];

        if let Some(delim) = &string_delimiter {
            // --- We are inside a string ---
            if remaining.starts_with(delim.as_bytes()) {
                // End of string
                output.extend_from_slice(delim.as_bytes());
                cursor += delim.len();
                string_delimiter = None;
            } else if remaining.starts_with(b"\\") && remaining.len() > 1 {
                // Escaped character
                output.extend_from_slice(&remaining[0..2]);
                cursor += 2;
            } else {
                // Normal character in string
                output.push(remaining[0]);
                cursor += 1;
            }
        } else if let Some(end_delim) = comment_stack.last() {
            // --- We are inside a multi-line comment ---
            if remaining.starts_with(end_delim.as_bytes()) {
                cursor += end_delim.len();
                comment_stack.pop();
            } else {
                cursor += 1; // Consume character without adding to output
            }
        } else {
            // --- We are in code ---
            let mut next_delimiter_pos: Option<usize> = None;

            // Find the earliest next delimiter (comment or string)
            let mut all_delims = Vec::new();
            all_delims.extend(lang.line_comments.iter());
            all_delims.extend(lang.multi_line_comments.iter().map(|(s, _)| s));
            all_delims.extend(lang.quotes.iter().map(|(s, _)| s));

            for delim in &all_delims {
                if let Some(pos) = find_subsequence(remaining, delim.as_bytes()) {
                    next_delimiter_pos = Some(next_delimiter_pos.map_or(pos, |p| p.min(pos)));
                }
            }

            if let Some(pos) = next_delimiter_pos {
                // Append the code before the delimiter
                output.extend_from_slice(&remaining[..pos]);
                cursor += pos;

                // Update state based on the delimiter we found
                let remaining_at_delim = &contents[cursor..];
                if let Some(_delim) = lang
                    .line_comments
                    .iter()
                    .find(|d| remaining_at_delim.starts_with(d.as_bytes()))
                {
                    // It's a line comment, skip to the end of the line
                    if let Some(end_of_line) = find_subsequence(remaining_at_delim, b"\n") {
                        cursor += end_of_line; // The newline will be handled in the next iteration
                    } else {
                        cursor = contents.len(); // End of file
                    }
                } else if let Some((start, end)) = lang
                    .multi_line_comments
                    .iter()
                    .find(|(s, _)| remaining_at_delim.starts_with(s.as_bytes()))
                {
                    if comment_stack.is_empty() || lang.allows_nested {
                        comment_stack.push(end.clone());
                    }
                    cursor += start.len();
                } else if let Some((start, end)) = lang
                    .quotes
                    .iter()
                    .find(|(s, _)| remaining_at_delim.starts_with(s.as_bytes()))
                {
                    string_delimiter = Some(end.clone());
                    output.extend_from_slice(start.as_bytes());
                    cursor += start.len();
                }
            } else {
                // No more delimiters, append the rest of the file
                output.extend_from_slice(remaining);
                cursor = contents.len();
            }
        }
    }

    output
}

// Helper to find a subsequence (needle) in a slice (haystack)
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

// --- CORRECTED Unit Tests ---
#[cfg(test)]
mod tests {
    use super::*;

    fn assert_stripped(ext: &str, input: &str, expected: &str) {
        let db = LanguageDB::new();
        let lang = db
            .find_by_extension(ext)
            .unwrap_or_else(|| panic!("Language for extension '{}' not found", ext));
        let stripped_bytes = strip_comments(input.as_bytes(), lang);
        let stripped_str = String::from_utf8(stripped_bytes).unwrap();

        // Normalize line endings for comparison on different OS
        let normalized_stripped = stripped_str.replace("\r\n", "\n");
        let normalized_expected = expected.replace("\r\n", "\n");

        assert_eq!(normalized_stripped.trim(), normalized_expected.trim());
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
    fn preserves_comment_syntax_in_string() {
        // This test now asserts the CORRECT behavior: strings are preserved.
        let input = r#"
let url = "http://example.com"; // This is a URL
let path = "C://Users/test";
"#;
        let expected = r#"
let url = "http://example.com";
let path = "C://Users/test";
"#;
        assert_stripped("rs", input, expected);
    }

    #[test]
    fn preserves_blank_lines() {
        let input = r#"
fn main() {

    // A comment

    println!("Hello");

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
    fn handles_unclosed_block_comment() {
        let input = r#"
fn main() {
    /* start of comment
    and it never ends...
"#;
        let expected = r#"
fn main() {
"#;
        assert_stripped("rs", input, expected);
    }
}
