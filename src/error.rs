use thiserror::Error;

#[derive(Debug, Error)]
#[error("error occured")]
pub enum VirtusError {
    #[error("failed to create disk")]
    DiskError,
    #[error("failed to create OVS port")]
    OVSError,
    UuidError(#[from] uuid::Error),
    XMLError(#[from] quick_xml::Error),
    IOError(#[from] std::io::Error),
    UTF8Error(#[from] std::str::Utf8Error),
    SerdeError(#[from] serde_json::Error),
    VirtError(#[from] virt::error::Error),
    SledError(#[from] sled::Error),
    DbError,
}
