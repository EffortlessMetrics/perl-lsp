//! Miscellaneous language feature handlers
//!
//! Handles various LSP features including:
//! - Inlay hints
//! - Document links
//! - Selection ranges
//! - Code lens
//! - Inline completion and values
//! - Monikers
//! - Linked editing ranges
//! - Test discovery
//! - Execute command

use super::super::*;
use crate::lsp::protocol::{invalid_params, req_position, req_uri};
use crate::lsp::state::{code_lens_cap, code_lens_resolve_deadline, inlay_hints_cap};
use std::time::Instant;

impl LspServer {
    /// Handle textDocument/inlayHint request
    pub(crate) fn handle_inlay_hints(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        use crate::lsp::protocol::req_range;
        let cap = inlay_hints_cap();

        if let Some(p) = params {
            let uri = req_uri(&p)?;

            // Extract the range parameter (required by LSP spec)
            // InlayHint range is required per spec, but we allow graceful degradation to full doc
            let range = if let Ok(((sl, sc), (el, ec))) = req_range(&p) {
                Some(crate::positions::Range::new(sl, sc, el, ec))
            } else {
                None
            };

            let documents = self.documents_guard();
            let doc = self.get_document(&documents, uri).ok_or_else(|| JsonRpcError {
                code: INVALID_REQUEST,
                message: format!("Document not open: {}", uri),
                data: None,
            })?;
            if let Some(ref ast) = doc.ast {
                let mut hints = Vec::new();
                hints.extend(crate::inlay_hints::parameter_hints(
                    ast,
                    &|off| self.offset_to_pos16(doc, off),
                    range,
                ));
                hints.extend(crate::inlay_hints::trivial_type_hints(
                    ast,
                    &|off| self.offset_to_pos16(doc, off),
                    range,
                ));
                // Apply cap to inlay hints
                if hints.len() > cap {
                    eprintln!("InlayHints: capping from {} to {}", hints.len(), cap);
                    hints.truncate(cap);
                }
                return Ok(Some(json!(hints)));
            }
        }
        Ok(Some(json!([])))
    }

    /// Handle textDocument/documentLink request
    pub(crate) fn handle_document_links(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(p) = params {
            let uri = p["textDocument"]["uri"].as_str().ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing textDocument.uri".into(),
                data: None,
            })?;
            let documents = self.documents_guard();
            let doc = self.get_document(&documents, uri).ok_or_else(|| JsonRpcError {
                code: INVALID_REQUEST,
                message: format!("Document not open: {}", uri),
                data: None,
            })?;

            // Get workspace roots from initialization params
            let roots = self.workspace_roots();
            let links = crate::document_links::compute_links(uri, &doc.text, &roots);
            Ok(Some(json!(links)))
        } else {
            Ok(Some(json!([])))
        }
    }

    /// Handle documentLink request (alternative)
    #[allow(dead_code)] // Alternative implementation
    pub(crate) fn handle_document_link(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = req_uri(&params)?;

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                let uri_parsed = url::Url::parse(uri).map_err(|_| JsonRpcError {
                    code: -32602,
                    message: "Invalid URI".to_string(),
                    data: None,
                })?;
                match crate::lsp_document_link::collect_document_links(&doc.text, &uri_parsed) {
                    Ok(links) => Ok(Some(serde_json::to_value(links).unwrap_or(Value::Null))),
                    Err(_) => Ok(Some(Value::Null)),
                }
            } else {
                Ok(Some(Value::Null))
            }
        } else {
            Ok(Some(Value::Null))
        }
    }

    /// Handle textDocument/selectionRange request
    pub(crate) fn handle_selection_range(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(p) = params {
            let uri = req_uri(&p)?;
            let positions = p["positions"]
                .as_array()
                .ok_or_else(|| invalid_params("Missing required parameter: positions"))?;

            let documents = self.documents_guard();
            let doc = self.get_document(&documents, uri).ok_or_else(|| JsonRpcError {
                code: INVALID_REQUEST,
                message: format!("Document not open: {}", uri),
                data: None,
            })?;

            let mut out = Vec::new();
            if let Some(ref ast) = doc.ast {
                // Build parent map if not cached
                let parent_map = crate::selection_range::build_parent_map(ast);

                for pos in positions {
                    // Positions in array still need per-item extraction with graceful handling
                    // Use try_from for safe u64→u32 conversion (strict-by-default)
                    let line =
                        pos["line"].as_u64().and_then(|v| u32::try_from(v).ok()).unwrap_or(0);
                    let col =
                        pos["character"].as_u64().and_then(|v| u32::try_from(v).ok()).unwrap_or(0);
                    let off = self.pos16_to_offset(doc, line, col);
                    let chain =
                        crate::selection_range::selection_chain(ast, &parent_map, off, &|o| {
                            self.offset_to_pos16(doc, o)
                        });
                    out.push(chain);
                }
            }
            Ok(Some(json!(out)))
        } else {
            Ok(Some(json!([])))
        }
    }

    /// Handle textDocument/codeLens request
    pub(crate) fn handle_code_lens(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        // Gate unadvertised feature
        if !self.advertised_features.lock().unwrap().code_lens {
            return Err(crate::lsp_errors::method_not_advertised());
        }

        let cap = code_lens_cap();

        if let Some(params) = params {
            let uri = req_uri(&params)?;

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let provider = CodeLensProvider::new(doc.text.clone());
                    let mut lenses = provider.extract(ast);

                    // Add shebang lens if applicable
                    if let Some(shebang_lens) = get_shebang_lens(&doc.text) {
                        lenses.insert(0, shebang_lens);
                    }

                    // Apply cap to code lenses
                    if lenses.len() > cap {
                        eprintln!("CodeLens: capping from {} to {}", lenses.len(), cap);
                        lenses.truncate(cap);
                    }

                    return Ok(Some(json!(lenses)));
                } else {
                    // Text-based fallback when AST is not available
                    let mut text_lenses = self.extract_text_based_code_lenses(&doc.text, uri);
                    // Apply cap to text-based lenses
                    if text_lenses.len() > cap {
                        eprintln!("CodeLens (text): capping from {} to {}", text_lenses.len(), cap);
                        text_lenses.truncate(cap);
                    }
                    return Ok(Some(json!(text_lenses)));
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle codeLens/resolve request
    ///
    /// This implementation uses the snapshot pattern to minimize lock hold time.
    /// The documents lock is held only during the snapshot creation, then released
    /// before the CPU-intensive reference counting work begins.
    ///
    /// Includes deadline enforcement to prevent blocking on large workspaces.
    pub(crate) fn handle_code_lens_resolve(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let start = Instant::now();
        let deadline = code_lens_resolve_deadline();

        if let Some(params) = params {
            // Parse the code lens
            if let Ok(lens) =
                serde_json::from_value::<crate::code_lens_provider::CodeLens>(params.clone())
            {
                // Extract the symbol name and kind from the lens data
                let symbol_name = lens
                    .data
                    .as_ref()
                    .and_then(|d| d.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("");

                let symbol_kind = lens
                    .data
                    .as_ref()
                    .and_then(|d| d.get("kind"))
                    .and_then(|k| k.as_str())
                    .unwrap_or("unknown");

                // Take a snapshot of all documents - lock is released after this line
                // This allows other LSP operations to proceed while we do CPU-intensive
                // reference counting across the workspace
                let snapshot = self.documents_scan_snapshot();

                // Now iterate without holding the lock
                let mut total_references = 0;
                for (scanned_docs, view) in snapshot.iter().enumerate() {
                    // Check deadline periodically (every 10 documents)
                    if scanned_docs % 10 == 0 && start.elapsed() >= deadline {
                        eprintln!(
                            "CodeLensResolve: deadline exceeded after {} docs, returning partial count {}",
                            scanned_docs, total_references
                        );
                        break;
                    }

                    if let Some(ref ast) = view.ast {
                        total_references += self.count_references(ast, symbol_name, symbol_kind);
                    } else {
                        // Text-based fallback when AST is not available
                        total_references +=
                            self.count_references_text_based(&view.text, symbol_name, symbol_kind);
                    }
                }

                let resolved = resolve_code_lens(lens, total_references);
                return Ok(Some(json!(resolved)));
            }
        }

        Err(JsonRpcError { code: -32602, message: "Invalid parameters".to_string(), data: None })
    }

    /// Handle textDocument/inlineCompletion request
    pub(crate) fn handle_inline_completion(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        use crate::inline_completions::InlineCompletionProvider;

        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                let provider = InlineCompletionProvider::new();
                let completions = provider.get_inline_completions(&doc.text, line, character);
                return Ok(Some(serde_json::to_value(completions).unwrap_or(Value::Null)));
            }
        }

        Ok(Some(json!({
            "items": []
        })))
    }

    /// Handle textDocument/inlineValue request
    pub(crate) fn handle_inline_value(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        use crate::lsp::protocol::req_range;
        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let ((start_line, _start_char), (end_line, _end_char)) = req_range(&params)?;
            let _context = &params["context"]; // Debug context (stopped at breakpoint, etc)

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                // Extract visible scalar variables in the range
                use super::super::byte_to_utf16_col;

                let mut inline_values = Vec::new();

                // Simple implementation: find scalar variables in the visible range
                let lines: Vec<&str> = doc.text.lines().collect();
                // Move regex construction outside loop
                let re = regex::Regex::new(r"\$([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();

                for line_num in start_line..=end_line.min((lines.len() - 1) as u32) {
                    let line_text = lines[line_num as usize];

                    // Find scalar variables using regex
                    for cap in re.captures_iter(line_text) {
                        if let Some(m) = cap.get(0) {
                            let var_text = m.as_str();
                            // Convert byte positions to UTF-16 code units for LSP compliance
                            let start_utf16 = byte_to_utf16_col(line_text, m.start());
                            let end_utf16 = byte_to_utf16_col(line_text, m.end());

                            // Create inline value text hint (showing the variable name as placeholder)
                            inline_values.push(json!({
                                "range": {
                                    "start": { "line": line_num, "character": start_utf16 as u32 },
                                    "end": { "line": line_num, "character": end_utf16 as u32 }
                                },
                                "text": format!("{} = ?", var_text)
                            }));
                        }
                    }
                }

                return Ok(Some(json!(inline_values)));
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle textDocument/moniker request
    ///
    /// Generates stable symbol identifiers for cross-project symbol linking.
    /// Supports:
    /// - Exported symbols (kind="export") for symbols in @EXPORT or @EXPORT_OK
    /// - Imported symbols (kind="import") for symbols from use statements
    /// - Local symbols with appropriate uniqueness classification
    /// - Multiple monikers for aliased symbols
    pub(crate) fn handle_moniker(
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

                    // Find the symbol at the cursor position
                    let current_pkg = crate::declaration::current_package_at(ast, offset);
                    if let Some(key) =
                        crate::declaration::symbol_at_cursor(ast, offset, current_pkg)
                    {
                        let mut monikers = Vec::new();

                        // Determine moniker properties based on symbol context
                        let (kind, unique) = self.classify_moniker(ast, &doc.text, &key);

                        // Generate fully qualified identifier
                        let qualified_id = format!("{}::{}", key.pkg, key.name).replace("::", ".");

                        // Primary moniker with full qualification
                        monikers.push(json!({
                            "scheme": "perl",
                            "identifier": qualified_id,
                            "unique": unique,
                            "kind": kind
                        }));

                        // For imported symbols, also add a moniker pointing to the source
                        if kind == "import" {
                            if let Some(source_pkg) = self.find_import_source(ast, &key.name) {
                                let source_id =
                                    format!("{}.{}", source_pkg.replace("::", "."), key.name);
                                monikers.push(json!({
                                    "scheme": "perl",
                                    "identifier": source_id,
                                    "unique": "global",
                                    "kind": "export"
                                }));
                            }
                        }

                        // For package-scoped variables (our), add a bare name alias
                        if key.sigil.is_some() && unique != "document" {
                            let sigil = key.sigil.unwrap_or('$');
                            let bare_id = format!("{}{}", sigil, key.name);
                            monikers.push(json!({
                                "scheme": "perl",
                                "identifier": bare_id,
                                "unique": "document",
                                "kind": "local"
                            }));
                        }

                        return Ok(Some(json!(monikers)));
                    }
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Classify a symbol's moniker kind and uniqueness
    fn classify_moniker(
        &self,
        ast: &crate::ast::Node,
        text: &str,
        key: &crate::workspace_index::SymbolKey,
    ) -> (&'static str, &'static str) {
        // Check if symbol is exported via @EXPORT or @EXPORT_OK
        let is_exported = self.is_symbol_exported(text, &key.name);

        // Check if symbol is imported from another module
        let is_imported = self.is_symbol_imported(ast, &key.name);

        // Determine kind
        let kind = if is_exported {
            "export"
        } else if is_imported {
            "import"
        } else {
            "local"
        };

        // Determine uniqueness
        let unique = match key.kind {
            crate::workspace_index::SymKind::Pack => "global",
            crate::workspace_index::SymKind::Sub => {
                if is_exported {
                    "global"
                } else if key.pkg.as_ref() != "main" {
                    "project"
                } else {
                    "document"
                }
            }
            crate::workspace_index::SymKind::Var => {
                if self.is_our_variable(ast, &key.name, key.sigil) { "project" } else { "document" }
            }
        };

        (kind, unique)
    }

    /// Check if a symbol name appears in @EXPORT or @EXPORT_OK
    fn is_symbol_exported(&self, text: &str, symbol_name: &str) -> bool {
        let export_re =
            regex::Regex::new(r"@EXPORT(?:_OK)?\s*=\s*qw[(\[{/|!]([^)\]}/|!]+)[)\]}/|!]").ok();

        if let Some(re) = export_re {
            for cap in re.captures_iter(text) {
                if let Some(content) = cap.get(1) {
                    if content.as_str().split_whitespace().any(|w| w == symbol_name) {
                        return true;
                    }
                }
            }
        }

        let array_re = regex::Regex::new(r"@EXPORT(?:_OK)?\s*=\s*\(([^)]+)\)").ok();
        if let Some(re) = array_re {
            for cap in re.captures_iter(text) {
                if let Some(content) = cap.get(1) {
                    let c = content.as_str();
                    if c.contains(&format!("'{}'", symbol_name))
                        || c.contains(&format!("\"{}\"", symbol_name))
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Check if a symbol is imported from another module
    fn is_symbol_imported(&self, ast: &crate::ast::Node, symbol_name: &str) -> bool {
        use crate::ast::NodeKind;

        fn check(node: &crate::ast::Node, name: &str) -> bool {
            match &node.kind {
                NodeKind::Use { args, .. } => {
                    for arg in args {
                        if arg == name {
                            return true;
                        }
                        if arg.starts_with("qw") {
                            let content = arg
                                .trim_start_matches("qw")
                                .trim_start_matches(|c: char| "([{/<|!".contains(c))
                                .trim_end_matches(|c: char| ")]}/|!>".contains(c));
                            if content.split_whitespace().any(|w| w == name) {
                                return true;
                            }
                        }
                    }
                }
                NodeKind::Program { statements } | NodeKind::Block { statements } => {
                    for stmt in statements {
                        if check(stmt, name) {
                            return true;
                        }
                    }
                }
                _ => {}
            }
            false
        }

        check(ast, symbol_name)
    }

    /// Find the source module for an imported symbol
    fn find_import_source(&self, ast: &crate::ast::Node, symbol_name: &str) -> Option<String> {
        use crate::ast::NodeKind;

        fn find(node: &crate::ast::Node, name: &str) -> Option<String> {
            match &node.kind {
                NodeKind::Use { module, args } => {
                    for arg in args {
                        if arg == name {
                            return Some(module.clone());
                        }
                        if arg.starts_with("qw") {
                            let content = arg
                                .trim_start_matches("qw")
                                .trim_start_matches(|c: char| "([{/<|!".contains(c))
                                .trim_end_matches(|c: char| ")]}/|!>".contains(c));
                            if content.split_whitespace().any(|w| w == name) {
                                return Some(module.clone());
                            }
                        }
                    }
                }
                NodeKind::Program { statements } | NodeKind::Block { statements } => {
                    for stmt in statements {
                        if let Some(src) = find(stmt, name) {
                            return Some(src);
                        }
                    }
                }
                _ => {}
            }
            None
        }

        find(ast, symbol_name)
    }

    /// Check if a variable is declared with 'our' (package-scoped)
    fn is_our_variable(&self, ast: &crate::ast::Node, var_name: &str, sigil: Option<char>) -> bool {
        use crate::ast::NodeKind;

        fn check(node: &crate::ast::Node, name: &str, sigil: Option<char>) -> bool {
            match &node.kind {
                NodeKind::VariableDeclaration { declarator, variable, .. }
                    if declarator == "our" =>
                {
                    if let NodeKind::Variable { name: n, sigil: s } = &variable.kind {
                        if n == name {
                            return sigil.map_or(true, |sig| s.starts_with(sig));
                        }
                    }
                }
                NodeKind::VariableListDeclaration { declarator, variables, .. }
                    if declarator == "our" =>
                {
                    for var in variables {
                        if let NodeKind::Variable { name: n, sigil: s } = &var.kind {
                            if n == name {
                                return sigil.map_or(true, |sig| s.starts_with(sig));
                            }
                        }
                    }
                }
                NodeKind::Program { statements } | NodeKind::Block { statements } => {
                    for stmt in statements {
                        if check(stmt, name, sigil) {
                            return true;
                        }
                    }
                }
                NodeKind::Subroutine { body, .. } => {
                    if check(body, name, sigil) {
                        return true;
                    }
                }
                _ => {}
            }
            false
        }

        check(ast, var_name, sigil)
    }

    /// Handle textDocument/documentColor request
    pub(crate) fn handle_document_color(
        &self,
        _params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        Err(JsonRpcError { code: -32601, message: "Method not found".into(), data: None })
    }

    /// Handle textDocument/colorPresentation request
    pub(crate) fn handle_color_presentation(
        &self,
        _params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        Err(JsonRpcError { code: -32601, message: "Method not found".into(), data: None })
    }

    /// Handle textDocument/linkedEditingRange request
    pub(crate) fn handle_linked_editing_range(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        // Gate unadvertised feature
        if !self.advertised_features.lock().unwrap().linked_editing {
            return Err(crate::lsp_errors::method_not_advertised());
        }

        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                let result =
                    crate::linked_editing::handle_linked_editing(&doc.text, line, character);
                return Ok(Some(serde_json::to_value(result).unwrap_or(Value::Null)));
            }
        }

        Ok(Some(Value::Null))
    }

    /// Handle test discovery request
    pub(crate) fn handle_test_discovery(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = req_uri(&params)?;

            eprintln!("Discovering tests for: {}", uri);

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let runner = TestRunner::new(doc.text.clone(), uri.to_string());
                    let tests = runner.discover_tests(ast);

                    // Convert test items to JSON
                    let test_items: Vec<Value> = tests
                        .into_iter()
                        .map(|test| {
                            json!({
                                "id": test.id,
                                "label": test.label,
                                "uri": test.uri,
                                "range": {
                                    "start": {
                                        "line": test.range.start_line,
                                        "character": test.range.start_character
                                    },
                                    "end": {
                                        "line": test.range.end_line,
                                        "character": test.range.end_character
                                    }
                                },
                                "kind": match test.kind {
                                    TestKind::File => "file",
                                    TestKind::Suite => "suite",
                                    TestKind::Test => "test"
                                },
                                "children": test.children.into_iter()
                                    .map(|child| json!({
                                        "id": child.id,
                                        "label": child.label,
                                        "uri": child.uri,
                                        "range": {
                                            "start": {
                                                "line": child.range.start_line,
                                                "character": child.range.start_character
                                            },
                                            "end": {
                                                "line": child.range.end_line,
                                                "character": child.range.end_character
                                            }
                                        },
                                        "kind": match child.kind {
                                            TestKind::File => "file",
                                            TestKind::Suite => "suite",
                                            TestKind::Test => "test"
                                        },
                                        "children": []
                                    }))
                                    .collect::<Vec<_>>()
                            })
                        })
                        .collect();

                    eprintln!("Found {} test items", test_items.len());

                    return Ok(Some(json!(test_items)));
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle execute command request
    pub(crate) fn handle_execute_command(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        use crate::execute_command::ExecuteCommandProvider;

        if let Some(params) = params {
            let command = params["command"]
                .as_str()
                .ok_or_else(|| invalid_params("Missing required parameter: command"))?;

            // LSP 3.17 compliance: arguments field is required even if empty
            if !params.as_object().unwrap_or(&serde_json::Map::new()).contains_key("arguments") {
                return Err(JsonRpcError {
                    code: -32602, // InvalidParams
                    message: "Missing required 'arguments' field in executeCommand request"
                        .to_string(),
                    data: Some(json!({
                        "command": command,
                        "errorType": "executeCommand",
                        "originalError": "Missing 'arguments' field"
                    })),
                });
            }

            let arguments = params["arguments"].as_array().cloned().unwrap_or_default();

            eprintln!("Executing command: {}", command);

            // Use the new execute command provider for new commands
            let provider = ExecuteCommandProvider::new();

            match command {
                // Keep existing test commands for backward compatibility
                "perl.runTest" => {
                    if let Some(test_id) = arguments.first().and_then(|v| v.as_str()) {
                        return self.run_test(test_id);
                    }
                }
                "perl.runTestFile" => {
                    if let Some(file_uri) = arguments.first().and_then(|v| v.as_str()) {
                        return self.run_test_file(file_uri);
                    }
                }
                // New commands handled by ExecuteCommandProvider
                "perl.runTests" | "perl.runFile" | "perl.runTestSub" | "perl.debugTests"
                | "perl.runCritic" => {
                    match provider.execute_command(command, arguments) {
                        Ok(result) => return Ok(Some(result)),
                        Err(e) => {
                            // Return proper JSON-RPC error according to LSP 3.17 specification
                            let error_code = if e.contains("Missing") || e.contains("argument") {
                                -32602 // InvalidParams
                            } else if e.contains("Unknown command") {
                                -32601 // MethodNotFound
                            } else if e.contains("Path traversal") || e.contains("security") {
                                -32603 // InternalError (security)
                            } else {
                                -32603 // InternalError (general)
                            };

                            return Err(JsonRpcError {
                                code: error_code,
                                message: format!("Execute command failed: {}", e),
                                data: Some(json!({
                                    "command": command,
                                    "errorType": "executeCommand",
                                    "originalError": e
                                })),
                            });
                        }
                    }
                }
                // Debug commands (stub implementation for now)
                "perl.debugFile" => {
                    eprintln!("Debug command requested: {}", command);
                    // Return a success status - actual DAP integration can be added later
                    return Ok(Some(
                        json!({"status": "started", "message": format!("Debug session {} initiated", command)}),
                    ));
                }
                _ => {
                    return Err(JsonRpcError {
                        code: METHOD_NOT_FOUND,
                        message: format!("Unknown command: {}", command),
                        data: None,
                    });
                }
            }
        }

        // Missing params entirely
        Err(JsonRpcError {
            code: -32602, // InvalidParams
            message: "Missing parameters for executeCommand request".to_string(),
            data: Some(json!({
                "errorType": "executeCommand",
                "originalError": "Missing params"
            })),
        })
    }

    /// Count references to a symbol using text-based search
    pub(crate) fn count_references_text_based(
        &self,
        text: &str,
        symbol_name: &str,
        symbol_kind: &str,
    ) -> usize {
        let mut count = 0;

        match symbol_kind {
            "package" => {
                // Count package usage (use statements, new() calls, etc.)
                use regex::Regex;

                // Count "use PackageName" statements
                if let Ok(use_regex) =
                    Regex::new(&format!(r"\buse\s+{}\b", regex::escape(symbol_name)))
                {
                    count += use_regex.find_iter(text).count();
                }

                // Count "PackageName->new()" or "PackageName->method()" calls
                if let Ok(call_regex) = Regex::new(&format!(r"\b{}->", regex::escape(symbol_name)))
                {
                    count += call_regex.find_iter(text).count();
                }

                // Count "bless ... PackageName" statements
                if let Ok(bless_regex) =
                    Regex::new(&format!(r"bless\s+.*?,\s*{}", regex::escape(symbol_name)))
                {
                    count += bless_regex.find_iter(text).count();
                }
            }
            "subroutine" => {
                // Count function calls
                use regex::Regex;

                // Count "function_name(" calls
                if let Ok(call_regex) =
                    Regex::new(&format!(r"\b{}\s*\(", regex::escape(symbol_name)))
                {
                    count += call_regex.find_iter(text).count();
                }

                // Count "&function_name" references
                if let Ok(ref_regex) = Regex::new(&format!(r"&{}\b", regex::escape(symbol_name))) {
                    count += ref_regex.find_iter(text).count();
                }
            }
            _ => {
                // Generic search
                use regex::Regex;
                if let Ok(re) = Regex::new(&format!(r"\b{}\b", regex::escape(symbol_name))) {
                    count += re.find_iter(text).count();
                }
            }
        }

        count
    }

    /// Get workspace roots from initialization
    pub(crate) fn workspace_roots(&self) -> Vec<url::Url> {
        // In a real implementation, store these from initialize params
        // For now, return empty vec
        vec![]
    }
}
