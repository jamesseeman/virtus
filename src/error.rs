use thiserror::Error;

#[derive(Debug, Error)]
#[error("Error occured")]
pub enum VirtusError {
    IOError(#[from] std::io::Error),
    UTF8Error(#[from] std::str::Utf8Error),
    SerdeError(#[from] serde_json::Error),
    DbError
}
