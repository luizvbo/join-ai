// FILE: ./src/lib.rs
use std::fs;

pub mod cli;
pub mod processor;
pub mod walker;

use cli::{Commands, JoinArgs};

/// The core logic of the application.
pub fn run(command: Commands) -> anyhow::Result<()> {
    match command {
        Commands::Join(args) => run_join(args),
        Commands::Update(_args) => {
            println!("Update functionality is not yet implemented.");
            println!("Please check for new releases at the GitHub repository:");
            println!("https://github.com/luizvbo/join-ai/releases");
            Ok(())
        }
    }
}

/// The logic for the 'join' command.
fn run_join(args: JoinArgs) -> anyhow::Result<()> {
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

    // 1. Find all relevant files using the walker module
    let receiver = walker::find_files(&args)?;

    // 2. Process the files found by the walker
    processor::process_files(receiver, &args.output_file)?;

    println!(
        "Files have been processed and written to {}",
        args.output_file.display()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Commands, JoinArgs};
    use assert_fs::TempDir;
    use assert_fs::prelude::*;
    use std::fs::File;
    use std::io::Read;
    use std::path::Path;

    fn get_test_args(input_folder: &Path, output_file: &Path) -> JoinArgs {
        JoinArgs {
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
    fn test_filter_by_multiple_patterns() -> anyhow::Result<()> {
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

        run(Commands::Join(args))?;

        let mut result = String::new();
        File::open(&output_file)?.read_to_string(&mut result)?;

        assert!(result.contains("main.rs"));
        assert!(result.contains("Cargo.toml"));
        assert!(!result.contains("README.md"));

        Ok(())
    }

    #[test]
    fn test_skip_binary_files() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        let input_dir_path = dir.path();

        dir.child("text.txt").write_str("some text")?;
        dir.child("binary.bin")
            .write_binary(&[b'b', b'i', b'n', 0, b'a', b'r', b'y'])?;

        let output_file = input_dir_path.join("output.txt");
        let args = get_test_args(input_dir_path, &output_file);

        run(Commands::Join(args))?;

        let mut result = String::new();
        File::open(&output_file)?.read_to_string(&mut result)?;

        assert!(result.contains("text.txt"));
        assert!(!result.contains("binary.bin"));

        Ok(())
    }
}
