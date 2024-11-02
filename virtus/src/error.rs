use thiserror::Error;

#[derive(Debug, Error)]
#[error("error")]
pub enum Error {
    #[error("Skiff error")]
    SkiffError(skiff::Error),
    #[error("IO error")]
    IOError(std::io::Error),
    #[error("Peer not found")]
    PeerNotFound,
    #[error("Failed to connect to peer")]
    PeerConnectFailed,
    #[error("No leader elected")]
    NoLeaderElected,
}

impl From<skiff::Error> for Error {
    fn from(err: skiff::Error) -> Self {
        Self::SkiffError(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::IOError(err)
    }
}
