//! Encrypted credential storage for AuroraHeart
//!
//! This module provides secure storage for API keys and other sensitive credentials
//! using AES-GCM encryption with a key derived from a user-provided password or
//! system-generated key.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use ring::pbkdf2;
use serde::{Deserialize, Serialize};
use std::{
    num::NonZeroU32,
    path::{Path, PathBuf},
};
use thiserror::Error;

/// Errors that can occur during credential storage operations
#[derive(Error, Debug)]
pub enum CredentialStoreError {
    /// IO error while reading or writing credentials
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON parsing error
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// Encryption error
    #[error("Encryption error")]
    Encryption,

    /// Decryption error
    #[error("Decryption error")]
    Decryption,

    /// Invalid credential format
    #[error("Invalid credential format")]
    InvalidFormat,

    /// Credential not found
    #[error("Credential not found: {0}")]
    NotFound(String),
}

/// Number of PBKDF2 iterations
const PBKDF2_ITERATIONS: u32 = 100_000;

/// Salt length in bytes
const SALT_LENGTH: usize = 32;

/// Nonce length in bytes for AES-GCM
const NONCE_LENGTH: usize = 12;

/// Encrypted credential data stored on disk
#[derive(Debug, Serialize, Deserialize)]
struct EncryptedCredential {
    /// Base64-encoded salt used for key derivation
    salt: String,
    /// Base64-encoded nonce used for encryption
    nonce: String,
    /// Base64-encoded encrypted data
    ciphertext: String,
}

/// Secure credential storage
pub struct CredentialStore {
    /// Path to the credentials file
    credentials_path: PathBuf,
}

impl CredentialStore {
    /// Create a new credential store at the specified path
    pub fn new<P: AsRef<Path>>(credentials_path: P) -> Self {
        Self {
            credentials_path: credentials_path.as_ref().to_path_buf(),
        }
    }

    /// Create a credential store in the project's .AuroraHeart directory
    pub fn for_project<P: AsRef<Path>>(project_root: P) -> Self {
        let credentials_path = project_root
            .as_ref()
            .join(".AuroraHeart")
            .join("credentials.enc");
        Self::new(credentials_path)
    }

    /// Store an encrypted credential
    pub fn store(&self, key: &str, value: &str, password: &str) -> Result<(), CredentialStoreError> {
        // Generate random salt
        let mut salt = [0u8; SALT_LENGTH];
        ring::rand::SecureRandom::fill(&ring::rand::SystemRandom::new(), &mut salt)
            .map_err(|_| CredentialStoreError::Encryption)?;

        // Derive encryption key from password using PBKDF2
        let mut derived_key = [0u8; 32];
        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA256,
            NonZeroU32::new(PBKDF2_ITERATIONS).unwrap(),
            &salt,
            password.as_bytes(),
            &mut derived_key,
        );

        // Create cipher
        let key_obj = Key::<Aes256Gcm>::from_slice(&derived_key);
        let cipher = Aes256Gcm::new(key_obj);

        // Generate random nonce
        let mut nonce_bytes = [0u8; NONCE_LENGTH];
        ring::rand::SecureRandom::fill(&ring::rand::SystemRandom::new(), &mut nonce_bytes)
            .map_err(|_| CredentialStoreError::Encryption)?;
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Prepare data to encrypt
        let data = serde_json::json!({
            "key": key,
            "value": value,
        });
        let plaintext = serde_json::to_vec(&data)?;

        // Encrypt
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_ref())
            .map_err(|_| CredentialStoreError::Encryption)?;

        // Create encrypted credential
        let encrypted = EncryptedCredential {
            salt: BASE64.encode(salt),
            nonce: BASE64.encode(nonce_bytes),
            ciphertext: BASE64.encode(ciphertext),
        };

        // Ensure directory exists
        if let Some(parent) = self.credentials_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Write to file
        let json = serde_json::to_string_pretty(&encrypted)?;
        std::fs::write(&self.credentials_path, json)?;

        Ok(())
    }

    /// Retrieve and decrypt a credential
    pub fn retrieve(&self, key: &str, password: &str) -> Result<String, CredentialStoreError> {
        // Read encrypted data
        let json = std::fs::read_to_string(&self.credentials_path)?;
        let encrypted: EncryptedCredential = serde_json::from_str(&json)?;

        // Decode salt and nonce
        let salt = BASE64
            .decode(&encrypted.salt)
            .map_err(|_| CredentialStoreError::InvalidFormat)?;
        let nonce_bytes = BASE64
            .decode(&encrypted.nonce)
            .map_err(|_| CredentialStoreError::InvalidFormat)?;
        let ciphertext = BASE64
            .decode(&encrypted.ciphertext)
            .map_err(|_| CredentialStoreError::InvalidFormat)?;

        // Derive decryption key
        let mut derived_key = [0u8; 32];
        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA256,
            NonZeroU32::new(PBKDF2_ITERATIONS).unwrap(),
            &salt,
            password.as_bytes(),
            &mut derived_key,
        );

        // Create cipher
        let key_obj = Key::<Aes256Gcm>::from_slice(&derived_key);
        let cipher = Aes256Gcm::new(key_obj);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Decrypt
        let plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|_| CredentialStoreError::Decryption)?;

        // Parse decrypted data
        let data: serde_json::Value = serde_json::from_slice(&plaintext)?;
        let stored_key = data["key"]
            .as_str()
            .ok_or(CredentialStoreError::InvalidFormat)?;

        if stored_key != key {
            return Err(CredentialStoreError::NotFound(key.to_string()));
        }

        let value = data["value"]
            .as_str()
            .ok_or(CredentialStoreError::InvalidFormat)?
            .to_string();

        Ok(value)
    }

    /// Check if credentials file exists
    pub fn exists(&self) -> bool {
        self.credentials_path.exists()
    }

    /// Delete the credentials file
    pub fn delete(&self) -> Result<(), CredentialStoreError> {
        if self.exists() {
            std::fs::remove_file(&self.credentials_path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_store_and_retrieve_credential() {
        let temp_dir = TempDir::new().unwrap();
        let cred_path = temp_dir.path().join("test_creds.enc");
        let store = CredentialStore::new(&cred_path);

        let key = "api_key";
        let value = "sk-test-1234567890";
        let password = "my_secure_password";

        // Store credential
        store.store(key, value, password).unwrap();
        assert!(store.exists());

        // Retrieve credential
        let retrieved = store.retrieve(key, password).unwrap();
        assert_eq!(retrieved, value);
    }

    #[test]
    fn test_wrong_password() {
        let temp_dir = TempDir::new().unwrap();
        let cred_path = temp_dir.path().join("test_creds.enc");
        let store = CredentialStore::new(&cred_path);

        let key = "api_key";
        let value = "sk-test-1234567890";
        let password = "correct_password";
        let wrong_password = "wrong_password";

        // Store credential
        store.store(key, value, password).unwrap();

        // Try to retrieve with wrong password
        let result = store.retrieve(key, wrong_password);
        assert!(result.is_err());
        assert!(matches!(result, Err(CredentialStoreError::Decryption)));
    }

    #[test]
    fn test_credential_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let cred_path = temp_dir.path().join("test_creds.enc");
        let store = CredentialStore::new(&cred_path);

        let password = "password";

        // Store one credential
        store.store("key1", "value1", password).unwrap();

        // Try to retrieve different key
        let result = store.retrieve("key2", password);
        assert!(result.is_err());
        assert!(matches!(result, Err(CredentialStoreError::NotFound(_))));
    }

    #[test]
    fn test_delete_credential() {
        let temp_dir = TempDir::new().unwrap();
        let cred_path = temp_dir.path().join("test_creds.enc");
        let store = CredentialStore::new(&cred_path);

        store.store("key", "value", "password").unwrap();
        assert!(store.exists());

        store.delete().unwrap();
        assert!(!store.exists());
    }

    #[test]
    fn test_for_project() {
        let temp_dir = TempDir::new().unwrap();
        let store = CredentialStore::for_project(temp_dir.path());

        let expected_path = temp_dir.path().join(".AuroraHeart").join("credentials.enc");
        assert_eq!(store.credentials_path, expected_path);
    }

    #[test]
    fn test_encryption_produces_different_ciphertexts() {
        let temp_dir = TempDir::new().unwrap();
        let store1 = CredentialStore::new(temp_dir.path().join("cred1.enc"));
        let store2 = CredentialStore::new(temp_dir.path().join("cred2.enc"));

        let key = "api_key";
        let value = "same_value";
        let password = "same_password";

        // Store same data twice
        store1.store(key, value, password).unwrap();
        store2.store(key, value, password).unwrap();

        // Read raw files - they should be different due to random salt and nonce
        let file1 = std::fs::read_to_string(&store1.credentials_path).unwrap();
        let file2 = std::fs::read_to_string(&store2.credentials_path).unwrap();
        assert_ne!(file1, file2);

        // But both should decrypt to same value
        let retrieved1 = store1.retrieve(key, password).unwrap();
        let retrieved2 = store2.retrieve(key, password).unwrap();
        assert_eq!(retrieved1, value);
        assert_eq!(retrieved2, value);
    }
}
