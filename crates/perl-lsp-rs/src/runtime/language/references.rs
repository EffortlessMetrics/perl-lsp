//! Reference handlers for find references and document highlights
//!
//! Handles textDocument/references and textDocument/documentHighlight requests.
//!
//! # Lifecycle-Aware Behavior
//!
//! Uses `IndexCoordinator` for state-aware dispatch:
//! - **Ready state**: Full workspace index + text search across all files
//! - **Building/Degraded state**: Same-file semantic analysis + open document scan

use super::super::{byte_to_utf16_col, *};
use crate::protocol::{req_position, req_uri};
use crate::state::{reference_search_deadline, references_cap};
use crate::util::{is_word_boundary, token_under_cursor};
use std::sync::OnceLock;
use std::time::Instant;

#[cfg(feature = "workspace")]
use crate::runtime::routing::{IndexAccessMode, route_index_access};

static QUALIFIED_NAME_RE: OnceLock<Result<regex::Regex, regex::Error>> = OnceLock::new();

fn get_qualified_name_regex() -> Option<&'static regex::Regex> {
    QUALIFIED_NAME_RE
        .get_or_init(|| regex::Regex::new(r"([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)"))
        .as_ref()
        .ok()
}

impl LspServer {
    /// Handle textDocument/references request with lifecycle-aware dispatch
    ///
    /// Uses `IndexCoordinator` for state-aware behavior:
    /// - **Ready state**: Full workspace index search + text-based fallback
    /// - **Building/Degraded state**: Same-file semantic analysis only
    ///
    /// Includes deadline enforcement to prevent blocking on large workspaces.
    pub(crate) fn handle_references(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let start = Instant::now();
        let deadline = reference_search_deadline();
        let cap = references_cap();

        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;
            let include_declaration = if let Some(context) = params.get("context") {
                context["includeDeclaration"].as_bool().unwrap_or(true)
            } else {
                true
            };

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let offset = self.pos16_to_offset(doc, line, character);
                    let needle = token_under_cursor(&doc.text, line as usize, character as usize)
                        .unwrap_or_default();

                    let current_package = crate::declaration::current_package_at(ast, offset);
                    let symbol_key = crate::declaration::symbol_at_cursor_with_source(
                        ast,
                        offset,
                        current_package,
                        &doc.text,
                    );

                    // Check index state and use appropriate search strategy
                    #[cfg(feature = "workspace")]
                    {
                        let access_mode = route_index_access(self.coordinator());

                        match access_mode {
                            IndexAccessMode::Full(coordinator) => {
                                let index = coordinator.index();
                                if let Some(symbol_key) = symbol_key.as_ref() {
                                    tracing::debug!(key = ?symbol_key, "Looking for references");

                                    // Try to find references using the symbol key
                                    let mut all_refs = index.find_refs(symbol_key);

                                    // Add the definition if includeDeclaration is true
                                    if include_declaration {
                                        if let Some(def) = index.find_def(symbol_key) {
                                            all_refs.push(def);
                                        }
                                    }

                                    let mut workspace_locations: Vec<Value> = Vec::new();
                                    if !all_refs.is_empty() {
                                        tracing::debug!(
                                            count = all_refs.len(),
                                            "Found references via find_refs"
                                        );
                                        // Convert internal Locations to LSP Locations
                                        let lsp_locations =
                                            crate::workspace_index::lsp_adapter::to_lsp_locations(
                                                all_refs,
                                            );
                                        for loc in lsp_locations {
                                            workspace_locations.push(json!(loc));
                                        }
                                    }

                                    // Check deadline before text search
                                    if start.elapsed() >= deadline {
                                        tracing::debug!(
                                            "References: deadline exceeded, returning partial results"
                                        );
                                        workspace_locations.truncate(cap);
                                        return Ok(Some(json!(workspace_locations)));
                                    }

                                    // Enhanced fallback: always search for both qualified and unqualified references
                                    // Snapshot only (uri, text) to minimize cloning overhead - we don't need
                                    // AST, rope, or other DocumentState fields for text search
                                    let docs_snapshot: Vec<(String, String)> = documents
                                        .iter()
                                        .map(|(k, v)| (k.clone(), v.text.clone()))
                                        .collect();

                                    let mut enhanced_locations = Vec::new();
                                    let symbol_name = &symbol_key.name;
                                    let package_name = &symbol_key.pkg;

                                    // Search patterns: both "symbol_name" and "package::symbol_name"
                                    let patterns = vec![
                                        format!(r"\b{}\b", regex::escape(symbol_name)),
                                        format!(
                                            r"\b{}::{}\b",
                                            regex::escape(package_name),
                                            regex::escape(symbol_name)
                                        ),
                                    ];

                                    'pattern_loop: for pattern in patterns {
                                        // Check deadline between patterns
                                        if start.elapsed() >= deadline {
                                            tracing::debug!(
                                                "References: deadline exceeded during text search"
                                            );
                                            break 'pattern_loop;
                                        }
                                        if let Ok(search_regex) = regex::Regex::new(&pattern) {
                                            for (doc_uri, doc_text) in &docs_snapshot {
                                                // Early exit on cap
                                                if enhanced_locations.len() >= cap {
                                                    break 'pattern_loop;
                                                }
                                                let lines: Vec<&str> = doc_text.lines().collect();
                                                for (line_num, line) in lines.iter().enumerate() {
                                                    for mat in search_regex.find_iter(line) {
                                                        // Convert byte offsets to UTF-16 columns for LSP compliance
                                                        let start_utf16 =
                                                            byte_to_utf16_col(line, mat.start());
                                                        let end_utf16 =
                                                            byte_to_utf16_col(line, mat.end());
                                                        enhanced_locations.push(json!({
                                                            "uri": doc_uri,
                                                            "range": {
                                                                "start": {
                                                                    "line": line_num,
                                                                    "character": start_utf16,
                                                                },
                                                                "end": {
                                                                    "line": line_num,
                                                                    "character": end_utf16,
                                                                },
                                                            },
                                                        }));
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // Combine workspace index results with text search results
                                    workspace_locations.extend(enhanced_locations);
                                    let mut all_combined_locations = workspace_locations;
                                    // Cap results
                                    all_combined_locations.truncate(cap);

                                    if !all_combined_locations.is_empty() {
                                        tracing::debug!(
                                            count = all_combined_locations.len(),
                                            cap,
                                            elapsed = ?start.elapsed(),
                                            "Found total references via combined search"
                                        );
                                        return Ok(Some(json!(all_combined_locations)));
                                    }

                                    // Also try with find_references for backward compatibility
                                    let symbol_name = if symbol_key.kind
                                        == crate::workspace_index::SymKind::Sub
                                    {
                                        format!("{}::{}", symbol_key.pkg, symbol_key.name)
                                    } else {
                                        symbol_key.name.to_string()
                                    };

                                    let refs = index.find_references(&symbol_name);
                                    if !refs.is_empty() {
                                        // Cap results before conversion
                                        let capped_refs: Vec<_> =
                                            refs.into_iter().take(cap).collect();
                                        tracing::debug!(
                                            count = capped_refs.len(),
                                            symbol = %symbol_name,
                                            cap,
                                            "Found references via find_references"
                                        );
                                        // Convert internal Locations to LSP Locations
                                        let lsp_locations =
                                            crate::workspace_index::lsp_adapter::to_lsp_locations(
                                                capped_refs,
                                            );
                                        if !lsp_locations.is_empty() {
                                            return Ok(Some(json!(lsp_locations)));
                                        }
                                    }
                                }

                                // Regex-based fallback for fully-qualified symbols like Package::sub references
                                let radius = 50;
                                let text_start = offset.saturating_sub(radius);
                                let text_around =
                                    self.get_text_around_offset(&doc.text, offset, radius);
                                let cursor_in_text = offset - text_start;

                                // Use cached regex to avoid per-request compilation overhead
                                if let Some(qualified_name_re) = get_qualified_name_regex() {
                                    for captures in qualified_name_re.captures_iter(&text_around) {
                                        if let Some(m) = captures.get(1) {
                                            if cursor_in_text >= m.start()
                                                && cursor_in_text <= m.end()
                                            {
                                                let parts: Vec<&str> =
                                                    m.as_str().split("::").collect();
                                                if parts.len() >= 2 {
                                                    let name = parts
                                                        .last()
                                                        .copied()
                                                        .unwrap_or("")
                                                        .to_string();
                                                    let pkg = parts[..parts.len() - 1].join("::");
                                                    let key = crate::workspace_index::SymbolKey {
                                                        pkg: pkg.clone().into(),
                                                        name: name.clone().into(),
                                                        sigil: None,
                                                        kind: crate::workspace_index::SymKind::Sub,
                                                    };

                                                    // Search for all references to this qualified symbol
                                                    let mut all_refs = Vec::new();

                                                    // Find references via symbol key
                                                    let refs = index.find_refs(&key);
                                                    all_refs.extend(refs);

                                                    // Also try with qualified name
                                                    let symbol_name = format!("{}::{}", pkg, name);
                                                    let alt_refs =
                                                        index.find_references(&symbol_name);
                                                    all_refs.extend(alt_refs);

                                                    // Add definition if includeDeclaration is true
                                                    if include_declaration {
                                                        if let Some(def) = index.find_def(&key) {
                                                            all_refs.push(def);
                                                        }
                                                    }

                                                    if !all_refs.is_empty() {
                                                        // Cap results
                                                        let capped_refs: Vec<_> = all_refs
                                                            .into_iter()
                                                            .take(cap)
                                                            .collect();
                                                        // Convert internal Locations to LSP Locations
                                                        let lsp_locations =
                                                    crate::workspace_index::lsp_adapter::to_lsp_locations(capped_refs);
                                                        if !lsp_locations.is_empty() {
                                                            return Ok(Some(json!(lsp_locations)));
                                                        }
                                                    }

                                                    // Fallback: scan open documents for qualified name references
                                                    // Snapshot only (uri, text) to minimize cloning overhead
                                                    let docs_snapshot: Vec<(String, String)> =
                                                        documents
                                                            .iter()
                                                            .map(|(k, v)| {
                                                                (k.clone(), v.text.clone())
                                                            })
                                                            .collect();

                                                    let mut all_locations = Vec::new();
                                                    let qualified_name =
                                                        format!("{}::{}", pkg, name);
                                                    let Ok(search_regex) =
                                                        regex::Regex::new(&format!(
                                                            r"\b{}\b",
                                                            regex::escape(&qualified_name)
                                                        ))
                                                    else {
                                                        continue;
                                                    };

                                                    'doc_scan: for (doc_uri, doc_text) in
                                                        docs_snapshot
                                                    {
                                                        // Check deadline
                                                        if start.elapsed() >= deadline {
                                                            break 'doc_scan;
                                                        }
                                                        let lines: Vec<&str> =
                                                            doc_text.lines().collect();
                                                        for (line_num, line) in
                                                            lines.iter().enumerate()
                                                        {
                                                            for mat in search_regex.find_iter(line)
                                                            {
                                                                // Convert byte offsets to UTF-16 columns for LSP compliance
                                                                let start_utf16 = byte_to_utf16_col(
                                                                    line,
                                                                    mat.start(),
                                                                );
                                                                let end_utf16 = byte_to_utf16_col(
                                                                    line,
                                                                    mat.end(),
                                                                );
                                                                all_locations.push(json!({
                                                                "uri": doc_uri,
                                                                "range": {
                                                                    "start": {
                                                                        "line": line_num,
                                                                        "character": start_utf16,
                                                                    },
                                                                    "end": {
                                                                        "line": line_num,
                                                                        "character": end_utf16,
                                                                    },
                                                                },
                                                            }));
                                                                // Early exit if we hit the cap
                                                                if all_locations.len() >= cap {
                                                                    break 'doc_scan;
                                                                }
                                                            }
                                                        }
                                                    }

                                                    if !all_locations.is_empty() {
                                                        // Truncate to cap
                                                        all_locations.truncate(cap);
                                                        return Ok(Some(json!(all_locations)));
                                                    }
                                                }
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                            IndexAccessMode::Partial(reason) => {
                                tracing::debug!(
                                    reason,
                                    "References: attempting partial workspace lookup"
                                );
                                if let (Some(coordinator), Some(symbol_key)) =
                                    (self.coordinator(), symbol_key.as_ref())
                                {
                                    let index = coordinator.index();
                                    let mut partial_refs = index.find_refs(symbol_key);

                                    if include_declaration
                                        && let Some(def) = index.find_def(symbol_key)
                                    {
                                        partial_refs.push(def);
                                    }

                                    if !partial_refs.is_empty() {
                                        let lsp_locations =
                                            crate::workspace_index::lsp_adapter::to_lsp_locations(
                                                partial_refs.into_iter().take(cap),
                                            );
                                        if !lsp_locations.is_empty() {
                                            tracing::debug!(
                                                count = lsp_locations.len(),
                                                elapsed = ?start.elapsed(),
                                                "References: returned partial-index results"
                                            );
                                            return Ok(Some(json!(lsp_locations)));
                                        }
                                    }
                                }

                                tracing::debug!(reason, "References: using same-file fallback");
                                if !needle.is_empty() {
                                    let open_doc_locations =
                                        self.search_open_document_references(&needle, cap);
                                    if !open_doc_locations.is_empty() {
                                        tracing::debug!(
                                            count = open_doc_locations.len(),
                                            elapsed = ?start.elapsed(),
                                            "References: returned open-document results"
                                        );
                                        return Ok(Some(json!(open_doc_locations)));
                                    }
                                }
                                // Fall through to same-file semantic analysis
                            }
                            IndexAccessMode::None => {
                                // Fall through to same-file semantic analysis
                            }
                        }
                    }

                    // Fall back to same-file references
                    let analyzer = crate::semantic::SemanticAnalyzer::analyze(ast);

                    // Find all references at the position
                    let references = analyzer.find_all_references(offset, include_declaration);

                    if !references.is_empty() {
                        // Cap same-file references
                        let locations: Vec<Value> = references
                            .iter()
                            .take(cap)
                            .map(|loc| {
                                let (start_line, start_char) = self.offset_to_pos16(doc, loc.start);
                                let (end_line, end_char) = self.offset_to_pos16(doc, loc.end);

                                json!({
                                    "uri": uri,
                                    "range": {
                                        "start": {
                                            "line": start_line,
                                            "character": start_char,
                                        },
                                        "end": {
                                            "line": end_line,
                                            "character": end_char,
                                        },
                                    },
                                })
                            })
                            .collect();

                        tracing::debug!(
                            count = locations.len(),
                            elapsed = ?start.elapsed(),
                            "References: returned same-file results"
                        );
                        return Ok(Some(json!(locations)));
                    }
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Search open documents for references to a token using word-boundary matching.
    #[cfg(feature = "workspace")]
    fn search_open_document_references(&self, needle: &str, cap: usize) -> Vec<Value> {
        if needle.is_empty() {
            return Vec::new();
        }

        let needle_bytes = needle.as_bytes();
        let mut out = Vec::new();

        for (doc_uri, doc_text) in self.iter_open_buffers() {
            if out.len() >= cap {
                break;
            }

            for (line_num, line) in doc_text.lines().enumerate() {
                let line_bytes = line.as_bytes();
                let mut start = 0usize;
                while let Some(idx) = line[start..].find(needle) {
                    let byte_pos = start + idx;
                    if is_word_boundary(line_bytes, byte_pos, needle_bytes.len()) {
                        let start_utf16 = byte_to_utf16_col(line, byte_pos);
                        let end_utf16 = byte_to_utf16_col(line, byte_pos + needle_bytes.len());
                        out.push(json!({
                            "uri": doc_uri,
                            "range": {
                                "start": {
                                    "line": line_num,
                                    "character": start_utf16,
                                },
                                "end": {
                                    "line": line_num,
                                    "character": end_utf16,
                                },
                            },
                        }));
                        if out.len() >= cap {
                            break;
                        }
                    }
                    start = byte_pos + needle_bytes.len();
                }
                if out.len() >= cap {
                    break;
                }
            }
        }

        out.sort_by_key(|loc| {
            (
                loc["uri"].as_str().unwrap_or("").to_string(),
                loc["range"]["start"]["line"].as_u64().unwrap_or(0),
                loc["range"]["start"]["character"].as_u64().unwrap_or(0),
            )
        });
        out.dedup();
        out.truncate(cap);
        out
    }

    /// Handle textDocument/documentHighlight request
    pub(crate) fn handle_document_highlight(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let offset = self.pos16_to_offset(doc, line, character);

                    // Guard: if the resolved offset doesn't map back to the
                    // requested line, the character position overflowed the
                    // line boundary (e.g. cursor on an empty line). Return
                    // empty highlights instead of highlighting the wrong line.
                    let (actual_line, _) = self.offset_to_pos16(doc, offset);
                    if actual_line != line {
                        return Ok(Some(json!([])));
                    }

                    // Create document highlight provider
                    let provider = DocumentHighlightProvider::new();

                    // Find all highlights at the position
                    let highlights = provider.find_highlights(ast, &doc.text, offset);

                    if !highlights.is_empty() {
                        let lsp_highlights: Vec<Value> = highlights
                            .iter()
                            .map(|highlight| {
                                let (start_line, start_char) =
                                    self.offset_to_pos16(doc, highlight.location.start);
                                let (end_line, end_char) =
                                    self.offset_to_pos16(doc, highlight.location.end);

                                json!({
                                    "range": {
                                        "start": {
                                            "line": start_line,
                                            "character": start_char,
                                        },
                                        "end": {
                                            "line": end_line,
                                            "character": end_char,
                                        },
                                    },
                                    "kind": highlight.kind as u32,
                                })
                            })
                            .collect();

                        return Ok(Some(json!(lsp_highlights)));
                    }
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Non-blocking references handler with fallback
    pub(crate) fn on_references(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let uri = params.pointer("/textDocument/uri").and_then(|v| v.as_str()).unwrap_or("");
        let line = params.pointer("/position/line").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let ch =
            params.pointer("/position/character").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        let text = self.buffer_text(uri).unwrap_or_default();
        let needle = token_under_cursor(&text, line, ch).unwrap_or_default();
        if needle.is_empty() {
            return Ok(serde_json::json!([]));
        }

        // Fallback: search all open docs with word boundary checking
        let mut out = Vec::new();
        for (doc_uri, doc_text) in self.iter_open_buffers() {
            for (ln, l) in doc_text.lines().enumerate() {
                let line_bytes = l.as_bytes();
                let mut start = 0usize;
                while let Some(idx) = l[start..].find(&needle) {
                    let col = start + idx;
                    // Only include if it's a word boundary match
                    if is_word_boundary(line_bytes, col, needle.len()) {
                        // Convert byte position to UTF-16 for LSP
                        let col_utf16 = byte_to_utf16_col(l, col);
                        let end_utf16 = byte_to_utf16_col(l, col + needle.len());
                        out.push(serde_json::json!({
                            "uri": doc_uri,
                            "range": {
                                "start": {"line": ln as u32, "character": col_utf16 as u32},
                                "end":   {"line": ln as u32, "character": end_utf16 as u32}
                            }
                        }));
                    }
                    start = col + needle.len();
                }
            }
        }

        // Sort for deterministic output and deduplicate
        out.sort_by_key(|loc| {
            (
                loc["uri"].as_str().unwrap_or("").to_string(),
                loc["range"]["start"]["line"].as_u64().unwrap_or(0),
                loc["range"]["start"]["character"].as_u64().unwrap_or(0),
            )
        });
        out.dedup();

        Ok(serde_json::Value::Array(out))
    }
}
