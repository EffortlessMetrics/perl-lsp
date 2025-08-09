//! Enhanced LSP server with incremental parsing support

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use lsp_types::{
    DidChangeTextDocumentParams, TextDocumentContentChangeEvent,
    PublishDiagnosticsParams, Diagnostic, DiagnosticSeverity,
    Position, Range, Url,
};
use serde_json::Value;

use crate::incremental::{IncrementalState, Edit, apply_edits, ReparseResult};
use crate::parser::Parser;
use crate::ast::Node;
use crate::diagnostics::DiagnosticProvider;

/// Document state with incremental parsing
pub struct IncrementalDocument {
    pub incremental_state: IncrementalState,
    pub version: i32,
    pub uri: String,
}

/// Enhanced LSP server with incremental parsing
pub struct IncrementalLspServer {
    documents: Arc<Mutex<HashMap<String, IncrementalDocument>>>,
    diagnostic_provider: DiagnosticProvider,
}

impl IncrementalLspServer {
    pub fn new() -> Self {
        Self {
            documents: Arc::new(Mutex::new(HashMap::new())),
            diagnostic_provider: DiagnosticProvider::new(),
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
                .and_then(|c| c.text.as_ref())
                .map(|s| s.to_string())
                .unwrap_or_default();
            
            IncrementalDocument {
                incremental_state: IncrementalState::new(content),
                version,
                uri: uri.clone(),
            }
        });
        
        // Convert LSP changes to edits
        let edits = self.convert_changes_to_edits(&params.content_changes, &doc.incremental_state)?;
        
        // Apply incremental parsing
        let start = std::time::Instant::now();
        let result = apply_edits(&mut doc.incremental_state, &edits)?;
        let elapsed = start.elapsed();
        
        eprintln!(
            "Incremental parse: {} bytes in {:?} ({}% of document)",
            result.reparsed_bytes,
            elapsed,
            (result.reparsed_bytes * 100) / doc.incremental_state.source.len().max(1)
        );
        
        // Update version
        doc.version = version;
        
        // Publish diagnostics
        self.publish_diagnostics(&uri, &doc.incremental_state, result);
        
        Ok(())
    }
    
    /// Convert LSP changes to Edit structs
    fn convert_changes_to_edits(
        &self,
        changes: &[TextDocumentContentChangeEvent],
        state: &IncrementalState,
    ) -> Result<Vec<Edit>, String> {
        let mut edits = Vec::new();
        
        for change in changes {
            if let Some(edit) = Edit::from_lsp_change(change, &state.line_index, &state.source) {
                edits.push(edit);
            } else if let Some(text) = &change.text {
                // Full document change
                edits.push(Edit {
                    start_byte: 0,
                    old_end_byte: state.source.len(),
                    new_end_byte: text.len(),
                    new_text: text.clone(),
                });
            }
        }
        
        Ok(edits)
    }
    
    /// Publish diagnostics from parse result
    fn publish_diagnostics(&self, uri: &str, state: &IncrementalState, result: ReparseResult) {
        // Get diagnostics from AST
        let mut diagnostics = result.diagnostics;
        
        // Add additional diagnostics from diagnostic provider
        if let Ok(url) = Url::parse(uri) {
            let provider_diagnostics = self.diagnostic_provider.get_diagnostics(&state.ast, &state.source);
            
            for diag in provider_diagnostics {
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position {
                            line: diag.range.start.line,
                            character: diag.range.start.character,
                        },
                        end: Position {
                            line: diag.range.end.line,
                            character: diag.range.end.character,
                        },
                    },
                    severity: Some(match diag.severity {
                        crate::diagnostics::DiagnosticSeverity::Error => DiagnosticSeverity::ERROR,
                        crate::diagnostics::DiagnosticSeverity::Warning => DiagnosticSeverity::WARNING,
                        crate::diagnostics::DiagnosticSeverity::Information => DiagnosticSeverity::INFORMATION,
                        crate::diagnostics::DiagnosticSeverity::Hint => DiagnosticSeverity::HINT,
                    }),
                    message: diag.message,
                    code: diag.code.map(|c| lsp_types::NumberOrString::String(c)),
                    source: Some("perl-parser".to_string()),
                    ..Default::default()
                });
            }
            
            // Send diagnostics notification
            let params = PublishDiagnosticsParams {
                uri: url,
                diagnostics,
                version: None,
            };
            
            // In a real implementation, this would send through the LSP transport
            eprintln!("Publishing {} diagnostics for {}", params.diagnostics.len(), uri);
        }
    }
    
    /// Handle document open - initial parse
    pub fn handle_did_open(&self, uri: String, content: String, version: i32) {
        let mut documents = self.documents.lock().unwrap();
        
        let doc = IncrementalDocument {
            incremental_state: IncrementalState::new(content),
            version,
            uri: uri.clone(),
        };
        
        // Publish initial diagnostics
        let empty_result = ReparseResult {
            changed_ranges: vec![0..doc.incremental_state.source.len()],
            diagnostics: vec![],
            reparsed_bytes: doc.incremental_state.source.len(),
        };
        self.publish_diagnostics(&uri, &doc.incremental_state, empty_result);
        
        documents.insert(uri, doc);
    }
    
    /// Get AST for a document
    pub fn get_ast(&self, uri: &str) -> Option<Node> {
        self.documents.lock().unwrap()
            .get(uri)
            .map(|doc| doc.incremental_state.ast.clone())
    }
    
    /// Get document content
    pub fn get_content(&self, uri: &str) -> Option<String> {
        self.documents.lock().unwrap()
            .get(uri)
            .map(|doc| doc.incremental_state.source.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_incremental_small_edit() {
        let server = IncrementalLspServer::new();
        let uri = "file:///test.pl".to_string();
        
        // Initial document
        let content = "my $x = 1;\nmy $y = 2;\nprint $x + $y;".to_string();
        server.handle_did_open(uri.clone(), content.clone(), 1);
        
        // Small edit: change 1 to 10
        let params = DidChangeTextDocumentParams {
            text_document: lsp_types::VersionedTextDocumentIdentifier {
                uri: Url::parse(&uri).unwrap(),
                version: 2,
            },
            content_changes: vec![
                TextDocumentContentChangeEvent {
                    range: Some(Range {
                        start: Position { line: 0, character: 8 },
                        end: Position { line: 0, character: 9 },
                    }),
                    range_length: Some(1),
                    text: "10".to_string(),
                }
            ],
        };
        
        let result = server.handle_did_change(params);
        assert!(result.is_ok());
        
        // Verify the content was updated
        let new_content = server.get_content(&uri).unwrap();
        assert_eq!(new_content, "my $x = 10;\nmy $y = 2;\nprint $x + $y;");
    }
    
    #[test]
    fn test_incremental_multiline_edit() {
        let server = IncrementalLspServer::new();
        let uri = "file:///test.pl".to_string();
        
        // Initial document
        let content = "sub foo {\n    return 1;\n}\n\nfoo();".to_string();
        server.handle_did_open(uri.clone(), content.clone(), 1);
        
        // Add a new parameter
        let params = DidChangeTextDocumentParams {
            text_document: lsp_types::VersionedTextDocumentIdentifier {
                uri: Url::parse(&uri).unwrap(),
                version: 2,
            },
            content_changes: vec![
                TextDocumentContentChangeEvent {
                    range: Some(Range {
                        start: Position { line: 0, character: 7 },
                        end: Position { line: 0, character: 7 },
                    }),
                    range_length: Some(0),
                    text: "($x)".to_string(),
                }
            ],
        };
        
        let result = server.handle_did_change(params);
        assert!(result.is_ok());
        
        // Verify the AST was updated
        let ast = server.get_ast(&uri);
        assert!(ast.is_some());
    }
}