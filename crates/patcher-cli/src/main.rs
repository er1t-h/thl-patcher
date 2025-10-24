use clap::Parser;
use walkdir::WalkDir;
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
            let count = WalkDir::new(&args.new).into_iter().filter(|entry| entry.as_ref().is_ok_and(|f| f.file_type().is_file())).count();
            let progress_bar = indicatif::ProgressBar::new(count as u64);
            thl_patcher::diff_in_tar(
                &args.old,
                &args.new,
                &mut tar::Builder::new(XzEncoder::new(File::create(args.destination)?, COMPRESSION_LEVEL)),
                |_| progress_bar.inc(1),
            )?;
        }
        Command::Patch => {
            let progress_bar = indicatif::ProgressBar::new_spinner();
            thl_patcher::patch_from_tar(
                &args.old,
                &mut tar::Archive::new(XzDecoder::new(File::open(args.new)?)),
                &args.destination,
                |current| progress_bar.set_message(format!("patching {}", current.path.display())),
            )?;
        }
    }
    anyhow::Ok(())
}
