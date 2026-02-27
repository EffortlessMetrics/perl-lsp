//! Hover and signature help handlers
//!
//! Provides hover information and function signature help for Perl code.

use super::super::*;
use crate::cancellation::RequestCleanupGuard;
use crate::protocol::{req_position, req_uri};

impl LspServer {
    /// Handle textDocument/hover request for symbol information display
    ///
    /// Provides rich hover information for Perl symbols including type information,
    /// documentation, and declaration context. Integrates with semantic analysis
    /// to show inferred types and cross-references.
    ///
    /// # LSP Protocol
    ///
    /// Request: `textDocument/hover`
    /// Response: `Hover | null`
    ///
    /// # Arguments
    ///
    /// * `params` - JSON-RPC parameters containing document URI and position
    ///
    /// # Returns
    ///
    /// Hover information with markdown content or null if no information available
    pub(crate) fn handle_hover(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;

            // Reject stale requests
            let req_version =
                params["textDocument"]["version"].as_i64().and_then(|n| i32::try_from(n).ok());
            self.ensure_latest(uri, req_version)?;

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ast) = &doc.ast {
                    let offset = self.pos16_to_offset(doc, line, character);

                    // Use SemanticAnalyzer for type information
                    // Pass the source text to enable proper symbol resolution
                    let analyzer =
                        crate::semantic::SemanticAnalyzer::analyze_with_source(ast, &doc.text);

                    // Try to find the symbol at this position, checking references first
                    // This allows hover on variable usages to show the variable's definition info
                    if let Some(symbol_info) = analyzer.find_definition(offset) {
                        // Get symbol kind as string
                        use crate::symbol::VarKind;
                        let kind_str = match symbol_info.kind {
                            crate::symbol::SymbolKind::Variable(VarKind::Scalar) => {
                                "Scalar Variable"
                            }
                            crate::symbol::SymbolKind::Variable(VarKind::Array) => "Array Variable",
                            crate::symbol::SymbolKind::Variable(VarKind::Hash) => "Hash Variable",
                            crate::symbol::SymbolKind::Subroutine => "Subroutine",
                            crate::symbol::SymbolKind::Package => "Package",
                            crate::symbol::SymbolKind::Constant => "Constant",
                            crate::symbol::SymbolKind::Label => "Label",
                            crate::symbol::SymbolKind::Format => "Format",
                            _ => "Symbol",
                        };

                        // Add sigil if applicable
                        let sigil = symbol_info.kind.sigil().unwrap_or("");
                        let full_name = format!("{}{}", sigil, symbol_info.name);

                        // Add declaration type if available
                        let decl_info = symbol_info
                            .declaration
                            .as_ref()
                            .map(|d| format!("\n**Declaration**: `{}`", d))
                            .unwrap_or_default();

                        // Include synthesized framework metadata (for example:
                        // `is=ro`, `isa=Str`) when available.
                        let attrs_info = if symbol_info.attributes.is_empty() {
                            String::new()
                        } else {
                            format!("\n**Attributes**: {}", symbol_info.attributes.join(", "))
                        };

                        // Add documentation if available
                        let doc_info = symbol_info
                            .documentation
                            .as_ref()
                            .map(|d| format!("\n\n{}", d))
                            .unwrap_or_default();

                        return Ok(Some(json!({
                            "contents": {
                                "kind": "markdown",
                                "value": format!("**{}**\n\n`{}`{}{}{}",
                                    kind_str,
                                    full_name,
                                    decl_info,
                                    attrs_info,
                                    doc_info
                                ),
                            },
                        })));
                    }

                    // Fall back to simple token display
                    let hover_text = self.get_token_at_position(&doc.text, offset);

                    if !hover_text.is_empty() {
                        return Ok(Some(json!({
                            "contents": {
                                "kind": "markdown",
                                "value": format!("**Perl**: `{}`", hover_text),
                            },
                        })));
                    }
                }
            }
        }

        Ok(Some(json!(null)))
    }

    /// Handle textDocument/hover request with cancellation support
    ///
    /// Provides hover information with request cancellation capability for
    /// responsive editing in large Perl codebases. Uses RAII cleanup guard
    /// to ensure proper resource cleanup on all exit paths.
    ///
    /// # Arguments
    ///
    /// * `params` - JSON-RPC parameters containing document URI and position
    /// * `request_id` - Optional request ID for cancellation tracking
    ///
    /// # Returns
    ///
    /// Hover information or cancellation error if request was cancelled
    pub(crate) fn handle_hover_cancellable(
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
                        "textDocument/hover".to_string(),
                    );
                    let _ = GLOBAL_CANCELLATION_REGISTRY.register_token(token.clone());
                    token
                })
            } else {
                PerlLspCancellationToken::new(
                    serde_json::Value::Null,
                    "textDocument/hover".to_string(),
                )
            };

            // Early cancellation check with relaxed read
            if token.is_cancelled_relaxed() {
                return Err(JsonRpcError {
                    code: REQUEST_CANCELLED,
                    message: "Request cancelled - hover provider".to_string(),
                    data: None,
                });
            }

            // Delegate to original handler
            self.handle_hover(Some(params))
        } else {
            self.handle_hover(params)
        }
    }

    /// Handle textDocument/signatureHelp request for function parameter hints
    ///
    /// Provides signature information for function calls showing parameter names,
    /// types, and documentation. Supports both built-in Perl functions and
    /// user-defined subroutines with signature extraction.
    ///
    /// # LSP Protocol
    ///
    /// Request: `textDocument/signatureHelp`
    /// Response: `SignatureHelp | null`
    ///
    /// # Arguments
    ///
    /// * `params` - JSON-RPC parameters containing document URI and position
    ///
    /// # Returns
    ///
    /// Signature information including parameter list and active parameter index
    pub(crate) fn handle_signature_help(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                let offset = self.pos16_to_offset(doc, line, character);

                // Find the function call context at this position
                if let Some((function_name, active_param)) =
                    self.find_function_context(&doc.text, offset)
                {
                    // Try to get signature from user-defined functions first (if AST exists)
                    if let Some(ref ast) = doc.ast {
                        if let Some(signature) =
                            self.get_user_function_signature(ast, &function_name)
                        {
                            return Ok(Some(json!({
                                "signatures": [signature],
                                "activeSignature": 0,
                                "activeParameter": active_param
                            })));
                        }
                    }

                    // Fall back to built-in functions
                    if let Some(signature) = self.get_builtin_function_signature(&function_name) {
                        return Ok(Some(json!({
                            "signatures": [signature],
                            "activeSignature": 0,
                            "activeParameter": active_param
                        })));
                    }

                    // If no signature found, return a generic one
                    return Ok(Some(json!({
                        "signatures": [json!({
                            "label": format!("{}(...)", function_name),
                            "documentation": null,
                            "parameters": []
                        })],
                        "activeSignature": 0,
                        "activeParameter": active_param
                    })));
                }
            }
        }

        Ok(None)
    }

    /// Find function context at position for signature help
    ///
    /// Analyzes source code at the given offset to determine if the cursor
    /// is within a function call, and if so, identifies the function name
    /// and current parameter position.
    ///
    /// # Arguments
    ///
    /// * `content` - Source code text to analyze
    /// * `offset` - Byte offset position to check
    ///
    /// # Returns
    ///
    /// Tuple of (function_name, active_parameter_index) if in function call context
    pub(crate) fn find_function_context(
        &self,
        content: &str,
        offset: usize,
    ) -> Option<(String, usize)> {
        let chars: Vec<char> = content.chars().collect();
        if offset > chars.len() {
            return None;
        }

        // Find the opening parenthesis, tracking all bracket types
        let mut paren_pos = None;
        let mut depth = 0;
        let mut i = if offset > 0 { offset - 1 } else { return None };

        loop {
            match chars[i] {
                ')' => depth += 1,
                ']' => depth += 1,
                '}' => depth += 1,
                '(' => {
                    if depth == 0 {
                        paren_pos = Some(i);
                        break;
                    }
                    depth -= 1;
                }
                '[' | '{' => {
                    if depth > 0 {
                        depth -= 1;
                    }
                }
                _ => {}
            }

            if i == 0 {
                break;
            }
            i -= 1;
        }

        let paren_pos = paren_pos?;

        // Now extract the function name before the parenthesis
        // Handle: func(), $obj->func(), Package::func()
        let mut j = if paren_pos > 0 {
            paren_pos - 1
        } else {
            return None;
        };

        // Skip whitespace before '('
        while j > 0 && chars[j].is_whitespace() {
            j -= 1;
        }

        if j == 0 {
            if let Some(&first) = chars.first() {
                if !first.is_alphanumeric() && first != '_' {
                    return None;
                }
            } else {
                return None;
            }
        }

        let mut end = j + 1;
        let mut start = j;

        // Check for method call pattern (->)
        if j >= 1 && chars[j] == '>' && chars[j - 1] == '-' {
            // This is a method call, extract method name after ->
            // First find where -> starts
            let arrow_end = j - 1; // Position of '-'

            // Now find method name after ->
            j = paren_pos - 1;
            while j > arrow_end + 1 && chars[j].is_whitespace() {
                j -= 1;
            }
            end = j + 1;

            j = arrow_end + 2; // Start after ->
            while j < end && chars[j].is_whitespace() {
                j += 1;
            }
            start = j;
        } else {
            // Regular function or Package::function
            while start > 0 {
                let ch = chars[start];
                if ch.is_alphanumeric() || ch == '_' {
                    start -= 1;
                } else if start >= 2 && ch == ':' && chars[start - 1] == ':' {
                    // Package separator
                    start -= 2;
                } else {
                    // Adjust if we overshot
                    if !ch.is_alphanumeric() && ch != '_' && ch != ':' {
                        start += 1;
                    }
                    break;
                }
            }

            // Handle case where we're at the beginning
            if start == 0 {
                if let Some(&first) = chars.first() {
                    if first.is_alphanumeric() || first == '_' {
                        // Include first character
                    } else {
                        start = 1;
                    }
                } else {
                    start = 1;
                }
            }
        }

        if start >= end {
            return None;
        }

        let full_name: String = chars[start..end].iter().collect();

        // Extract just the function name (strip package prefix if present)
        let func_name =
            if let Some(pos) = full_name.rfind("::") { &full_name[pos + 2..] } else { &full_name };

        // Count commas at depth 0 to determine active parameter
        let mut comma_count = 0;
        let mut depth = 0;
        for k in (paren_pos + 1)..offset.min(chars.len()) {
            match chars[k] {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                ',' if depth == 0 => comma_count += 1,
                _ => {}
            }
        }

        Some((func_name.trim().to_string(), comma_count))
    }

    /// Get signature for user-defined functions from AST
    ///
    /// Extracts function signature information by analyzing the AST for
    /// subroutine definitions. Supports both explicit signatures and
    /// parameter extraction from `my (...) = @_` patterns.
    ///
    /// # Arguments
    ///
    /// * `ast` - Parsed AST to search for subroutine definitions
    /// * `function_name` - Name of the function to find signature for
    ///
    /// # Returns
    ///
    /// LSP SignatureInformation JSON or None if function not found
    pub(crate) fn get_user_function_signature(
        &self,
        ast: &Node,
        function_name: &str,
    ) -> Option<Value> {
        // Walk the AST to find the subroutine definition
        let sub_node = self.find_subroutine_definition(ast, function_name)?;

        // Extract parameters from the subroutine
        let mut params = Vec::new();
        if let NodeKind::Subroutine { signature: sub_signature, body, .. } = &sub_node.kind {
            if let Some(sig) = sub_signature {
                if let NodeKind::Signature { parameters } = &sig.kind {
                    for param in parameters {
                        self.extract_signature_params(param, &mut params);
                    }
                }
            } else {
                // Look for my (...) = @_; pattern in the body
                self.extract_params_from_body(body, &mut params);
            }
        }

        // Build signature
        let label = if params.is_empty() {
            format!("sub {}", function_name)
        } else {
            format!("sub {}({})", function_name, params.join(", "))
        };

        let parameters: Vec<Value> = params
            .iter()
            .map(|p| {
                json!({
                    "label": p,
                    "documentation": null
                })
            })
            .collect();

        Some(json!({
            "label": label,
            "documentation": format!("User-defined function '{}'", function_name),
            "parameters": parameters
        }))
    }

    /// Find a subroutine definition by name in the AST
    fn find_subroutine_definition<'a>(&self, node: &'a Node, name: &str) -> Option<&'a Node> {
        match &node.kind {
            NodeKind::Subroutine { name: sub_name, .. } => {
                if let Some(sub_name) = sub_name {
                    if sub_name == name {
                        return Some(node);
                    }
                }
            }
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                for stmt in statements {
                    if let Some(found) = self.find_subroutine_definition(stmt, name) {
                        return Some(found);
                    }
                }
            }
            _ => {}
        }
        None
    }

    /// Extract parameter names from a params node (for signature help)
    fn extract_signature_params(&self, params_node: &Node, params: &mut Vec<String>) {
        if let NodeKind::Variable { sigil, name } = &params_node.kind {
            params.push(format!("{}{}", sigil, name));
        }
    }

    /// Extract parameters from my (...) = @_; pattern in the body
    fn extract_params_from_body(&self, body: &Node, params: &mut Vec<String>) {
        if let NodeKind::Block { statements } = &body.kind {
            if let Some(first_stmt) = statements.first() {
                // Look for my (...) = @_ pattern
                if let NodeKind::VariableListDeclaration { variables, initializer, .. } =
                    &first_stmt.kind
                {
                    // Check if initializer is @_
                    if let Some(init) = initializer {
                        if let NodeKind::Variable { sigil, name } = &init.kind {
                            if sigil == "@" && name == "_" {
                                // Extract params from variables
                                for var in variables {
                                    if let NodeKind::Variable { sigil: var_sigil, name: var_name } =
                                        &var.kind
                                    {
                                        params.push(format!("{}{}", var_sigil, var_name));
                                    }
                                }
                            }
                        }
                    }
                } else if let NodeKind::Assignment { lhs, rhs, .. } = &first_stmt.kind {
                    // Alternative pattern: ($x, $y) = @_
                    if let NodeKind::Variable { sigil, name } = &rhs.kind {
                        if sigil == "@" && name == "_" {
                            // Extract params from lhs
                            self.extract_params_from_lhs(lhs, params);
                        }
                    }
                }
            }
        }
    }

    /// Helper to extract params from left-hand side of assignment
    fn extract_params_from_lhs(&self, lhs: &Node, params: &mut Vec<String>) {
        match &lhs.kind {
            NodeKind::Variable { sigil, name } => {
                params.push(format!("{}{}", sigil, name));
            }
            NodeKind::VariableListDeclaration { variables, .. } => {
                for var in variables {
                    if let NodeKind::Variable { sigil, name } = &var.kind {
                        params.push(format!("{}{}", sigil, name));
                    }
                }
            }
            _ => {}
        }
    }

    /// Get function signature for built-in Perl functions
    ///
    /// Provides signature information for Perl's built-in functions including
    /// I/O operations, string manipulation, array/hash operations, and system calls.
    ///
    /// # Arguments
    ///
    /// * `function_name` - Name of the built-in function
    ///
    /// # Returns
    ///
    /// LSP SignatureInformation JSON or None if not a recognized built-in
    pub(crate) fn get_builtin_function_signature(&self, function_name: &str) -> Option<Value> {
        // Define signatures for common Perl built-in functions
        let signature = match function_name {
            "print" => Some(("print LIST", vec!["LIST"])),
            "printf" => Some(("printf FORMAT, LIST", vec!["FORMAT", "LIST"])),
            "open" => Some(("open FILEHANDLE, MODE, EXPR", vec!["FILEHANDLE", "MODE", "EXPR"])),
            "close" => Some(("close FILEHANDLE", vec!["FILEHANDLE"])),
            "read" => Some((
                "read FILEHANDLE, SCALAR, LENGTH, OFFSET",
                vec!["FILEHANDLE", "SCALAR", "LENGTH", "OFFSET"],
            )),
            "write" => Some(("write FILEHANDLE", vec!["FILEHANDLE"])),
            "die" => Some(("die LIST", vec!["LIST"])),
            "warn" => Some(("warn LIST", vec!["LIST"])),
            "substr" => Some((
                "substr EXPR, OFFSET, LENGTH, REPLACEMENT",
                vec!["EXPR", "OFFSET", "LENGTH", "REPLACEMENT"],
            )),
            "length" => Some(("length EXPR", vec!["EXPR"])),
            "index" => Some(("index STR, SUBSTR, POSITION", vec!["STR", "SUBSTR", "POSITION"])),
            "rindex" => Some(("rindex STR, SUBSTR, POSITION", vec!["STR", "SUBSTR", "POSITION"])),
            "sprintf" => Some(("sprintf FORMAT, LIST", vec!["FORMAT", "LIST"])),
            "join" => Some(("join EXPR, LIST", vec!["EXPR", "LIST"])),
            "split" => Some(("split /PATTERN/, EXPR, LIMIT", vec!["/PATTERN/", "EXPR", "LIMIT"])),
            "push" => Some(("push ARRAY, LIST", vec!["ARRAY", "LIST"])),
            "pop" => Some(("pop ARRAY", vec!["ARRAY"])),
            "shift" => Some(("shift ARRAY", vec!["ARRAY"])),
            "unshift" => Some(("unshift ARRAY, LIST", vec!["ARRAY", "LIST"])),
            "splice" => Some((
                "splice ARRAY, OFFSET, LENGTH, LIST",
                vec!["ARRAY", "OFFSET", "LENGTH", "LIST"],
            )),
            "grep" => Some(("grep BLOCK LIST", vec!["BLOCK", "LIST"])),
            "map" => Some(("map BLOCK LIST", vec!["BLOCK", "LIST"])),
            "sort" => Some(("sort BLOCK LIST", vec!["BLOCK", "LIST"])),
            "reverse" => Some(("reverse LIST", vec!["LIST"])),
            "keys" => Some(("keys HASH", vec!["HASH"])),
            "values" => Some(("values HASH", vec!["HASH"])),
            "each" => Some(("each HASH", vec!["HASH"])),
            "exists" => Some(("exists EXPR", vec!["EXPR"])),
            "delete" => Some(("delete EXPR", vec!["EXPR"])),
            "defined" => Some(("defined EXPR", vec!["EXPR"])),
            "undef" => Some(("undef EXPR", vec!["EXPR"])),
            "ref" => Some(("ref EXPR", vec!["EXPR"])),
            "bless" => Some(("bless REF, CLASSNAME", vec!["REF", "CLASSNAME"])),
            "chomp" => Some(("chomp VARIABLE", vec!["VARIABLE"])),
            "chop" => Some(("chop VARIABLE", vec!["VARIABLE"])),
            "chr" => Some(("chr NUMBER", vec!["NUMBER"])),
            "ord" => Some(("ord EXPR", vec!["EXPR"])),
            "lc" => Some(("lc EXPR", vec!["EXPR"])),
            "uc" => Some(("uc EXPR", vec!["EXPR"])),
            "lcfirst" => Some(("lcfirst EXPR", vec!["EXPR"])),
            "ucfirst" => Some(("ucfirst EXPR", vec!["EXPR"])),

            // File operations
            "seek" => Some((
                "seek FILEHANDLE, POSITION, WHENCE",
                vec!["FILEHANDLE", "POSITION", "WHENCE"],
            )),
            "tell" => Some(("tell FILEHANDLE", vec!["FILEHANDLE"])),
            "stat" => Some(("stat EXPR", vec!["EXPR"])),
            "lstat" => Some(("lstat EXPR", vec!["EXPR"])),
            "chmod" => Some(("chmod MODE, LIST", vec!["MODE", "LIST"])),
            "chown" => Some(("chown UID, GID, LIST", vec!["UID", "GID", "LIST"])),
            "unlink" => Some(("unlink LIST", vec!["LIST"])),
            "rename" => Some(("rename OLDNAME, NEWNAME", vec!["OLDNAME", "NEWNAME"])),
            "mkdir" => Some(("mkdir FILENAME, MODE", vec!["FILENAME", "MODE"])),
            "rmdir" => Some(("rmdir FILENAME", vec!["FILENAME"])),
            "opendir" => Some(("opendir DIRHANDLE, EXPR", vec!["DIRHANDLE", "EXPR"])),
            "readdir" => Some(("readdir DIRHANDLE", vec!["DIRHANDLE"])),
            "closedir" => Some(("closedir DIRHANDLE", vec!["DIRHANDLE"])),
            "link" => Some(("link OLDFILE, NEWFILE", vec!["OLDFILE", "NEWFILE"])),
            "symlink" => Some(("symlink OLDFILE, NEWFILE", vec!["OLDFILE", "NEWFILE"])),
            "readlink" => Some(("readlink EXPR", vec!["EXPR"])),
            "truncate" => Some(("truncate FILEHANDLE, LENGTH", vec!["FILEHANDLE", "LENGTH"])),

            // String/Data functions
            "pack" => Some(("pack TEMPLATE, LIST", vec!["TEMPLATE", "LIST"])),
            "unpack" => Some(("unpack TEMPLATE, EXPR", vec!["TEMPLATE", "EXPR"])),
            "quotemeta" => Some(("quotemeta EXPR", vec!["EXPR"])),
            "hex" => Some(("hex EXPR", vec!["EXPR"])),
            "oct" => Some(("oct EXPR", vec!["EXPR"])),
            "vec" => Some(("vec EXPR, OFFSET, BITS", vec!["EXPR", "OFFSET", "BITS"])),
            "crypt" => Some(("crypt PLAINTEXT, SALT", vec!["PLAINTEXT", "SALT"])),

            // Array/List functions
            "scalar" => Some(("scalar EXPR", vec!["EXPR"])),
            "wantarray" => Some(("wantarray", vec![])),

            // Math functions
            "abs" => Some(("abs VALUE", vec!["VALUE"])),
            "int" => Some(("int EXPR", vec!["EXPR"])),
            "sqrt" => Some(("sqrt EXPR", vec!["EXPR"])),
            "exp" => Some(("exp EXPR", vec!["EXPR"])),
            "log" => Some(("log EXPR", vec!["EXPR"])),
            "sin" => Some(("sin EXPR", vec!["EXPR"])),
            "cos" => Some(("cos EXPR", vec!["EXPR"])),
            "tan" => Some(("tan EXPR", vec!["EXPR"])),
            "atan2" => Some(("atan2 Y, X", vec!["Y", "X"])),
            "rand" => Some(("rand EXPR", vec!["EXPR"])),
            "srand" => Some(("srand EXPR", vec!["EXPR"])),

            // System/Process functions
            "system" => Some(("system LIST", vec!["LIST"])),
            "exec" => Some(("exec LIST", vec!["LIST"])),
            "fork" => Some(("fork", vec![])),
            "wait" => Some(("wait", vec![])),
            "waitpid" => Some(("waitpid PID, FLAGS", vec!["PID", "FLAGS"])),
            "kill" => Some(("kill SIGNAL, LIST", vec!["SIGNAL", "LIST"])),
            "sleep" => Some(("sleep EXPR", vec!["EXPR"])),
            "alarm" => Some(("alarm SECONDS", vec!["SECONDS"])),
            "exit" => Some(("exit EXPR", vec!["EXPR"])),
            "getpgrp" => Some(("getpgrp PID", vec!["PID"])),
            "setpgrp" => Some(("setpgrp PID, PGRP", vec!["PID", "PGRP"])),
            "getppid" => Some(("getppid", vec![])),
            "getpriority" => Some(("getpriority WHICH, WHO", vec!["WHICH", "WHO"])),
            "setpriority" => {
                Some(("setpriority WHICH, WHO, PRIORITY", vec!["WHICH", "WHO", "PRIORITY"]))
            }

            // Time functions
            "time" => Some(("time", vec![])),
            "localtime" => Some(("localtime EXPR", vec!["EXPR"])),
            "gmtime" => Some(("gmtime EXPR", vec!["EXPR"])),
            "times" => Some(("times", vec![])),

            // User/Group functions
            "getpwuid" => Some(("getpwuid UID", vec!["UID"])),
            "getpwnam" => Some(("getpwnam NAME", vec!["NAME"])),
            "getgrgid" => Some(("getgrgid GID", vec!["GID"])),
            "getgrnam" => Some(("getgrnam NAME", vec!["NAME"])),
            "getlogin" => Some(("getlogin", vec![])),

            // Network functions
            "socket" => Some((
                "socket SOCKET, DOMAIN, TYPE, PROTOCOL",
                vec!["SOCKET", "DOMAIN", "TYPE", "PROTOCOL"],
            )),
            "bind" => Some(("bind SOCKET, NAME", vec!["SOCKET", "NAME"])),
            "listen" => Some(("listen SOCKET, QUEUESIZE", vec!["SOCKET", "QUEUESIZE"])),
            "accept" => {
                Some(("accept NEWSOCKET, GENERICSOCKET", vec!["NEWSOCKET", "GENERICSOCKET"]))
            }
            "connect" => Some(("connect SOCKET, NAME", vec!["SOCKET", "NAME"])),
            "send" => Some(("send SOCKET, MSG, FLAGS, TO", vec!["SOCKET", "MSG", "FLAGS", "TO"])),
            "recv" => Some((
                "recv SOCKET, SCALAR, LENGTH, FLAGS",
                vec!["SOCKET", "SCALAR", "LENGTH", "FLAGS"],
            )),
            "shutdown" => Some(("shutdown SOCKET, HOW", vec!["SOCKET", "HOW"])),
            "getsockname" => Some(("getsockname SOCKET", vec!["SOCKET"])),
            "getpeername" => Some(("getpeername SOCKET", vec!["SOCKET"])),

            // Control flow
            "eval" => Some(("eval EXPR", vec!["EXPR"])),
            "require" => Some(("require EXPR", vec!["EXPR"])),
            "do" => Some(("do EXPR", vec!["EXPR"])),
            "caller" => Some(("caller EXPR", vec!["EXPR"])),
            "return" => Some(("return LIST", vec!["LIST"])),
            "goto" => Some(("goto LABEL", vec!["LABEL"])),
            "last" => Some(("last LABEL", vec!["LABEL"])),
            "next" => Some(("next LABEL", vec!["LABEL"])),
            "redo" => Some(("redo LABEL", vec!["LABEL"])),

            // Misc functions
            "tie" => Some(("tie VARIABLE, CLASSNAME, LIST", vec!["VARIABLE", "CLASSNAME", "LIST"])),
            "untie" => Some(("untie VARIABLE", vec!["VARIABLE"])),
            "tied" => Some(("tied VARIABLE", vec!["VARIABLE"])),
            "dbmopen" => Some(("dbmopen HASH, DBNAME, MODE", vec!["HASH", "DBNAME", "MODE"])),
            "dbmclose" => Some(("dbmclose HASH", vec!["HASH"])),
            "select" => Some(("select FILEHANDLE", vec!["FILEHANDLE"])),
            "syscall" => Some(("syscall NUMBER, LIST", vec!["NUMBER", "LIST"])),
            "dump" => Some(("dump LABEL", vec!["LABEL"])),
            "prototype" => Some(("prototype FUNCTION", vec!["FUNCTION"])),
            "lock" => Some(("lock THING", vec!["THING"])),

            _ => None,
        };

        if let Some((label, params)) = signature {
            let parameters: Vec<Value> = params
                .iter()
                .map(|p| {
                    json!({
                        "label": p.to_string()
                    })
                })
                .collect();

            Some(json!({
                "label": label,
                "parameters": parameters
            }))
        } else {
            None
        }
    }
}
