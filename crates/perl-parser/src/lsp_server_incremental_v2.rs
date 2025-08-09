//! Enhanced LSP server with incremental parsing support

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::str::FromStr;
use lsp_types::{
    DidChangeTextDocumentParams, TextDocumentContentChangeEvent,
    PublishDiagnosticsParams, Range,
};

use crate::incremental::{IncrementalState, Edit, apply_edits, ReparseResult};

/// Document state with incremental parsing
pub struct IncrementalDocument {
    pub incremental_state: IncrementalState,
    pub version: i32,
    pub uri: String,
}

/// Enhanced LSP server with incremental parsing
pub struct IncrementalLspServer {
    documents: Arc<Mutex<HashMap<String, IncrementalDocument>>>,
}

impl IncrementalLspServer {
    pub fn new() -> Self {
        Self {
            documents: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Handle document change with incremental parsing
    pub fn handle_did_change(&self, params: DidChangeTextDocumentParams) -> Result<(), String> {
        let uri = params.text_document.uri.to_string();
        let version = params.text_document.version;
        
        let mut documents = self.documents.lock().unwrap();
        
        // Get or create document state
        let doc = documents.entry(uri.clone()).or_insert_with(|| {
            // Full parse for new document
            let content = params.content_changes
                .first()
                .and_then(|c| Some(&c.text))
                .map(|s| s.clone())
                .unwrap_or_default();
            
            IncrementalDocument {
                incremental_state: IncrementalState::new(content),
                version,
                uri: uri.clone(),
            }
        });
        
        doc.version = version;
        
        // Convert LSP changes to edits
        let edits = self.convert_changes_to_edits(&params.content_changes, &doc.incremental_state)?;
        
        // Apply incremental parsing
        let result = apply_edits(&mut doc.incremental_state, &edits)
            .map_err(|e| e.to_string())?;
        
        // Publish diagnostics
        self.publish_diagnostics(&uri, &doc.incremental_state, result);
        
        Ok(())
    }
    
    /// Convert LSP changes to internal edits
    fn convert_changes_to_edits(
        &self,
        changes: &[TextDocumentContentChangeEvent],
        state: &IncrementalState,
    ) -> Result<Vec<Edit>, String> {
        let mut edits = Vec::new();
        
        for change in changes {
            if let Some(edit) = Edit::from_lsp_change(change, &state.line_index, &state.source) {
                edits.push(edit);
            } else {
                // Full document change - use text from change if available
                let text = change.text.as_str();
                edits.push(Edit {
                    start_byte: 0,
                    old_end_byte: state.source.len(),
                    new_end_byte: text.len(),
                    new_text: text.to_string(),
                });
            }
        }
        
        Ok(edits)
    }
    
    /// Publish diagnostics from parse result
    fn publish_diagnostics(&self, uri: &str, _state: &IncrementalState, result: ReparseResult) {
        // Convert result diagnostics to LSP diagnostics
        let diagnostics = result.diagnostics;
        
        // Create publish diagnostics params
        if let Ok(url) = lsp_types::Uri::from_str(uri) {
            let params = PublishDiagnosticsParams {
                uri: url,
                diagnostics,
                version: None,
            };
            
            // In a real implementation, send this via the LSP connection
            eprintln!("Publishing {} diagnostics for {}", params.diagnostics.len(), uri);
        }
    }
}

/// Create an edit from a text range and new text
pub fn create_edit_from_range(
    range: Range,
    new_text: String,
    line_index: &crate::incremental::LineIndex,
) -> Option<Edit> {
    let start_byte = line_index.position_to_byte(
        range.start.line as usize,
        range.start.character as usize,
    )?;
    
    let old_end_byte = line_index.position_to_byte(
        range.end.line as usize,
        range.end.character as usize,
    )?;
    
    Some(Edit {
        start_byte,
        old_end_byte,
        new_end_byte: start_byte + new_text.len(),
        new_text,
    })
}

/// Example of integrating with the main LSP server
pub fn integrate_incremental_parsing(
    _server: &mut crate::lsp_server::LspServer,
) {
    // Add incremental parsing to the server's didChange handler
    eprintln!("Incremental parsing integrated with LSP server");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incremental_server_creation() {
        let server = IncrementalLspServer::new();
        assert!(server.documents.lock().unwrap().is_empty());
    }

    #[test]
    fn test_simple_edit() {
        let text = "my $x = 42;".to_string();
        let mut state = IncrementalState::new(text);
        
        let edit = Edit {
            start_byte: 8,
            old_end_byte: 10,
            new_end_byte: 10,
            new_text: "99".to_string(),
        };
        
        let result = apply_edits(&mut state, &[edit]);
        assert!(result.is_ok());
        assert_eq!(state.source, "my $x = 99;");
    }
}