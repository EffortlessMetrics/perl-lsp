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
    SUB_REGEX.get_or_init(|| regex::Regex::new(r"^\s*sub\s+([a-zA-Z_]\w*)\b")).as_ref().ok()
}

fn get_package_regex() -> Option<&'static regex::Regex> {
    PACKAGE_REGEX
        .get_or_init(|| regex::Regex::new(r"^\s*package\s+([a-zA-Z_][\w:]*)\b"))
        .as_ref()
        .ok()
}

fn get_head_regex() -> Option<&'static regex::Regex> {
    HEAD_REGEX.get_or_init(|| regex::Regex::new(r"^=(head[1-4])\s+(.+)$")).as_ref().ok()
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
            let name = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("").to_string();
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
                let parsed = doc.current_parsed();
                if let Some(ast) = parsed.as_ref().and_then(|p| p.ast()) {
                    // Source-backed compiler document symbols are live for fresh,
                    // high-confidence parser syntax facts. Astless documents keep
                    // the legacy text fallback below.
                    let live_result =
                        perl_lsp_rs_core::providers::document_symbols::source_backed_document_symbols_from_ast(
                            ast,
                            &doc.text,
                        );
                    let mut document_symbols = document_symbols_to_json(live_result.symbols);

                    // Append Test2/Test::More subtest symbols so the outline shows
                    // the subtest tree. Subtest calls only exist in test files, so
                    // this is empty for ordinary source.
                    let subtests = perl_lsp_rs_core::providers::testing::subtest::discover_subtests(
                        ast, &doc.text,
                    );
                    if !subtests.is_empty() {
                        let subtest_symbols =
                            perl_lsp_rs_core::providers::testing::subtest::subtest_document_symbols(
                                &subtests,
                            );
                        document_symbols.extend(document_symbols_to_json(subtest_symbols));
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
                    let start_line = marker_line + 1;
                    let end_line = total_lines.saturating_sub(1);
                    push_multiline_folding_range(&mut lsp_ranges, start_line, end_line, "comment");
                }

                // Add heredoc folding ranges from lexer
                let heredoc_ranges =
                    crate::folding::FoldingRangeExtractor::extract_heredoc_ranges(&doc.text);
                for range in heredoc_ranges {
                    // Use saturating_sub to ensure we're inside the body
                    let (start_line, _) = self.offset_to_pos16(doc, range.start_offset);
                    let (end_line, _) =
                        self.offset_to_pos16(doc, range.end_offset.saturating_sub(1));

                    push_multiline_folding_range(&mut lsp_ranges, start_line, end_line, "region");
                }

                // Add #region/#endregion folding ranges
                let region_ranges =
                    crate::folding::FoldingRangeExtractor::extract_region_markers(&doc.text);
                for range in region_ranges {
                    let start_line = offset_to_line(&doc.text, range.start_offset);
                    let end_line = offset_to_line(&doc.text, range.end_offset);
                    push_multiline_folding_range(&mut lsp_ranges, start_line, end_line, "region");
                }

                let parsed = doc.current_parsed();
                if let Some(ast) = parsed.as_ref().and_then(|p| p.ast()) {
                    // Extract folding ranges from AST
                    let mut extractor = crate::folding::FoldingRangeExtractor::new();
                    let ranges = extractor.extract(ast);

                    // Convert to LSP JSON format with proper line offsets
                    for range in ranges {
                        // Calculate actual line numbers from document content
                        let start_line = offset_to_line(&doc.text, range.start_offset);
                        let end_line = offset_to_line(&doc.text, range.end_offset);
                        if let Some(lsp_end_line) =
                            lsp_inclusive_multiline_end_line(start_line, end_line)
                        {
                            let mut lsp_range = json!({
                                "startLine": start_line,
                                "endLine": lsp_end_line,  // LSP folding ranges are inclusive
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

                    // Dedup identical ranges (start+end+kind) that arise when both a
                    // Subroutine node and its inner Block node map to the same line span.
                    lsp_ranges.sort_by_key(|r| {
                        (
                            r["startLine"].as_u64().unwrap_or(0),
                            r["endLine"].as_u64().unwrap_or(0),
                            r.get("kind").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        )
                    });
                    lsp_ranges.dedup_by_key(|r| {
                        (
                            r["startLine"].as_u64().unwrap_or(0),
                            r["endLine"].as_u64().unwrap_or(0),
                            r.get("kind").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        )
                    });

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
        let uri = params.pointer("/textDocument/uri").and_then(|v| v.as_str()).unwrap_or("");
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

fn document_symbols_to_json(
    symbols: Vec<perl_lsp_rs_core::providers::document_symbols::DocumentSymbol>,
) -> Vec<Value> {
    match serde_json::to_value(symbols) {
        Ok(Value::Array(items)) => items,
        Ok(_) | Err(_) => Vec::new(),
    }
}

/// Helper function to convert offset to line number
fn offset_to_line(content: &str, offset: usize) -> usize {
    content[..offset.min(content.len())].chars().filter(|&c| c == '\n').count()
}

fn push_multiline_folding_range<T>(
    lsp_ranges: &mut Vec<Value>,
    start_line: T,
    end_line: T,
    kind: &str,
) where
    T: Copy + Ord + serde::Serialize,
{
    if end_line > start_line {
        lsp_ranges.push(json!({
            "startLine": start_line,
            "endLine": end_line,
            "kind": kind
        }));
    }
}

fn lsp_inclusive_multiline_end_line(start_line: usize, raw_end_line: usize) -> Option<usize> {
    let lsp_end_line = raw_end_line.saturating_sub(1);
    (lsp_end_line > start_line).then_some(lsp_end_line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_multiline_folding_range_boundary_discriminator_end_line_gt_start_line_rejects_equal_input()
     {
        let mut ranges = Vec::new();

        push_multiline_folding_range(&mut ranges, 4, 4, "region");

        assert!(ranges.is_empty());
    }

    #[test]
    fn push_multiline_folding_range_boundary_discriminator_input_that_hits_the_boundary_end_line_gt_start_line_accepts_multiline_input()
     {
        let mut ranges = Vec::new();

        push_multiline_folding_range(&mut ranges, 4, 6, "comment");

        assert_eq!(ranges.len(), 1, "input that hits the boundary: end_line > start_line");
        assert_eq!(ranges[0]["startLine"], json!(4));
        assert_eq!(ranges[0]["endLine"], json!(6));
        assert_eq!(ranges[0]["kind"], json!("comment"));
    }

    #[test]
    fn lsp_inclusive_multiline_end_line_boundary_discriminator_lsp_end_line_gt_start_line_rejects_short_span()
     {
        assert_eq!(lsp_inclusive_multiline_end_line(4, 5), None);
        assert_eq!(lsp_inclusive_multiline_end_line(4, 0), None);
    }

    #[test]
    fn lsp_inclusive_multiline_end_line_boundary_discriminator_input_that_hits_the_boundary_lsp_end_line_gt_start_line_accepts_multiline_span()
     {
        assert_eq!(
            lsp_inclusive_multiline_end_line(4, 6),
            Some(5),
            "input that hits the boundary: lsp_end_line > start_line"
        );
    }

    fn folding_ranges_for_source(source: &str) -> Result<Vec<Value>, JsonRpcError> {
        let server = LspServer::new();
        let uri = "file:///folding-observer.pl";
        server.test_handle_did_open(Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": source,
            }
        })))?;

        let response = server.handle_folding_range(Some(json!({
            "textDocument": { "uri": uri }
        })))?;

        Ok(response.and_then(|value| value.as_array().cloned()).unwrap_or_default())
    }

    #[test]
    fn handle_folding_range_call_presence_observer_push_multiline_folding_range_data_section_comment()
    -> Result<(), Box<dyn std::error::Error>> {
        let ranges = folding_ranges_for_source("print \"ok\\n\";\n__DATA__\nalpha\nbeta\n")?;

        assert!(
            ranges.iter().any(|range| {
                range.get("kind") == Some(&json!("comment"))
                    && range.get("startLine") == Some(&json!(2))
                    && range.get("endLine") == Some(&json!(3))
            }),
            "input that reaches call push_multiline_folding_range(&mut lsp_ranges, start_line, end_line, \"comment\")"
        );

        Ok(())
    }

    #[test]
    fn handle_folding_range_call_presence_observer_push_multiline_folding_range_heredoc_region()
    -> Result<(), Box<dyn std::error::Error>> {
        let ranges = folding_ranges_for_source("my $text = <<'TXT';\nalpha\nbeta\nTXT\n")?;

        assert!(
            ranges.iter().any(|range| {
                range.get("kind") == Some(&json!("region"))
                    && range.get("startLine") == Some(&json!(1))
                    && range.get("endLine") == Some(&json!(2))
            }),
            "input that reaches call push_multiline_folding_range(&mut lsp_ranges, start_line, end_line, \"region\")"
        );

        Ok(())
    }

    #[test]
    fn handle_folding_range_call_presence_observer_ast_lsp_end_line_gt_start_line()
    -> Result<(), Box<dyn std::error::Error>> {
        let ranges = folding_ranges_for_source("sub full {\n    my $value = 1;\n}\n")?;

        assert!(ranges.iter().any(|range| {
            range.get("startLine") == Some(&json!(0))
                && range.get("endLine").and_then(Value::as_u64).is_some_and(|end| end > 0)
        }));

        Ok(())
    }
}

impl LspServer {
    /// Document symbol runtime quality receipt for the source-backed live slice.
    ///
    /// Calls the live `textDocument/documentSymbol` handler and wraps the result
    /// in a typed receipt that records the fresh parser-syntax symbols promoted
    /// live. Astless, generated/no-source, stale, dynamic, low-confidence, and
    /// ambiguous cases keep fallback or gated behavior.
    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    pub(crate) fn document_symbols_runtime_quality_receipt(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let (compiler_receipt, source_backed_count) =
            self.document_symbols_source_backed_receipt(params.as_ref())?;
        let live_provider_result = self.handle_document_symbol(params)?;
        let live_provider_count = match live_provider_result.as_ref() {
            Some(Value::Array(items)) => items.len(),
            _ => 0,
        };
        let no_live_behavior_change = source_backed_count == 0;

        Ok(Some(json!({
            "provider": "document_symbols",
            "live_provider_result": live_provider_result,
            "live_provider_count": live_provider_count,
            "shadow_state": "partial_live_source_backed",
            "compiler_receipt": compiler_receipt,
            "no_live_behavior_change": no_live_behavior_change,
            "notes": [
                format!(
                    "document-symbol runtime quality receipt: live_provider_count={}; \
                     source_backed_compiler_symbols={}; \
                     source-backed parser syntax document symbols are live; \
                     astless, stale, dynamic, generated/no-source, and ambiguous cases keep fallback",
                    live_provider_count,
                    source_backed_count
                )
            ]
        })))
    }

    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    fn document_symbols_source_backed_receipt(
        &self,
        params: Option<&Value>,
    ) -> Result<(Value, usize), JsonRpcError> {
        let Some(params) = params else {
            return Ok((document_symbols_empty_compiler_receipt("missing_params"), 0));
        };
        let uri = req_uri(params)?;
        let documents = self.documents_guard();
        let Some(doc) = self.get_document(&documents, uri) else {
            return Ok((document_symbols_empty_compiler_receipt("unknown_uri"), 0));
        };
        let parsed = doc.current_parsed();
        let Some(ast) = parsed.as_ref().and_then(|p| p.ast()) else {
            return Ok((document_symbols_empty_compiler_receipt("ast_unavailable"), 0));
        };

        let live_result =
            perl_lsp_rs_core::providers::document_symbols::source_backed_document_symbols_from_ast(
                ast, &doc.text,
            );
        let source_backed_count = live_result.fact_traces.len();
        Ok((
            json!({
                "source": "ParserSyntax",
                "provenance": "ExactAst",
                "confidence": "High",
                "freshness": "Fresh",
                "fallback_state": "Primary",
                "source_backed_count": source_backed_count,
                "fact_source_traces": live_result.fact_traces,
            }),
            source_backed_count,
        ))
    }
}

#[cfg(any(test, feature = "expose_lsp_test_api"))]
fn document_symbols_empty_compiler_receipt(reason: &str) -> Value {
    json!({
        "source_backed_count": 0,
        "fallback_state": "Fallback",
        "reason": reason,
        "fact_source_traces": [],
    })
}
