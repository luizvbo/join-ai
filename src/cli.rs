use clap::{Args as ClapArgs, ColorChoice, Parser, Subcommand};
use std::path::PathBuf;

/// A CLI application to traverse files in a folder and concatenate them
/// into a single text file, suitable for GenAI model input.
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None, color = ColorChoice::Always)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Concatenate files into a single text file
    Join(JoinArgs),
    /// Update the application to the latest version [placeholder]
    Update(UpdateArgs),
}

/// Arguments for the 'join' command
#[derive(ClapArgs, Debug, Clone)]
pub struct JoinArgs {
    /// The input folder to traverse for files
    #[arg(required = true)]
    pub input_folder: PathBuf,

    /// The output file to write the concatenated content to
    #[arg(short, long, default_value = "concatenated.txt")]
    pub output_file: PathBuf,

    /// File patterns to include. Can be specified multiple times (e.g., -p "*.rs" -p "*.md")
    #[arg(short, long, action = clap::ArgAction::Append)]
    pub patterns: Option<Vec<String>>,

    /// Clear the output file before writing
    #[arg(short, long)]
    pub clear_file: bool,

    /// Folders to exclude from the search. Can be specified multiple times.
    #[arg(short, long, action = clap::ArgAction::Append)]
    pub exclude_folders: Option<Vec<String>>,

    /// File extensions to exclude. Can be specified multiple times.
    #[arg(long, action = clap::ArgAction::Append)]
    pub exclude_extensions: Option<Vec<String>>,

    /// Set the maximum search depth
    #[arg(long)]
    pub max_depth: Option<usize>,

    /// Include hidden files in the search
    #[arg(long)]
    pub hidden: bool,

    /// Do not follow symlinks
    #[arg(long, default_value_t = true)]
    pub no_follow: bool,
}

/// Arguments for the 'update' command
#[derive(ClapArgs, Debug, Clone)]
pub struct UpdateArgs {}

#[cfg(test)]
mod tests {
    use super::*; // Import all items from the parent cli module
    use clap::error::ErrorKind; // Import the ErrorKind enum

    #[test]
    fn test_basic_join_command_and_defaults() {
        let args = vec!["join-ai", "join", "./my-project"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Join(join_args) => {
                assert_eq!(join_args.input_folder, PathBuf::from("./my-project"));
                // Assert default values
                assert_eq!(join_args.output_file, PathBuf::from("concatenated.txt"));
                assert!(!join_args.clear_file);
                assert!(!join_args.hidden);
                assert!(join_args.patterns.is_none());
                assert!(join_args.exclude_folders.is_none());
                assert!(join_args.exclude_extensions.is_none());
                assert!(join_args.max_depth.is_none());
                assert!(join_args.no_follow); // Default is true
            }
            _ => panic!("Expected Join command to be parsed"),
        }
    }

    #[test]
    fn test_all_join_options_are_parsed() {
        let args = vec![
            "join-ai",
            "join",
            "src",
            "-o",
            "output.txt",
            "-p",
            "*.rs",
            "-p",
            "*.toml",
            "--clear-file",
            "-e",
            "target",
            "--exclude-folders",
            ".git", // Test long name for exclude
            "--exclude-extensions",
            "log",
            "--exclude-extensions",
            "tmp",
            "--max-depth",
            "10",
            "--hidden",
            // We omit --no-follow to ensure the default is still applied
        ];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Join(join_args) => {
                assert_eq!(join_args.input_folder, PathBuf::from("src"));
                assert_eq!(join_args.output_file, PathBuf::from("output.txt"));
                assert_eq!(
                    join_args.patterns,
                    Some(vec!["*.rs".to_string(), "*.toml".to_string()])
                );
                assert!(join_args.clear_file);
                assert_eq!(
                    join_args.exclude_folders,
                    Some(vec!["target".to_string(), ".git".to_string()])
                );
                assert_eq!(
                    join_args.exclude_extensions,
                    Some(vec!["log".to_string(), "tmp".to_string()])
                );
                assert_eq!(join_args.max_depth, Some(10));
                assert!(join_args.hidden);
                assert!(join_args.no_follow);
            }
            _ => panic!("Expected Join command to be parsed"),
        }
    }

    #[test]
    fn test_update_subcommand_is_parsed() {
        let args = vec!["join-ai", "update"];
        let cli = Cli::try_parse_from(args).unwrap();

        assert!(matches!(cli.command, Commands::Update(_)));
    }

    #[test]
    fn test_missing_required_argument_fails() {
        // Missing the required `input_folder` argument
        let args = vec!["join-ai", "join", "-o", "output.txt"];
        let result = Cli::try_parse_from(args);

        assert!(
            result.is_err(),
            "Parsing should fail without the required input_folder"
        );
        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::MissingRequiredArgument
        );
    }

    #[test]
    fn test_no_subcommand_fails() {
        let args = vec!["join-ai"];
        let result = Cli::try_parse_from(args);

        assert!(result.is_err(), "Parsing should fail without a subcommand");
        // THIS IS THE FIX: Assert against the correct ErrorKind
        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        );
    }
}
