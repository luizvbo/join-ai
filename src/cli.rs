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
