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
    fn token_span_at(content: &str, offset: usize) -> Option<(usize, usize)> {
        let chars: Vec<char> = content.chars().collect();
        if chars.is_empty() {
            return None;
        }

        let is_ident_char = |ch: char| ch.is_alphanumeric() || ch == '_';
        let is_sigil = |ch: char| ch == '$' || ch == '@' || ch == '%';

        // Allow cursor-at-end and cursor-next-to-token positions by probing the
        // previous character when needed.
        let mut probe = offset.min(chars.len().saturating_sub(1));
        if offset == chars.len()
            || (!is_ident_char(chars[probe])
                && !is_sigil(chars[probe])
                && probe > 0
                && (is_ident_char(chars[probe - 1]) || is_sigil(chars[probe - 1])))
        {
            probe = probe.saturating_sub(1);
        }

        if !is_ident_char(chars[probe]) && !is_sigil(chars[probe]) {
            return None;
        }

        // Skip from sigil to identifier body when the cursor is on sigil.
        let mut start = probe;
        if is_sigil(chars[start]) && start + 1 < chars.len() && is_ident_char(chars[start + 1]) {
            start += 1;
        }

        while start > 0 && is_ident_char(chars[start - 1]) {
            start -= 1;
        }
        if start > 0 && is_sigil(chars[start - 1]) {
            start -= 1;
        }

        let mut end = start;
        if is_sigil(chars[end]) {
            end += 1;
        }
        while end < chars.len() && is_ident_char(chars[end]) {
            end += 1;
        }

        // Require at least one identifier character so we don't rename standalone sigils.
        let body_start = if is_sigil(chars[start]) { start + 1 } else { start };
        if body_start >= end {
            return None;
        }

        Some((start, end))
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

            // Validate the new name
            if !self.is_valid_identifier(new_name) {
                return Err(JsonRpcError {
                    code: -32602,
                    message: format!("Invalid identifier: {}", new_name),
                    data: None,
                });
            }

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

                            edits.push(json!({
                                "range": {
                                    "start": { "line": start_line, "character": start_char },
                                    "end": { "line": end_line, "character": end_char }
                                },
                                "newText": new_name
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
                if !self.is_valid_identifier(new_name) {
                    return Err(JsonRpcError {
                        code: -32602,
                        message: format!("Invalid identifier: {}", new_name),
                        data: None,
                    });
                }

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
                                let planned = crate::workspace_rename::plan_workspace_rename(
                                    coordinator.index(),
                                    key,
                                    new_name,
                                );
                                match planned {
                                    Ok(edits) if !edits.is_empty() => {
                                        tracing::debug!(
                                            count = edits.len(),
                                            reason,
                                            "Rename: served partial-index workspace edits"
                                        );
                                        return Ok(Some(
                                            crate::workspace_rename::to_workspace_edit(edits),
                                        ));
                                    }
                                    Ok(_) => {}
                                    Err(refusal) => {
                                        return Err(JsonRpcError {
                                            code: -32803,
                                            message: refusal.to_string(),
                                            data: None,
                                        });
                                    }
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
                                let edits = crate::workspace_rename::plan_workspace_rename(
                                    idx, key, new_name,
                                )
                                .map_err(|refusal| {
                                    JsonRpcError {
                                        code: -32803,
                                        message: refusal.to_string(),
                                        data: None,
                                    }
                                })?;
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

                                edits.push(json!({
                                    "range": {
                                        "start": { "line": start_line, "character": start_char },
                                        "end": { "line": end_line, "character": end_char }
                                    },
                                    "newText": new_name
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
        if chars.is_empty() || offset > chars.len() {
            return String::new();
        }
        match Self::token_span_at(content, offset) {
            Some((start, end)) => chars[start..end].iter().collect(),
            None => String::new(),
        }
    }

    /// Get the bounds of the token at the given position
    pub(crate) fn get_token_bounds(&self, content: &str, offset: usize) -> (usize, usize) {
        let chars: Vec<char> = content.chars().collect();
        if chars.is_empty() || offset > chars.len() {
            return (offset, offset);
        }
        Self::token_span_at(content, offset).unwrap_or((offset, offset))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_helpers_support_cursor_on_sigil() -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::default();
        let text = "my $value = 1;";
        let offset = text.find('$').ok_or("missing sigil")?;

        let token = server.get_token_at_position(text, offset);
        let (start, end) = server.get_token_bounds(text, offset);

        assert_eq!(token, "$value");
        assert_eq!(
            &text.chars().collect::<Vec<_>>()[start..end].iter().collect::<String>(),
            "$value"
        );
        Ok(())
    }

    #[test]
    fn token_helpers_support_cursor_after_identifier() -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::default();
        let text = "my $value = 1;";
        let offset = text.find("$value").ok_or("missing variable")? + "$value".len();

        let token = server.get_token_at_position(text, offset);
        let (start, end) = server.get_token_bounds(text, offset);

        assert_eq!(token, "$value");
        assert_eq!(
            &text.chars().collect::<Vec<_>>()[start..end].iter().collect::<String>(),
            "$value"
        );
        Ok(())
    }
}
