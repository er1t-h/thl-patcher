#[allow(dead_code)]
#[derive(Debug)]
pub enum Error {
    Walkdir(walkdir::Error),
    Io(std::io::Error),
    Rustyline(rustyline::error::ReadlineError),
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
impl From<rustyline::error::ReadlineError> for Error {
    fn from(value: rustyline::error::ReadlineError) -> Self {
        Self::Rustyline(value)
    }
}
