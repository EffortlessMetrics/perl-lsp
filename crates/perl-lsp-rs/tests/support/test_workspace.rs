//! Temporary workspace helpers for LSP integration tests.
//!
//! Provides a `TempWorkspace` that creates a temporary directory on disk,
//! writes Perl source files into it, and produces `file://` URIs suitable
//! for the LSP protocol.

#![allow(dead_code)]

use perl_tdd_support::must;
use std::fs;
use tempfile::TempDir;
use url::Url;

/// Temporary workspace for testing with real files
pub struct TempWorkspace {
    pub dir: TempDir,
    pub root_uri: String,
}

impl TempWorkspace {
    /// Create a new temporary workspace
    pub fn new() -> Result<Self, String> {
        let dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {}", e))?;
        let root_uri = Url::from_directory_path(dir.path())
            .map_err(|_| "Failed to create file URL")?
            .to_string();
        Ok(Self { dir, root_uri })
    }

    /// Write a file to the workspace
    pub fn write(&self, relative_path: &str, content: &str) -> Result<(), String> {
        let path = self.dir.path().join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create dirs: {}", e))?;
        }
        fs::write(&path, content).map_err(|e| format!("Failed to write file: {}", e))?;
        Ok(())
    }

    /// Get the full URI for a relative path
    pub fn uri(&self, relative_path: &str) -> String {
        let path = self.dir.path().join(relative_path);
        match Url::from_file_path(&path) {
            Ok(url) => url.to_string(),
            Err(_) => must(Url::from_file_path(&path)).to_string(),
        }
    }
}
