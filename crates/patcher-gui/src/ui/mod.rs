pub mod global_error;
pub mod patcher;

pub enum AppScreen {
    Patcher(patcher::Patcher),
    SourceError(global_error::SourceError),
}

impl AppScreen {
    pub fn source_error(error: global_error::GlobalErrorType) -> Self {
        Self::SourceError(global_error::SourceError { error })
    }
}
