use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::mpsc;

use once_cell::sync::Lazy;

use crate::decommenter::LanguageDB;
use crate::decommenter::logic::remove_comments;

static LANG_DB: Lazy<LanguageDB> = Lazy::new(LanguageDB::new);

/// Processes file paths received from a channel, concatenating their content into an output file.
pub fn process_files(
    rx: mpsc::Receiver<PathBuf>,
    output_file_path: &PathBuf,
    strip_comments: bool,
) -> anyhow::Result<()> {
    let mut output_file = File::create(output_file_path)?;

    for path in rx {
        match fs::read(&path) {
            Ok(contents) => {
                if contents.contains(&0) {
                    println!("Skipping binary file: {}", path.display());
                    continue;
                }

                writeln!(output_file, "// FILE: {}", path.display())?;

                let final_contents = if strip_comments {
                    let lang_opt = path
                        .extension()
                        .and_then(|s| s.to_str())
                        .and_then(|ext| LANG_DB.find_by_extension(ext));

                    if let Some(lang) = lang_opt {
                        println!("Stripping comments from: {}", path.display());
                        remove_comments(&contents, lang)
                    } else {
                        contents
                    }
                } else {
                    contents
                };

                output_file.write_all(&final_contents)?;
                writeln!(output_file)?;
            }
            Err(e) => {
                if e.kind() != io::ErrorKind::InvalidData {
                    eprintln!("Failed to read file {}: {}", path.display(), e);
                }
            }
        }
    }

    Ok(())
}
