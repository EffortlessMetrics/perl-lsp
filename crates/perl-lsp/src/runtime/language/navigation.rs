//! Navigation handlers for go-to-definition, declaration, and related features
//!
//! Handles textDocument/declaration, textDocument/definition, textDocument/typeDefinition,
//! and textDocument/implementation requests.

#![allow(unsafe_code)]

use super::super::*;
use crate::cancellation::RequestCleanupGuard;
use crate::protocol::{req_position, req_uri};
use crate::util::token_under_cursor;
use std::collections::HashMap;

#[cfg(feature = "workspace")]
use crate::runtime::routing::{IndexAccessMode, route_index_access};
#[cfg(feature = "workspace")]
use std::sync::OnceLock;

#[cfg(feature = "workspace")]
static FQN_RE: OnceLock<Result<regex::Regex, regex::Error>> = OnceLock::new();

#[cfg(feature = "workspace")]
static ARROW_METHOD_RE: OnceLock<Result<regex::Regex, regex::Error>> = OnceLock::new();

#[cfg(feature = "workspace")]
fn get_fqn_regex() -> Option<&'static regex::Regex> {
    FQN_RE
        .get_or_init(|| regex::Regex::new(r"([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)"))
        .as_ref()
        .ok()
}

#[cfg(feature = "workspace")]
fn get_arrow_method_regex() -> Option<&'static regex::Regex> {
    ARROW_METHOD_RE
        .get_or_init(|| {
            regex::Regex::new(
                r"([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)\s*->\s*([A-Za-z_][A-Za-z0-9_]*)",
            )
        })
        .as_ref()
        .ok()
}

/// Look up a symbol definition in the workspace index.
///
/// Tries two lookup strategies:
/// 1. `find_def()` with a structured `SymbolKey`
/// 2. `find_definition()` with a formatted `Package::name` string
///
/// Returns the LSP location if found, or `None` to fall through to same-file resolution.
#[cfg(feature = "workspace")]
fn lookup_workspace_definition(
    coordinator: Option<&std::sync::Arc<crate::workspace_index::IndexCoordinator>>,
    pkg: &str,
    name: &str,
) -> Option<Value> {
    let access_mode = route_index_access(coordinator);
    let IndexAccessMode::Full(coord) = access_mode else {
        return None;
    };

    let key = crate::workspace_index::SymbolKey {
        pkg: pkg.to_string().into(),
        name: name.to_string().into(),
        sigil: None,
        kind: crate::workspace_index::SymKind::Sub,
    };

    let workspace_index = coord.index();

    // Try structured key lookup first
    if let Some(def_location) = workspace_index.find_def(&key) {
        if let Some(lsp_location) =
            crate::workspace_index::lsp_adapter::to_lsp_location(&def_location)
        {
            return Some(json!([lsp_location]));
        }
    }

    // Fall back to string-based lookup
    let symbol_name = format!("{}::{}", pkg, name);
    if let Some(def_location) = workspace_index.find_definition(&symbol_name) {
        if let Some(lsp_location) =
            crate::workspace_index::lsp_adapter::to_lsp_location(&def_location)
        {
            return Some(json!([lsp_location]));
        }
    }

    None
}

impl LspServer {
    /// Handle textDocument/declaration request
    pub(crate) fn handle_declaration(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let t0 = std::time::Instant::now();

        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let offset = self.pos16_to_offset(doc, line, character);

                    // Use the Declaration provider - ast is already an Arc
                    // SAFETY: parent_map is owned by the same DocumentState as the AST, ensuring pointer validity
                    let provider = unsafe {
                        crate::declaration::DeclarationProvider::new(
                            Arc::clone(ast),
                            doc.text.clone(),
                            uri.to_string(),
                        )
                        .with_parent_map(&doc.parent_map)
                        .with_doc_version(doc.version)
                    };

                    // Find declaration at the position
                    if let Some(location_links) = provider.find_declaration(offset, doc.version) {
                        // Check client capability and return appropriate format
                        if self.client_capabilities.declaration_link_support {
                            // Return LocationLink format
                            let result: Vec<Value> = location_links
                                .iter()
                                .map(|link| {
                                    let (orig_start_line, orig_start_char) =
                                        self.offset_to_pos16(doc, link.origin_selection_range.0);
                                    let (orig_end_line, orig_end_char) =
                                        self.offset_to_pos16(doc, link.origin_selection_range.1);

                                    let (target_start_line, target_start_char) =
                                        self.offset_to_pos16(doc, link.target_range.0);
                                    let (target_end_line, target_end_char) =
                                        self.offset_to_pos16(doc, link.target_range.1);

                                    let (sel_start_line, sel_start_char) =
                                        self.offset_to_pos16(doc, link.target_selection_range.0);
                                    let (sel_end_line, sel_end_char) =
                                        self.offset_to_pos16(doc, link.target_selection_range.1);

                                    json!({
                                            "originSelectionRange": {
                                                "start": {
                                                    "line": orig_start_line,
                                                    "character": orig_start_char,
                                                },
                                                "end": {
                                                    "line": orig_end_line,
                                                    "character": orig_end_char,
                                                },
                                            },
                                            "targetUri": link.target_uri,
                                            "targetRange": {
                                            "start": {
                                                "line": target_start_line,
                                                "character": target_start_char,
                                            },
                                            "end": {
                                                "line": target_end_line,
                                                "character": target_end_char,
                                            },
                                        },
                                        "targetSelectionRange": {
                                            "start": {
                                                "line": sel_start_line,
                                                "character": sel_start_char,
                                            },
                                            "end": {
                                                "line": sel_end_line,
                                                "character": sel_end_char,
                                            },
                                        },
                                    })
                                })
                                .collect();

                            return Ok(Some(json!(result)));
                        } else {
                            // Down-convert to Location format for clients that don't support LocationLink
                            let result: Vec<Value> = location_links
                                .iter()
                                .map(|link| {
                                    let (sel_start_line, sel_start_char) =
                                        self.offset_to_pos16(doc, link.target_selection_range.0);
                                    let (sel_end_line, sel_end_char) =
                                        self.offset_to_pos16(doc, link.target_selection_range.1);

                                    json!({
                                        "uri": link.target_uri,
                                        "range": {
                                            "start": {
                                                "line": sel_start_line,
                                                "character": sel_start_char,
                                            },
                                            "end": {
                                                "line": sel_end_line,
                                                "character": sel_end_char,
                                            },
                                        },
                                    })
                                })
                                .collect();

                            return Ok(Some(json!(result)));
                        }
                    }
                }

                // Performance monitoring
                let dt = t0.elapsed();
                if doc.text.len() < 50_000 && dt > std::time::Duration::from_millis(50) {
                    eprintln!("[warn] slow declaration: {:?} (uri={})", dt, uri);
                }
            }
        }
        Ok(Some(json!([])))
    }

    /// Handle textDocument/definition request
    pub(crate) fn handle_definition(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;

            // First, extract module reference info while holding the document lock briefly
            // We need to release the lock before calling resolve_module_to_path to avoid deadlock
            let module_lookup_info: Option<(String, String)> = {
                let documents = self.documents_guard();
                if let Some(doc) = self.get_document(&documents, uri) {
                    let offset = self.pos16_to_offset(doc, line, character);
                    let radius = 50;
                    let text_start = offset.saturating_sub(radius);
                    let text_around = self.get_text_around_offset(&doc.text, offset, radius);
                    let cursor_in_text = offset - text_start;

                    // Check for patterns like "use Module::Name", "require Module::Name"
                    if let Some(module_name) =
                        self.extract_module_reference(&text_around, cursor_in_text)
                    {
                        Some((module_name, text_around.clone()))
                    } else {
                        // Also check if we're on a package name followed by ->
                        let package_pattern = regex::Regex::new(
                            r"([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)\s*->",
                        )
                        .ok();
                        let mut package_name_result = None;
                        if let Some(re) = package_pattern {
                            for cap in re.captures_iter(&text_around) {
                                if let Some(package_match) = cap.get(1) {
                                    let match_start = package_match.start();
                                    let match_end = package_match.end();
                                    if cursor_in_text >= match_start && cursor_in_text <= match_end
                                    {
                                        package_name_result = Some((
                                            package_match.as_str().to_string(),
                                            text_around.clone(),
                                        ));
                                        break;
                                    }
                                }
                            }
                        }
                        package_name_result
                    }
                } else {
                    None
                }
            };
            // Lock is released here

            // Now resolve module to path WITHOUT holding the document lock
            if let Some((module_name, _text_around)) = module_lookup_info {
                if let Some(module_path) = self.resolve_module_to_path(&module_name) {
                    return Ok(Some(json!([{
                        "uri": module_path,
                        "range": {
                            "start": {
                                "line": 0,
                                "character": 0,
                            },
                            "end": {
                                "line": 0,
                                "character": 0,
                            },
                        },
                    }])));
                }
            }

            // Continue with remaining definition lookup logic that needs document access
            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                let offset = self.pos16_to_offset(doc, line, character);
                let radius = 50;
                let text_start = offset.saturating_sub(radius);
                let text_around = self.get_text_around_offset(&doc.text, offset, radius);
                let cursor_in_text = offset - text_start;

                #[cfg(feature = "workspace")]
                {
                    // Attempt to resolve fully-qualified symbols like Package::sub
                    if let Some(fqn_regex) = get_fqn_regex() {
                        for cap in fqn_regex.captures_iter(&text_around) {
                            if let Some(m) = cap.get(1) {
                                if cursor_in_text >= m.start() && cursor_in_text <= m.end() {
                                    let parts: Vec<&str> = m.as_str().split("::").collect();
                                    if parts.len() >= 2 {
                                        let name = parts.last().copied().unwrap_or("");
                                        let pkg = parts[..parts.len() - 1].join("::");

                                        if let Some(result) = lookup_workspace_definition(
                                            self.coordinator(),
                                            &pkg,
                                            name,
                                        ) {
                                            return Ok(Some(result));
                                        }
                                        // Partial/None: fall through to same-file resolution
                                    }
                                    break;
                                }
                            }
                        }
                    }

                    // Attempt to resolve Package->method calls
                    if let Some(arrow_re) = get_arrow_method_regex() {
                        for cap in arrow_re.captures_iter(&text_around) {
                            if let (Some(package_match), Some(method_match)) =
                                (cap.get(1), cap.get(2))
                            {
                                if cursor_in_text >= method_match.start()
                                    && cursor_in_text <= method_match.end()
                                {
                                    let package_name = package_match.as_str();
                                    let method_name = method_match.as_str();

                                    if let Some(result) = lookup_workspace_definition(
                                        self.coordinator(),
                                        package_name,
                                        method_name,
                                    ) {
                                        return Ok(Some(result));
                                    }
                                    // Partial/None: fall through to same-file resolution
                                    break;
                                }
                            }
                        }
                    }
                }

                if let Some(ref ast) = doc.ast {
                    let offset = self.pos16_to_offset(doc, line, character);

                    // Try DeclarationProvider first (it handles function calls properly)
                    // SAFETY: parent_map is owned by the same DocumentState as the AST, ensuring pointer validity
                    let provider = unsafe {
                        crate::declaration::DeclarationProvider::new(
                            Arc::clone(ast),
                            doc.text.clone(),
                            uri.to_string(),
                        )
                        .with_parent_map(&doc.parent_map)
                        .with_doc_version(doc.version)
                    };

                    if let Some(location_links) = provider.find_declaration(offset, doc.version) {
                        // Convert to Location format for definition
                        let result: Vec<Value> = location_links
                            .iter()
                            .map(|link| {
                                let (sel_start_line, sel_start_char) =
                                    self.offset_to_pos16(doc, link.target_selection_range.0);
                                let (sel_end_line, sel_end_char) =
                                    self.offset_to_pos16(doc, link.target_selection_range.1);

                                json!({
                                    "uri": link.target_uri,
                                    "range": {
                                        "start": {
                                            "line": sel_start_line,
                                            "character": sel_start_char,
                                        },
                                        "end": {
                                            "line": sel_end_line,
                                            "character": sel_end_char,
                                        },
                                    },
                                })
                            })
                            .collect();

                        if !result.is_empty() {
                            return Ok(Some(json!(result)));
                        }
                    }

                    // Try workspace index for cross-file definitions using routing policy
                    #[cfg(feature = "workspace")]
                    {
                        let access_mode = route_index_access(self.coordinator());
                        if let IndexAccessMode::Full(coordinator) = access_mode {
                            let workspace_index = coordinator.index();
                            // Use symbol_at_cursor to get the symbol key
                            let current_package =
                                crate::declaration::current_package_at(ast, offset);
                            if let Some(symbol_key) =
                                crate::declaration::symbol_at_cursor(ast, offset, current_package)
                            {
                                eprintln!("Looking for definition of {:?}", symbol_key);

                                // Try to find definition using the symbol key
                                if let Some(def_location) = workspace_index.find_def(&symbol_key) {
                                    eprintln!("Found definition at {:?}", def_location);
                                    // Convert internal Location to LSP Location
                                    if let Some(lsp_location) =
                                        crate::workspace_index::lsp_adapter::to_lsp_location(
                                            &def_location,
                                        )
                                    {
                                        return Ok(Some(json!([lsp_location])));
                                    }
                                }

                                // Also try with find_definition for backward compatibility
                                let symbol_name =
                                    if symbol_key.kind == crate::workspace_index::SymKind::Sub {
                                        format!("{}::{}", symbol_key.pkg, symbol_key.name)
                                    } else {
                                        symbol_key.name.to_string()
                                    };

                                if let Some(def_location) =
                                    workspace_index.find_definition(&symbol_name)
                                {
                                    eprintln!(
                                        "Found definition via find_definition for {}",
                                        symbol_name
                                    );
                                    // Convert internal Location to LSP Location
                                    if let Some(lsp_location) =
                                        crate::workspace_index::lsp_adapter::to_lsp_location(
                                            &def_location,
                                        )
                                    {
                                        return Ok(Some(json!([lsp_location])));
                                    }
                                }
                            }
                        }
                        // Partial/None: fall through to same-file semantic model
                    }

                    // Fall back to same-file definition
                    let model = crate::semantic::SemanticModel::build(ast, &doc.text);

                    // Find definition at the position
                    if let Some(definition) = model.definition_at(offset) {
                        let (def_line, def_char) =
                            self.offset_to_pos16(doc, definition.location.start);
                        let (def_end_line, def_end_char) =
                            self.offset_to_pos16(doc, definition.location.end);

                        return Ok(Some(json!([{
                            "uri": uri,
                            "range": {
                                "start": {
                                    "line": def_line,
                                    "character": def_char,
                                },
                                "end": {
                                    "line": def_end_line,
                                    "character": def_end_char,
                                },
                            },
                        }])));
                    }
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle definition request with cancellation support
    pub(crate) fn handle_definition_cancellable(
        &self,
        params: Option<Value>,
        request_id: Option<&Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        // RAII guard ensures cleanup on all exit paths (early returns, errors, panics)
        let _cleanup_guard = RequestCleanupGuard::from_ref(request_id);

        if let Some(params) = params {
            // Create or get cancellation token for this request
            let token = if let Some(req_id) = request_id {
                GLOBAL_CANCELLATION_REGISTRY.get_token(req_id).unwrap_or_else(|| {
                    let token = PerlLspCancellationToken::new(
                        req_id.clone(),
                        "textDocument/definition".to_string(),
                    );
                    let _ = GLOBAL_CANCELLATION_REGISTRY.register_token(token.clone());
                    token
                })
            } else {
                PerlLspCancellationToken::new(
                    serde_json::Value::Null,
                    "textDocument/definition".to_string(),
                )
            };

            // Early cancellation check with relaxed read
            if token.is_cancelled_relaxed() {
                return Err(JsonRpcError {
                    code: REQUEST_CANCELLED,
                    message: "Request cancelled - definition provider".to_string(),
                    data: None,
                });
            }

            // Delegate to original handler
            self.handle_definition(Some(params))
        } else {
            self.handle_definition(params)
        }
    }

    /// Handle textDocument/typeDefinition request
    pub(crate) fn handle_type_definition(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        use crate::features::type_definition::TypeDefinitionProvider;

        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;

            // Acquire minimal data under lock, then drop it
            let ast = {
                let documents = self.documents_guard();
                let Some(doc) = self.get_document(&documents, uri) else {
                    return Ok(Some(json!([])));
                };
                let Some(ast) = doc.ast.as_ref() else {
                    return Ok(Some(json!([])));
                };
                ast.clone()
            };

            // Build doc_map outside the lock using snapshot helper
            let doc_map: HashMap<String, String> =
                self.documents_text_snapshot().into_iter().collect();

            let provider = TypeDefinitionProvider::new();
            if let Some(locations) =
                provider.find_type_definition(ast.as_ref(), line, character, uri, &doc_map)
            {
                return Ok(Some(json!(locations)));
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle textDocument/implementation request
    pub(crate) fn handle_implementation(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;

            // Acquire minimal data under lock, then drop it
            let ast = {
                let documents = self.documents_guard();
                let Some(doc) = self.get_document(&documents, uri) else {
                    return Ok(Some(json!([])));
                };
                let Some(ast) = doc.ast.as_ref() else {
                    return Ok(Some(json!([])));
                };
                ast.clone()
            };

            #[cfg(feature = "workspace")]
            {
                // Build doc_map outside the lock using snapshot helper
                let doc_map: HashMap<String, String> =
                    self.documents_text_snapshot().into_iter().collect();

                // Use routing policy - only provide workspace index in Full mode
                let access_mode = route_index_access(self.coordinator());
                let workspace_index = if let IndexAccessMode::Full(coordinator) = access_mode {
                    Some(coordinator.index().clone())
                } else {
                    // Partial/None: same-file analysis only
                    None
                };

                let provider = ImplementationProvider::new(workspace_index);
                let locations =
                    provider.find_implementations(ast.as_ref(), line, character, uri, &doc_map);
                return Ok(Some(json!(locations)));
            }

            #[cfg(not(feature = "workspace"))]
            {
                let _ = (ast, line, character, uri); // Suppress unused warnings
            }
        }

        Ok(Some(json!([])))
    }

    /// Find all implementations (simplified version)
    ///
    /// Note: This function and its helpers are currently unused but preserved
    /// for future class hierarchy navigation features.
    #[allow(dead_code)]
    pub(crate) fn find_all_implementations(
        &self,
        ast: &Node,
        documents: &HashMap<String, DocumentState>,
    ) -> Vec<Location> {
        let mut results = Vec::new();

        // Find packages in current file and look for their implementations
        let mut packages = Vec::new();
        self.find_packages_in_ast(ast, &mut packages);

        for package_name in packages {
            let impls = self.find_subclasses(&package_name, documents);
            results.extend(impls);
        }

        results
    }

    /// Find all packages in an AST
    #[allow(dead_code)]
    fn find_packages_in_ast(&self, node: &Node, packages: &mut Vec<String>) {
        if let NodeKind::Package { name, .. } = &node.kind {
            packages.push(name.clone());
        }

        // Traverse based on node type
        match &node.kind {
            NodeKind::Program { statements } => {
                for stmt in statements {
                    self.find_packages_in_ast(stmt, packages);
                }
            }
            NodeKind::Block { statements } => {
                for stmt in statements {
                    self.find_packages_in_ast(stmt, packages);
                }
            }
            NodeKind::Package { block, .. } => {
                if let Some(b) = block {
                    self.find_packages_in_ast(b, packages);
                }
            }
            _ => {}
        }
    }

    /// Find classes that extend a given base class
    #[allow(dead_code)]
    fn find_subclasses(
        &self,
        base_class: &str,
        documents: &HashMap<String, DocumentState>,
    ) -> Vec<Location> {
        let mut results = Vec::new();

        for (uri, doc) in documents.iter() {
            if let Some(ref ast) = doc.ast {
                self.find_subclasses_in_ast(ast, base_class, uri, &mut results);
            }
        }

        results
    }

    /// Find subclasses in an AST
    #[allow(dead_code)]
    fn find_subclasses_in_ast(
        &self,
        node: &Node,
        base_class: &str,
        uri: &str,
        results: &mut Vec<Location>,
    ) {
        if let NodeKind::Package { name: _name, .. } = &node.kind {
            // Check if this package extends the base class
            // Look for @ISA assignment or 'use base' or 'use parent'
            // This would need proper traversal - simplified for now
            if self.check_inheritance_in_package(node, base_class) {
                // Get source text for position conversion
                let documents = self.documents_guard();
                if let Some(doc) = documents.get(uri) {
                    let source_text = &doc.text;
                    // Convert byte offsets to wire range using conversion waist
                    let wire_range = crate::convert::WireRange::from_byte_offsets(
                        source_text,
                        node.location.start,
                        node.location.end,
                    );

                    // Create typed Location
                    results.push(Location { uri: parse_uri(uri), range: wire_range.into() });
                }
            }
        }

        // Recurse into children based on node type
        match &node.kind {
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                for stmt in statements {
                    self.find_subclasses_in_ast(stmt, base_class, uri, results);
                }
            }
            NodeKind::Package { block, .. } => {
                if let Some(b) = block {
                    self.find_subclasses_in_ast(b, base_class, uri, results);
                }
            }
            _ => {}
        }
    }

    /// Check if a package inherits from base class (simplified)
    #[allow(dead_code)]
    fn check_inheritance_in_package(&self, _node: &Node, _base_class: &str) -> bool {
        // This is a simplified check - would need proper AST traversal
        // to find @ISA assignments and use base/parent statements
        false
    }

    /// Find method implementations in subclasses
    #[allow(dead_code)]
    pub(crate) fn find_method_implementations(
        &self,
        base_package: &str,
        method_name: &str,
        documents: &HashMap<String, DocumentState>,
    ) -> Vec<Value> {
        let mut results = Vec::new();

        // First find all subclasses
        let subclasses = self.find_subclasses(base_package, documents);

        // Then find the method in each subclass
        for subclass_loc in subclasses {
            let uri_str = subclass_loc.uri.as_str();
            if let Some(doc) = documents.get(uri_str) {
                if let Some(ref ast) = doc.ast {
                    self.find_method_in_ast(ast, method_name, uri_str, &mut results);
                }
            }
        }

        results
    }

    /// Find a specific method in an AST
    #[allow(dead_code)]
    fn find_method_in_ast(
        &self,
        node: &Node,
        method_name: &str,
        uri: &str,
        results: &mut Vec<Value>,
    ) {
        // Check for function declarations (simplified - actual AST uses Subroutine)
        if let NodeKind::Subroutine { name: Some(name), .. } = &node.kind {
            if name == method_name {
                results.push(json!({
                    "uri": uri,
                    "range": {
                        "start": {
                            "line": 0,
                            "character": 0,
                        },
                        "end": {
                            "line": 0,
                            "character": 0,
                        }
                    }
                }));
            }
        }

        // Recurse into children based on node type
        match &node.kind {
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                for stmt in statements {
                    self.find_method_in_ast(stmt, method_name, uri, results);
                }
            }
            NodeKind::Package { block, .. } => {
                if let Some(b) = block {
                    self.find_method_in_ast(b, method_name, uri, results);
                }
            }
            _ => {}
        }
    }

    /// Non-blocking definition handler with fallback
    pub(crate) fn on_definition(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let uri = params.pointer("/textDocument/uri").and_then(|v| v.as_str()).unwrap_or("");
        let line = params.pointer("/position/line").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let ch =
            params.pointer("/position/character").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        let text = self.buffer_text(uri).unwrap_or_default();
        let module = token_under_cursor(&text, line, ch).filter(|s| s.contains("::"));

        if let Some(m) = module {
            if let Some(path) = self.resolve_module_path(&m) {
                let loc = location_from_path(&path);
                return Ok(serde_json::json!([loc]));
            }
        }

        // Fallback: try existing analysis
        // For now, just return empty array
        Ok(serde_json::json!([]))
    }
}
