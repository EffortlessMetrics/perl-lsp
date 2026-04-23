//! Workspace folder URI/path parsing.
//!
//! Converts workspace folder entries into local filesystem paths with
//! deterministic behavior for both plain paths and `file://` URIs.

use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
use perl_uri::uri_to_fs_path;
use serde_json::Value;

/// URI lists extracted from an LSP workspace folder change event.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkspaceFolderChange {
    /// Added workspace folder URIs.
    pub added: Vec<String>,
    /// Removed workspace folder URIs.
    pub removed: Vec<String>,
}

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

/// Extract workspace folder URIs from an LSP `workspaceFolders` array.
///
/// Invalid entries are ignored.
#[must_use]
pub fn extract_workspace_folder_uris(workspace_folders: &[Value]) -> Vec<String> {
    workspace_folders
        .iter()
        .filter_map(|folder| {
            folder.get("uri").and_then(Value::as_str).map(std::string::ToString::to_string)
        })
        .collect()
}

/// Extract URI changes from an LSP `workspace/didChangeWorkspaceFolders` event payload.
///
/// Missing/invalid sections are treated as empty.
#[must_use]
pub fn extract_workspace_folder_change(event: &Value) -> WorkspaceFolderChange {
    let added = event
        .get("added")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |entries| extract_workspace_folder_uris(entries));

    let removed = event
        .get("removed")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |entries| extract_workspace_folder_uris(entries));

    WorkspaceFolderChange { added, removed }
}

/// Convert a legacy LSP `rootPath` string to a `file://` URI.
///
/// This keeps behavior deterministic across absolute POSIX and Windows-style paths.
#[must_use]
pub fn root_path_to_file_uri(root_path: &str) -> String {
    if root_path.starts_with("file://") {
        return root_path.to_string();
    }

    let path = std::path::Path::new(root_path);
    url::Url::from_file_path(path).map_or_else(
        |_| {
            if root_path.starts_with('/') {
                format!("file://{}", root_path)
            } else {
                format!("file:///{}", root_path.replace('\\', "/"))
            }
        },
        |uri| uri.to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        extract_workspace_folder_change, extract_workspace_folder_uris, root_path_to_file_uri,
        workspace_folder_to_path,
    };
    use serde_json::json;
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

    #[test]
    fn extracts_workspace_uris() {
        let entries = vec![
            json!({"uri": "file:///one"}),
            json!({"uri": "file:///two"}),
            json!({"name": "invalid"}),
        ];
        let uris = extract_workspace_folder_uris(&entries);
        assert_eq!(uris, vec!["file:///one", "file:///two"]);
    }

    #[test]
    fn extracts_workspace_change_entries() {
        let change = extract_workspace_folder_change(&json!({
            "added": [{"uri": "file:///add"}],
            "removed": [{"uri": "file:///remove"}],
        }));

        assert_eq!(change.added, vec!["file:///add"]);
        assert_eq!(change.removed, vec!["file:///remove"]);
    }

    #[test]
    fn converts_legacy_root_path_to_file_uri() {
        let uri = root_path_to_file_uri("/legacy/workspace");
        assert_eq!(uri, "file:///legacy/workspace");
    }

    #[test]
    fn preserves_file_uri_root_path_input() {
        let uri = root_path_to_file_uri("file:///already/uri");
        assert_eq!(uri, "file:///already/uri");
    }
}
