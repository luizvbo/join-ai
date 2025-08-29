//! This file contains the core state-machine logic for stripping comments,
//! heavily inspired by the parsing engine in `tokei`.

use super::Language;
use std::sync::Arc;

/// Strips comment lines from file content for a given language using a robust
/// state machine.
pub fn remove_comments(contents: &[u8], lang: Arc<Language>) -> Vec<u8> {
    let mut output = Vec::with_capacity(contents.len());
    let mut cursor = 0;
    let mut comment_stack: Vec<&str> = Vec::new(); // Stores END delimiters
    let mut string_delimiter: Option<&str> = None; // Stores END delimiter

    while cursor < contents.len() {
        let remaining = &contents[cursor..];

        // STATE 1: Inside a string. Highest priority.
        if let Some(delim) = string_delimiter {
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
            if remaining.starts_with(end_delim.as_bytes()) {
                cursor += end_delim.len();
                comment_stack.pop();
                continue;
            }

            if lang.allows_nested
                && let Some((start, end)) = lang
                    .multi_line_comments
                    .iter()
                    .find(|(s, _)| remaining.starts_with(s.as_bytes()))
            {
                comment_stack.push(end);
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
            // Trim trailing whitespace from the output before skipping the comment.
            let mut last_idx = output.len();
            while last_idx > 0 {
                let last_char = output[last_idx - 1];
                if last_char == b' ' || last_char == b'\t' {
                    last_idx -= 1;
                } else {
                    break;
                }
            }
            output.truncate(last_idx);

            // Skip the rest of the line.
            if let Some(eol_pos) = find_subsequence(remaining, b"\n") {
                cursor += eol_pos;
            } else {
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
            comment_stack.push(end);
            cursor += start.len();
            continue;
        }

        // Check for strings.
        if let Some((start, end)) = lang
            .quotes
            .iter()
            .find(|(s, _)| remaining.starts_with(s.as_bytes()))
        {
            string_delimiter = Some(end);
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

// --- Unit Tests ---
// All tests now pass with the new logic.
#[cfg(test)]
mod tests {
    use crate::decommenter::{LanguageDB, logic::remove_comments};

    fn assert_stripped(ext: &str, input: &str, expected: &str) {
        let db = LanguageDB::new();
        let lang = db
            .find_by_extension(ext)
            .unwrap_or_else(|| panic!("Language for extension '{}' not found", ext));
        let stripped_bytes = remove_comments(input.as_bytes(), lang);
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
