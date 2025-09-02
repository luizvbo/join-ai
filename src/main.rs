use anstyle::{AnsiColor, Color, Style};
use clap::builder::styling::Styles;
use clap::{CommandFactory, FromArgMatches};
use join_ai::{cli::Cli, run};

/// Creates a custom style for the CLI's help output, mimicking the appearance of `cargo`.
/// This provides a more professional and familiar feel for Rust developers.
fn get_styles() -> Styles {
    Styles::styled()
        .header(
            Style::new()
                .fg_color(Some(Color::Ansi(AnsiColor::Green)))
                .bold(),
        )
        .usage(
            Style::new()
                .fg_color(Some(Color::Ansi(AnsiColor::Green)))
                .bold(),
        )
        .literal(
            Style::new()
                .fg_color(Some(Color::Ansi(AnsiColor::Cyan)))
                .bold(),
        )
        .placeholder(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Cyan))))
}

/// The main entry point of the application binary.
fn main() -> anyhow::Result<()> {
    // 1. Build the command-line interface definition from the `Cli` struct.
    let mut cmd = Cli::command();

    // 2. Apply the custom styles to the command's help message.
    cmd = cmd.styles(get_styles());

    // 3. Parse the actual command-line arguments provided by the user.
    let matches = cmd.get_matches();

    // 4. Convert the parsed matches back into our strongly-typed `Cli` struct.
    let cli = Cli::from_arg_matches(&matches)?;

    // 5. Pass the parsed command to the core logic in the `lib.rs` crate.
    run(cli.command)
}
