use patcher_common::error::GlobalErrorType;

pub mod global_error;
pub mod patcher;

pub enum AppScreen {
    Patcher(patcher::Patcher),
    SourceError(global_error::SourceError),
}

impl AppScreen {
    pub const fn source_error(error: GlobalErrorType) -> Self {
        Self::SourceError(global_error::SourceError { error })
    }
}
