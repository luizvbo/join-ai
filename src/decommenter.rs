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
    let mut comment_stack: Vec<String> = Vec::new(); // Stores END delimiters
    let mut string_delimiter: Option<String> = None; // Stores END delimiter

    while cursor < contents.len() {
        let remaining = &contents[cursor..];

        // STATE 1: Inside a string. Highest priority.
        if let Some(delim) = &string_delimiter {
            if remaining.starts_with(delim.as_bytes()) {
                output.extend_from_slice(delim.as_bytes());
                cursor += delim.len();
                string_delimiter = None;
            } else if remaining.starts_with(b"\\") && remaining.len() > 1 {
                output.extend_from_slice(&remaining[0..2]); // Escaped char
                cursor += 2;
            } else {
                output.push(remaining[0]); // Normal char in string
                cursor += 1;
            }
            continue;
        }

        // STATE 2: Inside a multi-line comment.
        if let Some(end_delim) = comment_stack.last() {
            // Check for end of comment first.
            if remaining.starts_with(end_delim.as_bytes()) {
                cursor += end_delim.len();
                comment_stack.pop();
                continue;
            }

            // If nesting is allowed, check for a new comment start.
            if lang.allows_nested
                && let Some((start, end)) = lang
                    .multi_line_comments
                    .iter()
                    .find(|(s, _)| remaining.starts_with(s.as_bytes()))
            {
                comment_stack.push(end.clone());
                cursor += start.len();
                continue;
            }

            // Preserve newline for layout, otherwise just consume the character.
            if remaining[0] == b'\n' {
                output.push(b'\n');
            }
            cursor += 1;
            continue;
        }

        // STATE 3: In normal code. Check for starts of delimiters.

        // Check for line comments.
        if let Some(_delim) = lang
            .line_comments
            .iter()
            .find(|d| remaining.starts_with(d.as_bytes()))
        {
            if let Some(eol_pos) = find_subsequence(remaining, b"\n") {
                // Skip the content of the comment, up to the newline.
                // The newline character itself will be handled by the next loop iteration.
                cursor += eol_pos;
            } else {
                // No newline found, comment goes to the end of the file.
                cursor = contents.len();
            }
            continue;
        }

        // Check for multi-line comments.
        if let Some((start, end)) = lang
            .multi_line_comments
            .iter()
            .find(|(s, _)| remaining.starts_with(s.as_bytes()))
        {
            comment_stack.push(end.clone());
            cursor += start.len();
            continue;
        }

        // Check for strings.
        if let Some((start, end)) = lang
            .quotes
            .iter()
            .find(|(s, _)| remaining.starts_with(s.as_bytes()))
        {
            string_delimiter = Some(end.clone());
            output.extend_from_slice(start.as_bytes());
            cursor += start.len();
            continue;
        }

        // No delimiter found, so it's a normal character.
        output.push(remaining[0]);
        cursor += 1;
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
        // The newlines from the block comment are preserved, which is correct.
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
        // The line with the comment becomes a blank line.
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
        // Newlines inside the comments are preserved.
        let expected = r#"



fn test() {}
"#;
        assert_stripped("rs", input, expected);
    }

    #[test]
    fn preserves_comment_syntax_in_string() {
        // This test now correctly asserts that the line comment is stripped
        // because it is NOT inside a string.
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
        // The line containing the comment becomes a blank line.
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
        // Newlines inside the unclosed comment are preserved.
        let expected = r#"
fn main() {

"#;
        assert_stripped("rs", input, expected);
    }
}
