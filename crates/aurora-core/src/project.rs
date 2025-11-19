//! Project detection and analysis utilities
//!
//! This module provides utilities for detecting project types, languages,
//! and finding project roots.

use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during project detection
#[derive(Error, Debug)]
pub enum ProjectError {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Project root not found
    #[error("Project root not found")]
    RootNotFound,

    /// Language could not be detected
    #[error("Could not detect project language")]
    LanguageUnknown,
}

/// Programming languages supported by AuroraHeart
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    Python,
    TypeScript,
    JavaScript,
    Go,
    Java,
    CSharp,
    Cpp,
    C,
}

impl Language {
    /// Get the language name as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Python => "python",
            Language::TypeScript => "typescript",
            Language::JavaScript => "javascript",
            Language::Go => "go",
            Language::Java => "java",
            Language::CSharp => "csharp",
            Language::Cpp => "cpp",
            Language::C => "c",
        }
    }

    /// Get the language from a string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "rust" => Some(Language::Rust),
            "python" | "py" => Some(Language::Python),
            "typescript" | "ts" => Some(Language::TypeScript),
            "javascript" | "js" => Some(Language::JavaScript),
            "go" | "golang" => Some(Language::Go),
            "java" => Some(Language::Java),
            "csharp" | "c#" | "cs" => Some(Language::CSharp),
            "cpp" | "c++" => Some(Language::Cpp),
            "c" => Some(Language::C),
            _ => None,
        }
    }

    /// Get common file extensions for this language
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            Language::Rust => &["rs"],
            Language::Python => &["py"],
            Language::TypeScript => &["ts", "tsx"],
            Language::JavaScript => &["js", "jsx", "mjs"],
            Language::Go => &["go"],
            Language::Java => &["java"],
            Language::CSharp => &["cs"],
            Language::Cpp => &["cpp", "cc", "cxx", "hpp", "h"],
            Language::C => &["c", "h"],
        }
    }
}

/// Project type indicators (marker files)
struct ProjectMarker {
    file_name: &'static str,
    language: Language,
}

const PROJECT_MARKERS: &[ProjectMarker] = &[
    ProjectMarker {
        file_name: "Cargo.toml",
        language: Language::Rust,
    },
    ProjectMarker {
        file_name: "requirements.txt",
        language: Language::Python,
    },
    ProjectMarker {
        file_name: "pyproject.toml",
        language: Language::Python,
    },
    ProjectMarker {
        file_name: "package.json",
        language: Language::JavaScript, // Could be TS or JS
    },
    ProjectMarker {
        file_name: "tsconfig.json",
        language: Language::TypeScript,
    },
    ProjectMarker {
        file_name: "go.mod",
        language: Language::Go,
    },
    ProjectMarker {
        file_name: "pom.xml",
        language: Language::Java,
    },
    ProjectMarker {
        file_name: "build.gradle",
        language: Language::Java,
    },
    ProjectMarker {
        file_name: ".csproj",
        language: Language::CSharp,
    },
    ProjectMarker {
        file_name: "CMakeLists.txt",
        language: Language::Cpp,
    },
];

/// Detect the primary language of a project
pub fn detect_language<P: AsRef<Path>>(project_root: P) -> Result<Language, ProjectError> {
    let project_root = project_root.as_ref();

    if !project_root.exists() || !project_root.is_dir() {
        return Err(ProjectError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Project root does not exist or is not a directory",
        )));
    }

    // Check for project marker files
    for marker in PROJECT_MARKERS {
        let marker_path = project_root.join(marker.file_name);
        if marker_path.exists() {
            // Special case for package.json: check if tsconfig.json also exists
            if marker.file_name == "package.json" {
                let tsconfig = project_root.join("tsconfig.json");
                if tsconfig.exists() {
                    return Ok(Language::TypeScript);
                }
            }
            return Ok(marker.language);
        }
    }

    // If no marker file found, try to detect from file extensions
    let entries = std::fs::read_dir(project_root)?;
    let mut extension_counts: std::collections::HashMap<Language, usize> =
        std::collections::HashMap::new();

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                for lang in &[
                    Language::Rust,
                    Language::Python,
                    Language::TypeScript,
                    Language::JavaScript,
                    Language::Go,
                    Language::Java,
                    Language::CSharp,
                    Language::Cpp,
                    Language::C,
                ] {
                    if lang.extensions().contains(&ext.to_lowercase().as_str()) {
                        *extension_counts.entry(*lang).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    // Return the language with the most files
    extension_counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(lang, _)| lang)
        .ok_or(ProjectError::LanguageUnknown)
}

/// Find the project root by searching for marker files
pub fn find_project_root<P: AsRef<Path>>(start_path: P) -> Result<PathBuf, ProjectError> {
    let mut current = start_path.as_ref().to_path_buf();

    loop {
        // Check for any project marker file
        for marker in PROJECT_MARKERS {
            let marker_path = current.join(marker.file_name);
            if marker_path.exists() {
                return Ok(current);
            }
        }

        // Check for .git directory
        let git_dir = current.join(".git");
        if git_dir.exists() && git_dir.is_dir() {
            return Ok(current);
        }

        // Move up to parent directory
        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            // Reached filesystem root without finding project root
            return Err(ProjectError::RootNotFound);
        }
    }
}

/// Get the project name from the directory name
pub fn get_project_name<P: AsRef<Path>>(project_root: P) -> Option<String> {
    project_root
        .as_ref()
        .file_name()
        .and_then(|name| name.to_str())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_language_as_str() {
        assert_eq!(Language::Rust.as_str(), "rust");
        assert_eq!(Language::Python.as_str(), "python");
        assert_eq!(Language::TypeScript.as_str(), "typescript");
    }

    #[test]
    fn test_language_from_str() {
        assert_eq!(Language::from_str("rust"), Some(Language::Rust));
        assert_eq!(Language::from_str("Rust"), Some(Language::Rust));
        assert_eq!(Language::from_str("python"), Some(Language::Python));
        assert_eq!(Language::from_str("py"), Some(Language::Python));
        assert_eq!(Language::from_str("typescript"), Some(Language::TypeScript));
        assert_eq!(Language::from_str("ts"), Some(Language::TypeScript));
        assert_eq!(Language::from_str("unknown"), None);
    }

    #[test]
    fn test_language_extensions() {
        assert!(Language::Rust.extensions().contains(&"rs"));
        assert!(Language::Python.extensions().contains(&"py"));
        assert!(Language::TypeScript.extensions().contains(&"ts"));
        assert!(Language::TypeScript.extensions().contains(&"tsx"));
    }

    #[test]
    fn test_detect_language_rust() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("Cargo.toml"), "[package]").unwrap();

        let language = detect_language(temp_dir.path()).unwrap();
        assert_eq!(language, Language::Rust);
    }

    #[test]
    fn test_detect_language_python() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("requirements.txt"), "requests").unwrap();

        let language = detect_language(temp_dir.path()).unwrap();
        assert_eq!(language, Language::Python);
    }

    #[test]
    fn test_detect_language_typescript() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("package.json"), "{}").unwrap();
        fs::write(temp_dir.path().join("tsconfig.json"), "{}").unwrap();

        let language = detect_language(temp_dir.path()).unwrap();
        assert_eq!(language, Language::TypeScript);
    }

    #[test]
    fn test_detect_language_javascript() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("package.json"), "{}").unwrap();

        let language = detect_language(temp_dir.path()).unwrap();
        assert_eq!(language, Language::JavaScript);
    }

    #[test]
    fn test_detect_language_from_extensions() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(temp_dir.path().join("lib.rs"), "// lib").unwrap();

        let language = detect_language(temp_dir.path()).unwrap();
        assert_eq!(language, Language::Rust);
    }

    #[test]
    fn test_find_project_root() {
        let temp_dir = TempDir::new().unwrap();
        let subdir = temp_dir.path().join("src").join("nested");
        fs::create_dir_all(&subdir).unwrap();
        fs::write(temp_dir.path().join("Cargo.toml"), "[package]").unwrap();

        let root = find_project_root(&subdir).unwrap();
        assert_eq!(root, temp_dir.path());
    }

    #[test]
    fn test_find_project_root_git() {
        let temp_dir = TempDir::new().unwrap();
        let subdir = temp_dir.path().join("src");
        fs::create_dir_all(&subdir).unwrap();
        fs::create_dir(temp_dir.path().join(".git")).unwrap();

        let root = find_project_root(&subdir).unwrap();
        assert_eq!(root, temp_dir.path());
    }

    #[test]
    fn test_find_project_root_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let subdir = temp_dir.path().join("src");
        fs::create_dir_all(&subdir).unwrap();

        let result = find_project_root(&subdir);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_project_name() {
        let path = PathBuf::from("/home/user/my-project");
        let name = get_project_name(&path).unwrap();
        assert_eq!(name, "my-project");
    }
}
