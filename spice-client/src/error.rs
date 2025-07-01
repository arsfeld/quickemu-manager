//! Error types for the SPICE client library.

use thiserror::Error;

/// Errors that can occur when using the SPICE client.
///
/// This enum represents all possible errors that can be returned by the SPICE client
/// library. It includes network errors, protocol errors, authentication failures, and
/// other SPICE-specific error conditions.
#[derive(Error, Debug)]
pub enum SpiceError {
    /// An I/O error occurred during network communication.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    /// A connection error occurred.
    /// 
    /// This typically happens when the client cannot establish a connection
    /// to the SPICE server or when the connection is unexpectedly dropped.
    #[error("Connection error: {0}")]
    Connection(String),
    
    /// A protocol error occurred.
    /// 
    /// This indicates that the server sent invalid or unexpected data that
    /// doesn't conform to the SPICE protocol specification.
    #[error("Protocol error: {0}")]
    Protocol(String),
    
    /// The server's SPICE protocol version is incompatible with this client.
    #[error("Version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { 
        /// The protocol version expected by the client.
        expected: u32, 
        /// The protocol version reported by the server.
        actual: u32 
    },
    
    /// An error occurred in a specific SPICE channel.
    /// 
    /// This can happen during channel initialization, message processing,
    /// or when a channel encounters an unexpected state.
    #[error("Channel error: {0}")]
    Channel(String),
    
    /// Authentication with the SPICE server failed.
    /// 
    /// This occurs when the provided password or ticket is incorrect,
    /// or when the authentication method is not supported.
    #[error("Authentication failed")]
    AuthenticationFailed,
    
    /// The connection to the SPICE server was closed.
    /// 
    /// This can happen normally during shutdown or unexpectedly if
    /// the server terminates the connection.
    #[error("Connection closed")]
    ConnectionClosed,
    
    /// Serialization/deserialization error from bincode.
    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl From<Box<bincode::ErrorKind>> for SpiceError {
    fn from(err: Box<bincode::ErrorKind>) -> Self {
        SpiceError::Serialization(err.to_string())
    }
}

/// A type alias for `Result<T, SpiceError>`.
/// 
/// This is the standard result type used throughout the SPICE client library.
/// All fallible operations return this type.
pub type Result<T> = std::result::Result<T, SpiceError>;