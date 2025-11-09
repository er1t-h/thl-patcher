use std::io;

use thiserror::Error;

#[allow(dead_code)]
#[derive(Debug)]
pub enum Error {
    Walkdir(walkdir::Error),
    Io(std::io::Error),
    NoMatchingVersion,
    NoPathSelected,
}
impl From<walkdir::Error> for Error {
    fn from(value: walkdir::Error) -> Self {
        Self::Walkdir(value)
    }
}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Debug, Error)]
pub enum GlobalErrorType {
    #[error("source not found: {0}")]
    SourceNotFound(#[from] minreq::Error),
    #[error("source format error: {0}")]
    SourceFormatError(#[from] serde_yaml::Error),
}

#[derive(Error, Debug)]
pub enum DownloadAndPatchError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("io error: {0}")]
    WalkDir(#[from] walkdir::Error),
    #[error("minreq error: {0}")]
    Minreq(#[from] minreq::Error),
    #[error("patcher error: {0}")]
    PatchError(#[from] thl_patcher::PatchError),
    #[error("no update link indicated")]
    NoUpdateLink,
}


#[derive(Error, Debug)]
pub enum GetVersionError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("version not found")]
    VersionNotFound,
    #[error("missing path")]
    MissingPath,
}