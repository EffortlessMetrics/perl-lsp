//! Rename handlers for symbol renaming
//!
//! Handles textDocument/prepareRename and textDocument/rename requests.
//! Supports both single-file and workspace-wide renaming.
//!
//! # Lifecycle-Aware Behavior
//!
//! Uses the routing module for state-aware dispatch:
//! - **Ready state**: Full workspace rename across all indexed files
//! - **Building/Degraded state**: Same-file rename only; logs "workspace rename unavailable while index building"

use super::super::*;
use crate::protocol::{invalid_params, req_position, req_uri};
#[cfg(feature = "workspace")]
use crate::runtime::routing::{IndexAccessMode, route_index_access};

impl LspServer {
    fn normalized_new_name<'a>(&self, new_name: &'a str) -> Option<&'a str> {
        let bare = match new_name.chars().next() {
            Some('$' | '@' | '%') => &new_name[1..],
            _ => new_name,
        };

        if self.is_valid_identifier(bare) {
            Some(bare)
        } else {
            None
        }
    }

    fn replacement_for_token(&self, token_text: &str, new_name_bare: &str) -> String {
        let first = token_text.chars().next();
        match first {
            Some('$' | '@' | '%') => {
                let mut replacement = String::with_capacity(new_name_bare.len() + 1);
                if let Some(ch) = first {
                    replacement.push(ch);
                }
                replacement.push_str(new_name_bare);
                replacement
            }
            _ => new_name_bare.to_string(),
        }
    }

    /// Handle textDocument/prepareRename request
    pub(crate) fn handle_prepare_rename(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(_ast) = &doc.ast {
                    let offset = self.pos16_to_offset(doc, line, character);

                    // Get the token at the current position
                    let token = self.get_token_at_position(&doc.text, offset);
                    if !token.is_empty()
                        && (token.starts_with('$')
                            || token.starts_with('@')
                            || token.starts_with('%')
                            || token.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_'))
                    {
                        // Find the token bounds
                        let (start_offset, end_offset) = self.get_token_bounds(&doc.text, offset);
                        let (start_line, start_char) = self.offset_to_pos16(doc, start_offset);
                        let (end_line, end_char) = self.offset_to_pos16(doc, end_offset);

                        // Return the range and placeholder text
                        return Ok(Some(json!({
                            "range": {
                                "start": {
                                    "line": start_line,
                                    "character": start_char
                                },
                                "end": {
                                    "line": end_line,
                                    "character": end_char
                                }
                            },
                            "placeholder": token
                        })));
                    }
                }
            }
        }

        // Return null if rename is not possible at this position
        Ok(Some(json!(null)))
    }

    /// Handle textDocument/rename request (single file)
    #[allow(dead_code)] // Dispatch uses handle_rename_workspace instead
    pub(crate) fn handle_rename(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;
            let new_name = params["newName"]
                .as_str()
                .ok_or_else(|| invalid_params("Missing required parameter: newName"))?;
            let new_name_bare = self.normalized_new_name(new_name).ok_or_else(|| JsonRpcError {
                code: -32602,
                message: format!("Invalid identifier: {}", new_name),
                data: None,
            })?;

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let offset = self.pos16_to_offset(doc, line, character);

                    // Create semantic analyzer
                    let analyzer = crate::semantic::SemanticAnalyzer::analyze(ast);

                    // Find all references (including definition)
                    let references = analyzer.find_all_references(offset, true);

                    if !references.is_empty() {
                        // Create text edits for all references
                        let mut edits = Vec::new();
                        for location in references {
                            let (start_line, start_char) =
                                self.offset_to_pos16(doc, location.start);
                            let (end_line, end_char) = self.offset_to_pos16(doc, location.end);
                            let token_text = doc
                                .text
                                .get(location.start..location.end)
                                .unwrap_or_default();
                            let replacement = self.replacement_for_token(token_text, new_name_bare);

                            edits.push(json!({
                                "range": {
                                    "start": { "line": start_line, "character": start_char },
                                    "end": { "line": end_line, "character": end_char }
                                },
                                "newText": replacement
                            }));
                        }

                        // Return WorkspaceEdit
                        return Ok(Some(json!({
                            "changes": {
                                uri: edits
                            }
                        })));
                    }
                }
            }
        }

        // Return empty workspace edit if nothing to rename
        Ok(Some(json!({
            "changes": {}
        })))
    }

    /// Handle textDocument/rename request with workspace support
    ///
    /// Uses routing helper for lifecycle-aware behavior:
    /// - **Ready state**: Full workspace rename across all indexed files
    /// - **Building/Degraded state**: Same-file rename only with warning log
    pub(crate) fn handle_rename_workspace(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(p) = params {
            if let (Some(uri), Some(line), Some(ch), Some(new_name)) = (
                p.get("textDocument").and_then(|t| t.get("uri")).and_then(|s| s.as_str()),
                p.get("position").and_then(|p| p.get("line")).and_then(|n| n.as_u64()),
                p.get("position").and_then(|p| p.get("character")).and_then(|n| n.as_u64()),
                p.get("newName").and_then(|s| s.as_str()),
            ) {
                let new_name_bare =
                    self.normalized_new_name(new_name).ok_or_else(|| JsonRpcError {
                        code: -32602,
                        message: format!("Invalid identifier: {}", new_name),
                        data: None,
                    })?;

                // Check index access mode using routing helper
                #[cfg(feature = "workspace")]
                {
                    let access_mode = route_index_access(self.coordinator());
                    let symbol_key = {
                        let documents = self.documents_guard();
                        self.get_document(&documents, uri).and_then(|doc| {
                            doc.ast.as_ref().and_then(|ast| {
                                let offset = self.pos16_to_offset(doc, line as u32, ch as u32);
                                let current_pkg =
                                    crate::declaration::current_package_at(ast, offset);
                                crate::declaration::symbol_at_cursor(ast, offset, current_pkg)
                            })
                        })
                    };

                    match access_mode {
                        IndexAccessMode::Partial(reason) => {
                            if let (Some(coordinator), Some(key)) =
                                (self.coordinator(), symbol_key.as_ref())
                            {
                                let edits = crate::workspace_rename::build_rename_edit(
                                    coordinator.index(),
                                    key,
                                    new_name_bare,
                                );
                                if !edits.is_empty() {
                                    tracing::debug!(
                                        count = edits.len(),
                                        reason,
                                        "Rename: served partial-index workspace edits"
                                    );
                                    return Ok(Some(crate::workspace_rename::to_workspace_edit(
                                        edits,
                                    )));
                                }
                            }
                            tracing::debug!(
                                reason,
                                "Rename: workspace rename unavailable, using same-file only"
                            );
                            // Fall through to same-file rename
                        }
                        IndexAccessMode::None => {
                            tracing::debug!("Rename: no workspace feature, using same-file only");
                            // Fall through to same-file rename
                        }
                        IndexAccessMode::Full(coordinator) => {
                            if let Some(key) = symbol_key.as_ref() {
                                // Use coordinator.index() directly instead of workspace_index()
                                // to ensure we go through routing policy
                                let idx = coordinator.index();
                                let edits =
                                    crate::workspace_rename::build_rename_edit(
                                        idx,
                                        key,
                                        new_name_bare,
                                    );
                                let ws_edit = crate::workspace_rename::to_workspace_edit(edits);
                                return Ok(Some(ws_edit));
                            }
                        }
                    }
                }

                // Same-file fallback for degraded/partial modes
                let documents = self.documents_guard();
                if let Some(doc) = self.get_document(&documents, uri) {
                    if let Some(ref ast) = doc.ast {
                        let offset = self.pos16_to_offset(doc, line as u32, ch as u32);

                        // Create semantic analyzer for same-file rename
                        let analyzer = crate::semantic::SemanticAnalyzer::analyze(ast);

                        // Find all references (including definition)
                        let references = analyzer.find_all_references(offset, true);

                        if !references.is_empty() {
                            // Create text edits for all references
                            let mut edits = Vec::new();
                            for location in references {
                                let (start_line, start_char) =
                                    self.offset_to_pos16(doc, location.start);
                                let (end_line, end_char) = self.offset_to_pos16(doc, location.end);
                                let token_text = doc
                                    .text
                                    .get(location.start..location.end)
                                    .unwrap_or_default();
                                let replacement =
                                    self.replacement_for_token(token_text, new_name_bare);

                                edits.push(json!({
                                    "range": {
                                        "start": { "line": start_line, "character": start_char },
                                        "end": { "line": end_line, "character": end_char }
                                    },
                                    "newText": replacement
                                }));
                            }

                            // Return WorkspaceEdit with same-file changes only
                            return Ok(Some(json!({
                                "changes": {
                                    uri: edits
                                }
                            })));
                        }
                    }
                }
            }
        }
        // Return empty edit if we can't resolve
        Ok(Some(json!({"changes": {}})))
    }

    /// Validate if a string is a valid Perl identifier
    #[allow(dead_code)] // Used by handle_rename which is currently not dispatched
    pub(crate) fn is_valid_identifier(&self, name: &str) -> bool {
        if name.is_empty() {
            return false;
        }

        let chars: Vec<char> = name.chars().collect();

        // First character must be letter or underscore
        let first_char = match chars.first() {
            Some(c) => c,
            None => return false, // Empty string is not a valid identifier
        };
        if !first_char.is_alphabetic() && *first_char != '_' {
            return false;
        }

        // Rest must be alphanumeric or underscore
        for ch in &chars[1..] {
            if !ch.is_alphanumeric() && *ch != '_' {
                return false;
            }
        }

        true
    }

    /// Get token at position (simple implementation)
    pub(crate) fn get_token_at_position(&self, content: &str, offset: usize) -> String {
        let chars: Vec<char> = content.chars().collect();
        if offset >= chars.len() {
            return String::new();
        }

        // Find word boundaries
        let mut start = offset;
        while start > 0
            && (chars[start - 1].is_alphanumeric()
                || chars[start - 1] == '_'
                || chars[start - 1] == '$'
                || chars[start - 1] == '@'
                || chars[start - 1] == '%')
        {
            start -= 1;
        }

        let mut end = offset;
        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
            end += 1;
        }

        chars[start..end].iter().collect()
    }

    /// Get the bounds of the token at the given position
    pub(crate) fn get_token_bounds(&self, content: &str, offset: usize) -> (usize, usize) {
        let chars: Vec<char> = content.chars().collect();
        if offset >= chars.len() {
            return (offset, offset);
        }

        // Find word boundaries
        let mut start = offset;
        while start > 0
            && (chars[start - 1].is_alphanumeric()
                || chars[start - 1] == '_'
                || chars[start - 1] == '$'
                || chars[start - 1] == '@'
                || chars[start - 1] == '%')
        {
            start -= 1;
        }

        let mut end = offset;
        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
            end += 1;
        }

        (start, end)
    }
}
