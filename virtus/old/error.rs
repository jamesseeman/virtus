use thiserror::Error;

#[derive(Debug, Error)]
#[error("error occured")]
pub enum Error {
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
    #[error("VM of that name already exists")]
    VMExists,
    #[error("VM has not been defined")]
    VMUndefined,
    #[error("VM is being shut down")]
    VMShuttingDown,
    #[error("Physical network already exists")]
    PhysicalNetworkExists,
    #[error("Interface not found")]
    InterfaceNotFound,
}
