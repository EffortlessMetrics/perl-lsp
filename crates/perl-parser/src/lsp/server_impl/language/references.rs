//! Reference handlers for find references and document highlights
//!
//! Handles textDocument/references and textDocument/documentHighlight requests.

use super::super::*;

impl LspServer {
    /// Handle textDocument/references request
    pub(crate) fn handle_references(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let line = params["position"]["line"].as_u64().unwrap_or(0) as u32;
            let character = params["position"]["character"].as_u64().unwrap_or(0) as u32;
            let include_declaration = if let Some(context) = params.get("context") {
                context["includeDeclaration"].as_bool().unwrap_or(true)
            } else {
                true
            };

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let offset = self.pos16_to_offset(doc, line, character);

                    // Try workspace index first for cross-file references
                    #[cfg(feature = "workspace")]
                    if let Some(ref workspace_index) = self.workspace_index {
                        // Use symbol_at_cursor to get the symbol key
                        let current_package = crate::declaration::current_package_at(ast, offset);
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
                                    crate::workspace_index::lsp_adapter::to_lsp_locations(all_refs);
                                for loc in lsp_locations {
                                    workspace_locations.push(json!(loc));
                                }
                            }

                            // Enhanced fallback: always search for both qualified and unqualified references
                            let docs_snapshot: Vec<(String, DocumentState)> =
                                documents.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

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

                            for pattern in patterns {
                                if let Ok(search_regex) = regex::Regex::new(&pattern) {
                                    for (doc_uri, doc_state) in &docs_snapshot {
                                        let lines: Vec<&str> = doc_state.text.lines().collect();
                                        for (line_num, line) in lines.iter().enumerate() {
                                            for mat in search_regex.find_iter(line) {
                                                enhanced_locations.push(json!({
                                                    "uri": doc_uri,
                                                    "range": {
                                                        "start": {
                                                            "line": line_num,
                                                            "character": mat.start(),
                                                        },
                                                        "end": {
                                                            "line": line_num,
                                                            "character": mat.end(),
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
                            let all_combined_locations = workspace_locations;

                            if !all_combined_locations.is_empty() {
                                eprintln!(
                                    "Found {} total references via combined search",
                                    all_combined_locations.len()
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
                                eprintln!(
                                    "Found {} references via find_references for {}",
                                    refs.len(),
                                    symbol_name
                                );
                                // Convert internal Locations to LSP Locations
                                let lsp_locations =
                                    crate::workspace_index::lsp_adapter::to_lsp_locations(refs);
                                if !lsp_locations.is_empty() {
                                    return Ok(Some(json!(lsp_locations)));
                                }
                            }
                        }

                        // Regex-based fallback for fully-qualified symbols like Package::sub references
                        let radius = 50;
                        let text_start = offset.saturating_sub(radius);
                        let text_around = self.get_text_around_offset(&doc.text, offset, radius);
                        let cursor_in_text = offset - text_start;

                        let re = regex::Regex::new(
                            r"([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)",
                        )
                        .unwrap();

                        for cap in re.captures_iter(&text_around) {
                            if let Some(m) = cap.get(1) {
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

                                        if let Some(ref workspace_index) = self.workspace_index {
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
                                                if let Some(def) = workspace_index.find_def(&key) {
                                                    all_refs.push(def);
                                                }
                                            }

                                            if !all_refs.is_empty() {
                                                // Convert internal Locations to LSP Locations
                                                let lsp_locations =
                                                    crate::workspace_index::lsp_adapter::to_lsp_locations(all_refs);
                                                if !lsp_locations.is_empty() {
                                                    return Ok(Some(json!(lsp_locations)));
                                                }
                                            }

                                            // Fallback: scan open documents for qualified name references
                                            let docs_snapshot: Vec<(String, DocumentState)> =
                                                documents
                                                    .iter()
                                                    .map(|(k, v)| (k.clone(), v.clone()))
                                                    .collect();

                                            let mut all_locations = Vec::new();
                                            let qualified_name = format!("{}::{}", pkg, name);
                                            let search_regex = regex::Regex::new(&format!(
                                                r"\b{}\b",
                                                regex::escape(&qualified_name)
                                            ))
                                            .unwrap();

                                            for (doc_uri, doc_state) in docs_snapshot {
                                                let lines: Vec<&str> =
                                                    doc_state.text.lines().collect();
                                                for (line_num, line) in lines.iter().enumerate() {
                                                    for mat in search_regex.find_iter(line) {
                                                        all_locations.push(json!({
                                                            "uri": doc_uri,
                                                            "range": {
                                                                "start": {
                                                                    "line": line_num,
                                                                    "character": mat.start(),
                                                                },
                                                                "end": {
                                                                    "line": line_num,
                                                                    "character": mat.end(),
                                                                },
                                                            },
                                                        }));
                                                    }
                                                }
                                            }

                                            if !all_locations.is_empty() {
                                                return Ok(Some(json!(all_locations)));
                                            }
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                    }

                    // Fall back to same-file references
                    let analyzer = crate::semantic::SemanticAnalyzer::analyze(ast);

                    // Find all references at the position
                    let references = analyzer.find_all_references(offset, include_declaration);

                    if !references.is_empty() {
                        let locations: Vec<Value> = references
                            .iter()
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
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let line = params["position"]["line"].as_u64().unwrap_or(0) as u32;
            let character = params["position"]["character"].as_u64().unwrap_or(0) as u32;

            let documents = self.documents.lock().unwrap();
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
}
