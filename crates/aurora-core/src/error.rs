//! Error types for Aurora Core
//!
//! This module provides a unified error type for core operations.

use thiserror::Error;

pub use crate::config::ConfigError;
pub use crate::crypto::CredentialStoreError;
pub use crate::file_io::FileIoError;
pub use crate::project::ProjectError;

/// Top-level error type for Aurora Core operations
#[derive(Error, Debug)]
pub enum AuroraCoreError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    /// Credential store error
    #[error("Credential store error: {0}")]
    CredentialStore(#[from] CredentialStoreError),

    /// File I/O error
    #[error("File I/O error: {0}")]
    FileIo(#[from] FileIoError),

    /// Project detection error
    #[error("Project error: {0}")]
    Project(#[from] ProjectError),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

impl From<std::io::Error> for AuroraCoreError {
    fn from(err: std::io::Error) -> Self {
        AuroraCoreError::FileIo(FileIoError::from(err))
    }
}

impl From<String> for AuroraCoreError {
    fn from(msg: String) -> Self {
        AuroraCoreError::Other(msg)
    }
}

impl From<&str> for AuroraCoreError {
    fn from(msg: &str) -> Self {
        AuroraCoreError::Other(msg.to_string())
    }
}

/// Result type for Aurora Core operations
pub type Result<T> = std::result::Result<T, AuroraCoreError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion() {
        let _error: AuroraCoreError = "test error".into();
        let _result: Result<()> = Err("test error".into());
    }

    #[test]
    fn test_error_display() {
        let error = AuroraCoreError::Other("test message".to_string());
        assert_eq!(error.to_string(), "test message");
    }
}
