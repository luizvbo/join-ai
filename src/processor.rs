use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::mpsc;

/// This module handles the processing of files. It receives file paths from the
/// walker, reads their content, and writes it to the final output file.
///
/// # Arguments
/// * `rx` - The receiver end of a channel, which provides `PathBuf`s from the walker.
/// * `output_file_path` - The path to the file where content should be written.
pub fn process_files(
    rx: mpsc::Receiver<PathBuf>,
    output_file_path: &PathBuf,
) -> anyhow::Result<()> {
    // Create or truncate the output file, making it ready for writing.
    let mut output_file = File::create(output_file_path)?;

    // Iterate over every file path sent by the walker.
    // This loop will block until the channel is empty and the sender is dropped.
    for path in rx {
        match fs::read(&path) {
            Ok(contents) => {
                // A simple and robust way to detect binary files is to check for the NUL byte,
                // which is common in compiled files but rare in text files.
                if contents.contains(&0) {
                    println!("Skipping binary file: {}", path.display());
                    continue; // Skip to the next file.
                }

                // Write a header comment to delineate files in the concatenated output.
                writeln!(output_file, "// FILE: {}", path.display())?;
                // Write the actual content of the file.
                output_file.write_all(&contents)?;
                // Add a newline for spacing between files.
                writeln!(output_file)?;
            }
            Err(e) => {
                // It's possible to encounter files that can't be read (e.g., system pipes,
                // broken symlinks). We log these errors but don't stop the process.
                if e.kind() != io::ErrorKind::InvalidData {
                    eprintln!("Failed to read file {}: {}", path.display(), e);
                }
            }
        }
    }

    Ok(())
}
