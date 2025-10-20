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
