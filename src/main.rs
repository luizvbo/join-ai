use anstyle::{AnsiColor, Color, Style};
use clap::builder::styling::Styles;
use clap::{CommandFactory, FromArgMatches};
use join_ai::{cli::Args, run};

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
    let mut cmd = Args::command();
    cmd = cmd.styles(get_styles());
    let matches = cmd.get_matches();
    let args = Args::from_arg_matches(&matches)?;
    run(args)
}
