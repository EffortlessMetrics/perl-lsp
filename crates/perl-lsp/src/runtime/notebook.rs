//! Notebook Document Synchronization (LSP 3.17)
//!
//! Handles notebook document lifecycle: didOpen, didChange, didSave, didClose.
//! Cell text documents are stored in the main DocumentStore, with a mapping
//! to track which notebook owns each cell.
//!
//! ## Features
//!
//! - **Document Sync**: Full support for notebook lifecycle notifications
//! - **Cell Tracking**: Maps cell URIs to their parent notebook
//! - **Execution Summary**: Tracks cell execution order and success status (LSP 3.17)
//!
//! ## References
//!
//! - [LSP 3.17 Notebook Spec](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#notebookDocument_synchronization)

use super::*;
use crate::protocol::invalid_params;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

/// Execution summary for a notebook cell (LSP 3.17)
///
/// Tracks the execution state of a cell, including execution order
/// and whether the execution was successful.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)] // Fields read by handlers and for future cross-cell features
pub(crate) struct ExecutionSummary {
    /// Execution order - monotonically increasing value indicating
    /// when this cell was executed relative to other cells
    pub execution_order: Option<u32>,
    /// Whether the execution was successful
    pub success: Option<bool>,
}

/// State for a notebook document
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields stored for future cross-cell features
pub(crate) struct NotebookDocState {
    /// Notebook URI
    pub uri: String,
    /// Notebook type (e.g., "jupyter-notebook")
    pub notebook_type: String,
    /// Notebook version
    pub version: i32,
    /// Notebook metadata (optional JSON object)
    pub metadata: Option<serde_json::Map<String, serde_json::Value>>,
    /// Cell URIs in order
    pub cells: Vec<NotebookCellState>,
}

/// State for a notebook cell
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields stored for future cross-cell features
pub(crate) struct NotebookCellState {
    /// Cell kind: 1=Markup, 2=Code
    pub kind: i32,
    /// Cell document URI
    pub document: String,
    /// Cell metadata (optional)
    pub metadata: Option<serde_json::Map<String, serde_json::Value>>,
    /// Execution summary (LSP 3.17) - tracks execution order and success
    pub execution_summary: Option<ExecutionSummary>,
}

/// Store for notebook documents and cell-to-notebook mapping
///
/// Thread-safe storage for notebook state, including cell tracking
/// and execution summary management.
#[derive(Debug)]
pub(crate) struct NotebookStore {
    /// Notebook documents indexed by URI
    notebooks: Arc<Mutex<HashMap<String, NotebookDocState>>>,
    /// Mapping from cell URI to notebook URI
    cell_to_notebook: Arc<Mutex<HashMap<String, String>>>,
}

impl NotebookStore {
    /// Create a new empty notebook store
    pub(crate) fn new() -> Self {
        Self {
            notebooks: Arc::new(Mutex::new(HashMap::new())),
            cell_to_notebook: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a notebook with its cells
    pub(crate) fn register_notebook(&self, notebook: NotebookDocState) {
        let notebook_uri = notebook.uri.clone();
        let cell_uris: Vec<String> = notebook.cells.iter().map(|c| c.document.clone()).collect();

        // Register all cells
        {
            let mut cell_map = self.cell_to_notebook.lock();
            for cell_uri in cell_uris {
                cell_map.insert(cell_uri, notebook_uri.clone());
            }
        }

        // Store notebook
        {
            let mut notebooks = self.notebooks.lock();
            notebooks.insert(notebook_uri, notebook);
        }
    }

    /// Register a cell as belonging to a notebook
    #[allow(dead_code)] // Infrastructure for future cross-cell features
    pub(crate) fn register_cell(&self, cell_uri: String, notebook_uri: String) {
        let mut cell_map = self.cell_to_notebook.lock();
        cell_map.insert(cell_uri, notebook_uri);
    }

    /// Unregister a cell
    #[allow(dead_code)] // Infrastructure for future cross-cell features
    pub(crate) fn unregister_cell(&self, cell_uri: &str) {
        let mut cell_map = self.cell_to_notebook.lock();
        cell_map.remove(cell_uri);
    }

    /// Unregister a notebook and all its cells
    pub(crate) fn unregister_notebook(&self, notebook_uri: &str) {
        // Remove notebook and get its cells
        let cells = {
            let mut notebooks = self.notebooks.lock();
            notebooks.remove(notebook_uri).map(|n| n.cells)
        };

        // Remove all cell mappings
        if let Some(cells) = cells {
            let mut cell_map = self.cell_to_notebook.lock();
            for cell in cells {
                cell_map.remove(&cell.document);
            }
        }
    }

    /// Get the notebook URI for a cell
    #[allow(dead_code)] // Infrastructure for future cross-cell features
    pub(crate) fn get_notebook_for_cell(&self, cell_uri: &str) -> Option<String> {
        let cell_map = self.cell_to_notebook.lock();
        cell_map.get(cell_uri).cloned()
    }

    /// Get a notebook by URI
    #[allow(dead_code)] // Infrastructure for future cross-cell features
    pub(crate) fn get_notebook(&self, notebook_uri: &str) -> Option<NotebookDocState> {
        let notebooks = self.notebooks.lock();
        notebooks.get(notebook_uri).cloned()
    }

    /// Update notebook version
    pub(crate) fn update_version(&self, notebook_uri: &str, version: i32) {
        let mut notebooks = self.notebooks.lock();
        if let Some(notebook) = notebooks.get_mut(notebook_uri) {
            notebook.version = version;
        }
    }

    /// Update cell execution summary
    pub(crate) fn update_cell_execution(
        &self,
        cell_uri: &str,
        execution_summary: Option<ExecutionSummary>,
    ) {
        // Find the notebook containing this cell
        let notebook_uri = {
            let cell_map = self.cell_to_notebook.lock();
            cell_map.get(cell_uri).cloned()
        };

        if let Some(notebook_uri) = notebook_uri {
            let mut notebooks = self.notebooks.lock();
            if let Some(notebook) = notebooks.get_mut(&notebook_uri) {
                // Find and update the cell
                for cell in &mut notebook.cells {
                    if cell.document == cell_uri {
                        cell.execution_summary = execution_summary;
                        break;
                    }
                }
            }
        }
    }

    /// Update cells in a notebook (for structure changes)
    #[allow(dead_code)] // Infrastructure for future cross-cell features
    pub(crate) fn update_cells(&self, notebook_uri: &str, cells: Vec<NotebookCellState>) {
        let mut notebooks = self.notebooks.lock();
        if let Some(notebook) = notebooks.get_mut(notebook_uri) {
            // Update cell-to-notebook mapping
            let mut cell_map = self.cell_to_notebook.lock();

            // Remove old cell mappings
            for old_cell in &notebook.cells {
                cell_map.remove(&old_cell.document);
            }

            // Add new cell mappings
            for new_cell in &cells {
                cell_map.insert(new_cell.document.clone(), notebook_uri.to_string());
            }

            // Update cells
            notebook.cells = cells;
        }
    }

    /// Get all notebooks
    #[allow(dead_code)] // Infrastructure for future cross-cell features
    pub(crate) fn all_notebooks(&self) -> Vec<NotebookDocState> {
        let notebooks = self.notebooks.lock();
        notebooks.values().cloned().collect()
    }
}

impl Default for NotebookStore {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for NotebookStore {
    fn clone(&self) -> Self {
        Self {
            notebooks: Arc::new(Mutex::new(self.notebooks.lock().clone())),
            cell_to_notebook: Arc::new(Mutex::new(self.cell_to_notebook.lock().clone())),
        }
    }
}

impl LspServer {
    fn parse_notebook_cell(cell: &Value) -> Result<NotebookCellState, JsonRpcError> {
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

        // Extract cell metadata (optional)
        let metadata = cell.get("metadata").and_then(|v| v.as_object()).cloned();

        // Extract execution summary if present (LSP 3.17)
        let execution_summary = cell.get("executionSummary").map(|es| ExecutionSummary {
            execution_order: es
                .get("executionOrder")
                .and_then(|v| v.as_u64())
                .and_then(|v| u32::try_from(v).ok()),
            success: es.get("success").and_then(|v| v.as_bool()),
        });

        Ok(NotebookCellState { kind, document, metadata, execution_summary })
    }

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

        // Extract notebook metadata (optional)
        let metadata =
            params.pointer("/notebookDocument/metadata").and_then(|v| v.as_object()).cloned();

        let mut cells = Vec::new();
        for cell in cells_array {
            cells.push(Self::parse_notebook_cell(cell)?);
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

            eprintln!("Notebook cell opened: {} (notebook: {})", cell_uri, notebook_uri);
        }

        // Create and store notebook state
        let notebook_state = NotebookDocState {
            uri: notebook_uri.to_string(),
            notebook_type: notebook_type.to_string(),
            version,
            metadata,
            cells,
        };

        // Register notebook in store
        self.notebook_store.register_notebook(notebook_state);

        eprintln!(
            "Notebook opened: {} (type: {}, version: {})",
            notebook_uri, notebook_type, version
        );

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

        // Update notebook version
        if let Some(version) = params
            .pointer("/notebookDocument/version")
            .and_then(|v| v.as_i64())
            .and_then(|v| i32::try_from(v).ok())
        {
            self.notebook_store.update_version(notebook_uri, version);
        }

        eprintln!("Notebook changed: {}", notebook_uri);

        // Handle change events
        if let Some(change) = params.get("change") {
            // Handle cells changes
            if let Some(cells) = change.get("cells") {
                // Handle structure changes (add/remove/move cells)
                if let Some(structure) = cells.get("structure") {
                    let mut updated_cells = self
                        .notebook_store
                        .get_notebook(notebook_uri)
                        .map_or_else(Vec::new, |notebook| notebook.cells);
                    let mut structure_changed = false;

                    // Handle array operations
                    if let Some(array) = structure.get("array") {
                        let start = array
                            .get("start")
                            .and_then(|v| v.as_u64())
                            .and_then(|v| usize::try_from(v).ok())
                            .unwrap_or(0);
                        let delete_count = array
                            .get("deleteCount")
                            .and_then(|v| v.as_u64())
                            .and_then(|v| usize::try_from(v).ok())
                            .unwrap_or(0);
                        let replacement = array
                            .get("cells")
                            .and_then(|v| v.as_array())
                            .map(|new_cells| {
                                new_cells
                                    .iter()
                                    .map(Self::parse_notebook_cell)
                                    .collect::<Result<Vec<_>, _>>()
                            })
                            .transpose()?
                            .unwrap_or_default();

                        let splice_start = start.min(updated_cells.len());
                        let splice_end =
                            splice_start.saturating_add(delete_count).min(updated_cells.len());
                        updated_cells.splice(splice_start..splice_end, replacement);
                        structure_changed = true;
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

                            if !updated_cells.iter().any(|cell| cell.document == cell_uri) {
                                updated_cells.push(NotebookCellState {
                                    kind: 2,
                                    document: cell_uri.to_string(),
                                    metadata: None,
                                    execution_summary: None,
                                });
                                structure_changed = true;
                            }
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

                            let previous_len = updated_cells.len();
                            updated_cells.retain(|cell| cell.document != cell_uri);
                            if updated_cells.len() != previous_len {
                                structure_changed = true;
                            }
                        }
                    }

                    if structure_changed {
                        self.notebook_store.update_cells(notebook_uri, updated_cells);
                    }
                }

                // Handle data changes (metadata and execution summary updates)
                if let Some(data) = cells.get("data").and_then(|v| v.as_array()) {
                    for cell_data in data {
                        // Each item is a NotebookCell object with updated data
                        if let Some(cell_doc_uri) =
                            cell_data.get("document").and_then(|v| v.as_str())
                        {
                            // Update execution summary if present (LSP 3.17)
                            if let Some(es) = cell_data.get("executionSummary") {
                                let execution_summary = ExecutionSummary {
                                    execution_order: es
                                        .get("executionOrder")
                                        .and_then(|v| v.as_u64())
                                        .and_then(|v| u32::try_from(v).ok()),
                                    success: es.get("success").and_then(|v| v.as_bool()),
                                };
                                self.notebook_store
                                    .update_cell_execution(cell_doc_uri, Some(execution_summary));
                                eprintln!(
                                    "Cell execution summary updated: {} (order: {:?})",
                                    cell_doc_uri,
                                    es.get("executionOrder")
                                );
                            }
                        }
                    }
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

        // Unregister notebook from store
        self.notebook_store.unregister_notebook(notebook_uri);

        eprintln!("Notebook closed: {}", notebook_uri);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn notebook_did_open_registers_cells_and_documents() -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let notebook_uri = "file:///open-test.ipynb";
        let cell_uri = "file:///open-test.ipynb#cell1";

        server.handle_notebook_did_open(Some(json!({
            "notebookDocument": {
                "uri": notebook_uri,
                "notebookType": "jupyter-notebook",
                "version": 1,
                "cells": [
                    {
                        "kind": 2,
                        "document": cell_uri
                    }
                ]
            },
            "cellTextDocuments": [
                {
                    "uri": cell_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "my $x = 1;\n"
                }
            ]
        })))?;

        assert_eq!(
            server.notebook_store.get_notebook_for_cell(cell_uri).as_deref(),
            Some(notebook_uri)
        );
        assert!(server.documents_guard().contains_key(cell_uri));

        let notebook = server
            .notebook_store
            .get_notebook(notebook_uri)
            .ok_or("notebook missing from store")?;
        assert_eq!(notebook.cells.len(), 1);
        assert_eq!(notebook.cells[0].document, cell_uri);

        Ok(())
    }

    #[test]
    fn notebook_structure_change_updates_cell_mapping_and_execution_summary()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let notebook_uri = "file:///change-test.ipynb";
        let cell1_uri = "file:///change-test.ipynb#cell1";
        let cell2_uri = "file:///change-test.ipynb#cell2";

        server.handle_notebook_did_open(Some(json!({
            "notebookDocument": {
                "uri": notebook_uri,
                "notebookType": "jupyter-notebook",
                "version": 1,
                "cells": [
                    {
                        "kind": 2,
                        "document": cell1_uri
                    }
                ]
            },
            "cellTextDocuments": [
                {
                    "uri": cell1_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "my $x = 1;\n"
                }
            ]
        })))?;

        server.handle_notebook_did_change(Some(json!({
            "notebookDocument": {
                "uri": notebook_uri,
                "version": 2
            },
            "change": {
                "cells": {
                    "structure": {
                        "array": {
                            "start": 1,
                            "deleteCount": 0,
                            "cells": [
                                {
                                    "kind": 2,
                                    "document": cell2_uri
                                }
                            ]
                        },
                        "didOpen": [
                            {
                                "uri": cell2_uri,
                                "languageId": "perl",
                                "version": 1,
                                "text": "my $y = 2;\n"
                            }
                        ]
                    },
                    "data": [
                        {
                            "document": cell2_uri,
                            "executionSummary": {
                                "executionOrder": 7,
                                "success": true
                            }
                        }
                    ]
                }
            }
        })))?;

        assert_eq!(
            server.notebook_store.get_notebook_for_cell(cell2_uri).as_deref(),
            Some(notebook_uri)
        );
        assert!(server.documents_guard().contains_key(cell2_uri));

        let notebook = server
            .notebook_store
            .get_notebook(notebook_uri)
            .ok_or("notebook missing after didChange")?;
        assert_eq!(notebook.cells.len(), 2);

        let cell2 = notebook
            .cells
            .iter()
            .find(|cell| cell.document == cell2_uri)
            .ok_or("cell2 missing from notebook state")?;
        assert_eq!(
            cell2.execution_summary.as_ref().and_then(|summary| summary.execution_order),
            Some(7)
        );
        assert_eq!(
            cell2.execution_summary.as_ref().and_then(|summary| summary.success),
            Some(true)
        );

        server.handle_notebook_did_change(Some(json!({
            "notebookDocument": {
                "uri": notebook_uri,
                "version": 3
            },
            "change": {
                "cells": {
                    "structure": {
                        "array": {
                            "start": 0,
                            "deleteCount": 1,
                            "cells": []
                        },
                        "didClose": [
                            { "uri": cell1_uri }
                        ]
                    }
                }
            }
        })))?;

        assert!(server.notebook_store.get_notebook_for_cell(cell1_uri).is_none());
        assert!(!server.documents_guard().contains_key(cell1_uri));

        let notebook = server
            .notebook_store
            .get_notebook(notebook_uri)
            .ok_or("notebook missing after removing cell1")?;
        assert_eq!(notebook.cells.len(), 1);
        assert_eq!(notebook.cells[0].document, cell2_uri);

        Ok(())
    }
}
