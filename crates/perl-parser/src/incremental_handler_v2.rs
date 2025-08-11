//! Simplified incremental parsing handler
//! 
//! This module provides incremental didChange handling without tree-sitter dependency

use crate::{
    lsp_server::{JsonRpcError, LspServer},
    parser::Parser,
    position_mapper::{PositionMapper, apply_edit_utf8, Position},
    positions::LineStartsCache,
};
use serde_json::Value;
use std::sync::Arc;

impl LspServer {
    /// Handle didChange with incremental text updates
    #[cfg(feature = "incremental")]
    pub(crate) fn handle_did_change_incremental(&self, params: Option<Value>) -> Result<(), JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError { code: -32602, message: "Missing params".to_string(), data: None })?;
        let uri = params["textDocument"]["uri"]
            .as_str()
            .ok_or_else(|| JsonRpcError { code: -32602, message: "Invalid URI".to_string(), data: None })?
            .to_string();
        
        let changes = params["contentChanges"]
            .as_array()
            .ok_or_else(|| JsonRpcError { code: -32602, message: "Invalid changes".to_string(), data: None })?;
        
        // Apply edits and get new text
        let new_text = {
            let mut docs = self.documents.lock().unwrap();
            let doc = docs.get_mut(&uri)
                .ok_or_else(|| JsonRpcError { code: -32602, message: "Document not found".to_string(), data: None })?;
            
            // Check if it's a full document update
            if changes.len() == 1 && changes[0].get("range").is_none() {
                // Full document replacement
                let text = changes[0]["text"].as_str().unwrap_or_default();
                doc.content = text.to_string();
            } else {
                // Incremental updates
                for change in changes {
                    if let Some(range) = change.get("range") {
                        let text = change["text"].as_str().unwrap_or_default();
                        let mapper = PositionMapper::new(&doc.content);
                        
                        // Convert LSP positions to byte offsets
                        let start_pos = Position {
                            line: range["start"]["line"].as_u64().unwrap_or(0) as u32,
                            character: range["start"]["character"].as_u64().unwrap_or(0) as u32,
                        };
                        let end_pos = Position {
                            line: range["end"]["line"].as_u64().unwrap_or(0) as u32,
                            character: range["end"]["character"].as_u64().unwrap_or(0) as u32,
                        };
                        
                        if let (Some(start_byte), Some(end_byte)) = 
                            (mapper.lsp_pos_to_byte(start_pos), 
                             mapper.lsp_pos_to_byte(end_pos)) {
                            apply_edit_utf8(&mut doc.content, start_byte, end_byte, text);
                        }
                    }
                }
            }
            
            doc.content.clone()
        }; // lock dropped
        
        // Parse the updated text
        let mut parser = Parser::new(&new_text);
        let ast_result = parser.parse();
        
        // Update document state
        {
            let mut docs = self.documents.lock().unwrap();
            if let Some(doc) = docs.get_mut(&uri) {
                match ast_result {
                    Ok(ast) => {
                        let ast_arc = Arc::new(ast);
                        doc.ast = Some(ast_arc.clone());
                        doc.parent_map.clear();
                        crate::declaration::DeclarationProvider::build_parent_map(&ast_arc, &mut doc.parent_map, None);
                        doc.parse_errors.clear();
                    }
                    Err(e) => {
                        doc.ast = None;
                        doc.parse_errors = vec![e];
                    }
                }
                doc.line_starts = LineStartsCache::new(&doc.content);
            }
        }
        
        // Publish diagnostics without holding lock
        self.publish_diagnostics(&uri);
        Ok(())
    }
}

// Fallback for non-incremental builds
#[cfg(not(feature = "incremental"))]
impl LspServer {
    pub(crate) fn handle_did_change_incremental(&self, params: Option<Value>) -> Result<(), JsonRpcError> {
        self.handle_did_change(params)
    }
}