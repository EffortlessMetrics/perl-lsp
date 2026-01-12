//! Notebook Document Synchronization (LSP 3.17)
//!
//! Handles notebook document lifecycle: didOpen, didChange, didSave, didClose.
//! Cell text documents are stored in the main DocumentStore, with a mapping
//! to track which notebook owns each cell.

use super::*;
use crate::lsp::protocol::invalid_params;
use std::collections::HashMap;

/// State for a notebook document
#[derive(Debug, Clone)]
#[allow(dead_code)] // Infrastructure for future notebook state management
struct NotebookDocState {
    /// Notebook URI
    pub uri: String,
    /// Notebook type (e.g., "jupyter-notebook")
    pub notebook_type: String,
    /// Notebook version
    pub version: i32,
    /// Cell URIs in order
    pub cells: Vec<NotebookCellState>,
}

/// State for a notebook cell
#[derive(Debug, Clone)]
#[allow(dead_code)] // Infrastructure for future notebook state management
struct NotebookCellState {
    /// Cell kind: 1=Markup, 2=Code
    pub kind: i32,
    /// Cell document URI
    pub document: String,
}

/// Store for notebook documents and cell-to-notebook mapping
#[allow(dead_code)] // Infrastructure for future notebook state management
struct NotebookStore {
    /// Notebook documents indexed by URI
    notebooks: HashMap<String, NotebookDocState>,
    /// Mapping from cell URI to notebook URI
    cell_to_notebook: HashMap<String, String>,
}

#[allow(dead_code)] // Infrastructure methods for future use
impl NotebookStore {
    fn new() -> Self {
        Self { notebooks: HashMap::new(), cell_to_notebook: HashMap::new() }
    }

    /// Register a cell as belonging to a notebook
    fn register_cell(&mut self, cell_uri: String, notebook_uri: String) {
        self.cell_to_notebook.insert(cell_uri, notebook_uri);
    }

    /// Unregister a cell
    fn unregister_cell(&mut self, cell_uri: &str) {
        self.cell_to_notebook.remove(cell_uri);
    }

    /// Get the notebook URI for a cell
    fn get_notebook_for_cell(&self, cell_uri: &str) -> Option<&str> {
        self.cell_to_notebook.get(cell_uri).map(String::as_str)
    }
}

impl Default for NotebookStore {
    fn default() -> Self {
        Self::new()
    }
}

impl LspServer {
    /// Handle notebookDocument/didOpen notification
    pub(crate) fn handle_notebook_did_open(
        &self,
        params: Option<Value>,
    ) -> Result<(), JsonRpcError> {
        let params = params.ok_or_else(|| invalid_params("Missing params"))?;

        // Extract notebook document metadata
        let notebook_uri = params
            .pointer("/notebookDocument/uri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| invalid_params("Missing notebookDocument.uri"))?;

        let notebook_type = params
            .pointer("/notebookDocument/notebookType")
            .and_then(|v| v.as_str())
            .unwrap_or("jupyter-notebook");

        let version = params
            .pointer("/notebookDocument/version")
            .and_then(|v| v.as_i64())
            .and_then(|v| i32::try_from(v).ok())
            .unwrap_or(1);

        // Extract cells metadata
        let cells_array = params
            .pointer("/notebookDocument/cells")
            .and_then(|v| v.as_array())
            .ok_or_else(|| invalid_params("Missing notebookDocument.cells"))?;

        let mut cells = Vec::new();
        for cell in cells_array {
            let kind = cell
                .get("kind")
                .and_then(|v| v.as_i64())
                .and_then(|v| i32::try_from(v).ok())
                .unwrap_or(2); // Default to Code

            let document = cell
                .get("document")
                .and_then(|v| v.as_str())
                .ok_or_else(|| invalid_params("Missing cell.document"))?
                .to_string();

            cells.push(NotebookCellState { kind, document });
        }

        // Extract cell text documents
        let cell_text_docs = params
            .pointer("/cellTextDocuments")
            .and_then(|v| v.as_array())
            .ok_or_else(|| invalid_params("Missing cellTextDocuments"))?;

        // Open each cell as a text document in the main DocumentStore
        for cell_doc in cell_text_docs {
            let cell_uri = cell_doc
                .get("uri")
                .and_then(|v| v.as_str())
                .ok_or_else(|| invalid_params("Missing cell uri"))?;

            let text = cell_doc
                .get("text")
                .and_then(|v| v.as_str())
                .ok_or_else(|| invalid_params("Missing cell text"))?;

            let cell_version = cell_doc
                .get("version")
                .and_then(|v| v.as_i64())
                .and_then(|v| i32::try_from(v).ok())
                .unwrap_or(1);

            // Open cell as a text document using existing didOpen logic
            let did_open_params = json!({
                "textDocument": {
                    "uri": cell_uri,
                    "languageId": cell_doc.get("languageId").and_then(|v| v.as_str()).unwrap_or("perl"),
                    "version": cell_version,
                    "text": text
                }
            });

            // Use existing didOpen handler for the cell
            self.handle_did_open(Some(did_open_params))?;

            // Track cell-to-notebook mapping
            // Note: We'll need to add a notebook_store field to LspServer
            // For now, we'll skip the mapping as it requires server state changes
            eprintln!("Notebook cell opened: {} (notebook: {})", cell_uri, notebook_uri);
        }

        eprintln!(
            "Notebook opened: {} (type: {}, version: {}, cells: {})",
            notebook_uri,
            notebook_type,
            version,
            cells.len()
        );

        // Note: Storing notebook metadata would require adding a notebook_store
        // field to LspServer. For minimal implementation, we rely on cell
        // documents being tracked in the main DocumentStore.

        Ok(())
    }

    /// Handle notebookDocument/didChange notification
    pub(crate) fn handle_notebook_did_change(
        &self,
        params: Option<Value>,
    ) -> Result<(), JsonRpcError> {
        let params = params.ok_or_else(|| invalid_params("Missing params"))?;

        let notebook_uri = params
            .pointer("/notebookDocument/uri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| invalid_params("Missing notebookDocument.uri"))?;

        let _version = params
            .pointer("/notebookDocument/version")
            .and_then(|v| v.as_i64())
            .and_then(|v| i32::try_from(v).ok());

        eprintln!("Notebook changed: {}", notebook_uri);

        // Handle change events
        if let Some(change) = params.get("change") {
            // Handle cells changes
            if let Some(cells) = change.get("cells") {
                // Handle structure changes (add/remove/move cells)
                if let Some(structure) = cells.get("structure") {
                    // Handle array operations
                    if let Some(array) = structure.get("array") {
                        let _start = array.get("start").and_then(|v| v.as_u64());
                        let _delete_count = array.get("deleteCount").and_then(|v| v.as_u64());

                        // Handle newly opened cells
                        if let Some(new_cells) = array.get("cells").and_then(|v| v.as_array()) {
                            for _cell in new_cells {
                                // Cell metadata added to notebook
                                eprintln!("Cell added to notebook structure");
                            }
                        }
                    }

                    // Handle didOpen for new cells
                    if let Some(did_open) = structure.get("didOpen").and_then(|v| v.as_array()) {
                        for cell_doc in did_open {
                            let cell_uri = cell_doc
                                .get("uri")
                                .and_then(|v| v.as_str())
                                .ok_or_else(|| invalid_params("Missing cell uri"))?;

                            let text = cell_doc
                                .get("text")
                                .and_then(|v| v.as_str())
                                .ok_or_else(|| invalid_params("Missing cell text"))?;

                            let cell_version = cell_doc
                                .get("version")
                                .and_then(|v| v.as_i64())
                                .and_then(|v| i32::try_from(v).ok())
                                .unwrap_or(1);

                            // Open new cell as text document
                            let did_open_params = json!({
                                "textDocument": {
                                    "uri": cell_uri,
                                    "languageId": cell_doc.get("languageId").and_then(|v| v.as_str()).unwrap_or("perl"),
                                    "version": cell_version,
                                    "text": text
                                }
                            });

                            self.handle_did_open(Some(did_open_params))?;
                            eprintln!("New cell opened: {}", cell_uri);
                        }
                    }

                    // Handle didClose for removed cells
                    if let Some(did_close) = structure.get("didClose").and_then(|v| v.as_array()) {
                        for cell_doc in did_close {
                            let cell_uri = cell_doc
                                .get("uri")
                                .and_then(|v| v.as_str())
                                .ok_or_else(|| invalid_params("Missing cell uri"))?;

                            // Close cell text document
                            let did_close_params = json!({
                                "textDocument": {
                                    "uri": cell_uri
                                }
                            });

                            self.handle_did_close(Some(did_close_params))?;
                            eprintln!("Cell closed: {}", cell_uri);
                        }
                    }
                }

                // Handle data changes (metadata updates)
                if let Some(_data) = cells.get("data") {
                    eprintln!("Notebook cell metadata updated");
                }

                // Handle textContent changes (cell text edits)
                if let Some(text_content) = cells.get("textContent").and_then(|v| v.as_array()) {
                    for change_event in text_content {
                        let cell_uri = change_event
                            .get("document")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| invalid_params("Missing document uri in textContent"))?;

                        // Get changes array
                        if let Some(changes) =
                            change_event.get("changes").and_then(|v| v.as_array())
                        {
                            // Apply changes using existing didChange logic
                            let did_change_params = json!({
                                "textDocument": {
                                    "uri": cell_uri,
                                    "version": change_event.get("version").and_then(|v| v.as_i64()).unwrap_or(1)
                                },
                                "contentChanges": changes
                            });

                            self.handle_did_change(Some(did_change_params))?;
                            eprintln!("Cell text changed: {}", cell_uri);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle notebookDocument/didSave notification
    pub(crate) fn handle_notebook_did_save(
        &self,
        params: Option<Value>,
    ) -> Result<(), JsonRpcError> {
        let params = params.ok_or_else(|| invalid_params("Missing params"))?;

        let notebook_uri = params
            .pointer("/notebookDocument/uri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| invalid_params("Missing notebookDocument.uri"))?;

        eprintln!("Notebook saved: {}", notebook_uri);

        // Optionally trigger post-save actions on all cells
        // For now, just log the event

        Ok(())
    }

    /// Handle notebookDocument/didClose notification
    pub(crate) fn handle_notebook_did_close(
        &self,
        params: Option<Value>,
    ) -> Result<(), JsonRpcError> {
        let params = params.ok_or_else(|| invalid_params("Missing params"))?;

        let notebook_uri = params
            .pointer("/notebookDocument/uri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| invalid_params("Missing notebookDocument.uri"))?;

        // Get cell URIs that need closing
        if let Some(cell_docs) = params.pointer("/cellTextDocuments").and_then(|v| v.as_array()) {
            for cell_doc in cell_docs {
                let cell_uri = cell_doc
                    .get("uri")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| invalid_params("Missing cell uri"))?;

                // Close cell text document
                let did_close_params = json!({
                    "textDocument": {
                        "uri": cell_uri
                    }
                });

                self.handle_did_close(Some(did_close_params))?;
                eprintln!("Cell closed: {}", cell_uri);
            }
        }

        eprintln!("Notebook closed: {}", notebook_uri);

        // Note: Removing notebook metadata would require notebook_store

        Ok(())
    }
}
