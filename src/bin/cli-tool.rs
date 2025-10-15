use clap::Parser;
use std::path::PathBuf;

#[derive(clap::ValueEnum, Clone)]
pub enum Command {
    Diff,
    Patch,
}

#[derive(clap::Parser, Clone)]
pub struct Argument {
    pub subcommand: Command,
    pub old: PathBuf,
    pub new: PathBuf,
    pub destination: PathBuf,
}

fn main() {
    let args = Argument::parse();
    match args.subcommand {
        Command::Diff => {
            thl_patcher::diff(&args.old, &args.new, &args.destination);
        }
        Command::Patch => {
            thl_patcher::patch(&args.old, &args.new, &args.destination);
        }
    }
}
