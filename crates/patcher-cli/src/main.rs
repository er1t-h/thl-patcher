use clap::Parser;
use std::{fs::File, path::PathBuf};
use xz2::{read::XzDecoder, write::XzEncoder};

const COMPRESSION_LEVEL: u32 = 9;

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

fn main() -> anyhow::Result<()> {
    let args = Argument::parse();
    match args.subcommand {
        Command::Diff => {
            thl_patcher::diff_in_tar(
                &args.old,
                &args.new,
                &mut tar::Builder::new(XzEncoder::new(File::create(args.destination)?, COMPRESSION_LEVEL)),
                |_| (),
            )?;
        }
        Command::Patch => {
            thl_patcher::patch_from_tar(
                &args.old,
                &mut tar::Archive::new(XzDecoder::new(File::open(args.new)?)),
                &args.destination,
                |_| (),
            )?;
        }
    }
    anyhow::Ok(())
}
