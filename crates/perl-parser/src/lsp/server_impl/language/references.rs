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
use crate::lsp::protocol::{req_position, req_uri};
use crate::lsp::state::{reference_search_deadline, references_cap};
use crate::lsp::utils::{is_word_boundary, token_under_cursor};
use lazy_static::lazy_static;
use std::time::Instant;

#[cfg(feature = "workspace")]
use crate::workspace_index::IndexState;

lazy_static! {
    /// Regex for matching fully-qualified Perl symbol names (e.g., Package::SubPackage::function)
    /// Compiled once at startup to avoid per-request regex compilation overhead.
    static ref QUALIFIED_NAME_RE: regex::Regex =
        regex::Regex::new(r"([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)").unwrap();
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

                    // Check index state and use appropriate search strategy
                    #[cfg(feature = "workspace")]
                    {
                        let is_ready = self
                            .coordinator()
                            .map(|c| matches!(c.state(), IndexState::Ready { .. }))
                            .unwrap_or(false);

                        if !is_ready {
                            eprintln!("References: index not ready, using same-file fallback");
                            // Fall through to same-file semantic analysis at the end
                        } else if let Some(ref workspace_index) = self.workspace_index {
                            // Use symbol_at_cursor to get the symbol key
                            let current_package =
                                crate::declaration::current_package_at(ast, offset);
                            if let Some(symbol_key) =
                                crate::declaration::symbol_at_cursor(ast, offset, current_package)
                            {
                                eprintln!("Looking for references of {:?}", symbol_key);

                                // Try to find references using the symbol key
                                let mut all_refs = workspace_index.find_refs(&symbol_key);

                                // Add the definition if includeDeclaration is true
                                if include_declaration {
                                    if let Some(def) = workspace_index.find_def(&symbol_key) {
                                        all_refs.push(def);
                                    }
                                }

                                let mut workspace_locations: Vec<Value> = Vec::new();
                                if !all_refs.is_empty() {
                                    eprintln!("Found {} references via find_refs", all_refs.len());
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
                                    eprintln!(
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
                                        eprintln!(
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
                                    eprintln!(
                                        "Found {} total references via combined search (capped at {}, elapsed {:?})",
                                        all_combined_locations.len(),
                                        cap,
                                        start.elapsed()
                                    );
                                    return Ok(Some(json!(all_combined_locations)));
                                }

                                // Also try with find_references for backward compatibility
                                let symbol_name =
                                    if symbol_key.kind == crate::workspace_index::SymKind::Sub {
                                        format!("{}::{}", symbol_key.pkg, symbol_key.name)
                                    } else {
                                        symbol_key.name.to_string()
                                    };

                                let refs = workspace_index.find_references(&symbol_name);
                                if !refs.is_empty() {
                                    // Cap results before conversion
                                    let capped_refs: Vec<_> = refs.into_iter().take(cap).collect();
                                    eprintln!(
                                        "Found {} references via find_references for {} (capped at {})",
                                        capped_refs.len(),
                                        symbol_name,
                                        cap
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
                            for captures in QUALIFIED_NAME_RE.captures_iter(&text_around) {
                                if let Some(m) = captures.get(1) {
                                    if cursor_in_text >= m.start() && cursor_in_text <= m.end() {
                                        let parts: Vec<&str> = m.as_str().split("::").collect();
                                        if parts.len() >= 2 {
                                            let name = parts.last().unwrap().to_string();
                                            let pkg = parts[..parts.len() - 1].join("::");
                                            let key = crate::workspace_index::SymbolKey {
                                                pkg: pkg.clone().into(),
                                                name: name.clone().into(),
                                                sigil: None,
                                                kind: crate::workspace_index::SymKind::Sub,
                                            };

                                            if let Some(ref workspace_index) = self.workspace_index
                                            {
                                                // Search for all references to this qualified symbol
                                                let mut all_refs = Vec::new();

                                                // Find references via symbol key
                                                let refs = workspace_index.find_refs(&key);
                                                all_refs.extend(refs);

                                                // Also try with qualified name
                                                let symbol_name = format!("{}::{}", pkg, name);
                                                let alt_refs =
                                                    workspace_index.find_references(&symbol_name);
                                                all_refs.extend(alt_refs);

                                                // Add definition if includeDeclaration is true
                                                if include_declaration {
                                                    if let Some(def) =
                                                        workspace_index.find_def(&key)
                                                    {
                                                        all_refs.push(def);
                                                    }
                                                }

                                                if !all_refs.is_empty() {
                                                    // Cap results
                                                    let capped_refs: Vec<_> =
                                                        all_refs.into_iter().take(cap).collect();
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
                                                        .map(|(k, v)| (k.clone(), v.text.clone()))
                                                        .collect();

                                                let mut all_locations = Vec::new();
                                                let qualified_name = format!("{}::{}", pkg, name);
                                                let Ok(search_regex) = regex::Regex::new(&format!(
                                                    r"\b{}\b",
                                                    regex::escape(&qualified_name)
                                                )) else {
                                                    continue;
                                                };

                                                'doc_scan: for (doc_uri, doc_text) in docs_snapshot
                                                {
                                                    // Check deadline
                                                    if start.elapsed() >= deadline {
                                                        break 'doc_scan;
                                                    }
                                                    let lines: Vec<&str> =
                                                        doc_text.lines().collect();
                                                    for (line_num, line) in lines.iter().enumerate()
                                                    {
                                                        for mat in search_regex.find_iter(line) {
                                                            // Convert byte offsets to UTF-16 columns for LSP compliance
                                                            let start_utf16 = byte_to_utf16_col(
                                                                line,
                                                                mat.start(),
                                                            );
                                                            let end_utf16 =
                                                                byte_to_utf16_col(line, mat.end());
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
                                        }
                                        break;
                                    }
                                }
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

                        eprintln!(
                            "References: returned {} same-file results (elapsed {:?})",
                            locations.len(),
                            start.elapsed()
                        );
                        return Ok(Some(json!(locations)));
                    }
                }
            }
        }

        Ok(Some(json!([])))
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
