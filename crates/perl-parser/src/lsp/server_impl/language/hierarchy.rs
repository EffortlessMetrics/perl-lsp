//! Hierarchy handlers for type and call hierarchy
//!
//! Handles prepareTypeHierarchy, typeHierarchy/supertypes, typeHierarchy/subtypes,
//! prepareCallHierarchy, callHierarchy/incomingCalls, and callHierarchy/outgoingCalls.

use super::super::*;

impl LspServer {
    /// Handle textDocument/prepareTypeHierarchy request
    pub(crate) fn handle_prepare_type_hierarchy(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let line = params["position"]["line"].as_u64().unwrap_or(0) as u32;
            let character = params["position"]["character"].as_u64().unwrap_or(0) as u32;

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                let offset = self.pos16_to_offset(doc, line, character);

                // Try AST-based approach first
                if let Some(ref ast) = doc.ast {
                    // Create type hierarchy provider
                    let provider = TypeHierarchyProvider::new();

                    // Prepare type hierarchy at the position
                    if let Some(items) = provider.prepare(ast, &doc.text, offset) {
                        let lsp_items: Vec<Value> = items
                            .iter()
                            .map(|item| {
                                json!({
                                    "name": item.name,
                                    "kind": item.kind as u32,
                                    "uri": uri,
                                    "range": {
                                        "start": {
                                            "line": item.range.start.line,
                                            "character": item.range.start.character,
                                        },
                                        "end": {
                                            "line": item.range.end.line,
                                            "character": item.range.end.character,
                                        },
                                    },
                                    "selectionRange": {
                                        "start": {
                                            "line": item.selection_range.start.line,
                                            "character": item.selection_range.start.character,
                                        },
                                        "end": {
                                            "line": item.selection_range.end.line,
                                            "character": item.selection_range.end.character,
                                        },
                                    },
                                    "detail": item.detail,
                                    "data": {
                                        "uri": uri,
                                        "name": item.name,
                                    },
                                })
                            })
                            .collect();

                        return Ok(Some(json!(lsp_items)));
                    }
                }

                // Fallback to regex-based approach
                let sub_regex = regex::Regex::new(r"\bsub\s+([a-zA-Z_]\w*)\b").unwrap();
                let package_regex = regex::Regex::new(r"\bpackage\s+([a-zA-Z_][\w:]*)\b").unwrap();

                // Find all subs and packages with their positions
                let mut exact_sub: Option<(String, usize, usize)> = None;
                for cap in sub_regex.captures_iter(&doc.text) {
                    if let (Some(m), Some(name)) = (cap.get(0), cap.get(1)) {
                        if offset >= m.start() && offset <= m.end() {
                            // Exact match - cursor is on this sub
                            exact_sub = Some((name.as_str().to_string(), m.start(), m.end()));
                            break;
                        }
                    }
                }

                if let Some((name, start, end)) = exact_sub {
                    let start_pos = doc.line_starts.offset_to_position_rope(&doc.rope, start);
                    let end_pos = doc.line_starts.offset_to_position_rope(&doc.rope, end);
                    return Ok(Some(json!([{
                        "name": name,
                        "kind": 12, // Function
                        "uri": uri,
                        "range": {
                            "start": { "line": start_pos.0, "character": start_pos.1 },
                            "end": { "line": end_pos.0, "character": end_pos.1 },
                        },
                        "selectionRange": {
                            "start": { "line": start_pos.0, "character": start_pos.1 },
                            "end": { "line": end_pos.0, "character": end_pos.1 },
                        },
                        "detail": "sub",
                        "data": { "uri": uri, "name": name },
                    }])));
                }

                // Check packages
                let mut exact_pkg: Option<(String, usize, usize)> = None;
                for cap in package_regex.captures_iter(&doc.text) {
                    if let (Some(m), Some(name)) = (cap.get(0), cap.get(1)) {
                        if offset >= m.start() && offset <= m.end() {
                            // Exact match - cursor is on this package
                            exact_pkg = Some((name.as_str().to_string(), m.start(), m.end()));
                            break;
                        }
                    }
                }

                if let Some((name, start, end)) = exact_pkg {
                    let start_pos = doc.line_starts.offset_to_position_rope(&doc.rope, start);
                    let end_pos = doc.line_starts.offset_to_position_rope(&doc.rope, end);
                    return Ok(Some(json!([{
                        "name": name,
                        "kind": 5, // Class
                        "uri": uri,
                        "range": {
                            "start": { "line": start_pos.0, "character": start_pos.1 },
                            "end": { "line": end_pos.0, "character": end_pos.1 },
                        },
                        "selectionRange": {
                            "start": { "line": start_pos.0, "character": start_pos.1 },
                            "end": { "line": end_pos.0, "character": end_pos.1 },
                        },
                        "detail": "package",
                        "data": { "uri": uri, "name": name },
                    }])));
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle typeHierarchy/supertypes request
    pub(crate) fn handle_type_hierarchy_supertypes(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            if let Some(item) = params.get("item") {
                let uri = item["data"]["uri"].as_str().unwrap_or("");
                let name = item["data"]["name"].as_str().unwrap_or("");

                let documents = self.documents_guard();
                if let Some(doc) = documents.get(uri) {
                    if let Some(ref ast) = doc.ast {
                        // Create type hierarchy provider
                        let provider = TypeHierarchyProvider::new();

                        // Extract range from request item (LSP uses camelCase)
                        let type_item = crate::type_hierarchy::TypeHierarchyItem {
                            name: name.to_string(),
                            kind: crate::type_hierarchy::SymbolKind::Class,
                            uri: uri.to_string(),
                            range: crate::type_hierarchy::Range {
                                start: crate::type_hierarchy::Position {
                                    line: item["range"]["start"]["line"].as_u64().unwrap_or(0)
                                        as u32,
                                    character: item["range"]["start"]["character"]
                                        .as_u64()
                                        .unwrap_or(0)
                                        as u32,
                                },
                                end: crate::type_hierarchy::Position {
                                    line: item["range"]["end"]["line"].as_u64().unwrap_or(0) as u32,
                                    character: item["range"]["end"]["character"]
                                        .as_u64()
                                        .unwrap_or(0)
                                        as u32,
                                },
                            },
                            selection_range: crate::type_hierarchy::Range {
                                start: crate::type_hierarchy::Position {
                                    line: item["selectionRange"]["start"]["line"]
                                        .as_u64()
                                        .unwrap_or(0)
                                        as u32,
                                    character: item["selectionRange"]["start"]["character"]
                                        .as_u64()
                                        .unwrap_or(0)
                                        as u32,
                                },
                                end: crate::type_hierarchy::Position {
                                    line: item["selectionRange"]["end"]["line"]
                                        .as_u64()
                                        .unwrap_or(0)
                                        as u32,
                                    character: item["selectionRange"]["end"]["character"]
                                        .as_u64()
                                        .unwrap_or(0)
                                        as u32,
                                },
                            },
                            detail: item["detail"].as_str().map(String::from),
                            data: item.get("data").cloned(),
                        };

                        // Find supertypes
                        let supertypes = provider.find_supertypes(ast, &type_item);

                        let lsp_items: Vec<Value> = supertypes
                            .iter()
                            .map(|item| {
                                json!({
                                    "name": item.name,
                                    "kind": item.kind as u32,
                                    "uri": uri,
                                    "range": {
                                        "start": {
                                            "line": item.range.start.line,
                                            "character": item.range.start.character,
                                        },
                                        "end": {
                                            "line": item.range.end.line,
                                            "character": item.range.end.character,
                                        },
                                    },
                                    "selectionRange": {
                                        "start": {
                                            "line": item.selection_range.start.line,
                                            "character": item.selection_range.start.character,
                                        },
                                        "end": {
                                            "line": item.selection_range.end.line,
                                            "character": item.selection_range.end.character,
                                        },
                                    },
                                    "detail": item.detail,
                                    "data": {
                                        "uri": uri,
                                        "name": item.name,
                                    },
                                })
                            })
                            .collect();

                        return Ok(Some(json!(lsp_items)));
                    }
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle typeHierarchy/subtypes request
    pub(crate) fn handle_type_hierarchy_subtypes(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            if let Some(item) = params.get("item") {
                let uri = item["data"]["uri"].as_str().unwrap_or("");
                let name = item["data"]["name"].as_str().unwrap_or("");

                let documents = self.documents_guard();
                if let Some(doc) = documents.get(uri) {
                    if let Some(ref ast) = doc.ast {
                        // Create type hierarchy provider
                        let provider = TypeHierarchyProvider::new();

                        // Extract range from request item (LSP uses camelCase)
                        let type_item = crate::type_hierarchy::TypeHierarchyItem {
                            name: name.to_string(),
                            kind: crate::type_hierarchy::SymbolKind::Class,
                            uri: uri.to_string(),
                            range: crate::type_hierarchy::Range {
                                start: crate::type_hierarchy::Position {
                                    line: item["range"]["start"]["line"].as_u64().unwrap_or(0)
                                        as u32,
                                    character: item["range"]["start"]["character"]
                                        .as_u64()
                                        .unwrap_or(0)
                                        as u32,
                                },
                                end: crate::type_hierarchy::Position {
                                    line: item["range"]["end"]["line"].as_u64().unwrap_or(0) as u32,
                                    character: item["range"]["end"]["character"]
                                        .as_u64()
                                        .unwrap_or(0)
                                        as u32,
                                },
                            },
                            selection_range: crate::type_hierarchy::Range {
                                start: crate::type_hierarchy::Position {
                                    line: item["selectionRange"]["start"]["line"]
                                        .as_u64()
                                        .unwrap_or(0)
                                        as u32,
                                    character: item["selectionRange"]["start"]["character"]
                                        .as_u64()
                                        .unwrap_or(0)
                                        as u32,
                                },
                                end: crate::type_hierarchy::Position {
                                    line: item["selectionRange"]["end"]["line"]
                                        .as_u64()
                                        .unwrap_or(0)
                                        as u32,
                                    character: item["selectionRange"]["end"]["character"]
                                        .as_u64()
                                        .unwrap_or(0)
                                        as u32,
                                },
                            },
                            detail: item["detail"].as_str().map(String::from),
                            data: item.get("data").cloned(),
                        };

                        // Find subtypes
                        let subtypes = provider.find_subtypes(ast, &type_item);

                        let lsp_items: Vec<Value> = subtypes
                            .iter()
                            .map(|item| {
                                json!({
                                    "name": item.name,
                                    "kind": item.kind as u32,
                                    "uri": uri,
                                    "range": {
                                        "start": {
                                            "line": item.range.start.line,
                                            "character": item.range.start.character,
                                        },
                                        "end": {
                                            "line": item.range.end.line,
                                            "character": item.range.end.character,
                                        },
                                    },
                                    "selectionRange": {
                                        "start": {
                                            "line": item.selection_range.start.line,
                                            "character": item.selection_range.start.character,
                                        },
                                        "end": {
                                            "line": item.selection_range.end.line,
                                            "character": item.selection_range.end.character,
                                        },
                                    },
                                    "detail": item.detail,
                                    "data": {
                                        "uri": uri,
                                        "name": item.name,
                                    },
                                })
                            })
                            .collect();

                        return Ok(Some(json!(lsp_items)));
                    }
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle prepare call hierarchy request
    pub(crate) fn handle_prepare_call_hierarchy(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        // Gate unadvertised feature
        if !self.advertised_features.lock().unwrap().call_hierarchy {
            return Err(crate::lsp_errors::method_not_advertised());
        }

        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let position = &params["position"];
            let line = position["line"].as_u64().unwrap_or(0) as u32;
            let character = position["character"].as_u64().unwrap_or(0) as u32;

            eprintln!("Preparing call hierarchy at: {} ({}:{})", uri, line, character);

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let provider = CallHierarchyProvider::new(doc.text.clone(), uri.to_string());
                    if let Some(items) = provider.prepare(ast, line, character) {
                        let json_items: Vec<_> = items.iter().map(|item| item.to_json()).collect();
                        return Ok(Some(json!(json_items)));
                    }
                }
            }
        }

        Ok(Some(json!(null)))
    }

    /// Handle incoming calls request
    pub(crate) fn handle_incoming_calls(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let item = &params["item"];
            let uri = item["uri"].as_str().unwrap_or("");

            eprintln!("Getting incoming calls for: {}", item["name"].as_str().unwrap_or(""));

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    // Reconstruct the CallHierarchyItem from JSON
                    let ch_item = self.json_to_call_hierarchy_item(item)?;

                    let provider = CallHierarchyProvider::new(doc.text.clone(), uri.to_string());
                    let calls = provider.incoming_calls(ast, &ch_item);

                    let json_calls: Vec<_> = calls.iter().map(|call| call.to_json()).collect();
                    return Ok(Some(json!(json_calls)));
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle outgoing calls request
    pub(crate) fn handle_outgoing_calls(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let item = &params["item"];
            let uri = item["uri"].as_str().unwrap_or("");

            eprintln!("Getting outgoing calls for: {}", item["name"].as_str().unwrap_or(""));

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    // Reconstruct the CallHierarchyItem from JSON
                    let ch_item = self.json_to_call_hierarchy_item(item)?;

                    let provider = CallHierarchyProvider::new(doc.text.clone(), uri.to_string());
                    let calls = provider.outgoing_calls(ast, &ch_item);

                    let json_calls: Vec<_> = calls.iter().map(|call| call.to_json()).collect();
                    return Ok(Some(json!(json_calls)));
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Convert JSON to CallHierarchyItem
    pub(crate) fn json_to_call_hierarchy_item(
        &self,
        json: &Value,
    ) -> Result<crate::call_hierarchy_provider::CallHierarchyItem, JsonRpcError> {
        use crate::call_hierarchy_provider::{CallHierarchyItem, Position, Range};

        let name = json["name"].as_str().unwrap_or("").to_string();
        let kind = match json["kind"].as_u64().unwrap_or(12) {
            6 => "method",
            _ => "function",
        }
        .to_string();
        let uri = json["uri"].as_str().unwrap_or("").to_string();

        let range = Range {
            start: Position {
                line: json["range"]["start"]["line"].as_u64().unwrap_or(0) as u32,
                character: json["range"]["start"]["character"].as_u64().unwrap_or(0) as u32,
            },
            end: Position {
                line: json["range"]["end"]["line"].as_u64().unwrap_or(0) as u32,
                character: json["range"]["end"]["character"].as_u64().unwrap_or(0) as u32,
            },
        };

        let selection_range = Range {
            start: Position {
                line: json["selectionRange"]["start"]["line"].as_u64().unwrap_or(0) as u32,
                character: json["selectionRange"]["start"]["character"].as_u64().unwrap_or(0)
                    as u32,
            },
            end: Position {
                line: json["selectionRange"]["end"]["line"].as_u64().unwrap_or(0) as u32,
                character: json["selectionRange"]["end"]["character"].as_u64().unwrap_or(0) as u32,
            },
        };

        let detail = json["detail"].as_str().map(|s| s.to_string());

        Ok(CallHierarchyItem { name, kind, uri, range, selection_range, detail })
    }
}
