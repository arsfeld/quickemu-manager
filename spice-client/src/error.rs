use thiserror::Error;

#[derive(Error, Debug)]
pub enum SpiceError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Protocol error: {0}")]
    Protocol(String),
    
    #[error("Version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u32, actual: u32 },
    
    #[error("Channel error: {0}")]
    Channel(String),
    
    #[error("Authentication failed")]
    AuthenticationFailed,
    
    #[error("Connection closed")]
    ConnectionClosed,
}

pub type Result<T> = std::result::Result<T, SpiceError>;