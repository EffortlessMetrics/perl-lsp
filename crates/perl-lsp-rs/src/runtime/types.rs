use super::workspace_folder::WorkspaceFolderState;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ServerRequestId(i32);

impl ServerRequestId {
    pub(crate) fn new(value: i32) -> Option<Self> {
        (value >= 1).then_some(Self(value))
    }

    pub(crate) fn as_i32(self) -> i32 {
        self.0
    }

    /// Build a request id for unit tests that use fixed positive constants.
    #[cfg(test)]
    pub(crate) const fn for_test(value: i32) -> Self {
        Self(value)
    }
}

pub(super) fn source_path_from_uri(uri: &str) -> Option<PathBuf> {
    perl_uri::source_path_from_uri_or_path(uri)
}

pub(super) fn workspace_folder_path(folder: &WorkspaceFolderState) -> Option<PathBuf> {
    folder.path.clone().or_else(|| source_path_from_uri(&folder.uri))
}

pub(super) fn workspace_folder_matches_doc_uri(
    folder: &WorkspaceFolderState,
    doc_uri: &str,
) -> bool {
    let doc_path = source_path_from_uri(doc_uri);
    match (doc_path, workspace_folder_path(folder)) {
        (Some(doc_path), Some(folder_path)) => doc_path.starts_with(folder_path),
        _ => {
            let folder_uri = folder.uri.trim_end_matches('/');
            doc_uri == folder.uri
                || doc_uri == folder_uri
                || doc_uri.strip_prefix(folder_uri).is_some_and(|suffix| suffix.starts_with('/'))
        }
    }
}

/// Pick the most-specific (deepest) workspace folder that contains `doc_uri`.
///
/// When workspace folders are nested (e.g. `/repo` and `/repo/app`), a document
/// inside the inner folder matches both. Returning the deepest folder makes
/// folder, config, and include-path lookups consistent with
/// `workspace_root_for_doc` in `runtime/lifecycle/module_resolution.rs`, which
/// already picks the deepest match. Mixing first-match with deepest-match
/// causes the runtime root and the per-doc config to disagree on which
/// folder owns the document.
///
/// Rank by component count of the workspace folder's local path. For
/// non-`file://` URIs (vscode-remote, etc.) the path is unavailable, so fall
/// back to URI slash count, which preserves nesting order across hosts.
pub(super) fn best_workspace_folder_for_doc<'a>(
    folders: &'a [WorkspaceFolderState],
    doc_uri: &str,
) -> Option<&'a WorkspaceFolderState> {
    folders.iter().filter(|folder| workspace_folder_matches_doc_uri(folder, doc_uri)).max_by_key(
        |folder| {
            workspace_folder_path(folder)
                .map(|path| path.components().count())
                .unwrap_or_else(|| folder.uri.trim_end_matches('/').matches('/').count())
        },
    )
}

/// Tracks metadata for a pending `workspace/configuration` reverse request.
#[derive(Debug, Clone)]
pub(crate) struct PendingWorkspaceConfigurationRequest {
    /// Workspace folder URIs requested in this call.
    pub(crate) folder_uris: Vec<String>,
    /// Whether the response includes an unscoped global `perl` settings item first.
    pub(crate) includes_global_item: bool,
    /// Request creation time used for stale-request cleanup.
    pub(crate) created_at: Instant,
}

/// Lightweight view of a document for scan-heavy operations
///
/// This struct provides the minimal data needed for workspace-wide scans
/// (code lens resolve, reference counting) without requiring the full
/// DocumentState. Using this snapshot pattern allows the documents lock
/// to be released before CPU-intensive work begins.
///
/// ## Design Rationale
/// - `uri`: Needed to construct LSP Location responses
/// - `text`: Needed for text-based fallback searches (regex, line iteration)
/// - `ast`: Arc clone allows AST traversal without deep copying the tree
///
/// The rope, line_starts cache, parent_map, and other fields are omitted
/// as they're not typically needed for bulk scan operations.
pub(crate) struct DocumentScanView {
    /// Document URI for constructing Location responses
    #[allow(dead_code)] // Preserved for future scan operations that build Location responses
    pub uri: String,
    /// Document text content for text-based searches
    pub text: String,
    /// Optional AST reference (Arc clone) for AST-based operations
    pub ast: Option<std::sync::Arc<perl_parser::ast::Node>>,
}
