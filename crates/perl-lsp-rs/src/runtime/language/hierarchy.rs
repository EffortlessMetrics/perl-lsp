//! Hierarchy handlers for type and call hierarchy
//!
//! Handles prepareTypeHierarchy, typeHierarchy/supertypes, typeHierarchy/subtypes,
//! prepareCallHierarchy, callHierarchy/incomingCalls, and callHierarchy/outgoingCalls.

use super::super::*;
use crate::protocol::{req_position, req_uri};
#[cfg(feature = "workspace")]
use crate::runtime::readiness::IndexReadinessPolicy;
#[cfg(feature = "workspace")]
use crate::runtime::routing::{IndexAccessMode, route_index_access};
#[cfg(feature = "workspace")]
use crate::workspace_index::{
    Location as IndexLocation, SymKind, SymbolKey, SymbolKind, WorkspaceSymbol,
};
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

#[cfg(feature = "workspace")]
fn is_callable_symbol(symbol: &WorkspaceSymbol) -> bool {
    matches!(symbol.kind, SymbolKind::Subroutine | SymbolKind::Method)
}

#[cfg(feature = "workspace")]
fn item_package_name(item: &crate::call_hierarchy_provider::CallHierarchyItem) -> Option<&str> {
    item.package_name.as_deref().or_else(|| {
        item.qualified_name
            .as_deref()
            .and_then(|qualified| qualified.rsplit_once("::").map(|(pkg, _)| pkg))
    })
}

#[cfg(feature = "workspace")]
fn range_contains_points(
    outer_start: (u32, u32),
    outer_end: (u32, u32),
    inner_start: (u32, u32),
    inner_end: (u32, u32),
) -> bool {
    outer_start <= inner_start && outer_end >= inner_end
}

#[cfg(feature = "workspace")]
fn index_location_to_wire_range(location: &IndexLocation) -> WireRange {
    WireRange::new(
        WirePosition::new(location.range.start.line, location.range.start.column),
        WirePosition::new(location.range.end.line, location.range.end.column),
    )
}

#[cfg(feature = "workspace")]
fn workspace_symbol_to_item(
    symbol: &WorkspaceSymbol,
) -> crate::call_hierarchy_provider::CallHierarchyItem {
    let qualified_name = symbol.qualified_name.clone().or_else(|| {
        symbol.container_name.as_ref().map(|package| format!("{package}::{}", symbol.name))
    });
    crate::call_hierarchy_provider::CallHierarchyItem {
        name: symbol.name.clone(),
        kind: match symbol.kind {
            SymbolKind::Method => "method",
            _ => "function",
        }
        .to_string(),
        uri: symbol.uri.clone(),
        range: WireRange::new(
            WirePosition::new(symbol.range.start.line, symbol.range.start.column),
            WirePosition::new(symbol.range.end.line, symbol.range.end.column),
        ),
        selection_range: WireRange::new(
            WirePosition::new(symbol.range.start.line, symbol.range.start.column),
            WirePosition::new(symbol.range.end.line, symbol.range.end.column),
        ),
        detail: None,
        package_name: symbol.container_name.clone(),
        qualified_name,
    }
}

impl LspServer {
    #[cfg(feature = "workspace")]
    fn enrich_call_hierarchy_item(
        &self,
        item: crate::call_hierarchy_provider::CallHierarchyItem,
    ) -> crate::call_hierarchy_provider::CallHierarchyItem {
        let access_mode = route_index_access(self.coordinator());
        let IndexAccessMode::Full(coordinator) = access_mode else {
            return item;
        };

        coordinator
            .index()
            .search_symbols(&item.name)
            .into_iter()
            .filter(|symbol| is_callable_symbol(symbol) && symbol.uri == item.uri)
            .find(|symbol| {
                symbol.range.start.line == item.selection_range.start.line
                    && symbol.range.start.column == item.selection_range.start.character
            })
            .map(|symbol| {
                let mut enriched = workspace_symbol_to_item(&symbol);
                enriched.detail = item.detail.clone();
                enriched
            })
            .unwrap_or(item)
    }

    #[cfg(feature = "workspace")]
    fn find_workspace_enclosing_callable(
        &self,
        symbols: &[WorkspaceSymbol],
        location: &IndexLocation,
    ) -> Option<crate::call_hierarchy_provider::CallHierarchyItem> {
        symbols
            .iter()
            .filter(|symbol| is_callable_symbol(symbol) && symbol.uri == location.uri)
            .filter(|symbol| {
                range_contains_points(
                    (symbol.range.start.line, symbol.range.start.column),
                    (symbol.range.end.line, symbol.range.end.column),
                    (location.range.start.line, location.range.start.column),
                    (location.range.end.line, location.range.end.column),
                )
            })
            .min_by_key(|symbol| {
                (
                    symbol.range.end.line.saturating_sub(symbol.range.start.line),
                    symbol.range.end.column.saturating_sub(symbol.range.start.column),
                )
            })
            .map(workspace_symbol_to_item)
    }

    #[cfg(feature = "workspace")]
    fn workspace_symbol_key(
        &self,
        item: &crate::call_hierarchy_provider::CallHierarchyItem,
    ) -> Option<SymbolKey> {
        let package_name = item_package_name(item)?;
        Some(SymbolKey {
            pkg: package_name.to_string().into(),
            name: item.name.clone().into(),
            sigil: None,
            kind: SymKind::Sub,
        })
    }

    #[cfg(feature = "workspace")]
    fn resolve_workspace_outgoing_target(
        &self,
        workspace_symbols: &[WorkspaceSymbol],
        current_item: &crate::call_hierarchy_provider::CallHierarchyItem,
        call: &crate::call_hierarchy_provider::CallHierarchyOutgoingCall,
    ) -> Option<crate::call_hierarchy_provider::CallHierarchyItem> {
        let access_mode = route_index_access(self.coordinator());
        let IndexAccessMode::Full(coordinator) = access_mode else {
            return None;
        };
        let index = coordinator.index();

        let mut candidates = Vec::new();
        if let Some(qualified_name) = &call.to.qualified_name {
            candidates.push(qualified_name.clone());
        } else if call.to.name.contains("::") {
            candidates.push(call.to.name.clone());
        } else if let Some(package_name) = item_package_name(current_item) {
            candidates.push(format!("{package_name}::{}", call.to.name));
        }
        candidates.push(call.to.name.clone());

        for candidate in candidates {
            if let Some(location) = index.find_definition(&candidate) {
                if let Some(symbol) = workspace_symbols.iter().find(|symbol| {
                    is_callable_symbol(symbol)
                        && symbol.uri == location.uri
                        && range_contains_points(
                            (symbol.range.start.line, symbol.range.start.column),
                            (symbol.range.end.line, symbol.range.end.column),
                            (location.range.start.line, location.range.start.column),
                            (location.range.end.line, location.range.end.column),
                        )
                }) {
                    return Some(workspace_symbol_to_item(symbol));
                }

                let bare_name = candidate.rsplit("::").next().unwrap_or(&candidate);
                let range = index_location_to_wire_range(&location);
                return Some(crate::call_hierarchy_provider::CallHierarchyItem {
                    name: bare_name.to_string(),
                    kind: call.to.kind.clone(),
                    uri: location.uri.clone(),
                    range,
                    selection_range: range,
                    detail: call.to.detail.clone(),
                    package_name: candidate
                        .rsplit_once("::")
                        .map(|(package_name, _)| package_name.to_string()),
                    qualified_name: candidate.contains("::").then_some(candidate),
                });
            }
        }

        None
    }

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

            tracing::debug!(uri, line, character, "Preparing call hierarchy");

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let provider = CallHierarchyProvider::new(doc.text.clone(), uri.to_string());
                    if let Some(items) = provider.prepare(ast, line, character) {
                        // Wait for the workspace index to finish building before enriching items.
                        // enrich_call_hierarchy_item calls route_index_access; without the wait
                        // items are returned with no workspace-enriched detail during indexing.
                        // Mirrors the pattern used by completion (#3069) and workspace/symbol (#1514).
                        #[cfg(feature = "workspace")]
                        let _ = self.check_index_readiness(IndexReadinessPolicy::WaitBriefly);
                        #[cfg(feature = "workspace")]
                        let items: Vec<_> = items
                            .into_iter()
                            .map(|item| self.enrich_call_hierarchy_item(item))
                            .collect();
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
            let target_name = item["name"].as_str().unwrap_or("");

            tracing::debug!(target = target_name, "Getting incoming calls");

            let ch_item = self.json_to_call_hierarchy_item(item)?;

            let mut all_calls: Vec<crate::call_hierarchy_provider::CallHierarchyIncomingCall> =
                Vec::new();
            let mut seen: std::collections::HashMap<(String, String), usize> =
                std::collections::HashMap::new();

            // Wait for the workspace index to finish building before querying it.
            // Without this, an incomingCalls request while the index is in Building
            // state routes to Partial and returns no cross-file callers.
            // Mirrors the pattern used by completion (#3069) and workspace/symbol (#1514).
            #[cfg(feature = "workspace")]
            let _ = self.check_index_readiness(IndexReadinessPolicy::WaitBriefly);
            #[cfg(feature = "workspace")]
            if let Some(symbol_key) = self.workspace_symbol_key(&ch_item) {
                let access_mode = route_index_access(self.coordinator());
                if let IndexAccessMode::Full(coordinator) = access_mode {
                    let index = coordinator.index();
                    let callable_symbols = index.search_symbols("");
                    let refs = index.find_refs(&symbol_key);

                    for location in refs {
                        let from_range = index_location_to_wire_range(&location);
                        let from = self
                            .find_workspace_enclosing_callable(&callable_symbols, &location)
                            .unwrap_or_else(|| {
                                // Top-level call site — no enclosing callable in the
                                // workspace index.  Synthesize a file-level caller so the
                                // script appears in incomingCalls instead of being dropped.
                                crate::call_hierarchy_provider::synthetic_file_level_caller(
                                    &location.uri,
                                    from_range,
                                )
                            });
                        let key = (from.name.clone(), from.uri.clone());
                        if let Some(&idx) = seen.get(&key) {
                            all_calls[idx].from_ranges.push(from_range);
                        } else {
                            seen.insert(key, all_calls.len());
                            all_calls.push(
                                crate::call_hierarchy_provider::CallHierarchyIncomingCall {
                                    from,
                                    from_ranges: vec![from_range],
                                },
                            );
                        }
                    }
                }
            }

            // Snapshot (doc_uri, text, ast) for the open-document fallback so we can
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
            let ch_item = self.json_to_call_hierarchy_item(item)?;

            tracing::debug!(target = item["name"].as_str().unwrap_or(""), "Getting outgoing calls");

            // Snapshot all open documents for fallback callee resolution.
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
                    let provider = CallHierarchyProvider::new(doc.text.clone(), uri.to_string());
                    provider.outgoing_calls(ast, &ch_item)
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };
            drop(documents);

            let mut resolved_with_workspace = vec![false; calls.len()];

            // Wait for the workspace index to finish building before querying it.
            // Without this, an outgoingCalls request while the index is in Building
            // state routes to Partial and callees are not resolved cross-file.
            // Mirrors the pattern used by completion (#3069) and workspace/symbol (#1514).
            #[cfg(feature = "workspace")]
            let _ = self.check_index_readiness(IndexReadinessPolicy::WaitBriefly);
            #[cfg(feature = "workspace")]
            {
                let access_mode = route_index_access(self.coordinator());
                if let IndexAccessMode::Full(coordinator) = access_mode {
                    let workspace_symbols = coordinator.index().search_symbols("");
                    for (idx, call) in calls.iter_mut().enumerate() {
                        if let Some(resolved_item) = self.resolve_workspace_outgoing_target(
                            &workspace_symbols,
                            &ch_item,
                            call,
                        ) {
                            call.to = resolved_item;
                            resolved_with_workspace[idx] = true;
                        }
                    }
                }
            }

            // Resolve each callee's definition URI from workspace documents.
            // Strip any package qualifier (e.g. "Utils::format_string" -> "format_string")
            // before searching, since the provider stores bare names from AST nodes.
            for (idx, call) in calls.iter_mut().enumerate() {
                if resolved_with_workspace[idx] {
                    continue;
                }
                let bare_name =
                    call.to.name.split("::").last().unwrap_or(&call.to.name).to_string();
                // For qualified calls (e.g. `Utils::format_string`), prefer the
                // package-matching file first.  For bare calls, accept any match.
                let qualified_pkg =
                    call.to.name.rsplit_once("::").map(|(pkg, _)| pkg.replace("::", "/"));

                'outer: for (doc_uri, doc_text, ast) in &doc_snapshots {
                    let provider = CallHierarchyProvider::new(doc_text.clone(), doc_uri.clone());
                    if let Some(ref pkg_path) = qualified_pkg {
                        // Qualified call — only match files whose URI contains the
                        // package path (e.g. "Utils" → ".../Utils.pm").
                        if !doc_uri.as_str().contains(pkg_path) {
                            continue;
                        }
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
        let package_name = json["data"]["packageName"].as_str().map(|s| s.to_string());
        let qualified_name = json["data"]["qualifiedName"].as_str().map(|s| s.to_string());

        Ok(CallHierarchyItem {
            name,
            kind,
            uri,
            range,
            selection_range,
            detail,
            package_name,
            qualified_name,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_doc(server: &LspServer, uri: &str, text: &str) {
        let result = server.test_handle_did_open(Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": text,
            }
        })));
        assert!(result.is_ok(), "didOpen failed: {result:?}");
    }

    /// Verifies that `handle_prepare_call_hierarchy` executes the workspace
    /// index-readiness wait when indexing is in progress (#3095).
    ///
    /// The wait call short-circuits immediately (coordinator is Ready by
    /// default) but the line must execute to satisfy patch coverage.
    #[cfg(feature = "workspace")]
    #[test]
    fn test_wait_guard_fires_in_prepare_call_hierarchy_when_indexing_in_progress() {
        let server = LspServer::new();
        // Expose the feature gate so the handler reaches the wait line.
        server.test_enable_call_hierarchy();
        let uri = "file:///test-hierarchy.pl";
        open_doc(
            &server,
            uri,
            "\nsub main {\n    helper();\n}\nsub helper {\n    print \"hi\\n\";\n}\n",
        );
        // Simulate the race window: indexing flag set, coordinator still Ready.
        // The wait exits immediately on the first Ready check.
        server.test_simulate_indexing_start();
        let result = server.handle_prepare_call_hierarchy(Some(json!({
            "textDocument": { "uri": uri },
            "position": { "line": 1, "character": 5 }
        })));
        // The handler must not panic and must return a result.
        assert!(result.is_ok(), "handle_prepare_call_hierarchy must not error: {result:?}");
    }

    /// Verifies that `handle_incoming_calls` executes the workspace
    /// index-readiness wait when indexing is in progress (#3095).
    #[cfg(feature = "workspace")]
    #[test]
    fn test_wait_guard_fires_in_incoming_calls_when_indexing_in_progress() {
        let server = LspServer::new();
        // Simulate the race window; coordinator is Ready so the wait returns instantly.
        server.test_simulate_indexing_start();
        let result = server.handle_incoming_calls(Some(json!({
            "item": {
                "name": "main",
                "kind": 12,
                "uri": "file:///test.pl",
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 2, "character": 1 }
                },
                "selectionRange": {
                    "start": { "line": 0, "character": 4 },
                    "end": { "line": 0, "character": 8 }
                }
            }
        })));
        assert!(result.is_ok(), "handle_incoming_calls must not error: {result:?}");
    }

    /// Verifies that the workspace-index path in `handle_incoming_calls` synthesizes
    /// a file-level `CallHierarchyItem` (kind=1/File) when a reference location in
    /// the index has no enclosing callable symbol — i.e., it is a top-level call.
    ///
    /// Covers lines 633-645 (the `unwrap_or_else` closure + seen-map insert) for the
    /// Codecov/Patch-95 gate (#3093).
    #[cfg(feature = "workspace")]
    #[test]
    fn test_incoming_calls_workspace_path_synthesizes_file_level_caller() {
        let server = LspServer::new();
        server.test_enable_call_hierarchy();

        // script.pl: static method call at the TOP LEVEL (no enclosing sub).
        // App->run() is a static call so workspace_index stores it as "App::run".
        let script_uri = "file:///script.pl";
        let script_text = "App->run();\n";

        // Index the file (transitions coordinator to Building internally).
        server
            .test_index_file_in_building_state(script_uri, script_text)
            .expect("indexing script.pl");
        // Transition coordinator to Ready so workspace path is taken.
        server.test_simulate_indexing_complete();

        // Also open as a document so open-doc fallback doesn't add duplicates.
        open_doc(&server, script_uri, script_text);

        // incomingCalls for "App::run" — data.packageName drives workspace_symbol_key.
        let result = server.handle_incoming_calls(Some(json!({
            "item": {
                "name": "run",
                "kind": 6,
                "uri": "file:///App.pm",
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end":   { "line": 2, "character": 1 }
                },
                "selectionRange": {
                    "start": { "line": 1, "character": 4 },
                    "end":   { "line": 1, "character": 7 }
                },
                "data": {
                    "packageName": "App",
                    "qualifiedName": "App::run"
                }
            }
        })));

        assert!(result.is_ok(), "handle_incoming_calls must not error: {result:?}");
        let value = result.expect("already checked");
        let value = value.expect("handler must return Some value");
        // handle_incoming_calls returns the calls array directly (not wrapped in {"result":...})
        let calls = value.as_array().expect("result should be an array");

        // The reference in script.pl has no enclosing callable, so the workspace
        // path must synthesize a file-level caller with kind=1 (SymbolKind.File).
        let file_caller = calls
            .iter()
            .find(|c| c["from"]["uri"].as_str().map_or(false, |u| u.contains("script.pl")));
        assert!(file_caller.is_some(), "expected file-level caller from script.pl, got: {calls:?}");
        let from = &file_caller.expect("already checked")["from"];
        assert_eq!(
            from["kind"].as_u64(),
            Some(1),
            "file-level caller must have SymbolKind.File=1, got: {from:?}"
        );
        assert_eq!(from["name"].as_str(), Some("script.pl"));
    }

    #[test]
    fn test_incoming_calls_open_doc_fallback_finds_top_level_script_method_call()
    -> anyhow::Result<()> {
        let server = LspServer::new();
        server.test_enable_call_hierarchy();

        let app_uri = "file:///lib/RealBaseline/App.pm";
        let app_text = "package RealBaseline::App;\n\nsub run {\n    return 1;\n}\n\n1;\n";
        let script_uri = "file:///script/real-baseline.pl";
        let script_text =
            "use RealBaseline::App;\n\nmy $app = RealBaseline::App->new();\n$app->run;\n";
        open_doc(&server, app_uri, app_text);
        open_doc(&server, script_uri, script_text);

        let prepared = server
            .handle_prepare_call_hierarchy(Some(json!({
                "textDocument": { "uri": app_uri },
                "position": { "line": 2, "character": 5 }
            })))
            .map_err(|err| anyhow::anyhow!("prepareCallHierarchy failed: {err:?}"))?
            .ok_or_else(|| anyhow::anyhow!("prepareCallHierarchy returned no response"))?;
        let items = prepared
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("prepareCallHierarchy must return an array"))?;
        let item = items
            .first()
            .ok_or_else(|| anyhow::anyhow!("prepareCallHierarchy returned no items"))?;

        let incoming = server
            .handle_incoming_calls(Some(json!({ "item": item })))
            .map_err(|err| anyhow::anyhow!("incomingCalls failed: {err:?}"))?
            .ok_or_else(|| anyhow::anyhow!("incomingCalls returned no response"))?;
        let calls = incoming
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("incomingCalls must return an array"))?;

        let script_caller = calls.iter().any(|call| {
            call["from"]["uri"].as_str().map_or(false, |uri| uri.ends_with("real-baseline.pl"))
        });
        assert!(
            script_caller,
            "expected incomingCalls to include script/real-baseline.pl, got: {calls:?}"
        );
        Ok(())
    }

    /// Verifies that `handle_outgoing_calls` executes the workspace
    /// index-readiness wait when indexing is in progress (#3095).
    #[cfg(feature = "workspace")]
    #[test]
    fn test_wait_guard_fires_in_outgoing_calls_when_indexing_in_progress() {
        let server = LspServer::new();
        // Simulate the race window; coordinator is Ready so the wait returns instantly.
        server.test_simulate_indexing_start();
        let result = server.handle_outgoing_calls(Some(json!({
            "item": {
                "name": "helper",
                "kind": 12,
                "uri": "file:///test.pl",
                "range": {
                    "start": { "line": 4, "character": 0 },
                    "end": { "line": 6, "character": 1 }
                },
                "selectionRange": {
                    "start": { "line": 4, "character": 4 },
                    "end": { "line": 4, "character": 10 }
                }
            }
        })));
        assert!(result.is_ok(), "handle_outgoing_calls must not error: {result:?}");
    }
}
