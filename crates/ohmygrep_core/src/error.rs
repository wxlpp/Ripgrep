use thiserror::Error;

#[derive(Debug, Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum OhMyGrepError {
    #[error("invalid regex: {0}")]
    InvalidPattern(String),

    #[error("invalid arguments: {0}")]
    InvalidArguments(String),

    #[error("path not found: {0}")]
    PathNotFound(String),

    #[error("io error: {0}")]
    Io(String),

    #[error("internal panic: {0}")]
    InternalPanic(String),
}

impl From<std::io::Error> for OhMyGrepError {
    fn from(e: std::io::Error) -> Self {
        OhMyGrepError::Io(e.to_string())
    }
}
