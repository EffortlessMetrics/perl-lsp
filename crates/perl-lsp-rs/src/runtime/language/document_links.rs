//! Document link handlers.
//!
//! Keeps document-link feature logic isolated from other language handlers.

use super::super::*;
use crate::protocol::req_uri;
use std::borrow::Cow;
use std::path::Path;

fn is_windows_drive_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'/'
}

fn resolve_file_link_target(base_uri: &str, file_path: &str) -> Option<String> {
    if file_path.starts_with("//") {
        return url::Url::parse(&format!("file:{file_path}")).ok().map(|url| url.to_string());
    }

    if is_windows_drive_path(file_path) {
        return url::Url::parse(&format!("file:///{file_path}")).ok().map(|url| url.to_string());
    }

    if Path::new(file_path).is_absolute() {
        if let Ok(target_url) = url::Url::from_file_path(file_path) {
            return Some(target_url.to_string());
        }
    }

    let base_url = url::Url::parse(base_uri).ok()?;
    if let Ok(target_url) = base_url.join(file_path) {
        return Some(target_url.to_string());
    }

    if let Ok(base_path) = base_url.to_file_path()
        && let Some(parent) = base_path.parent()
    {
        let resolved = parent.join(file_path);
        if let Ok(target_url) = url::Url::from_file_path(&resolved) {
            return Some(target_url.to_string());
        }
    }

    None
}

fn normalize_document_link_file_path(file_path: &str) -> Cow<'_, str> {
    if !file_path.contains(['\\', '/']) {
        return Cow::Borrowed(file_path);
    }

    let preserve_unc_prefix = file_path.starts_with("\\\\") || file_path.starts_with("//");
    let mut normalized = String::with_capacity(file_path.len());
    let mut chars = file_path.chars().peekable();

    if preserve_unc_prefix {
        normalized.push_str("//");
        while matches!(chars.peek(), Some('\\' | '/')) {
            chars.next();
        }
    }

    let mut saw_separator = false;

    for ch in chars {
        let is_separator = ch == '\\' || ch == '/';
        if is_separator {
            if !saw_separator {
                normalized.push('/');
            }
        } else {
            normalized.push(ch);
        }
        saw_separator = is_separator;
    }

    Cow::Owned(normalized)
}

impl LspServer {
    /// Handle textDocument/documentLink request
    pub(crate) fn handle_document_links(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(p) = params {
            let uri = p["textDocument"]["uri"].as_str().ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing textDocument.uri".into(),
                data: None,
            })?;
            // Snapshot document text under lock, then release before acquiring
            // workspace_folders via workspace_roots() to prevent ABBA deadlock.
            // Path A (here): documents -> workspace_folders
            // Path B (workspace.rs): workspace_folders -> documents
            let doc_text = {
                let documents = self.documents_guard();
                let doc = self.get_document(&documents, uri).ok_or_else(|| JsonRpcError {
                    code: INVALID_REQUEST,
                    message: format!("Document not open: {}", uri),
                    data: None,
                })?;
                doc.text.clone()
            };
            // documents lock released here

            let roots = self.workspace_roots();
            let links = crate::document_links::compute_links(uri, &doc_text, &roots);
            Ok(Some(json!(links)))
        } else {
            Ok(Some(json!([])))
        }
    }

    /// Handle documentLink/resolve request
    ///
    /// Resolves a document link by filling in the target URI based on the data field.
    /// This allows the initial documentLink response to defer expensive resolution
    /// until the user actually hovers over or clicks the link.
    pub(crate) fn handle_document_link_resolve(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(mut link) = params {
            // Extract data field to determine link type
            let data = link.get("data").cloned();

            // If link already has a target, return as-is (already resolved)
            if link.get("target").and_then(|t| t.as_str()).is_some() {
                return Ok(Some(link));
            }

            // Resolve based on data field
            if let Some(data_obj) = data {
                let link_type = data_obj.get("type").and_then(|t| t.as_str());

                match link_type {
                    Some("module") => {
                        // Module reference - resolve to file path or MetaCPAN
                        let module_name = data_obj
                            .get("module")
                            .and_then(|m| m.as_str())
                            .ok_or_else(|| JsonRpcError {
                                code: INVALID_PARAMS,
                                message: "Missing module name in data".into(),
                                data: None,
                            })?;

                        // Try to resolve module to local file
                        if let Some(target) = self.resolve_module_to_path(module_name) {
                            link["target"] = json!(target);
                        } else {
                            // Fallback to MetaCPAN
                            let target = format!("https://metacpan.org/pod/{}", module_name);
                            link["target"] = json!(target);
                        }
                    }
                    Some("file") => {
                        // File reference - resolve to absolute path
                        let file_path =
                            data_obj.get("path").and_then(|p| p.as_str()).ok_or_else(|| {
                                JsonRpcError {
                                    code: INVALID_PARAMS,
                                    message: "Missing file path in data".into(),
                                    data: None,
                                }
                            })?;
                        let normalized_file_path = normalize_document_link_file_path(file_path);

                        let base_uri = data_obj
                            .get("baseUri")
                            .and_then(|u| u.as_str())
                            .ok_or_else(|| JsonRpcError {
                                code: INVALID_PARAMS,
                                message: "Missing base URI in data".into(),
                                data: None,
                            })?;

                        if let Some(target) =
                            resolve_file_link_target(base_uri, normalized_file_path.as_ref())
                        {
                            link["target"] = json!(target);
                        }
                    }
                    Some("url") => {
                        // URL reference - already resolved, just copy from data
                        if let Some(url) = data_obj.get("url").and_then(|u| u.as_str()) {
                            link["target"] = json!(url);
                        }
                    }
                    _ => {
                        // Unknown link type - return error
                        return Err(JsonRpcError {
                            code: INVALID_PARAMS,
                            message: "Unknown link type in data field".into(),
                            data: Some(json!({"linkType": link_type})),
                        });
                    }
                }
            }

            Ok(Some(link))
        } else {
            Err(JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing parameters for documentLink/resolve".into(),
                data: None,
            })
        }
    }

    /// Handle documentLink request (alternative)
    #[allow(dead_code)] // Alternative implementation
    pub(crate) fn handle_document_link(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = req_uri(&params)?;

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                let uri_parsed = url::Url::parse(uri).map_err(|_| JsonRpcError {
                    code: -32602,
                    message: "Invalid URI".to_string(),
                    data: None,
                })?;
                match crate::lsp_document_link::collect_document_links(&doc.text, &uri_parsed) {
                    Ok(links) => Ok(Some(serde_json::to_value(links).map_err(|e| {
                        crate::protocol::internal_error(&format!(
                            "Failed to serialize document links: {}",
                            e
                        ))
                    })?)),
                    Err(_) => Ok(Some(Value::Null)),
                }
            } else {
                Ok(Some(Value::Null))
            }
        } else {
            Ok(Some(Value::Null))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_document_link_file_path, resolve_file_link_target};

    #[test]
    fn normalize_document_link_file_path_collapses_windows_separators() {
        assert_eq!(normalize_document_link_file_path(r"lib\\Thing.pm"), "lib/Thing.pm");
        assert_eq!(normalize_document_link_file_path(r"lib\Thing.pm"), "lib/Thing.pm");
        assert_eq!(normalize_document_link_file_path("lib/Thing.pm"), "lib/Thing.pm");
    }

    #[test]
    fn normalize_document_link_file_path_preserves_unc_prefix() {
        assert_eq!(
            normalize_document_link_file_path(r"\\server\share\Thing.pm"),
            "//server/share/Thing.pm"
        );
    }

    #[test]
    fn resolve_file_link_target_handles_windows_absolute_paths() {
        let resolved =
            resolve_file_link_target("file:///workspace/project/file.pl", "C:/Users/me/Thing.pm");
        assert_eq!(resolved, Some("file:///C:/Users/me/Thing.pm".to_string()));
    }

    #[test]
    fn resolve_file_link_target_handles_unc_paths() {
        let resolved = resolve_file_link_target(
            "file:///workspace/project/file.pl",
            "//server/share/lib/Thing.pm",
        );
        assert_eq!(resolved, Some("file://server/share/lib/Thing.pm".to_string()));
    }
}
