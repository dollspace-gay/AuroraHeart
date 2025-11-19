//! Shared types used across AuroraHeart

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents a file path in the project
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FilePath(PathBuf);

impl FilePath {
    /// Create a new FilePath from a PathBuf
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    /// Get the inner PathBuf
    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }

    /// Convert to a PathBuf
    pub fn into_path_buf(self) -> PathBuf {
        self.0
    }
}

impl From<PathBuf> for FilePath {
    fn from(path: PathBuf) -> Self {
        Self::new(path)
    }
}

impl AsRef<std::path::Path> for FilePath {
    fn as_ref(&self) -> &std::path::Path {
        &self.0
    }
}

/// Represents the content of a file
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileContent {
    /// The file path
    pub path: FilePath,
    /// The content as a UTF-8 string
    pub content: String,
}

impl FileContent {
    /// Create a new FileContent
    pub fn new(path: FilePath, content: String) -> Self {
        Self { path, content }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_path_creation() {
        let path = PathBuf::from("test/file.rs");
        let file_path = FilePath::new(path.clone());
        assert_eq!(file_path.as_path(), path.as_path());
    }

    #[test]
    fn test_file_path_conversion() {
        let path = PathBuf::from("test/file.rs");
        let file_path: FilePath = path.clone().into();
        assert_eq!(file_path.into_path_buf(), path);
    }

    #[test]
    fn test_file_content_creation() {
        let path = FilePath::new(PathBuf::from("test.rs"));
        let content = String::from("fn main() {}");
        let file_content = FileContent::new(path.clone(), content.clone());

        assert_eq!(file_content.path, path);
        assert_eq!(file_content.content, content);
    }

    #[test]
    fn test_file_path_serialization() {
        let path = FilePath::new(PathBuf::from("test/file.rs"));
        let serialized = serde_json::to_string(&path).unwrap();
        let deserialized: FilePath = serde_json::from_str(&serialized).unwrap();
        assert_eq!(path, deserialized);
    }
}
