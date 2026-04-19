//! Symbol and folding handlers for document outline features
//!
//! Handles textDocument/documentSymbol and textDocument/foldingRange requests.

use super::super::{byte_to_utf16_col, *};
use crate::fallback::text::folding_ranges_from_text;
use crate::protocol::req_uri;
use crate::state::document_symbol_cap;
use std::sync::OnceLock;

static SUB_REGEX: OnceLock<Result<regex::Regex, regex::Error>> = OnceLock::new();
static PACKAGE_REGEX: OnceLock<Result<regex::Regex, regex::Error>> = OnceLock::new();
static HEAD_REGEX: OnceLock<Result<regex::Regex, regex::Error>> = OnceLock::new();

fn get_sub_regex() -> Option<&'static regex::Regex> {
    SUB_REGEX
        .get_or_init(|| regex::Regex::new(r"^\s*sub\s+([a-zA-Z_]\w*)\b"))
        .as_ref()
        .ok()
}

fn get_package_regex() -> Option<&'static regex::Regex> {
    PACKAGE_REGEX
        .get_or_init(|| regex::Regex::new(r"^\s*package\s+([a-zA-Z_][\w:]*)\b"))
        .as_ref()
        .ok()
}

fn get_head_regex() -> Option<&'static regex::Regex> {
    HEAD_REGEX
        .get_or_init(|| regex::Regex::new(r"^=(head[1-4])\s+(.+)$"))
        .as_ref()
        .ok()
}

/// Scan source text for POD =head1..=head4 directives and return them as document symbols.
/// Stops scanning at __DATA__ or __END__ blocks. Uses LSP SymbolKind 26 (TypeParameter).
fn pod_section_symbols(source: &str) -> Vec<Value> {
    let Some(head_regex) = get_head_regex() else {
        return Vec::new();
    };
    let mut symbols = Vec::new();
    for (line_num, line) in source.lines().enumerate() {
        if line == "__DATA__" || line == "__END__" {
            break;
        }
        if let Some(caps) = head_regex.captures(line) {
            let name = caps
                .get(2)
                .map(|m| m.as_str().trim())
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                continue;
            }
            let line_end_char = byte_to_utf16_col(line, line.len());
            symbols.push(json!({
                "name": name,
                "detail": "",
                "kind": 26,  // TypeParameter -- used for POD sections
                "range": {
                    "start": { "line": line_num, "character": 0 },
                    "end": { "line": line_num, "character": line_end_char }
                },
                "selectionRange": {
                    "start": { "line": line_num, "character": 0 },
                    "end": { "line": line_num, "character": line_end_char }
                },
                "children": []
            }));
        }
    }
    symbols
}

impl LspServer {
    /// Handle textDocument/documentSymbol request
    pub(crate) fn handle_document_symbol(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let cap = document_symbol_cap();

        if let Some(params) = params {
            let uri = req_uri(&params)?;

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    // Extract symbols from AST
                    let extractor = crate::symbol::SymbolExtractor::new_with_source(&doc.text);
                    let symbol_table = extractor.extract(ast);

                    // Convert to DocumentSymbol format
                    let mut document_symbols = Vec::new();

                    // Group symbols by scope and kind
                    let mut symbols_by_scope: std::collections::HashMap<
                        crate::symbol::ScopeId,
                        Vec<crate::symbol::Symbol>,
                    > = std::collections::HashMap::new();
                    for symbols in symbol_table.symbols.values() {
                        for symbol in symbols {
                            symbols_by_scope
                                .entry(symbol.scope_id)
                                .or_default()
                                .push(symbol.clone());
                        }
                    }

                    // Build hierarchical structure starting from global scope
                    let empty_vec = Vec::new();
                    let global_symbols = symbols_by_scope.get(&0).unwrap_or(&empty_vec);

                    for symbol in global_symbols {
                        let (start_line, start_char) =
                            self.offset_to_pos16(doc, symbol.location.start);
                        let (end_line, end_char) = self.offset_to_pos16(doc, symbol.location.end);

                        let symbol_kind = document_symbol_kind(symbol);
                        let display_name = document_symbol_name(symbol);
                        let detail = document_symbol_detail(symbol);

                        // Find child symbols for this scope (if it's a package or subroutine)
                        let mut children = Vec::new();
                        if symbol.kind == crate::symbol::SymbolKind::Package
                            || symbol.kind == crate::symbol::SymbolKind::Class
                            || symbol.kind == crate::symbol::SymbolKind::Subroutine
                        {
                            // Find scope ID for this symbol
                            for (scope_id, scope) in &symbol_table.scopes {
                                if scope.location.start == symbol.location.start {
                                    // Get symbols in this scope
                                    if let Some(child_symbols) = symbols_by_scope.get(scope_id) {
                                        for child in child_symbols {
                                            let (child_start_line, child_start_char) =
                                                self.offset_to_pos16(doc, child.location.start);
                                            let (child_end_line, child_end_char) =
                                                self.offset_to_pos16(doc, child.location.end);

                                            let child_kind = document_symbol_kind(child);
                                            let child_display_name = document_symbol_name(child);
                                            let child_detail = document_symbol_detail(child);

                                            if child.location == symbol.location
                                                && child.kind == symbol.kind
                                                && child.name == symbol.name
                                            {
                                                continue;
                                            }

                                            children.push((
                                                document_symbol_priority(child),
                                                child.location.start,
                                                child.location.end,
                                                json!({
                                                    "name": child_display_name,
                                                    "detail": child_detail,
                                                    "kind": child_kind,
                                                    "range": {
                                                        "start": { "line": child_start_line, "character": child_start_char },
                                                        "end": { "line": child_end_line, "character": child_end_char }
                                                    },
                                                    "selectionRange": {
                                                        "start": { "line": child_start_line, "character": child_start_char },
                                                        "end": { "line": child_end_line, "character": child_end_char }
                                                    },
                                                    "children": []
                                                }),
                                            ));
                                        }
                                    }
                                    break;
                                }
                            }
                        }

                        children.sort_by_key(|(priority, start, end, _)| (*priority, *start, *end));
                        let children: Vec<Value> =
                            children.into_iter().map(|(_, _, _, child)| child).collect();

                        let symbol_info = json!({
                            "name": display_name,
                            "detail": detail,
                            "kind": symbol_kind,
                            "range": {
                                "start": { "line": start_line, "character": start_char },
                                "end": { "line": end_line, "character": end_char }
                            },
                            "selectionRange": {
                                "start": { "line": start_line, "character": start_char },
                                "end": { "line": end_line, "character": end_char }
                            },
                            "children": children
                        });

                        document_symbols.push(symbol_info);
                    }

                    // Append POD section symbols from a direct line scan
                    document_symbols.extend(pod_section_symbols(&doc.text));

                    // Apply cap to document symbols
                    if document_symbols.len() > cap {
                        tracing::debug!(
                            from = document_symbols.len(),
                            to = cap,
                            "DocumentSymbol: capping"
                        );
                        document_symbols.truncate(cap);
                    }

                    return Ok(Some(json!(document_symbols)));
                } else {
                    // Fallback: Extract symbols via regex when parse fails
                    tracing::debug!(uri, "Using fallback symbol extraction");
                    let mut symbols = self.extract_symbols_fallback(&doc.text);
                    // Append POD section symbols from a direct line scan
                    symbols.extend(pod_section_symbols(&doc.text));
                    // Apply cap to fallback symbols
                    if symbols.len() > cap {
                        tracing::debug!(
                            from = symbols.len(),
                            to = cap,
                            "DocumentSymbol (fallback): capping"
                        );
                        symbols.truncate(cap);
                    }
                    tracing::debug!(count = symbols.len(), "Returning fallback symbols");
                    return Ok(Some(json!(symbols)));
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle textDocument/foldingRange request
    pub(crate) fn handle_folding_range(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = req_uri(&params)?;

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                let mut lsp_ranges = Vec::new();

                // Add text-based data section folding
                if let Some(marker_offset) = crate::util::find_data_marker_byte_lexed(&doc.text) {
                    let marker_line = offset_to_line(&doc.text, marker_offset);
                    let total_lines = doc.text.lines().count();

                    // Add fold for data section body if it exists
                    if marker_line + 1 < total_lines {
                        lsp_ranges.push(json!({
                            "startLine": marker_line + 1,
                            "endLine": total_lines - 1,
                            "kind": "comment"
                        }));
                    }
                }

                // Add heredoc folding ranges from lexer
                let heredoc_ranges =
                    crate::folding::FoldingRangeExtractor::extract_heredoc_ranges(&doc.text);
                for range in heredoc_ranges {
                    // Use saturating_sub to ensure we're inside the body
                    let (start_line, _) = self.offset_to_pos16(doc, range.start_offset);
                    let (end_line, _) =
                        self.offset_to_pos16(doc, range.end_offset.saturating_sub(1));

                    if start_line <= end_line {
                        lsp_ranges.push(json!({
                            "startLine": start_line,
                            "endLine": end_line,
                            "kind": "region"
                        }));
                    }
                }

                if let Some(ref ast) = doc.ast {
                    // Extract folding ranges from AST
                    let mut extractor = crate::folding::FoldingRangeExtractor::new();
                    let ranges = extractor.extract(ast);

                    // Convert to LSP JSON format with proper line offsets
                    for range in ranges {
                        // Calculate actual line numbers from document content
                        let start_line = offset_to_line(&doc.text, range.start_offset);
                        let end_line = offset_to_line(&doc.text, range.end_offset);

                        if end_line > start_line {
                            let mut lsp_range = json!({
                                "startLine": start_line,
                                "endLine": end_line - 1,  // LSP folding ranges are inclusive
                            });

                            if let Some(ref kind) = range.kind {
                                lsp_range["kind"] = match kind {
                                    crate::folding::FoldingRangeKind::Comment => json!("comment"),
                                    crate::folding::FoldingRangeKind::Imports => json!("imports"),
                                    crate::folding::FoldingRangeKind::Region => json!("region"),
                                };
                            }

                            lsp_ranges.push(lsp_range);
                        }
                    }

                    // If no ranges from AST, try fallback
                    if lsp_ranges.is_empty() {
                        return Ok(Some(json!(folding_ranges_from_text(&doc.text, 1000))));
                    }

                    return Ok(Some(json!(lsp_ranges)));
                } else {
                    // No AST, use fallback
                    return Ok(Some(json!(folding_ranges_from_text(&doc.text, 1000))));
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Non-blocking folding range handler with text-based fallback
    pub(crate) fn on_folding_range(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let uri = params
            .pointer("/textDocument/uri")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let text = self.buffer_text(uri).unwrap_or_default();
        let ranges = folding_ranges_from_text(&text, 128);
        Ok(serde_json::to_value(ranges).unwrap_or(serde_json::json!([])))
    }

    /// Fallback symbol extraction using regex when parser fails
    fn extract_symbols_fallback(&self, content: &str) -> Vec<Value> {
        let mut symbols = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        // Get pre-compiled regexes
        let Some(sub_regex) = get_sub_regex() else {
            return symbols;
        };
        let Some(package_regex) = get_package_regex() else {
            return symbols;
        };

        for (line_num, line) in lines.iter().enumerate() {
            // Check for subroutines
            if let Some(captures) = sub_regex.captures(line) {
                if let Some(name_match) = captures.get(1) {
                    let name = name_match.as_str().to_string();
                    // Convert byte positions to UTF-16 code units for LSP compliance
                    let start_char = byte_to_utf16_col(line, name_match.start());
                    let end_char = byte_to_utf16_col(line, name_match.end());
                    let line_end_utf16 = byte_to_utf16_col(line, line.len());

                    symbols.push(json!({
                        "name": name,
                        "kind": 12, // Function
                        "range": {
                            "start": { "line": line_num, "character": 0 },
                            "end": { "line": line_num, "character": line_end_utf16 }
                        },
                        "selectionRange": {
                            "start": { "line": line_num, "character": start_char },
                            "end": { "line": line_num, "character": end_char }
                        }
                    }));
                }
            }

            // Check for packages
            if let Some(captures) = package_regex.captures(line) {
                if let Some(name_match) = captures.get(1) {
                    let name = name_match.as_str().to_string();
                    // Convert byte positions to UTF-16 code units for LSP compliance
                    let start_char = byte_to_utf16_col(line, name_match.start());
                    let end_char = byte_to_utf16_col(line, name_match.end());
                    let line_end_utf16 = byte_to_utf16_col(line, line.len());

                    symbols.push(json!({
                        "name": name,
                        "kind": 4, // Module
                        "range": {
                            "start": { "line": line_num, "character": 0 },
                            "end": { "line": line_num, "character": line_end_utf16 }
                        },
                        "selectionRange": {
                            "start": { "line": line_num, "character": start_char },
                            "end": { "line": line_num, "character": end_char }
                        }
                    }));
                }
            }
        }

        symbols
    }
}

/// Map symbol kind to LSP SymbolKind numeric value for document symbols.
///
/// Uses the richer document symbol profile that distinguishes
/// array and hash variables with distinct icons.
#[inline]
fn symbol_kind_to_lsp(kind: crate::symbol::SymbolKind) -> u32 {
    kind.to_lsp_kind_document_symbol()
}

fn document_symbol_kind(symbol: &crate::symbol::Symbol) -> u32 {
    if symbol.declaration.as_deref() == Some("has")
        && symbol.kind == crate::symbol::SymbolKind::scalar()
    {
        7
    } else {
        symbol_kind_to_lsp(symbol.kind)
    }
}

fn document_symbol_name(symbol: &crate::symbol::Symbol) -> String {
    if symbol.declaration.as_deref() == Some("has") {
        symbol.name.clone()
    } else if let Some(sigil) = symbol.kind.sigil() {
        format!("{}{}", sigil, symbol.name)
    } else {
        symbol.name.clone()
    }
}

fn document_symbol_detail(symbol: &crate::symbol::Symbol) -> String {
    if symbol.declaration.as_deref() == Some("has")
        && symbol.kind == crate::symbol::SymbolKind::scalar()
    {
        if !symbol.attributes.is_empty() {
            symbol.attributes.join(", ")
        } else {
            symbol.documentation.clone().unwrap_or_default()
        }
    } else if symbol.declaration.as_deref() == Some("has") {
        symbol.documentation.clone().unwrap_or_default()
    } else {
        symbol.declaration.as_deref().unwrap_or("").to_string()
    }
}

fn document_symbol_priority(symbol: &crate::symbol::Symbol) -> u8 {
    if symbol.declaration.as_deref() == Some("has")
        && symbol.kind == crate::symbol::SymbolKind::scalar()
    {
        0
    } else if symbol.declaration.as_deref() == Some("has") {
        2
    } else if matches!(
        symbol.kind,
        crate::symbol::SymbolKind::Package
            | crate::symbol::SymbolKind::Class
            | crate::symbol::SymbolKind::Role
    ) {
        1
    } else if symbol.kind.is_callable() {
        3
    } else {
        4
    }
}

/// Helper function to convert offset to line number
fn offset_to_line(content: &str, offset: usize) -> usize {
    content[..offset.min(content.len())]
        .chars()
        .filter(|&c| c == '\n')
        .count()
}
