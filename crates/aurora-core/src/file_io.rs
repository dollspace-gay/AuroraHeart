//! File I/O utilities for AuroraHeart
//!
//! This module provides safe, high-level file I/O operations with proper
//! error handling and encoding support.

use crate::types::{FileContent, FilePath};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during file I/O operations
#[derive(Error, Debug)]
pub enum FileIoError {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// File not found
    #[error("File not found: {0}")]
    NotFound(PathBuf),

    /// Invalid UTF-8 in file
    #[error("Invalid UTF-8 in file: {0}")]
    InvalidUtf8(PathBuf),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(PathBuf),

    /// File is a directory
    #[error("Path is a directory, not a file: {0}")]
    IsDirectory(PathBuf),
}

/// Read a file and return its contents as a UTF-8 string
pub fn read_file<P: AsRef<Path>>(path: P) -> Result<String, FileIoError> {
    let path = path.as_ref();

    if !path.exists() {
        return Err(FileIoError::NotFound(path.to_path_buf()));
    }

    if path.is_dir() {
        return Err(FileIoError::IsDirectory(path.to_path_buf()));
    }

    match fs::read_to_string(path) {
        Ok(content) => Ok(content),
        Err(e) => match e.kind() {
            io::ErrorKind::NotFound => Err(FileIoError::NotFound(path.to_path_buf())),
            io::ErrorKind::PermissionDenied => {
                Err(FileIoError::PermissionDenied(path.to_path_buf()))
            }
            io::ErrorKind::InvalidData => Err(FileIoError::InvalidUtf8(path.to_path_buf())),
            _ => Err(FileIoError::Io(e)),
        },
    }
}

/// Read a file and return it as FileContent
pub fn read_file_content<P: AsRef<Path>>(path: P) -> Result<FileContent, FileIoError> {
    let path = path.as_ref();
    let content = read_file(path)?;
    Ok(FileContent::new(
        FilePath::new(path.to_path_buf()),
        content,
    ))
}

/// Write content to a file, creating parent directories if needed
pub fn write_file<P: AsRef<Path>, C: AsRef<str>>(
    path: P,
    content: C,
) -> Result<(), FileIoError> {
    let path = path.as_ref();

    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    match fs::write(path, content.as_ref()) {
        Ok(()) => Ok(()),
        Err(e) => match e.kind() {
            io::ErrorKind::PermissionDenied => {
                Err(FileIoError::PermissionDenied(path.to_path_buf()))
            }
            _ => Err(FileIoError::Io(e)),
        },
    }
}

/// Write FileContent to disk
pub fn write_file_content(file_content: &FileContent) -> Result<(), FileIoError> {
    write_file(&file_content.path, &file_content.content)
}

/// Check if a file exists
pub fn file_exists<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref();
    path.exists() && path.is_file()
}

/// Check if a directory exists
pub fn dir_exists<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref();
    path.exists() && path.is_dir()
}

/// List all files in a directory (non-recursive)
pub fn list_files<P: AsRef<Path>>(dir: P) -> Result<Vec<PathBuf>, FileIoError> {
    let dir = dir.as_ref();

    if !dir.exists() {
        return Err(FileIoError::NotFound(dir.to_path_buf()));
    }

    if !dir.is_dir() {
        return Err(FileIoError::IsDirectory(dir.to_path_buf()));
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            files.push(path);
        }
    }

    Ok(files)
}

/// List all files in a directory recursively
pub fn list_files_recursive<P: AsRef<Path>>(dir: P) -> Result<Vec<PathBuf>, FileIoError> {
    let dir = dir.as_ref();

    if !dir.exists() {
        return Err(FileIoError::NotFound(dir.to_path_buf()));
    }

    if !dir.is_dir() {
        return Err(FileIoError::IsDirectory(dir.to_path_buf()));
    }

    let mut files = Vec::new();
    visit_dirs(dir, &mut files)?;
    Ok(files)
}

fn visit_dirs(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), FileIoError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            files.push(path);
        } else if path.is_dir() {
            visit_dirs(&path, files)?;
        }
    }
    Ok(())
}

/// Get the file extension as a lowercase string
pub fn get_extension<P: AsRef<Path>>(path: P) -> Option<String> {
    path.as_ref()
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_read_write_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let content = "Hello, AuroraHeart!";
        write_file(&file_path, content).unwrap();

        let read_content = read_file(&file_path).unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_read_nonexistent_file() {
        let result = read_file("nonexistent_file_12345.txt");
        assert!(result.is_err());
        assert!(matches!(result, Err(FileIoError::NotFound(_))));
    }

    #[test]
    fn test_write_creates_parent_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("subdir").join("nested").join("test.txt");

        write_file(&file_path, "test content").unwrap();
        assert!(file_path.exists());

        let content = read_file(&file_path).unwrap();
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_file_content_read_write() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let original = FileContent::new(
            FilePath::new(file_path.clone()),
            "Test content".to_string(),
        );

        write_file_content(&original).unwrap();
        let loaded = read_file_content(&file_path).unwrap();

        assert_eq!(loaded.content, original.content);
    }

    #[test]
    fn test_file_exists() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        assert!(!file_exists(&file_path));
        write_file(&file_path, "test").unwrap();
        assert!(file_exists(&file_path));
    }

    #[test]
    fn test_dir_exists() {
        let temp_dir = TempDir::new().unwrap();
        assert!(dir_exists(temp_dir.path()));

        let file_path = temp_dir.path().join("test.txt");
        write_file(&file_path, "test").unwrap();
        assert!(!dir_exists(&file_path)); // File, not directory
    }

    #[test]
    fn test_list_files() {
        let temp_dir = TempDir::new().unwrap();

        write_file(temp_dir.path().join("file1.txt"), "content1").unwrap();
        write_file(temp_dir.path().join("file2.txt"), "content2").unwrap();
        fs::create_dir(temp_dir.path().join("subdir")).unwrap();

        let files = list_files(temp_dir.path()).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_list_files_recursive() {
        let temp_dir = TempDir::new().unwrap();

        write_file(temp_dir.path().join("file1.txt"), "content1").unwrap();
        write_file(
            temp_dir.path().join("subdir").join("file2.txt"),
            "content2",
        )
        .unwrap();
        write_file(
            temp_dir.path().join("subdir").join("nested").join("file3.txt"),
            "content3",
        )
        .unwrap();

        let files = list_files_recursive(temp_dir.path()).unwrap();
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_get_extension() {
        assert_eq!(get_extension("test.txt"), Some("txt".to_string()));
        assert_eq!(get_extension("test.RS"), Some("rs".to_string()));
        assert_eq!(get_extension("Cargo.toml"), Some("toml".to_string()));
        assert_eq!(get_extension("no_extension"), None);
        assert_eq!(get_extension(".gitignore"), None);
    }
}
