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
use crate::protocol::{req_position, req_uri};
#[cfg(feature = "workspace")]
use crate::runtime::routing::{IndexAccessMode, route_index_access};
use perl_lsp_rs_core::providers::rename::{RenameOptions, RenameProvider, TextEdit as RenameEdit};

/// Returns true if `c` is a Perl variable sigil (`$`, `@`, or `%`).
fn is_perl_sigil(c: char) -> bool {
    matches!(c, '$' | '@' | '%')
}

fn strip_perl_sigil(name: &str) -> &str {
    match name.chars().next() {
        Some(c) if is_perl_sigil(c) => &name[c.len_utf8()..],
        _ => name,
    }
}

fn lexical_declaration_keyword_before(source: &str, symbol_start: usize) -> bool {
    let line_start =
        if symbol_start == 0 { 0 } else { source[..symbol_start].rfind('\n').map_or(0, |p| p + 1) };
    let prefix = source[line_start..symbol_start].trim_end();
    let previous_word =
        prefix.split(|c: char| !c.is_alphanumeric() && c != '_').rfind(|word| !word.is_empty());
    matches!(previous_word, Some("my" | "state"))
}

impl LspServer {
    fn scoped_lexical_rename_edits(
        &self,
        doc: &crate::state::DocumentState,
        ast: &perl_parser_core::Node,
        offset: usize,
        normalized_name: &str,
    ) -> Option<Vec<Value>> {
        if normalized_name.chars().next().is_none_or(|c| !is_perl_sigil(c)) {
            return None;
        }

        let provider = RenameProvider::new(ast, doc.text.clone());
        let result = provider.scoped_rename(
            offset,
            strip_perl_sigil(normalized_name),
            &RenameOptions::default(),
        );
        if !result.is_valid || result.edits.is_empty() {
            return None;
        }
        let lexical_declaration_edit_count =
            result.edits.iter().filter(|edit| self.is_lexical_declaration_edit(doc, edit)).count();
        if lexical_declaration_edit_count != 1 {
            return None;
        }

        Some(
            result
                .edits
                .iter()
                .map(|edit| self.rename_edit_to_lsp_text_edit(doc, edit, normalized_name))
                .collect(),
        )
    }

    fn is_lexical_declaration_edit(
        &self,
        doc: &crate::state::DocumentState,
        edit: &RenameEdit,
    ) -> bool {
        if edit.location.start == 0 || edit.location.start > doc.text.len() {
            return false;
        }
        let Some(prefix) = doc.text.get(..edit.location.start) else {
            return false;
        };
        let Some(previous) = prefix.chars().next_back() else {
            return false;
        };
        if !is_perl_sigil(previous) {
            return false;
        }
        lexical_declaration_keyword_before(&doc.text, edit.location.start - previous.len_utf8())
    }

    fn rename_edit_to_lsp_text_edit(
        &self,
        doc: &crate::state::DocumentState,
        edit: &RenameEdit,
        normalized_name: &str,
    ) -> Value {
        let mut start = edit.location.start;
        let mut new_text = edit.new_text.clone();

        if start > 0
            && let Some(prefix) = doc.text.get(..start)
            && let Some(previous) = prefix.chars().next_back()
            && is_perl_sigil(previous)
        {
            start = start.saturating_sub(previous.len_utf8());
            new_text = normalized_name.to_string();
        }

        let (start_line, start_char) = self.offset_to_pos16(doc, start);
        let (end_line, end_char) = self.offset_to_pos16(doc, edit.location.end);

        json!({
            "range": {
                "start": { "line": start_line, "character": start_char },
                "end": { "line": end_line, "character": end_char }
            },
            "newText": new_text
        })
    }

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

    fn offset_is_inside_quoted_string(content: &str, offset: usize) -> bool {
        let mut in_single = false;
        let mut in_double = false;
        let mut escaped = false;
        let mut in_comment = false;

        for (byte_offset, ch) in content.char_indices() {
            if byte_offset >= offset {
                break;
            }

            if in_comment {
                if ch == '\n' {
                    in_comment = false;
                }
                continue;
            }

            if escaped {
                escaped = false;
                continue;
            }

            if in_single {
                match ch {
                    '\\' => escaped = true,
                    '\'' => in_single = false,
                    _ => {}
                }
                continue;
            }

            if in_double {
                match ch {
                    '\\' => escaped = true,
                    '"' => in_double = false,
                    _ => {}
                }
                continue;
            }

            match ch {
                '#' => in_comment = true,
                '\'' => in_single = true,
                '"' => in_double = true,
                _ => {}
            }
        }

        in_single || in_double
    }

    /// Normalize a rename target against the current symbol, validating the sigil and identifier.
    ///
    /// If `current_symbol` starts with a sigil, the returned name is sigil-prefixed.
    /// If `requested_name` is missing its sigil, the current symbol's sigil is applied.
    /// If `requested_name` has a mismatching sigil, this returns an error.
    fn normalize_rename_target(
        &self,
        current_symbol: Option<&str>,
        requested_name: &str,
    ) -> Result<String, JsonRpcError> {
        if requested_name.is_empty() {
            return Err(JsonRpcError {
                code: -32602,
                message: "Invalid identifier: empty rename target".to_string(),
                data: None,
            });
        }

        let current_sigil =
            current_symbol.and_then(|symbol| symbol.chars().next()).filter(|c| is_perl_sigil(*c));

        match current_sigil {
            Some(sigil) => {
                let mut requested_chars = requested_name.chars();
                let requested_first = requested_chars.next();
                let bare_name = if let Some(first) = requested_first {
                    if is_perl_sigil(first) {
                        if first != sigil {
                            return Err(JsonRpcError {
                                code: -32602,
                                message: format!(
                                    "Invalid identifier: sigil '{}' does not match '{}'",
                                    first, sigil
                                ),
                                data: None,
                            });
                        }
                        requested_chars.collect::<String>()
                    } else {
                        requested_name.to_string()
                    }
                } else {
                    String::new()
                };

                if !self.is_valid_identifier(&bare_name) {
                    return Err(JsonRpcError {
                        code: -32602,
                        message: format!("Invalid identifier: {}", requested_name),
                        data: None,
                    });
                }

                Ok(format!("{}{}", sigil, bare_name))
            }
            None => {
                if !self.is_valid_identifier(requested_name) {
                    return Err(JsonRpcError {
                        code: -32602,
                        message: format!("Invalid identifier: {}", requested_name),
                        data: None,
                    });
                }
                Ok(requested_name.to_string())
            }
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
                    if Self::offset_is_inside_quoted_string(&doc.text, offset) {
                        return Ok(Some(json!(null)));
                    }

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
                let rename_starts_in_quoted_string = {
                    let documents = self.documents_guard();
                    self.get_document(&documents, uri)
                        .map(|doc| {
                            let offset = self.pos16_to_offset(doc, line as u32, ch as u32);
                            Self::offset_is_inside_quoted_string(&doc.text, offset)
                        })
                        .unwrap_or(false)
                };
                if rename_starts_in_quoted_string {
                    return Ok(Some(json!({"changes": {}})));
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
                                crate::declaration::symbol_at_cursor_with_source(
                                    ast,
                                    offset,
                                    current_pkg,
                                    &doc.text,
                                )
                            })
                        })
                    };
                    let current_symbol = {
                        let documents = self.documents_guard();
                        self.get_document(&documents, uri).map(|doc| {
                            let offset = self.pos16_to_offset(doc, line as u32, ch as u32);
                            self.get_token_at_position(&doc.text, offset)
                        })
                    };
                    let normalized_name =
                        self.normalize_rename_target(current_symbol.as_deref(), new_name)?;
                    // build_rename_edit re-applies the sigil from key.sigil, so pass the
                    // bare identifier to avoid double-sigil output like "$$total".
                    let normalized_bare = strip_perl_sigil(&normalized_name);
                    let workspace_symbol_key =
                        symbol_key.as_ref().map(super::to_workspace_symbol_key);

                    match access_mode {
                        IndexAccessMode::Partial(reason) => {
                            if let (Some(coordinator), Some(key)) =
                                (self.coordinator(), workspace_symbol_key.as_ref())
                            {
                                match crate::workspace_rename::build_rename_edit(
                                    coordinator.index(),
                                    key,
                                    normalized_bare,
                                ) {
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
                                    Ok(_) => {
                                        tracing::debug!(
                                            reason,
                                            "Rename: workspace rename unavailable, using same-file only"
                                        );
                                    }
                                    Err(refusal) => {
                                        return Err(JsonRpcError {
                                            code: -32602,
                                            message: refusal.to_string(),
                                            data: None,
                                        });
                                    }
                                }
                            } else {
                                tracing::debug!(
                                    reason,
                                    "Rename: workspace rename unavailable, using same-file only"
                                );
                            }
                            // Fall through to same-file rename
                        }
                        IndexAccessMode::None => {
                            tracing::debug!("Rename: no workspace feature, using same-file only");
                            // Fall through to same-file rename
                        }
                        IndexAccessMode::Full(coordinator) => {
                            if let Some(key) = workspace_symbol_key.as_ref() {
                                // Use coordinator.index() directly instead of workspace_index()
                                // to ensure we go through routing policy
                                let idx = coordinator.index();
                                let edits = crate::workspace_rename::build_rename_edit(
                                    idx,
                                    key,
                                    normalized_bare,
                                )
                                .map_err(|refusal| {
                                    JsonRpcError {
                                        code: -32602,
                                        message: refusal.to_string(),
                                        data: None,
                                    }
                                })?;
                                if edits.is_empty() {
                                    // Fall through to same-file rename
                                } else {
                                    let ws_edit = crate::workspace_rename::to_workspace_edit(edits);
                                    return Ok(Some(ws_edit));
                                }
                            }
                        }
                    }
                }

                // Same-file fallback for degraded/partial modes
                let documents = self.documents_guard();
                if let Some(doc) = self.get_document(&documents, uri) {
                    if let Some(ref ast) = doc.ast {
                        let offset = self.pos16_to_offset(doc, line as u32, ch as u32);
                        let current_symbol = self.get_token_at_position(&doc.text, offset);
                        let normalized_name =
                            self.normalize_rename_target(Some(current_symbol.as_str()), new_name)?;

                        if let Some(edits) =
                            self.scoped_lexical_rename_edits(doc, ast, offset, &normalized_name)
                        {
                            return Ok(Some(json!({
                                "changes": {
                                    uri: edits
                                }
                            })));
                        }

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
                                    "newText": normalized_name
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

    #[test]
    fn rename_guard_detects_dynamic_typeglob_string_positions()
    -> Result<(), Box<dyn std::error::Error>> {
        let text = r#"*{"Mojolicious::Routes::Route::$name"} = sub { $cb->(@_) };"#;
        let string_offset = text.find("Routes::Route").ok_or("missing dynamic package")? + 2;
        let code_offset = text.find("$cb").ok_or("missing callback")? + 1;

        assert!(LspServer::offset_is_inside_quoted_string(text, string_offset));
        assert!(!LspServer::offset_is_inside_quoted_string(text, code_offset));
        Ok(())
    }

    #[test]
    fn rename_guard_uses_byte_offsets_with_unicode_prefix() -> Result<(), Box<dyn std::error::Error>>
    {
        let text = "my $emoji = \"🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀\";\n*{\"Mojolicious::Routes::Route::$name\"} = sub { $cb->(@_) };";
        let string_offset = text.find("Routes::Route").ok_or("missing dynamic package")? + 2;
        let code_offset = text.find("$cb").ok_or("missing callback")? + 1;

        assert!(LspServer::offset_is_inside_quoted_string(text, string_offset));
        assert!(!LspServer::offset_is_inside_quoted_string(text, code_offset));
        Ok(())
    }
}
