//! LSP incremental parsing integration v2
//! 
//! Clean integration of incremental parsing with the LSP server

use crate::{
    incremental_parser::{IncrementalParser, Edit, lsp_change_to_edit},
    lsp_server::{DocumentState, JsonRpcError, LspServer},
    position_mapper::{Position, json_to_position},
    declaration::{DeclarationProvider, ParentMap},
    positions::LineStartsCache,
};
use serde_json::Value;
use std::sync::Arc;

impl LspServer {
    /// Handle didChange notification with incremental parsing
    pub fn handle_did_change_incremental(&self, params: Option<Value>) -> Result<(), JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let version = params["textDocument"]["version"].as_i64().unwrap_or(0) as i32;
            
            let mut documents = self.documents.lock().unwrap();
            
            // Process content changes
            if let Some(changes) = params["contentChanges"].as_array() {
                // Get or create document state
                let doc = documents.entry(uri.to_string()).or_insert_with(|| {
                    DocumentState {
                        content: String::new(),
                        _version: version,
                        ast: None,
                        parse_errors: Vec::new(),
                        parent_map: ParentMap::new(),
                        line_starts: LineStartsCache::new(""),
                    }
                });
                
                // Update version
                doc._version = version;
                
                // Check if we have an incremental parser for this document
                let parser_key = format!("{}_parser", uri);
                let mut incremental_parser = self.get_incremental_parser(&parser_key);
                
                // Apply changes
                for change in changes {
                    if let Some(range) = change["range"].as_object() {
                        // Incremental change
                        let start = json_to_position(&range["start"]).unwrap_or(Position { line: 0, character: 0 });
                        let end = json_to_position(&range["end"]).unwrap_or(Position { line: 0, character: 0 });
                        let text = change["text"].as_str().unwrap_or("");
                        
                        if let Some(edit) = lsp_change_to_edit(incremental_parser.mapper(), Some((start, end)), text) {
                            match incremental_parser.apply_edit(&edit) {
                                Ok(ast) => {
                                    doc.ast = Some(ast);
                                    doc.parse_errors.clear();
                                    doc.content = incremental_parser.text();
                                    
                                    // Update caches
                                    doc.line_starts = LineStartsCache::new(&doc.content);
                                    if let Some(ast) = &doc.ast {
                                        doc.parent_map = DeclarationProvider::build_parent_map(ast);
                                    }
                                }
                                Err(e) => {
                                    doc.parse_errors = vec![e];
                                    doc.ast = None;
                                }
                            }
                        }
                    } else {
                        // Full document change
                        let text = change["text"].as_str().unwrap_or("");
                        doc.content = text.to_string();
                        
                        match incremental_parser.parse_full(text) {
                            Ok(ast) => {
                                doc.ast = Some(ast);
                                doc.parse_errors.clear();
                                
                                // Update caches
                                doc.line_starts = LineStartsCache::new(&doc.content);
                                if let Some(ast) = &doc.ast {
                                    doc.parent_map = DeclarationProvider::build_parent_map(ast);
                                }
                            }
                            Err(e) => {
                                doc.parse_errors = vec![e];
                                doc.ast = None;
                            }
                        }
                    }
                }
                
                // Store the incremental parser for next time
                self.store_incremental_parser(&parser_key, incremental_parser);
                
                // Log stats if in debug mode
                if std::env::var("PERL_LSP_DEBUG").is_ok() {
                    let stats = incremental_parser.stats();
                    eprintln!(
                        "Incremental parse: {}ms, {}% reused, {}/{} bytes",
                        stats.last_parse_time_ms,
                        stats.reuse_percentage as i32,
                        stats.bytes_reparsed,
                        stats.total_bytes
                    );
                }
            }
            
            // Publish diagnostics after processing all changes
            drop(documents); // Release lock before publishing
            self.publish_diagnostics(uri);
            
            Ok(())
        } else {
            Err(JsonRpcError {
                code: -32602,
                message: "Invalid params".to_string(),
            })
        }
    }
    
    /// Get or create incremental parser for a document
    fn get_incremental_parser(&self, key: &str) -> IncrementalParser {
        // In a real implementation, we'd store these in a cache
        // For now, just create a new one
        IncrementalParser::new()
    }
    
    /// Store incremental parser for reuse
    fn store_incremental_parser(&self, _key: &str, _parser: IncrementalParser) {
        // In a real implementation, we'd cache these
        // For now, this is a no-op
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_incremental_change() {
        let server = LspServer::new();
        
        // Open document
        let open_params = json!({
            "textDocument": {
                "uri": "file:///test.pl",
                "version": 1,
                "text": "my $x = 42;\nprint $x;"
            }
        });
        server.handle_did_open(Some(open_params)).unwrap();
        
        // Incremental change
        let change_params = json!({
            "textDocument": {
                "uri": "file:///test.pl",
                "version": 2
            },
            "contentChanges": [{
                "range": {
                    "start": { "line": 0, "character": 8 },
                    "end": { "line": 0, "character": 10 }
                },
                "text": "100"
            }]
        });
        
        // Enable incremental parsing
        std::env::set_var("PERL_LSP_INCREMENTAL", "1");
        
        let result = server.handle_did_change_incremental(Some(change_params));
        assert!(result.is_ok());
        
        // Verify document was updated
        let documents = server.documents.lock().unwrap();
        let doc = documents.get("file:///test.pl").unwrap();
        assert_eq!(doc.content, "my $x = 100;\nprint $x;");
        
        std::env::remove_var("PERL_LSP_INCREMENTAL");
    }
    
    #[test] 
    fn test_full_document_change() {
        let server = LspServer::new();
        
        // Open document
        let open_params = json!({
            "textDocument": {
                "uri": "file:///test.pl",
                "version": 1,
                "text": "my $x = 42;"
            }
        });
        server.handle_did_open(Some(open_params)).unwrap();
        
        // Full document change (no range)
        let change_params = json!({
            "textDocument": {
                "uri": "file:///test.pl",
                "version": 2
            },
            "contentChanges": [{
                "text": "my $y = 'hello';\nprint $y;"
            }]
        });
        
        let result = server.handle_did_change_incremental(Some(change_params));
        assert!(result.is_ok());
        
        // Verify document was replaced
        let documents = server.documents.lock().unwrap();
        let doc = documents.get("file:///test.pl").unwrap();
        assert_eq!(doc.content, "my $y = 'hello';\nprint $y;");
    }
}