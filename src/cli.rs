use clap::{Args as ClapArgs, ColorChoice, Parser, Subcommand};
use std::path::PathBuf;

/// A CLI application to traverse files in a folder and concatenate them
/// into a single text file, suitable for GenAI model input.
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None, color = ColorChoice::Always)]
pub struct Cli {
    /// The subcommand to execute (e.g., 'join' or 'update').
    #[command(subcommand)]
    pub command: Commands,
}

/// Defines the available subcommands for the application.
#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Concatenate files into a single text file.
    Join(JoinArgs),
    /// Update the application to the latest version [placeholder].
    Update(UpdateArgs),
}

/// Defines the arguments for the 'join' subcommand.
#[derive(ClapArgs, Debug, Clone)]
pub struct JoinArgs {
    /// The root folder to start traversing for files. This is a required argument.
    #[arg(required = true)]
    pub input_folder: PathBuf,

    /// The path to the output file where the concatenated content will be written.
    #[arg(short, long, default_value = "concatenated.txt")]
    pub output_file: PathBuf,

    /// Glob patterns for files to *include*. Can be specified multiple times.
    /// If not provided, all files are considered (subject to exclusions).
    /// Example: -p "*.rs" -p "*.md"
    #[arg(short = 'p', long, action = clap::ArgAction::Append, value_name = "PATTERN")]
    pub patterns: Option<Vec<String>>,

    /// Glob patterns for files or folders to *exclude*. Can be specified multiple times.
    /// This is a powerful way to filter out unwanted content like build artifacts or logs.
    /// Example: -x "*.log" -x "target/"
    #[arg(short = 'x', long, action = clap::ArgAction::Append, value_name = "PATTERN")]
    pub exclude: Option<Vec<String>>,

    /// If set, the output file will be deleted before writing new content.
    #[arg(short, long)]
    pub clear_file: bool,

    /// Sets the maximum depth for directory traversal. A depth of 0 means only the
    /// input folder itself will be scanned.
    #[arg(long)]
    pub max_depth: Option<usize>,

    /// If set, hidden files and directories (those starting with a '.') will be included.
    #[arg(long)]
    pub hidden: bool,

    /// If set to false, the walker will follow symbolic links. Defaults to true (no-follow).
    #[arg(long, default_value_t = true)]
    pub no_follow: bool,

    /// Enable verbose output. Use -v for basic info, -vv for detailed debugging.
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

/// Defines the arguments for the 'update' subcommand. Currently a placeholder.
#[derive(ClapArgs, Debug, Clone)]
pub struct UpdateArgs {}

// --- Unit Tests for CLI Parsing ---
#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    /// Verifies that the `join` command parses the required input folder and
    /// correctly applies default values for all optional arguments.
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
                assert!(join_args.exclude.is_none());
                assert!(join_args.max_depth.is_none());
                assert!(join_args.no_follow); // Default is true
                assert_eq!(join_args.verbose, 0);
            }
            _ => panic!("Expected Join command to be parsed"),
        }
    }

    /// Verifies that all provided flags and options for the `join` command
    /// are parsed correctly into the `JoinArgs` struct.
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
            "-x",
            "target/",
            "--exclude",
            "*.log",
            "--max-depth",
            "10",
            "--hidden",
            "-vv",
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
                    join_args.exclude,
                    Some(vec!["target/".to_string(), "*.log".to_string()])
                );
                assert_eq!(join_args.max_depth, Some(10));
                assert!(join_args.hidden);
                assert!(join_args.no_follow);
                assert_eq!(join_args.verbose, 2);
            }
            _ => panic!("Expected Join command to be parsed"),
        }
    }

    /// Ensures the `update` subcommand is recognized and parsed correctly.
    #[test]
    fn test_update_subcommand_is_parsed() {
        let args = vec!["join-ai", "update"];
        let cli = Cli::try_parse_from(args).unwrap();

        assert!(matches!(cli.command, Commands::Update(_)));
    }

    /// Confirms that parsing fails if the required `input_folder` argument is missing.
    #[test]
    fn test_missing_required_argument_fails() {
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

    /// Confirms that parsing fails if no subcommand (like 'join') is provided.
    #[test]
    fn test_no_subcommand_fails() {
        let args = vec!["join-ai"];
        let result = Cli::try_parse_from(args);

        assert!(result.is_err(), "Parsing should fail without a subcommand");
        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        );
    }
}
