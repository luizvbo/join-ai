use std::fs;

// Public modules that make up the library's functionality.
pub mod cli;
pub mod processor;
pub mod walker;

use cli::{Commands, JoinArgs};

/// The primary entry point for the library's logic.
/// It takes a parsed `Commands` enum and dispatches to the appropriate handler.
pub fn run(command: Commands) -> anyhow::Result<()> {
    match command {
        Commands::Join(args) => run_join(args),
        Commands::Update(_args) => {
            // Placeholder for future update functionality.
            println!("Update functionality is not yet implemented.");
            println!("Please check for new releases at the GitHub repository:");
            println!("https://github.com/luizvbo/join-ai/releases");
            Ok(())
        }
    }
}

/// Handles the logic for the 'join' command.
/// This function orchestrates the file finding and processing steps.
fn run_join(args: JoinArgs) -> anyhow::Result<()> {
    // --- 1. Log the configuration for user feedback ---
    println!(
        "Processing files in folder: {}",
        args.input_folder.display()
    );
    if let Some(patterns) = &args.patterns {
        println!("Using patterns: {}", patterns.join(", "));
    } else {
        println!("Using patterns: all files");
    }
    if let Some(exclude_patterns) = &args.exclude {
        println!("Excluding patterns: {}", exclude_patterns.join(", "));
    }

    // --- 2. Prepare the output file ---
    if args.clear_file && args.output_file.exists() {
        fs::remove_file(&args.output_file)?;
        println!(
            "Output file {} has been cleared.",
            args.output_file.display()
        );
    }

    // --- 3. Find all relevant files using the walker module ---
    // The walker runs in a background thread and sends file paths via a channel.
    let receiver = walker::find_files(&args)?;

    // --- 4. Process the files found by the walker ---
    // The processor reads each file and appends its content to the output file.
    processor::process_files(receiver, &args.output_file)?;

    println!(
        "Files have been processed and written to {}",
        args.output_file.display()
    );

    Ok(())
}

// --- Integration-style Tests for Core Logic ---
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Commands, JoinArgs};
    use assert_fs::TempDir;
    use assert_fs::prelude::*;
    use std::fs::{self};
    use std::path::Path;

    /// Test helper to create a standard `JoinArgs` struct with common defaults.
    fn get_test_args(input_folder: &Path, output_file: &Path) -> JoinArgs {
        JoinArgs {
            input_folder: input_folder.to_path_buf(),
            output_file: output_file.to_path_buf(),
            patterns: None,
            exclude: None,
            clear_file: false,
            max_depth: None,
            hidden: false,
            no_follow: true,
        }
    }

    /// Test helper to execute the `run_join` command and return the content of the output file.
    fn run_join_and_read_output(args: JoinArgs) -> anyhow::Result<String> {
        let output_path = args.output_file.clone();
        run(Commands::Join(args))?;
        Ok(fs::read_to_string(output_path).unwrap_or_default())
    }

    /// Verifies that only files matching the include patterns are processed.
    #[test]
    fn test_filter_by_multiple_patterns() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        dir.child("Cargo.toml").write_str("[package]")?;
        dir.child("README.md").write_str("# Project")?;
        dir.child("src/main.rs").write_str("fn main(){}")?;

        let output_file = dir.path().join("output.txt");
        let mut args = get_test_args(dir.path(), &output_file);
        args.patterns = Some(vec!["*.rs".to_string(), "*.toml".to_string()]);

        let result = run_join_and_read_output(args)?;

        assert!(result.contains("// FILE:"));
        assert!(result.contains("main.rs"));
        assert!(result.contains("Cargo.toml"));
        assert!(!result.contains("README.md"));

        Ok(())
    }

    /// Verifies that binary files (containing NUL bytes) are automatically skipped.
    #[test]
    fn test_skip_binary_files() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        dir.child("text.txt").write_str("some text")?;
        dir.child("binary.bin")
            .write_binary(&[b'b', b'i', b'n', 0, b'a', b'r', b'y'])?;

        let output_file = dir.path().join("output.txt");
        let args = get_test_args(dir.path(), &output_file);

        let result = run_join_and_read_output(args)?;

        assert!(result.contains("text.txt"));
        assert!(!result.contains("binary.bin"));

        Ok(())
    }

    /// Verifies that the `--max-depth` argument correctly limits traversal.
    #[test]
    fn test_max_depth() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        dir.child("level1.txt").write_str("1")?;
        dir.child("a/level2.txt").write_str("2")?;
        dir.child("a/b/level3.txt").write_str("3")?;

        let output_file = dir.path().join("output.txt");
        let mut args = get_test_args(dir.path(), &output_file);
        args.max_depth = Some(2);

        let result = run_join_and_read_output(args)?;

        assert!(result.contains("level1.txt"));
        assert!(result.contains("level2.txt"));
        assert!(!result.contains("level3.txt"));

        Ok(())
    }

    /// Verifies that hidden files are ignored by default.
    #[test]
    fn test_hidden_files_are_skipped_by_default() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        dir.child(".env").write_str("secret")?;
        dir.child("visible.txt").write_str("visible")?;

        let output_file = dir.path().join("output.txt");
        let args = get_test_args(dir.path(), &output_file);

        let result = run_join_and_read_output(args)?;

        assert!(!result.contains(".env"));
        assert!(result.contains("visible.txt"));

        Ok(())
    }

    /// Verifies that the `--hidden` flag includes hidden files in the output.
    #[test]
    fn test_hidden_files_are_included_with_flag() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        dir.child(".env").write_str("secret")?;
        dir.child("visible.txt").write_str("visible")?;

        let output_file = dir.path().join("output.txt");
        let mut args = get_test_args(dir.path(), &output_file);
        args.hidden = true;

        let result = run_join_and_read_output(args)?;

        assert!(result.contains(".env"));
        assert!(result.contains("visible.txt"));

        Ok(())
    }

    /// Verifies that the application does not read and include its own output file.
    #[test]
    fn test_output_file_is_skipped() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        let output_file = dir.path().join("output.txt");
        fs::write(&output_file, "initial content")?;
        dir.child("input.txt").write_str("input")?;

        let args = get_test_args(dir.path(), &output_file);
        let result = run_join_and_read_output(args)?;

        assert!(!result.contains("initial content"));
        assert!(result.contains("input.txt"));

        Ok(())
    }

    /// Verifies that the `--clear-file` flag deletes existing content before writing.
    #[test]
    fn test_clear_file_option() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        let output_file = dir.path().join("output.txt");
        fs::write(&output_file, "this should be cleared")?;
        dir.child("input.txt").write_str("new content")?;

        let mut args = get_test_args(dir.path(), &output_file);
        args.clear_file = true;

        let result = run_join_and_read_output(args)?;

        assert!(!result.contains("this should be cleared"));
        assert!(result.contains("new content"));

        Ok(())
    }

    /// Verifies that running on an empty directory produces an empty output file.
    #[test]
    fn test_empty_directory_produces_empty_file() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        let output_file = dir.path().join("output.txt");
        let args = get_test_args(dir.path(), &output_file);

        let result = run_join_and_read_output(args)?;

        assert!(result.is_empty());

        Ok(())
    }

    /// Verifies that the `update` command can be called without error.
    #[test]
    fn test_update_command_placeholder() -> anyhow::Result<()> {
        let update_args = cli::UpdateArgs {};
        run(Commands::Update(update_args))?;
        Ok(())
    }

    // --- New Tests for Exclude Functionality ---

    /// Verifies that a folder pattern (e.g., "target/") excludes all its contents.
    #[test]
    fn test_exclude_by_folder_pattern() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        dir.child("src/main.rs").write_str("main")?;
        dir.child("target/debug/app").write_str("binary")?;
        dir.child("docs/guide.md").write_str("guide")?;

        let output_file = dir.path().join("output.txt");
        let mut args = get_test_args(dir.path(), &output_file);
        args.exclude = Some(vec!["target/".to_string(), "docs/".to_string()]);

        let result = run_join_and_read_output(args)?;

        assert!(result.contains("main.rs"));
        assert!(!result.contains("app"));
        assert!(!result.contains("guide.md"));

        Ok(())
    }

    /// Verifies that a file extension pattern (e.g., "*.log") excludes matching files.
    #[test]
    fn test_exclude_by_extension_pattern() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        dir.child("code.rs").write_str("main")?;
        dir.child("notes.md").write_str("notes")?;
        dir.child("log.log").write_str("log")?;

        let output_file = dir.path().join("output.txt");
        let mut args = get_test_args(dir.path(), &output_file);
        args.exclude = Some(vec!["*.log".to_string(), "*.md".to_string()]);

        let result = run_join_and_read_output(args)?;

        assert!(result.contains("code.rs"));
        assert!(!result.contains("notes.md"));
        assert!(!result.contains("log.log"));

        Ok(())
    }

    /// Verifies that multiple, different exclusion patterns work together.
    #[test]
    fn test_exclude_by_multiple_patterns() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        dir.child("src/main.rs").write_str("main")?;
        dir.child("src/error.log").write_str("log")?;
        dir.child("target/app").write_str("binary")?;

        let output_file = dir.path().join("output.txt");
        let mut args = get_test_args(dir.path(), &output_file);
        args.exclude = Some(vec!["target/".to_string(), "*.log".to_string()]);

        let result = run_join_and_read_output(args)?;

        assert!(result.contains("main.rs"));
        assert!(!result.contains("error.log"));
        assert!(!result.contains("app"));

        Ok(())
    }

    /// Verifies that an exclude pattern will override an include pattern.
    #[test]
    fn test_exclude_takes_precedence_over_include() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        dir.child("src/main.rs").write_str("main")?;
        dir.child("src/lib.rs").write_str("lib")?;
        dir.child("tests/integration_test.rs").write_str("test")?;

        let output_file = dir.path().join("output.txt");
        let mut args = get_test_args(dir.path(), &output_file);
        args.patterns = Some(vec!["*.rs".to_string()]); // Include all .rs files
        args.exclude = Some(vec!["tests/".to_string()]); // But exclude the tests folder

        let result = run_join_and_read_output(args)?;

        assert!(result.contains("main.rs"));
        assert!(result.contains("lib.rs"));
        assert!(!result.contains("integration_test.rs"));

        Ok(())
    }

    /// Verifies that a specific file can be excluded by its full path relative to the input.
    #[test]
    fn test_exclude_specific_file() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        dir.child("src/main.rs").write_str("main")?;
        dir.child("src/config.rs").write_str("config")?;
        dir.child("README.md").write_str("readme")?;

        let output_file = dir.path().join("output.txt");
        let mut args = get_test_args(dir.path(), &output_file);
        args.exclude = Some(vec!["src/config.rs".to_string()]);

        let result = run_join_and_read_output(args)?;

        assert!(result.contains("main.rs"));
        assert!(result.contains("README.md"));
        assert!(!result.contains("config.rs"));

        Ok(())
    }
}
