use thiserror::Error;

#[derive(Debug, Error)]
#[error("Error occured")]
pub enum VirtusError {
    DiskError,
    UuidError(#[from] uuid::Error),
    XMLError(#[from] quick_xml::Error),
    IOError(#[from] std::io::Error),
    UTF8Error(#[from] std::str::Utf8Error),
    SerdeError(#[from] serde_json::Error),
    VirtError(#[from] virt::error::Error),
    DbError
}
