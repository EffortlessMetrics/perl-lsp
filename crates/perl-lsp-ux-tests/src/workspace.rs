//! Temporary workspace helpers for UX scenario tests.
//!
//! `FakeWorkspace` creates an isolated temporary directory with Perl project
//! structure and provides helpers for writing source files and producing
//! `file://` URIs suitable for the LSP protocol.

use anyhow::{Context, Result};
use std::fs;
use tempfile::TempDir;
use url::Url;

/// An isolated temporary directory that acts as a Perl project root.
pub struct FakeWorkspace {
    /// The underlying temporary directory (auto-deleted on drop).
    pub dir: TempDir,
    /// The `file://` URI of the workspace root, suitable for `rootUri` in
    /// LSP `initialize` requests.
    pub root_uri: String,
}

impl FakeWorkspace {
    /// Create a new empty workspace in a fresh temporary directory.
    pub fn new() -> Result<Self> {
        let dir = TempDir::new().context("Failed to create temporary workspace directory")?;
        let root_uri = Url::from_directory_path(dir.path())
            .map_err(|_| anyhow::anyhow!("Failed to produce file:// URI for workspace root"))?
            .to_string();
        Ok(Self { dir, root_uri })
    }

    /// Write a file at `relative_path` (e.g., `"lib/Foo.pm"`) with `content`.
    /// Creates intermediate directories automatically.
    pub fn write(&self, relative_path: &str, content: &str) -> Result<()> {
        let path = self.dir.path().join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create dirs for {:?}", parent))?;
        }
        fs::write(&path, content)
            .with_context(|| format!("Failed to write workspace file {:?}", path))?;
        Ok(())
    }

    /// Ensure a directory exists at `relative_path`.
    pub fn ensure_dir(&self, relative_path: &str) -> Result<()> {
        let path = self.dir.path().join(relative_path);
        fs::create_dir_all(&path)
            .with_context(|| format!("Failed to create workspace directory {:?}", path))?;
        Ok(())
    }

    /// Delete a file at `relative_path` if it exists.
    pub fn delete(&self, relative_path: &str) -> Result<()> {
        let path = self.dir.path().join(relative_path);
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("Failed to delete workspace file {:?}", path))?;
        }
        Ok(())
    }

    /// Get the full `file://` URI for a relative path.
    pub fn uri(&self, relative_path: &str) -> String {
        let path = self.dir.path().join(relative_path);
        match Url::from_file_path(&path) {
            Ok(url) => url.to_string(),
            Err(_) => {
                // Fallback: manually construct the URI (handles edge cases on Windows).
                format!("{}/{}", self.root_uri.trim_end_matches('/'), relative_path)
            }
        }
    }

    /// Get the full `file://` URI for a relative directory path.
    pub fn dir_uri(&self, relative_path: &str) -> Result<String> {
        let path = self.dir.path().join(relative_path);
        Url::from_directory_path(&path)
            .map(|url| url.to_string())
            .map_err(|_| anyhow::anyhow!("Failed to produce file:// URI for directory {:?}", path))
    }

    /// Get the filesystem path for a relative path.
    pub fn path(&self, relative_path: &str) -> std::path::PathBuf {
        self.dir.path().join(relative_path)
    }

    /// Write a minimal `cpanfile` to the workspace root.
    /// Useful for scenarios that test multi-file workspace detection.
    pub fn write_cpanfile(&self, requires: &[(&str, &str)]) -> Result<()> {
        let mut content = String::new();
        for (module, version) in requires {
            content.push_str(&format!("requires '{}', '{}';\n", module, version));
        }
        self.write("cpanfile", &content)
    }

    /// Seed a minimal Perl project with a single script and lib directory.
    pub fn seed_minimal_project(&self) -> Result<()> {
        self.write(
            "script.pl",
            "#!/usr/bin/env perl\nuse strict;\nuse warnings;\nprint \"hello\\n\";\n",
        )?;
        self.write(
            "lib/MyApp.pm",
            "package MyApp;\nuse strict;\nuse warnings;\nsub new { bless {}, shift }\n1;\n",
        )?;
        Ok(())
    }
}
