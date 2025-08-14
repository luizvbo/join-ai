use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::mpsc;

/// Processes file paths received from a channel, concatenating their content into an output file.
pub fn process_files(
    rx: mpsc::Receiver<PathBuf>,
    output_file_path: &PathBuf,
) -> anyhow::Result<()> {
    let mut output_file = File::create(output_file_path)?;

    for path in rx {
        match fs::read(&path) {
            Ok(contents) => {
                // A robust way to detect binary files is to check for the NUL byte.
                if contents.contains(&0) {
                    println!("Skipping binary file: {}", path.display());
                    continue;
                }

                // Write the header and file content
                writeln!(output_file, "// FILE: {}", path.display())?;
                output_file.write_all(&contents)?;
                writeln!(output_file)?;
            }
            Err(e) => {
                // Ignore errors from special files that can't be read (e.g., pipes)
                if e.kind() != io::ErrorKind::InvalidData {
                    eprintln!("Failed to read file {}: {}", path.display(), e);
                }
            }
        }
    }

    Ok(())
}
