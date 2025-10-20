#[cfg(feature = "diff")]
pub use diff::*;

#[cfg(feature = "diff")]
mod diff {
    use std::{
        fs::File,
        io::{self, BufReader, BufWriter, Write},
        path::Path,
    };
    use tempfile::NamedTempFile;
    use thiserror::Error;
    use walkdir::WalkDir;

    const CHUNK_SIZE: usize = 400_000_000;

    fn count_files(path: &Path) -> usize {
        WalkDir::new(&path)
            .into_iter()
            .filter(|file| file.as_ref().is_ok_and(|entry| entry.file_type().is_file()))
            .count()
    }

    #[derive(Default, Clone, Copy)]
    pub struct DiffState {
        pub done: usize,
        pub out_of: usize,
    }

    #[derive(Debug, Error)]
    pub enum DiffError {
        #[error("io error: {0}")]
        Io(#[from] io::Error),
        #[error("walkdir error: {0}")]
        Walkdir(#[from] walkdir::Error),
        #[error("error with ddelta diff")]
        DdeltaDiff(#[from] ddelta::DiffError),
        #[error("old and new should both be files or dir")]
        TypeMismatch,
    }

    pub fn diff_in_tar(
        old: &Path,
        new: &Path,
        destination: &mut tar::Builder<impl Write>,
        mut update: impl FnMut(DiffState),
    ) -> Result<(), DiffError> {
        if old.is_dir() && new.is_dir() {
            let mut state = DiffState {
                done: 0,
                out_of: count_files(new),
            };

            for file in WalkDir::new(&new) {
                let file = file?;
                if !file.file_type().is_file() {
                    continue;
                }
                let new_file_path = file.into_path();
                let file_relative_path = match new_file_path.strip_prefix(&new) {
                    Ok(file_relative_path) => file_relative_path,
                    Err(_) => unreachable!("new_file_path is always a child of new"),
                };
                let old_file_path = old.join(file_relative_path);
                if !old_file_path.is_file() {
                    tracing::warn!("ignoring {}", file_relative_path.display());
                    continue;
                }

                let mut tmp_file = NamedTempFile::new()?;
                ddelta::generate_chunked(
                    &mut BufReader::new(File::open(&old_file_path)?),
                    &mut BufReader::new(File::open(&new_file_path)?),
                    &mut BufWriter::new(&mut tmp_file),
                    CHUNK_SIZE,
                    |_| (),
                )?;
                destination.append_file(file_relative_path, tmp_file.as_file_mut())?;
                state.done += 1;
                (update)(state)
            }
            Ok(())
        } else {
            Err(DiffError::TypeMismatch)
        }
    }
}

#[cfg(feature = "patch")]
pub use patch::*;
#[cfg(feature = "patch")]
mod patch {
    use std::{
        fs::File,
        io::{self, BufReader, BufWriter, Read},
        path::{Path, PathBuf},
    };
    use thiserror::Error;

    pub struct CurrentPatchingPath {
        pub path: PathBuf,
    }

    #[derive(Debug, Error)]
    pub enum PatchError {
        #[error("io error: {0}")]
        Io(#[from] io::Error),
        #[error("error with ddelta patch")]
        DdeltaPatch(#[from] ddelta::PatchError),
        #[error("old and new should both be files or dir")]
        TypeMismatch,
    }

    pub fn patch_from_tar(
        old: &Path,
        new: &mut tar::Archive<impl Read>,
        destination: &Path,
        mut update: impl FnMut(CurrentPatchingPath),
    ) -> Result<(), PatchError> {
        if old.is_dir() && (destination.is_dir() || !destination.exists()) {
            std::fs::create_dir_all(&destination)?;
            for file in new.entries()? {
                let file = file?;
                (update)(CurrentPatchingPath {
                    path: file.path()?.into_owned(),
                });

                let path = file.path()?;
                let suffix = path.as_ref();
                let equivalent_in_old = old.join(suffix);
                if !equivalent_in_old.exists() {
                    tracing::warn!("ignoring {}", suffix.display());
                    continue;
                }

                let equivalent_in_destination = destination.join(suffix);
                let parent = match equivalent_in_destination.parent() {
                    Some(x) => x,
                    None => unreachable!("equivalent_in_destination should always have a parent"),
                };
                std::fs::create_dir_all(parent)?;

                ddelta::apply_chunked(
                    &mut BufReader::new(File::open(equivalent_in_old)?),
                    &mut BufWriter::new(File::create(equivalent_in_destination)?),
                    &mut BufReader::new(file),
                )
                .map_err(PatchError::DdeltaPatch)?;
            }
            Ok(())
        } else {
            Err(PatchError::TypeMismatch)
        }
    }
}
