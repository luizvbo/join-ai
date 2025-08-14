use clap::Parser;
use ignore::{WalkBuilder, WalkState};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::mpsc;

/// A CLI application to traverse files in a folder and concatenate them
/// into a single text file, suitable for GenAI model input.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The input folder to traverse for files
    #[arg(required = true)]
    input_folder: PathBuf,

    /// The output file to write the concatenated content to
    #[arg(short, long, default_value = "concatenated.txt")]
    output_file: PathBuf,

    /// File patterns to include (e.g., "*.rs *.md")
    #[arg(short, long, value_delimiter = ' ', num_args = 1..)]
    patterns: Option<Vec<String>>,

    /// Clear the output file before writing
    #[arg(short, long)]
    clear_file: bool,

    /// Folders to exclude from the search
    #[arg(short, long, value_delimiter = ' ', num_args = 1..)]
    exclude_folders: Option<Vec<String>>,

    /// File extensions to exclude (e.g., "log png")
    #[arg(long, value_delimiter = ' ', num_args = 1..)]
    exclude_extensions: Option<Vec<String>>,

    /// Set the maximum search depth
    #[arg(long)]
    max_depth: Option<usize>,

    /// Include hidden files in the search
    #[arg(long)]
    hidden: bool,

    /// Do not follow symlinks
    #[arg(long, default_value_t = true)]
    no_follow: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    run_concatenation(args)
}

/// The core logic of the application, separated from main for testability.
fn run_concatenation(args: Args) -> anyhow::Result<()> {
    // Log the arguments being used
    println!(
        "Processing files in folder: {}",
        args.input_folder.display()
    );
    if let Some(patterns) = &args.patterns {
        println!("Using patterns: {}", patterns.join(", "));
    } else {
        println!("Using patterns: all files");
    }
    if let Some(exclude_folders) = &args.exclude_folders {
        println!("Excluding folders: {}", exclude_folders.join(", "));
    }
    if let Some(exclude_extensions) = &args.exclude_extensions {
        println!("Excluding extensions: {}", exclude_extensions.join(", "));
    }

    // Clear the output file if specified
    if args.clear_file && args.output_file.exists() {
        fs::remove_file(&args.output_file)?;
        println!(
            "Output file {} has been cleared.",
            args.output_file.display()
        );
    }

    let mut output_file = File::create(&args.output_file)?;

    let (tx, rx) = mpsc::channel();

    let mut walker_builder = WalkBuilder::new(&args.input_folder);
    walker_builder
        .hidden(!args.hidden)
        .follow_links(!args.no_follow)
        .max_depth(args.max_depth);

    // --- FIX: Use a single OverrideBuilder for all include/exclude patterns ---
    let mut override_builder = ignore::overrides::OverrideBuilder::new(&args.input_folder);

    if let Some(patterns) = &args.patterns {
        for pattern in patterns {
            override_builder.add(pattern)?;
        }
    } else {
        // If no patterns are specified, default to including everything.
        override_builder.add("*")?;
    }

    if let Some(exclude_folders) = &args.exclude_folders {
        for folder in exclude_folders {
            // Add a negative pattern to exclude the folder. The '!' negates the pattern.
            override_builder.add(&format!("!{}", folder))?;
        }
    }

    let overrides = override_builder.build()?;
    walker_builder.overrides(overrides);

    // Build the parallel walker
    let walker = walker_builder.build_parallel();

    // The extensions to exclude need to be available in the closure
    let exclude_extensions = args.exclude_extensions.clone();
    let output_file_path = args.output_file.clone();

    walker.run(|| {
        let tx = tx.clone();
        let exclude_extensions = exclude_extensions.clone();
        let output_file_path = output_file_path.clone();

        Box::new(move |result| {
            if let Ok(entry) = result {
                let path = entry.path();
                if path.is_dir() {
                    return WalkState::Continue;
                }

                if path == output_file_path {
                    return WalkState::Continue;
                }

                if let Some(ext_str) = path.extension().and_then(|s| s.to_str())
                    && let Some(exts_to_exclude) = &exclude_extensions
                    && exts_to_exclude.contains(&ext_str.to_string())
                {
                    return WalkState::Continue;
                }
                tx.send(path.to_path_buf()).expect("Failed to send path");
            }
            WalkState::Continue
        })
    });

    // Close the sender side
    drop(tx);

    // Process files and write to the output file
    for path in rx {
        match fs::read(&path) {
            Ok(contents) => {
                // Check if the file is binary using content_inspector
                if content_inspector::inspect(&contents).is_binary() {
                    println!("Skipping binary file: {}", path.display());
                    continue;
                }

                // Write the header and file content
                writeln!(output_file, "// FILE: {}", path.display())?;
                output_file.write_all(&contents)?;
                writeln!(output_file)?;
            }
            Err(e) => {
                if e.kind() != io::ErrorKind::InvalidData {
                    eprintln!("Failed to read file {}: {}", path.display(), e);
                }
            }
        }
    }

    println!(
        "Files have been processed and written to {}",
        args.output_file.display()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::TempDir;
    use assert_fs::prelude::*;
    use std::fs::File;
    use std::io::Read;
    use std::path::Path;

    fn get_test_args(input_folder: &Path, output_file: &Path) -> Args {
        Args {
            input_folder: input_folder.to_path_buf(),
            output_file: output_file.to_path_buf(),
            patterns: None,
            clear_file: true,
            exclude_folders: None,
            exclude_extensions: None,
            max_depth: None,
            hidden: false,
            no_follow: true,
        }
    }

    #[test]
    fn test_basic_concatenation() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        let input_dir_path = dir.path();

        let file1 = dir.child("file1.txt");
        let file2 = dir.child("file2.txt");
        file1.write_str("Hello")?;
        file2.write_str("World")?;

        let output_file = input_dir_path.join("output.txt");
        let args = get_test_args(input_dir_path, &output_file);

        run_concatenation(args)?;

        let mut result = String::new();
        File::open(&output_file)?.read_to_string(&mut result)?;

        assert!(result.contains("// FILE:"));
        assert!(result.contains("file1.txt"));
        assert!(result.contains("Hello"));
        assert!(result.contains("file2.txt"));
        assert!(result.contains("World"));

        Ok(())
    }

    #[test]
    fn test_exclude_folders() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        let input_dir_path = dir.path();

        let src_dir = dir.child("src");
        src_dir.create_dir_all()?;
        src_dir.child("main.rs").write_str("fn main() {}")?;

        let exclude_dir = dir.child("exclude");
        exclude_dir.create_dir_all()?;
        exclude_dir.child("me.txt").write_str("ignore")?;

        let output_file = input_dir_path.join("output.txt");
        let mut args = get_test_args(input_dir_path, &output_file);
        args.exclude_folders = Some(vec!["exclude".to_string()]);

        run_concatenation(args)?;

        let mut result = String::new();
        File::open(&output_file)?.read_to_string(&mut result)?;

        assert!(result.contains("main.rs"));
        assert!(!result.contains("me.txt"));

        Ok(())
    }

    #[test]
    fn test_exclude_extensions() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        let input_dir_path = dir.path();

        dir.child("code.rs").write_str("let x = 1;")?;
        dir.child("image.png").write_binary(b"binarydata")?;

        let output_file = input_dir_path.join("output.txt");
        let mut args = get_test_args(input_dir_path, &output_file);
        args.exclude_extensions = Some(vec!["png".to_string()]);

        run_concatenation(args)?;

        let mut result = String::new();
        File::open(&output_file)?.read_to_string(&mut result)?;

        assert!(result.contains("code.rs"));
        assert!(!result.contains("image.png"));

        Ok(())
    }

    #[test]
    fn test_filter_by_pattern() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        let input_dir_path = dir.path();

        dir.child("Cargo.toml").write_str("[package]")?;
        dir.child("README.md").write_str("# Project")?;
        let src_dir = dir.child("src");
        src_dir.create_dir_all()?;
        src_dir.child("main.rs").write_str("fn main(){}")?;

        let output_file = input_dir_path.join("output.txt");
        let mut args = get_test_args(input_dir_path, &output_file);
        args.patterns = Some(vec!["*.rs".to_string(), "*.toml".to_string()]);

        run_concatenation(args)?;

        let mut result = String::new();
        File::open(&output_file)?.read_to_string(&mut result)?;

        assert!(result.contains("main.rs"));
        assert!(result.contains("Cargo.toml"));
        assert!(!result.contains("README.md"));

        Ok(())
    }

    #[test]
    fn test_max_depth() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        let input_dir_path = dir.path();

        dir.child("file1.txt").write_str("level 1")?;

        let level2_dir = dir.child("level2");
        level2_dir.create_dir_all()?;
        level2_dir.child("file2.txt").write_str("level 2")?;

        let level3_dir = level2_dir.child("level3");
        level3_dir.create_dir_all()?;
        level3_dir.child("file3.txt").write_str("level 3")?;

        let output_file = input_dir_path.join("output.txt");
        let mut args = get_test_args(input_dir_path, &output_file);
        args.max_depth = Some(2); // Should include file1 and file2, but not file3

        run_concatenation(args)?;

        let mut result = String::new();
        File::open(&output_file)?.read_to_string(&mut result)?;

        assert!(result.contains("file1.txt"));
        assert!(result.contains("file2.txt"));
        assert!(!result.contains("file3.txt"));

        Ok(())
    }

    #[test]
    fn test_skip_binary_files() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        let input_dir_path = dir.path();

        dir.child("text.txt").write_str("some text")?;
        // 0xFF is a common byte in binary files but not valid in UTF-8
        dir.child("binary.bin").write_binary(&[0xFF, 0xFE, 0xFD])?;

        let output_file = input_dir_path.join("output.txt");
        let args = get_test_args(input_dir_path, &output_file);

        run_concatenation(args)?;

        let mut result = String::new();
        File::open(&output_file)?.read_to_string(&mut result)?;

        assert!(result.contains("text.txt"));
        assert!(!result.contains("binary.bin"));

        Ok(())
    }
}
