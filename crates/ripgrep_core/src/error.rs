use thiserror::Error;

#[derive(Debug, Error)]
pub enum RipgrepError {
    #[error("invalid regex: {0}")]
    InvalidPattern(String),

    #[error("path not found: {0}")]
    PathNotFound(String),

    #[error("io error: {0}")]
    Io(String),

    #[error("internal panic: {0}")]
    InternalPanic(String),
}

impl From<std::io::Error> for RipgrepError {
    fn from(e: std::io::Error) -> Self {
        RipgrepError::Io(e.to_string())
    }
}
