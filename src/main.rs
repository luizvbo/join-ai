use clap::Parser;
use join_ai::{cli::Args, run};

pub mod cli;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    run(args)
}
