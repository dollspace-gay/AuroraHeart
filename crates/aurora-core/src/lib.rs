//! Aurora Core - Core types, configuration, and utilities for AuroraHeart IDE
//!
//! This crate provides the foundational types and utilities used across the AuroraHeart
//! IDE, including:
//! - Configuration management
//! - Encrypted credential storage
//! - Shared types and error handling

pub mod config;
pub mod crypto;
pub mod types;

pub use config::{Config, ConfigError};
pub use crypto::{CredentialStore, CredentialStoreError};
pub use types::*;

/// Result type alias for core operations
pub type Result<T> = std::result::Result<T, anyhow::Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_imports() {
        // Verify that basic types are accessible
        let _result: Result<()> = Ok(());
    }
}
