//! Hierarchy handlers for type and call hierarchy
//!
//! Handles prepareTypeHierarchy, typeHierarchy/supertypes, typeHierarchy/subtypes,
//! prepareCallHierarchy, callHierarchy/incomingCalls, and callHierarchy/outgoingCalls.

use super::super::*;
use crate::protocol::{req_position, req_uri};
use perl_position_tracking::{WirePosition, WireRange};
use std::sync::OnceLock;

static SUB_REGEX: OnceLock<Result<regex::Regex, regex::Error>> = OnceLock::new();
static PACKAGE_REGEX: OnceLock<Result<regex::Regex, regex::Error>> = OnceLock::new();

fn get_sub_regex() -> Option<&'static regex::Regex> {
    SUB_REGEX.get_or_init(|| regex::Regex::new(r"\bsub\s+([a-zA-Z_]\w*)\b")).as_ref().ok()
}

fn get_package_regex() -> Option<&'static regex::Regex> {
    PACKAGE_REGEX
        .get_or_init(|| regex::Regex::new(r"\bpackage\s+([a-zA-Z_][\w:]*)\b"))
        .as_ref()
        .ok()
}

impl LspServer {
    /// Handle textDocument/prepareTypeHierarchy request
    pub(crate) fn handle_prepare_type_hierarchy(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;

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
                let Some(sub_regex) = get_sub_regex() else {
                    return Ok(Some(json!([])));
                };
                let Some(package_regex) = get_package_regex() else {
                    return Ok(Some(json!([])));
                };

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
                            kind: crate::type_hierarchy::TypeHierarchySymbolKind::Class,
                            uri: uri.to_string(),
                            range: WireRange::new(
                                WirePosition::new(
                                    item["range"]["start"]["line"].as_u64().unwrap_or(0) as u32,
                                    item["range"]["start"]["character"].as_u64().unwrap_or(0)
                                        as u32,
                                ),
                                WirePosition::new(
                                    item["range"]["end"]["line"].as_u64().unwrap_or(0) as u32,
                                    item["range"]["end"]["character"].as_u64().unwrap_or(0) as u32,
                                ),
                            ),
                            selection_range: WireRange::new(
                                WirePosition::new(
                                    item["selectionRange"]["start"]["line"].as_u64().unwrap_or(0)
                                        as u32,
                                    item["selectionRange"]["start"]["character"]
                                        .as_u64()
                                        .unwrap_or(0) as u32,
                                ),
                                WirePosition::new(
                                    item["selectionRange"]["end"]["line"].as_u64().unwrap_or(0)
                                        as u32,
                                    item["selectionRange"]["end"]["character"].as_u64().unwrap_or(0)
                                        as u32,
                                ),
                            ),
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
                            kind: crate::type_hierarchy::TypeHierarchySymbolKind::Class,
                            uri: uri.to_string(),
                            range: WireRange::new(
                                WirePosition::new(
                                    item["range"]["start"]["line"].as_u64().unwrap_or(0) as u32,
                                    item["range"]["start"]["character"].as_u64().unwrap_or(0)
                                        as u32,
                                ),
                                WirePosition::new(
                                    item["range"]["end"]["line"].as_u64().unwrap_or(0) as u32,
                                    item["range"]["end"]["character"].as_u64().unwrap_or(0) as u32,
                                ),
                            ),
                            selection_range: WireRange::new(
                                WirePosition::new(
                                    item["selectionRange"]["start"]["line"].as_u64().unwrap_or(0)
                                        as u32,
                                    item["selectionRange"]["start"]["character"]
                                        .as_u64()
                                        .unwrap_or(0) as u32,
                                ),
                                WirePosition::new(
                                    item["selectionRange"]["end"]["line"].as_u64().unwrap_or(0)
                                        as u32,
                                    item["selectionRange"]["end"]["character"].as_u64().unwrap_or(0)
                                        as u32,
                                ),
                            ),
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
        if !self.advertised_features.lock().call_hierarchy {
            return Err(crate::protocol::method_not_advertised());
        }

        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;

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
    ///
    /// Searches ALL open workspace documents for callers of the target function,
    /// not just the document that contains the function definition.
    pub(crate) fn handle_incoming_calls(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let item = &params["item"];
            let target_name = item
                .get("data")
                .and_then(|d| d.get("qualified_name"))
                .and_then(Value::as_str)
                .filter(|name| !name.is_empty())
                .or_else(|| item["name"].as_str())
                .unwrap_or("");

            eprintln!("Getting incoming calls for: {}", target_name);

            let ch_item = self.json_to_call_hierarchy_item(item)?;

            // Snapshot (doc_uri, text, ast) for all open documents so we can
            // release the lock before the per-document provider work.
            let documents = self.documents_guard();
            let doc_snapshots: Vec<(String, String, std::sync::Arc<perl_parser::ast::Node>)> =
                documents
                    .iter()
                    .filter_map(|(doc_uri, doc)| {
                        doc.ast.as_ref().map(|ast| (doc_uri.clone(), doc.text.clone(), ast.clone()))
                    })
                    .collect();
            drop(documents);

            // Incoming call results keyed by (from_name, from_uri) to deduplicate
            // callers that appear in multiple scan passes.
            let mut seen: std::collections::HashMap<(String, String), usize> =
                std::collections::HashMap::new();
            let mut all_calls: Vec<crate::call_hierarchy_provider::CallHierarchyIncomingCall> =
                Vec::new();

            for (doc_uri, doc_text, ast) in doc_snapshots {
                let provider = CallHierarchyProvider::new(doc_text, doc_uri.clone());
                let calls = provider.incoming_calls(&ast, &ch_item);
                for call in calls {
                    let key = (call.from.name.clone(), call.from.uri.clone());
                    if let Some(&idx) = seen.get(&key) {
                        all_calls[idx].from_ranges.extend(call.from_ranges);
                    } else {
                        seen.insert(key, all_calls.len());
                        all_calls.push(call);
                    }
                }
            }

            let json_calls: Vec<_> = all_calls.iter().map(|c| c.to_json()).collect();
            return Ok(Some(json!(json_calls)));
        }

        Ok(Some(json!([])))
    }

    /// Handle outgoing calls request
    ///
    /// Finds all calls made within the target function, then resolves each
    /// callee's definition URI by searching all open workspace documents.
    pub(crate) fn handle_outgoing_calls(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let item = &params["item"];
            let uri = item["uri"].as_str().unwrap_or("");

            eprintln!("Getting outgoing calls for: {}", item["name"].as_str().unwrap_or(""));

            // Snapshot all open documents for cross-file callee resolution.
            let documents = self.documents_guard();
            let doc_snapshots: Vec<(String, String, std::sync::Arc<perl_parser::ast::Node>)> =
                documents
                    .iter()
                    .filter_map(|(doc_uri, doc)| {
                        doc.ast.as_ref().map(|ast| (doc_uri.clone(), doc.text.clone(), ast.clone()))
                    })
                    .collect();

            // Find outgoing calls within the target function's file.
            let mut calls = if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let ch_item = self.json_to_call_hierarchy_item(item)?;
                    let provider = CallHierarchyProvider::new(doc.text.clone(), uri.to_string());
                    provider.outgoing_calls(ast, &ch_item)
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };
            drop(documents);

            // Resolve each callee's definition URI from workspace documents.
            // Strip any package qualifier (e.g. "Utils::format_string" -> "format_string")
            // before searching, since the provider stores bare names from AST nodes.
            for call in &mut calls {
                let bare_name =
                    call.to.name.split("::").last().unwrap_or(&call.to.name).to_string();
                'outer: for (doc_uri, doc_text, ast) in &doc_snapshots {
                    let provider = CallHierarchyProvider::new(doc_text.clone(), doc_uri.clone());
                    if doc_uri == &call.to.uri {
                        // Already pointing at this file — keep if definition exists here.
                        if provider.find_definition(&bare_name, ast).is_some() {
                            break 'outer;
                        }
                        continue;
                    }
                    if let Some(def_item) = provider.find_definition(&bare_name, ast) {
                        call.to.uri = def_item.uri;
                        call.to.range = def_item.range;
                        call.to.selection_range = def_item.selection_range;
                        break 'outer;
                    }
                }
            }

            let json_calls: Vec<_> = calls.iter().map(|c| c.to_json()).collect();
            return Ok(Some(json!(json_calls)));
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
        let qualified_name = json
            .get("data")
            .and_then(|d| d.get("qualified_name"))
            .and_then(Value::as_str)
            .map(str::to_string);

        Ok(CallHierarchyItem { name, kind, uri, range, selection_range, detail, qualified_name })
    }
}
