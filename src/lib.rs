use std::fs;

pub mod cli;
pub mod decommenter;
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
    if args.strip_comments {
        println!("Comment stripping: enabled");
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
    processor::process_files(receiver, &args.output_file, args.strip_comments)?;

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
    use std::fs::{self};
    use std::path::Path;

    /// Helper to create a standard JoinArgs struct for tests.
    fn get_test_args(input_folder: &Path, output_file: &Path) -> JoinArgs {
        JoinArgs {
            input_folder: input_folder.to_path_buf(),
            output_file: output_file.to_path_buf(),
            patterns: None,
            clear_file: false,
            strip_comments: false, // <-- FIX WAS HERE: Added the missing field
            exclude_folders: None,
            exclude_extensions: None,
            max_depth: None,
            hidden: false,
            no_follow: true,
        }
    }

    /// Helper to run the join command and read the resulting output file.
    fn run_join_and_read_output(args: JoinArgs) -> anyhow::Result<String> {
        let output_path = args.output_file.clone();
        run(Commands::Join(args))?;
        Ok(fs::read_to_string(output_path).unwrap_or_default())
    }

    #[test]
    fn test_strip_comments_flag() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        let code_with_comments = r#"
// This is a single line comment.
fn main() {
    /* block comment */
    println!("Hello, world!");
}
"#;
        dir.child("src/main.rs").write_str(code_with_comments)?;

        let output_file = dir.path().join("output.txt");
        let mut args = get_test_args(dir.path(), &output_file);
        args.strip_comments = true; // Enable the feature

        let result = run_join_and_read_output(args)?;

        assert!(result.contains("fn main()"));
        assert!(!result.contains("single line comment"));
        assert!(!result.contains("block comment"));

        Ok(())
    }

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

    #[test]
    fn test_exclude_folders() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        dir.child("src/main.rs").write_str("main")?;
        dir.child("target/debug/app").write_str("binary")?;
        dir.child("docs/guide.md").write_str("guide")?;

        let output_file = dir.path().join("output.txt");
        let mut args = get_test_args(dir.path(), &output_file);
        args.exclude_folders = Some(vec!["target".to_string(), "docs".to_string()]);

        let result = run_join_and_read_output(args)?;

        assert!(result.contains("main.rs"));
        assert!(!result.contains("app"));
        assert!(!result.contains("guide.md"));

        Ok(())
    }

    #[test]
    fn test_exclude_extensions() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        dir.child("code.rs").write_str("main")?;
        dir.child("notes.txt").write_str("notes")?;
        dir.child("log.log").write_str("log")?;

        let output_file = dir.path().join("output.txt");
        let mut args = get_test_args(dir.path(), &output_file);
        args.exclude_extensions = Some(vec!["log".to_string(), "txt".to_string()]);

        let result = run_join_and_read_output(args)?;

        assert!(result.contains("code.rs"));
        assert!(!result.contains("notes.txt"));
        assert!(!result.contains("log.log"));

        Ok(())
    }

    #[test]
    fn test_max_depth() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        dir.child("level1.txt").write_str("1")?;
        dir.child("a/level2.txt").write_str("2")?;
        dir.child("a/b/level3.txt").write_str("3")?;

        let output_file = dir.path().join("output.txt");
        let mut args = get_test_args(dir.path(), &output_file);
        args.max_depth = Some(2); // Root (depth 0), level1 (depth 1), level2 (depth 2)

        let result = run_join_and_read_output(args)?;

        assert!(result.contains("level1.txt"));
        assert!(result.contains("level2.txt"));
        assert!(!result.contains("level3.txt"));

        Ok(())
    }

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

    #[test]
    fn test_output_file_is_skipped() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        let output_file = dir.path().join("output.txt");
        // Create the output file beforehand to ensure it exists during the walk
        fs::write(&output_file, "initial content")?;
        dir.child("input.txt").write_str("input")?;

        let args = get_test_args(dir.path(), &output_file);
        let result = run_join_and_read_output(args)?;

        // The result should not contain its own initial content
        assert!(!result.contains("initial content"));
        assert!(result.contains("input.txt"));

        Ok(())
    }

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

    #[test]
    fn test_empty_directory_produces_empty_file() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        let output_file = dir.path().join("output.txt");
        let args = get_test_args(dir.path(), &output_file);

        let result = run_join_and_read_output(args)?;

        assert!(result.is_empty());

        Ok(())
    }

    #[test]
    fn test_update_command_placeholder() -> anyhow::Result<()> {
        // This test simply ensures the update command can be called without error.
        // A more advanced test would capture stdout, but this is sufficient.
        let update_args = cli::UpdateArgs {};
        run(Commands::Update(update_args))?;
        Ok(())
    }
}
