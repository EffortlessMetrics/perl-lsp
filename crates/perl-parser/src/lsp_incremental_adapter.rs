//! Adapter to add incremental parsing to the existing LSP server
//!
//! This module provides a drop-in replacement for handle_did_change that
//! uses incremental parsing when enabled via environment variable.

use crate::{
    lsp_server::{DocumentState, JsonRpcError, LspServer},
    parser::Parser,
    declaration::{DeclarationProvider, ParentMap},
    positions::LineStartsCache,
};
use serde_json::Value;
use std::sync::Arc;

#[cfg(feature = "incremental")]
use crate::incremental_integration::{
    DocumentParser, IncrementalConfig, lsp_pos_to_byte
};

#[cfg(feature = "incremental")]
use ropey::Rope;

impl LspServer {
    /// Enhanced handle_did_change with incremental parsing support
    pub fn handle_did_change_incremental(&self, params: Option<Value>) -> Result<(), JsonRpcError> {
        #[cfg(feature = "incremental")]
        {
            // Check if incremental parsing is enabled
            let config = IncrementalConfig::default();
            if config.enabled {
                return self.handle_did_change_incremental_impl(params, config);
            }
        }
        
        // Fall back to original implementation
        self.handle_did_change(params)
    }
    
    #[cfg(feature = "incremental")]
    fn handle_did_change_incremental_impl(
        &self,
        params: Option<Value>,
        config: IncrementalConfig,
    ) -> Result<(), JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let version = params["textDocument"]["version"].as_i64().unwrap_or(0) as i32;
            
            eprintln!("[INCREMENTAL] Document changed: {}", uri);
            
            // Get or create document parser
            let mut documents = self.documents.lock().unwrap();
            
            if let Some(changes) = params["contentChanges"].as_array() {
                // Check if we have an existing incremental document
                let needs_full_parse = !documents.contains_key(uri) || changes.is_empty();
                
                if needs_full_parse {
                    // Full document parse needed
                    if let Some(change) = changes.first() {
                        let text = change["text"].as_str().unwrap_or("");
                        eprintln!("[INCREMENTAL] Creating new document parser for {}", uri);
                        
                        // Create incremental document
                        match DocumentParser::new(text.to_string(), &config) {
                            Ok(doc_parser) => {
                                // Get AST and build parent map
                                let ast_arc = doc_parser.ast();
                                let mut parent_map = ParentMap::default();
                                if let Some(ref arc) = ast_arc {
                                    DeclarationProvider::build_parent_map(&**arc, &mut parent_map, None);
                                }
                                
                                // Log metrics if available
                                if let Some(metrics) = doc_parser.metrics() {
                                    eprintln!("[INCREMENTAL] {}", metrics);
                                }
                                
                                // Create line starts cache
                                let line_starts = LineStartsCache::new(doc_parser.content());
                                
                                // Store as regular DocumentState (temporary)
                                documents.insert(
                                    uri.to_string(),
                                    DocumentState {
                                        content: doc_parser.content().to_string(),
                                        _version: version,
                                        ast: ast_arc,
                                        parse_errors: vec![],
                                        parent_map,
                                        line_starts,
                                    },
                                );
                                
                                // Index symbols
                                if let Err(e) = self.workspace_index.index_file(uri, doc_parser.content(), 0) {
                                    eprintln!("Failed to index file {}: {}", uri, e);
                                }
                            }
                            Err(e) => {
                                eprintln!("[INCREMENTAL] Failed to create document parser: {}", e);
                                // Fall back to regular parsing
                                return self.handle_did_change(Some(params));
                            }
                        }
                    }
                } else {
                    // Incremental updates
                    if let Some(doc) = documents.get(uri) {
                        // Create rope from current content
                        let mut rope = Rope::from_str(&doc.content);
                        let mut content = doc.content.clone();
                        
                        // Apply each change incrementally
                        for change in changes {
                            if let Some(range) = change.get("range") {
                                // Incremental change
                                let start_line = range["start"]["line"].as_u64().unwrap_or(0) as usize;
                                let start_char = range["start"]["character"].as_u64().unwrap_or(0) as usize;
                                let end_line = range["end"]["line"].as_u64().unwrap_or(0) as usize;
                                let end_char = range["end"]["character"].as_u64().unwrap_or(0) as usize;
                                
                                let start_byte = lsp_pos_to_byte(&rope, start_line, start_char);
                                let end_byte = lsp_pos_to_byte(&rope, end_line, end_char);
                                
                                let new_text = change["text"].as_str().unwrap_or("");
                                
                                // Apply change to content
                                let mut new_content = String::with_capacity(content.len() + new_text.len());
                                new_content.push_str(&content[..start_byte]);
                                new_content.push_str(new_text);
                                new_content.push_str(&content[end_byte..]);
                                content = new_content;
                                
                                // Update rope
                                rope = Rope::from_str(&content);
                                
                                eprintln!("[INCREMENTAL] Applied range edit: {}:{} to {}:{} ({} bytes -> {})",
                                    start_line, start_char, end_line, end_char,
                                    end_byte - start_byte, new_text.len());
                            } else {
                                // Full replacement
                                content = change["text"].as_str().unwrap_or("").to_string();
                                rope = Rope::from_str(&content);
                                eprintln!("[INCREMENTAL] Full document replacement");
                            }
                        }
                        
                        // Reparse with new content
                        eprintln!("[INCREMENTAL] Reparsing document");
                        let start = std::time::Instant::now();
                        
                        // For now, do a full reparse until we properly integrate IncrementalDocument
                        let mut parser = Parser::new(&content);
                        let ast = parser.parse().ok().map(Arc::new);
                        
                        let parse_time = start.elapsed();
                        eprintln!("[INCREMENTAL] Parse time: {:.1}ms", parse_time.as_secs_f64() * 1000.0);
                        
                        // Build parent map
                        let mut parent_map = ParentMap::default();
                        if let Some(ref arc) = ast {
                            DeclarationProvider::build_parent_map(&**arc, &mut parent_map, None);
                        }
                        
                        // Update document state
                        let line_starts = LineStartsCache::new(&content);
                        documents.insert(
                            uri.to_string(),
                            DocumentState {
                                content,
                                _version: version,
                                ast,
                                parse_errors: vec![],
                                parent_map,
                                line_starts,
                            },
                        );
                    } else {
                        // Document not found, fall back
                        return self.handle_did_change(Some(params));
                    }
                }
                
                // Release lock before publishing diagnostics
                drop(documents);
                
                // Send diagnostics
                self.publish_diagnostics(uri);
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    #[cfg(feature = "incremental")]
    fn test_incremental_parsing_enabled() {
        // Set environment variable
        std::env::set_var("PERL_LSP_INCREMENTAL", "1");
        
        let config = IncrementalConfig::default();
        assert!(config.enabled);
        
        // Clean up
        std::env::remove_var("PERL_LSP_INCREMENTAL");
    }
    
    #[test]
    #[cfg(feature = "incremental")]
    fn test_incremental_parsing_disabled_by_default() {
        // Ensure variable is not set
        std::env::remove_var("PERL_LSP_INCREMENTAL");
        
        let config = IncrementalConfig::default();
        assert!(!config.enabled);
    }
}