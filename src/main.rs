use anstyle::{AnsiColor, Color, Style};
use clap::builder::styling::Styles;
use clap::{CommandFactory, FromArgMatches};
use join_ai::{cli::Cli, run}; // Import Cli instead of Args

// Function to create the cargo-like styles
fn get_styles() -> Styles {
    Styles::styled()
        .header(
            Style::new()
                .fg_color(Some(Color::Ansi(AnsiColor::Yellow)))
                .bold(),
        )
        .usage(
            Style::new()
                .fg_color(Some(Color::Ansi(AnsiColor::Yellow)))
                .bold(),
        )
        .literal(
            Style::new()
                .fg_color(Some(Color::Ansi(AnsiColor::Green)))
                .bold(),
        )
        .placeholder(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Cyan))))
}

fn main() -> anyhow::Result<()> {
    // 1. Get the command definition from your derived Cli struct
    let mut cmd = Cli::command();

    // 2. Apply the custom styles
    cmd = cmd.styles(get_styles());

    // 3. Get the matches from the styled command
    let matches = cmd.get_matches();

    // 4. Create the Cli struct from the matches
    let cli = Cli::from_arg_matches(&matches)?;

    // 5. Run your application logic with the chosen command
    run(cli.command)
}
