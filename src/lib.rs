use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::Path,
};

use walkdir::WalkDir;
use xz2::{read::XzDecoder, write::XzEncoder};
const CHUNK_SIZE: usize = 2_000_000_000;

pub fn diff(old: &Path, new: &Path, destination: &Path) {
    if old.is_file() && new.is_file() {
        ddelta::generate_chunked(
            &mut BufReader::new(File::open(old).unwrap()),
            &mut BufReader::new(File::open(new).unwrap()),
            &mut BufWriter::new(XzEncoder::new(File::create(destination).unwrap(), 9)),
            CHUNK_SIZE,
            |_| (),
        )
        .unwrap();
    } else if old.is_dir() && new.is_dir() && (destination.is_dir() || !destination.exists()) {
        std::fs::create_dir_all(&destination).unwrap();
        for file in WalkDir::new(&new) {
            let file = file.unwrap();
            if !file.file_type().is_file() {
                continue;
            }
            let path = file.into_path();
            let file_relative_path = path.strip_prefix(&new).unwrap();
            let equivalent_in_old = old.join(file_relative_path);
            if !equivalent_in_old.is_file() {
                eprintln!("ignoring {}", file_relative_path.display());
                continue;
            }
            let equivalent_in_destination = destination.join(file_relative_path);
            std::fs::create_dir_all(equivalent_in_destination.parent().unwrap()).unwrap();
            ddelta::generate_chunked(
                &mut BufReader::new(File::open(&equivalent_in_old).unwrap()),
                &mut BufReader::new(File::open(&path).unwrap()),
                &mut BufWriter::new(XzEncoder::new(
                    File::create(&equivalent_in_destination).unwrap(),
                    9,
                )),
                CHUNK_SIZE,
                |_| (),
            )
            .unwrap();
        }
    } else {
        eprintln!("error");
    }
}

#[derive(Default, Clone, Copy)]
pub struct State {
    pub done: usize,
    pub out_of: usize,
}

pub fn patch(old: &Path, new: &Path, destination: &Path, mut update: impl FnMut(State)) {
    if old.is_file() && new.is_file() {
        ddelta::apply_chunked(
            &mut BufReader::new(File::open(old).unwrap()),
            &mut BufWriter::new(File::create(destination).unwrap()),
            &mut BufReader::new(XzDecoder::new(File::open(new).unwrap())),
        )
        .unwrap();
    } else if old.is_dir() && new.is_dir() && (destination.is_dir() || !destination.exists()) {
        let mut state = State {
            done: 0,
            out_of: WalkDir::new(&new)
                .into_iter()
                .filter(|file| file.as_ref().is_ok_and(|entry| entry.file_type().is_file()))
                .count(),
        };

        std::fs::create_dir_all(&destination).unwrap();
        for file in WalkDir::new(&new) {
            let file = file.unwrap();
            if !file.file_type().is_file() {
                continue;
            }
            let path = file.into_path();
            let suffix = path.strip_prefix(&new).unwrap();
            let equivalent_in_old = old.join(suffix);
            if !equivalent_in_old.exists() {
                eprintln!("ignoring {}", suffix.display());
                continue;
            }
            let equivalent_in_destination = destination.join(suffix);
            std::fs::create_dir_all(equivalent_in_destination.parent().unwrap()).unwrap();

            ddelta::apply_chunked(
                &mut BufReader::new(File::open(equivalent_in_old).unwrap()),
                &mut BufWriter::new(File::create(equivalent_in_destination).unwrap()),
                &mut BufReader::new(XzDecoder::new(File::open(path).unwrap())),
            )
            .unwrap();
            state.done += 1;
            (update)(state)
        }
    }
}
