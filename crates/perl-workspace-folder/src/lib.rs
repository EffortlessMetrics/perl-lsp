//! Workspace folder URI/path parsing.
//!
//! This crate has one narrow responsibility: convert workspace folder entries into
//! local filesystem paths with deterministic behavior for both plain paths and
//! `file://` URIs.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
use perl_uri::uri_to_fs_path;

/// Parse a workspace folder declaration into a filesystem path.
///
/// Workspace folders can be passed as absolute paths or `file://` URIs. For
/// `file://` URIs this attempts to resolve through `perl_uri::uri_to_fs_path`.
/// If URI resolution fails, the scheme prefix is trimmed and the remainder is
/// interpreted as a path fallback.
#[must_use]
pub fn workspace_folder_to_path(workspace_folder: &str) -> PathBuf {
    if workspace_folder.starts_with("file://") {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = uri_to_fs_path(workspace_folder) {
            return path;
        }

        return PathBuf::from(workspace_folder.trim_start_matches("file://"));
    }

    PathBuf::from(workspace_folder)
}

#[cfg(test)]
mod tests {
    use super::workspace_folder_to_path;
    use std::path::PathBuf;

    #[test]
    fn parses_plain_folder_path() {
        assert_eq!(workspace_folder_to_path("/tmp/project"), PathBuf::from("/tmp/project"));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn parses_file_uri_when_possible() {
        let parsed = workspace_folder_to_path("file:///tmp/project");
        assert!(parsed.to_string_lossy().contains("tmp"));
        assert!(parsed.to_string_lossy().contains("project"));
    }
}
