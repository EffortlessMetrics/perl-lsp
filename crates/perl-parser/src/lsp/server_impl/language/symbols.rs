//! Symbol and folding handlers for document outline features
//!
//! Handles textDocument/documentSymbol and textDocument/foldingRange requests.

use super::super::*;

impl LspServer {
    /// Handle textDocument/documentSymbol request
    pub(crate) fn handle_document_symbol(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");

            let documents = self.documents.lock().unwrap();
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

                        // Map symbol kind to LSP SymbolKind
                        let symbol_kind = symbol_kind_to_lsp(symbol.kind);

                        // Create display name with sigil if applicable
                        let display_name = if let Some(sigil) = symbol.kind.sigil() {
                            format!("{}{}", sigil, symbol.name)
                        } else {
                            symbol.name.clone()
                        };

                        // Find child symbols for this scope (if it's a package or subroutine)
                        let mut children = Vec::new();
                        if symbol.kind == crate::symbol::SymbolKind::Package
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

                                            let child_kind = symbol_kind_to_lsp(child.kind);

                                            let child_display_name =
                                                if let Some(sigil) = child.kind.sigil() {
                                                    format!("{}{}", sigil, child.name)
                                                } else {
                                                    child.name.clone()
                                                };

                                            children.push(json!({
                                                "name": child_display_name,
                                                "detail": child.declaration.as_deref().unwrap_or(""),
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
                                            }));
                                        }
                                    }
                                    break;
                                }
                            }
                        }

                        let symbol_info = json!({
                            "name": display_name,
                            "detail": symbol.declaration.as_deref().unwrap_or(""),
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

                    return Ok(Some(json!(document_symbols)));
                } else {
                    // Fallback: Extract symbols via regex when parse fails
                    eprintln!("Using fallback symbol extraction for {}", uri);
                    let symbols = self.extract_symbols_fallback(&doc.text);
                    eprintln!("Returning {} fallback symbols", symbols.len());
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
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");

            let documents = self.documents.lock().unwrap();
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
                        return Ok(Some(json!(extract_folding_fallback(&doc.text))));
                    }

                    return Ok(Some(json!(lsp_ranges)));
                } else {
                    // No AST, use fallback
                    return Ok(Some(json!(extract_folding_fallback(&doc.text))));
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Fallback symbol extraction using regex when parser fails
    fn extract_symbols_fallback(&self, content: &str) -> Vec<Value> {
        let mut symbols = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        // Regex for subroutine declarations
        let sub_regex = regex::Regex::new(r"^\s*sub\s+([a-zA-Z_]\w*)\b").unwrap();
        let package_regex = regex::Regex::new(r"^\s*package\s+([a-zA-Z_][\w:]*)\b").unwrap();

        for (line_num, line) in lines.iter().enumerate() {
            // Check for subroutines
            if let Some(captures) = sub_regex.captures(line) {
                if let Some(name_match) = captures.get(1) {
                    let name = name_match.as_str().to_string();
                    let start_char = name_match.start();
                    let end_char = name_match.end();

                    symbols.push(json!({
                        "name": name,
                        "kind": 12, // Function
                        "range": {
                            "start": { "line": line_num, "character": 0 },
                            "end": { "line": line_num, "character": line.len() }
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
                    let start_char = name_match.start();
                    let end_char = name_match.end();

                    symbols.push(json!({
                        "name": name,
                        "kind": 4, // Module
                        "range": {
                            "start": { "line": line_num, "character": 0 },
                            "end": { "line": line_num, "character": line.len() }
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

/// Map symbol kind to LSP SymbolKind numeric value
fn symbol_kind_to_lsp(kind: crate::symbol::SymbolKind) -> u32 {
    match kind {
        crate::symbol::SymbolKind::Package => 4,         // Module
        crate::symbol::SymbolKind::Subroutine => 12,     // Function
        crate::symbol::SymbolKind::ScalarVariable => 13, // Variable
        crate::symbol::SymbolKind::ArrayVariable => 18,  // Array
        crate::symbol::SymbolKind::HashVariable => 19,   // Object (closest match)
        crate::symbol::SymbolKind::Constant => 14,       // Constant
        crate::symbol::SymbolKind::Label => 16,          // String (closest match)
        crate::symbol::SymbolKind::Format => 12,         // Function
    }
}

/// Helper function to convert offset to line number
fn offset_to_line(content: &str, offset: usize) -> usize {
    content[..offset.min(content.len())].chars().filter(|&c| c == '\n').count()
}

/// Fallback folding extraction using text-based analysis
fn extract_folding_fallback(content: &str) -> Vec<Value> {
    let mut ranges = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut brace_stack: Vec<usize> = Vec::new();
    let mut sub_start: Option<usize> = None;
    let mut pod_start: Option<usize> = None;

    for (line_num, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Handle POD sections
        if trimmed.starts_with("=") {
            if trimmed == "=cut" {
                if let Some(start) = pod_start {
                    if line_num > start {
                        ranges.push(json!({
                            "startLine": start,
                            "endLine": line_num,
                            "kind": "comment"
                        }));
                    }
                    pod_start = None;
                }
            } else if pod_start.is_none() {
                pod_start = Some(line_num);
            }
            continue;
        }

        // Skip if we're in POD
        if pod_start.is_some() {
            continue;
        }

        // Handle subroutines
        if trimmed.starts_with("sub ") {
            // If we had a previous sub, close it
            if let Some(start) = sub_start {
                if line_num > start + 1 {
                    ranges.push(json!({
                        "startLine": start,
                        "endLine": line_num - 1
                    }));
                }
            }
            sub_start = Some(line_num);
        }

        // Count braces (simple approach, doesn't handle strings/comments perfectly)
        let mut in_string = false;
        let mut escape_next = false;
        let mut prev_char = ' ';

        for ch in line.chars() {
            if escape_next {
                escape_next = false;
                prev_char = ch;
                continue;
            }

            if ch == '\\' {
                escape_next = true;
                prev_char = ch;
                continue;
            }

            // Simple string detection (not perfect but good enough)
            if (ch == '"' || ch == '\'') && (!in_string || prev_char != '\\') {
                in_string = !in_string;
            }

            if !in_string {
                if ch == '{' {
                    brace_stack.push(line_num);
                } else if ch == '}' {
                    if let Some(start_line) = brace_stack.pop() {
                        // Only create fold if it spans multiple lines
                        if line_num > start_line {
                            ranges.push(json!({
                                "startLine": start_line,
                                "endLine": line_num
                            }));
                        }
                    }
                }
            }

            prev_char = ch;
        }
    }

    // Close any remaining sub
    if let Some(start) = sub_start {
        if lines.len() > start + 1 {
            ranges.push(json!({
                "startLine": start,
                "endLine": lines.len() - 1
            }));
        }
    }

    ranges
}
