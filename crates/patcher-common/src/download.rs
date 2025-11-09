use std::{io::{self, Cursor}, path::Path};

use tar::Archive;
use tempfile::tempdir;
use walkdir::WalkDir;
use xz2::read::XzDecoder;

use crate::{error::DownloadAndPatchError, structures::source::VersionTransitionRef};

pub trait ProgressReporter {
    /// Ran each time a new version is processed. Typically a good time to print a "Downloading" message
    fn on_start_new_version(&mut self, _transition: &VersionTransitionRef) {}
    /// Ran each time a new file is being processed. Typically a good time to print a "Patching" message
    fn on_patching_file(&mut self, _path: &Path) {}
    /// Ran each time a version patch ends. Can be useful for cleaning up some of the interface
    fn on_version_patch_end(&mut self) {}
    /// Ran on finish
    fn on_finish(&mut self) {}
}

#[allow(clippy::cast_precision_loss)]
pub fn download_and_patch<'a>(
    original: &Path,
    transitions: impl Iterator<Item = VersionTransitionRef<'a>>,
    mut progress: impl ProgressReporter,
) -> Result<(), DownloadAndPatchError> {
    for transition @ VersionTransitionRef { old, .. } in transitions {
        progress.on_start_new_version(&transition);
        // A temporary directory where patched files go
        let temp_dir = tempdir()?;

        let update_link = old
            .update_link
            .as_ref()
            .ok_or(DownloadAndPatchError::NoUpdateLink)?;
        let archive_content = minreq::get(update_link).send()?.into_bytes();
        let decoder = XzDecoder::new(Cursor::new(archive_content));
        let mut archive = Archive::new(decoder);

        thl_patcher::patch_from_tar(original, &mut archive, temp_dir.path(), |s| {
            progress.on_patching_file(&s.path);
        })?;

        for file in WalkDir::new(temp_dir.path()) {
            let file = file?;
            if !file.file_type().is_file() {
                continue;
            }
            let path = file.into_path();
            let Ok(suffix) = path.strip_prefix(temp_dir.path()) else {
                unreachable!("path is always a child of temp_dir");
            };
            let destination = original.join(suffix);
            match std::fs::rename(&path, &destination) {
                Ok(()) => (),
                Err(e) if e.kind() == io::ErrorKind::CrossesDevices => {
                    std::fs::copy(&path, original.join(suffix))?;
                }
                Err(e) => Err(e)?,
            }
        }
        progress.on_version_patch_end();
    }
    progress.on_finish();
    Ok(())
}
