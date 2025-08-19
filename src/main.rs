use anstyle::{AnsiColor, Color, Style};
use clap::builder::styling::Styles;
use clap::{CommandFactory, FromArgMatches};
use join_ai::{cli::Cli, run};

// Function to create the cargo-like styles
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

fn main() -> anyhow::Result<()> {
    let mut cmd = Cli::command();
    cmd = cmd.styles(get_styles());
    let matches = cmd.get_matches();
    let cli = Cli::from_arg_matches(&matches)?;
    run(cli.command)
}
