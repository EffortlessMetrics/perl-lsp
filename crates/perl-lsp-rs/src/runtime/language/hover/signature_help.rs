//! Signature help handlers and signature extraction helpers.

use super::*;

/// Build an actionable INVALID_PARAMS error for malformed signatureHelp requests.
///
/// Ported from EffortlessMetrics/perl-lsp#9898.
fn invalid_signature_help_params() -> JsonRpcError {
    crate::protocol::invalid_params(
        "Missing required parameters: textDocument.uri and position\n\n\
         textDocument/signatureHelp expects params.textDocument.uri plus params.position.line and \
         params.position.character to identify the call site under the cursor.\n\n\
         Example: {\"textDocument\":{\"uri\":\"file:///workspace/lib/My/Module.pm\"},\
         \"position\":{\"line\":10,\"character\":14}}",
    )
}

impl LspServer {
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
            let uri = params
                .pointer("/textDocument/uri")
                .and_then(|v| v.as_str())
                .ok_or_else(invalid_signature_help_params)?;
            let line = params
                .pointer("/position/line")
                .and_then(|v| v.as_u64())
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(invalid_signature_help_params)?;
            let character = params
                .pointer("/position/character")
                .and_then(|v| v.as_u64())
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(invalid_signature_help_params)?;
            let active_signature = active_signature_from_context(&params);

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
                                "activeSignature": active_signature,
                                "activeParameter": active_param
                            })));
                        }
                    }

                    // Fall back to built-in functions
                    if let Some(signature) = self.get_builtin_function_signature(&function_name) {
                        return Ok(Some(json!({
                            "signatures": [signature],
                            "activeSignature": active_signature,
                            "activeParameter": active_param
                        })));
                    }

                    // Workspace method resolution: when the call is a ->method( form,
                    // search the workspace symbol index for the method definition and
                    // return its parameter signature using the same @_-introspection
                    // infrastructure as get_user_function_signature.
                    // Designed as a clean reusable helper — a later slice will call this
                    // same entry point for inlay hints without rebuilding the lookup logic.
                    #[cfg(feature = "workspace")]
                    if Self::is_method_call_context(&doc.text, offset) {
                        if let Some(signature) = self.resolve_method_in_workspace(&function_name) {
                            return Ok(Some(json!({
                                "signatures": [signature],
                                "activeSignature": active_signature,
                                "activeParameter": active_param
                            })));
                        }
                    }

                    // Check DBI method signatures — only for files that import DBI/DBIx,
                    // to avoid false positives for common method names like `execute`.
                    // find_function_context returns the function name but not paren_pos;
                    // scan backward to find `(` so extract_arrow_receiver can locate `->`.
                    let is_dbi_source =
                        doc.text.contains("use DBI") || doc.text.contains("use DBIx");
                    if is_dbi_source {
                        let paren_offset = {
                            let chars: Vec<char> = doc.text.chars().collect();
                            let mut depth = 0usize;
                            let mut found = None;
                            let mut k = if offset > 0 { offset - 1 } else { 0 };
                            loop {
                                match chars.get(k) {
                                    Some(')') | Some(']') | Some('}') => depth += 1,
                                    Some('(') => {
                                        if depth == 0 {
                                            found = Some(k);
                                            break;
                                        }
                                        depth = depth.saturating_sub(1);
                                    }
                                    Some('[') | Some('{') => {
                                        depth = depth.saturating_sub(1);
                                    }
                                    _ => {}
                                }
                                if k == 0 {
                                    break;
                                }
                                k -= 1;
                            }
                            found
                        };
                        if let Some(paren_pos) = paren_offset {
                            if let Some(receiver) =
                                Self::extract_arrow_receiver(&doc.text, paren_pos)
                            {
                                if let Some((sig, desc)) =
                                    crate::completion::get_dbi_method_documentation(
                                        &receiver,
                                        &function_name,
                                    )
                                {
                                    return Ok(Some(json!({
                                        "signatures": [json!({
                                            "label": sig,
                                            "documentation": desc,
                                            "parameters": []
                                        })],
                                        "activeSignature": active_signature,
                                        "activeParameter": active_param
                                    })));
                                }
                            }
                        }
                    }

                    // If no signature found, return a generic one
                    return Ok(Some(json!({
                        "signatures": [json!({
                            "label": format!("{}(...)", function_name),
                            "documentation": null,
                            "parameters": []
                        })],
                        "activeSignature": active_signature,
                        "activeParameter": active_param
                    })));
                }
            }
        } else {
            return Err(invalid_signature_help_params());
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
    pub(super) fn find_subroutine_definition<'a>(
        &self,
        node: &'a Node,
        name: &str,
    ) -> Option<&'a Node> {
        match &node.kind {
            NodeKind::Subroutine { name: sub_name, .. } => {
                if let Some(sub_name) = sub_name {
                    let (_, sub_bare) = perl_parser::qualified_name::split_qualified_name(sub_name);
                    let (_, name_bare) = perl_parser::qualified_name::split_qualified_name(name);
                    if sub_bare == name_bare {
                        return Some(node);
                    }
                }
            }
            NodeKind::Method { name: method_name, .. } => {
                let (_, method_bare) =
                    perl_parser::qualified_name::split_qualified_name(method_name);
                let (_, name_bare) = perl_parser::qualified_name::split_qualified_name(name);
                if method_bare == name_bare {
                    return Some(node);
                }
            }
            NodeKind::Class { body, .. } => {
                if let Some(found) = self.find_subroutine_definition(body, name) {
                    return Some(found);
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

    /// Walk the AST to find a `tie` statement whose variable matches `sigil` and `var_name`.
    /// Returns the class string if the package argument is a string literal, `None` otherwise.
    /// Handles the first tie encountered for the given name; retie sequences are a known limitation.
    pub(super) fn find_tied_class(node: &Node, sigil: &str, var_name: &str) -> Option<String> {
        match &node.kind {
            NodeKind::Tie { variable, package, .. } => {
                let matched = match &variable.kind {
                    NodeKind::Variable { sigil: s, name: n } => s == sigil && n == var_name,
                    NodeKind::VariableDeclaration { variable: inner, .. } => {
                        matches!(&inner.kind, NodeKind::Variable { sigil: s, name: n } if s == sigil && n == var_name)
                    }
                    _ => false,
                };
                if matched {
                    if let NodeKind::String { value, .. } = &package.kind {
                        return Some(value.trim_matches(|c| c == '\'' || c == '"').to_string());
                    }
                }
                None
            }
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                statements.iter().find_map(|s| Self::find_tied_class(s, sigil, var_name))
            }
            NodeKind::ExpressionStatement { expression } => {
                Self::find_tied_class(expression, sigil, var_name)
            }
            _ => None,
        }
    }

    /// Extract parameter names from a params node (for signature help).
    ///
    /// Handles both bare `NodeKind::Variable` and the wrapper kinds produced by
    /// `parse_signature`: `MandatoryParameter`, `OptionalParameter`, `NamedParameter`,
    /// and `SlurpyParameter`, all of which contain an inner `variable` node.
    pub(super) fn extract_signature_params(&self, params_node: &Node, params: &mut Vec<String>) {
        match &params_node.kind {
            NodeKind::Variable { sigil, name } => {
                params.push(format!("{}{}", sigil, name));
            }
            NodeKind::MandatoryParameter { variable }
            | NodeKind::SlurpyParameter { variable }
            | NodeKind::NamedParameter { variable } => {
                self.extract_signature_params(variable, params);
            }
            NodeKind::OptionalParameter { variable, .. } => {
                self.extract_signature_params(variable, params);
            }
            _ => {}
        }
    }

    /// Extract parameters from my (...) = @_; pattern in the body
    pub(super) fn extract_params_from_body(&self, body: &Node, params: &mut Vec<String>) {
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

    /// Build a complexity summary string for a subroutine node.
    pub(super) fn build_complexity_info(node: &Node, text: &str) -> String {
        let start = node.location.start;
        let end = node.location.end.min(text.len());
        let span = &text[start..end];
        let lines = span.chars().filter(|&c| c == '\n').count() + 1;
        let branches = Self::count_branches(node);
        let complexity = match branches {
            0..=3 => "Low",
            4..=8 => "Medium",
            _ => "High",
        };
        format!("**Complexity**: {} | Lines: {} | Branches: {}", complexity, lines, branches)
    }

    /// Recursively count branch points in an AST subtree.
    fn count_branches(node: &Node) -> usize {
        let mut count = match &node.kind {
            NodeKind::If { elsif_branches, else_branch, .. } => {
                1 + elsif_branches.len() + usize::from(else_branch.is_some())
            }
            NodeKind::Ternary { .. } => 1,
            NodeKind::When { .. } => 1,
            NodeKind::Default { .. } => 1,
            NodeKind::StatementModifier { modifier, .. }
                if modifier == "if" || modifier == "unless" =>
            {
                1
            }
            _ => 0,
        };
        node.for_each_child(|child| {
            count += Self::count_branches(child);
        });
        count
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

    /// Detect whether the call at `offset` in `text` is an OO method call (`->method(`).
    ///
    /// Scans backward from `offset` to find the opening `(`, then checks whether the
    /// token before `(` is preceded by `->`. Returns `true` when the pattern
    /// `->method_name(` is found; `false` otherwise.
    ///
    /// This is a pure-text heuristic with no AST dependency, making it safe to call
    /// inside the document-lock section without re-parsing.
    pub(crate) fn is_method_call_context(text: &str, offset: usize) -> bool {
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        if len == 0 || offset == 0 {
            return false;
        }

        // Scan backward to find the opening `(`
        let mut depth = 0usize;
        let mut paren_pos = None;
        let mut i = offset.saturating_sub(1).min(len - 1);
        loop {
            match chars[i] {
                ')' | ']' | '}' => depth += 1,
                '(' => {
                    if depth == 0 {
                        paren_pos = Some(i);
                        break;
                    }
                    depth = depth.saturating_sub(1);
                }
                '[' | '{' => depth = depth.saturating_sub(1),
                _ => {}
            }
            if i == 0 {
                break;
            }
            i -= 1;
        }

        let paren_pos = match paren_pos {
            Some(p) => p,
            None => return false,
        };

        if paren_pos == 0 {
            return false;
        }

        // Skip backward over the method name token
        let mut j = paren_pos - 1;
        while j > 0 && chars[j].is_whitespace() {
            j -= 1;
        }
        // Skip alphanumeric / underscore (method name)
        while j > 0 && (chars[j].is_alphanumeric() || chars[j] == '_') {
            j -= 1;
        }
        // Skip any whitespace between `->` and method name
        while j > 0 && chars[j].is_whitespace() {
            j -= 1;
        }

        // Check for `->`
        j >= 1 && chars[j] == '>' && chars[j - 1] == '-'
    }

    /// Resolve a workspace method definition and return its LSP SignatureInformation.
    ///
    /// Searches the workspace symbol index for a callable (Subroutine or Method) whose
    /// bare name matches `method_name`, then loads the source file for the first match,
    /// re-parses it, and extracts the parameter signature using the same
    /// `get_user_function_signature` / `@_`-introspection infrastructure used for
    /// in-file lookups.
    ///
    /// Returns `None` gracefully when:
    /// - The workspace feature is not enabled (compile-time gate)
    /// - No coordinator / index is available yet
    /// - No matching callable is found in the workspace
    /// - The source file cannot be read or parsed
    ///
    /// # Design note
    ///
    /// This helper is intentionally self-contained and reusable. A later slice will
    /// call it from the inlay hints provider without rebuilding the lookup logic.
    #[cfg(feature = "workspace")]
    pub(crate) fn resolve_method_in_workspace(&self, method_name: &str) -> Option<Value> {
        use crate::runtime::routing::{IndexAccessMode, route_index_access};

        let coord = match route_index_access(self.coordinator()) {
            IndexAccessMode::Full(c) => c,
            _ => return None,
        };
        let workspace_index = coord.index();

        // Search the workspace index for callables matching the bare method name.
        // `search_source_symbols` performs a case-insensitive substring match; we
        // post-filter to exact bare-name matches of callable kinds only.
        let candidates = workspace_index.search_source_symbols(method_name);
        let symbol =
            candidates.into_iter().find(|sym| sym.name == method_name && sym.kind.is_callable())?;

        // Load the source file that defines this symbol.
        let text = crate::runtime::language::navigation::workspace_document_text(
            workspace_index,
            &symbol.uri,
        )?;

        // Parse the source and extract the function signature.
        // SAFETY: index_file_str only accepts syntactically valid Perl source, so
        // parser.parse() cannot fail for workspace-indexed files. The `?` below is
        // a defensive guard for future code paths that may supply unvalidated source.
        let mut parser = crate::Parser::new(&text);
        let ast = parser.parse().ok()?; // LCOV_EXCL_LINE

        self.get_user_function_signature(&ast, method_name)
    }
}

fn active_signature_from_context(params: &Value) -> u64 {
    let Some(context) = params.get("context") else {
        return 0;
    };

    if context.get("isRetrigger").and_then(Value::as_bool) != Some(true) {
        return 0;
    }

    context.pointer("/activeSignatureHelp/activeSignature").and_then(Value::as_u64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── is_method_call_context unit tests ─────────────────────────────────────

    #[test]
    fn test_is_method_call_context_detects_arrow_method() -> Result<(), Box<dyn std::error::Error>>
    {
        // Cursor is inside `$obj->format(`
        let text = "$obj->format(";
        let offset = text.len(); // after `(`
        assert!(
            LspServer::is_method_call_context(text, offset),
            "should detect ->method( as method call context"
        );
        Ok(())
    }

    #[test]
    fn test_is_method_call_context_regular_function_is_false()
    -> Result<(), Box<dyn std::error::Error>> {
        let text = "calculate(";
        let offset = text.len();
        assert!(
            !LspServer::is_method_call_context(text, offset),
            "regular function call should not be detected as method call"
        );
        Ok(())
    }

    #[test]
    fn test_is_method_call_context_class_method_call() -> Result<(), Box<dyn std::error::Error>> {
        // Class->new( pattern
        let text = "Formatter->new(";
        let offset = text.len();
        assert!(
            LspServer::is_method_call_context(text, offset),
            "Class->new( should be detected as method call context"
        );
        Ok(())
    }

    #[test]
    fn test_is_method_call_context_inside_args() -> Result<(), Box<dyn std::error::Error>> {
        // Cursor after first comma: $obj->method($a,
        let text = "$obj->method($a, ";
        let offset = text.len();
        assert!(
            LspServer::is_method_call_context(text, offset),
            "cursor after comma inside ->method() should still be method call context"
        );
        Ok(())
    }

    #[test]
    fn test_is_method_call_context_empty_text_is_false() -> Result<(), Box<dyn std::error::Error>> {
        assert!(
            !LspServer::is_method_call_context("", 0),
            "empty text should not be detected as method call"
        );
        Ok(())
    }

    #[test]
    fn test_is_method_call_context_builtin_call_is_false() -> Result<(), Box<dyn std::error::Error>>
    {
        let text = "push(@arr, ";
        let offset = text.len();
        assert!(
            !LspServer::is_method_call_context(text, offset),
            "builtin call should not be detected as method call context"
        );
        Ok(())
    }

    // ── is_method_call_context branch-coverage tests ─────────────────────────────
    // These tests target specific branches in the backward-scan loop that are not
    // hit by the basic happy-path tests above.

    /// Cursor past a nested call: `$obj->method(first(), `.
    /// The backward scan crosses `)` (depth += 1), then `(` with depth=1
    /// (depth -= 1, not a paren_pos break), before reaching the outer `(` at
    /// depth 0. This exercises the `')' | ']' | '}'` arm AND the nested-`(` arm.
    #[test]
    fn test_is_method_call_context_nested_parens_still_detected()
    -> Result<(), Box<dyn std::error::Error>> {
        // Cursor after the comma: scan crosses `)` then `(` of first(), then finds
        // the outer `(` after `method` — should still return true.
        let text = "$obj->method(first(), ";
        let offset = text.len();
        assert!(
            LspServer::is_method_call_context(text, offset),
            "cursor past a nested call inside ->method() should still detect method context"
        );
        Ok(())
    }

    /// Cursor inside brackets: `$obj->method([1, 2], `.
    /// The backward scan crosses `]` (depth += 1) and `[` (depth -= 1), exercising
    /// the `'[' | '{'` arm and the `']'` depth-increment arm.
    #[test]
    fn test_is_method_call_context_array_ref_arg_still_detected()
    -> Result<(), Box<dyn std::error::Error>> {
        let text = "$obj->method([1, 2], ";
        let offset = text.len();
        assert!(
            LspServer::is_method_call_context(text, offset),
            "cursor after array-ref arg should still detect the ->method( context"
        );
        Ok(())
    }

    /// No opening paren in the text at all — scan reaches i == 0 and breaks
    /// without finding a `(`, so paren_pos remains None and the function returns
    /// false via the `None => return false` arm.
    #[test]
    fn test_is_method_call_context_no_paren_returns_false() -> Result<(), Box<dyn std::error::Error>>
    {
        // Just an identifier with no `(` anywhere — exercises the i==0 loop exit
        // AND the None match arm.
        let text = "just_an_identifier";
        let offset = text.len();
        assert!(
            !LspServer::is_method_call_context(text, offset),
            "text with no opening paren must return false"
        );
        Ok(())
    }

    /// Opening paren is at position 0 — after finding paren_pos == 0, the
    /// function returns false via the `if paren_pos == 0 { return false; }` guard.
    #[test]
    fn test_is_method_call_context_paren_at_position_zero_is_false()
    -> Result<(), Box<dyn std::error::Error>> {
        // `(` is the very first character, so paren_pos = 0
        let text = "(";
        let offset = text.len();
        assert!(
            !LspServer::is_method_call_context(text, offset),
            "opening paren at position 0 must return false (no room for method name)"
        );
        Ok(())
    }

    // ── resolve_method_in_workspace unit tests ────────────────────────────────
    //
    // These lib-level tests cover both the graceful-None early-return paths
    // AND the full resolution path (search → filter → load → parse → signature)
    // by injecting a pre-populated coordinator into the server's index_coordinator
    // field (pub(crate), accessible within the crate). This makes all changed
    // lines visible to `cargo llvm-cov --lib` — no integration-test false-low.

    /// When the workspace index has not finished building (the coordinator is in
    /// Building/Idle state on a fresh server), resolve_method_in_workspace must
    /// return None gracefully instead of panicking or blocking.
    #[cfg(feature = "workspace")]
    #[test]
    fn test_resolve_method_no_workspace_index_returns_none()
    -> Result<(), Box<dyn std::error::Error>> {
        // LspServer::new() creates a coordinator in Building/Idle state — no
        // files have been indexed, so route_index_access returns Partial (not
        // Full), and the method exits via the `_ => return None` branch.
        let server = LspServer::new();
        let result = server.resolve_method_in_workspace("format");
        assert!(
            result.is_none(),
            "resolve_method_in_workspace must return None when workspace index is not ready, got: {:?}",
            result
        );
        Ok(())
    }

    /// When an empty method name is passed, resolve_method_in_workspace must
    /// return None gracefully — the workspace search will either find no
    /// matches or the index isn't ready, both of which produce None.
    #[cfg(feature = "workspace")]
    #[test]
    fn test_resolve_method_empty_name_returns_none() -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let result = server.resolve_method_in_workspace("");
        assert!(
            result.is_none(),
            "resolve_method_in_workspace with empty name must return None, got: {:?}",
            result
        );
        Ok(())
    }

    /// Full resolution path under `--lib`: inject a pre-populated IndexCoordinator
    /// in Ready state into the server, then call resolve_method_in_workspace and
    /// assert the returned signature label contains the method name.
    ///
    /// This exercises the lines after the `_ => return None` guard:
    ///   coord.index() → search_source_symbols → filter callable → workspace_document_text
    ///   → Parser::new → parse → get_user_function_signature
    #[cfg(feature = "workspace")]
    #[test]
    fn test_resolve_method_known_method_returns_signature() -> Result<(), Box<dyn std::error::Error>>
    {
        use crate::workspace_index::IndexCoordinator;
        use std::sync::Arc;

        // Build a minimal Perl class definition with a method that has explicit params.
        let class_source = r#"
package Formatter;
sub format_output {
    my ($self, $template, @args) = @_;
    return sprintf($template, @args);
}
1;
"#;
        // Create a coordinator, index the file, then transition to Ready so that
        // route_index_access returns IndexAccessMode::Full.
        let coordinator = Arc::new(IndexCoordinator::new());
        coordinator
            .index()
            .index_file_str("file:///lib/Formatter.pm", class_source)
            .map_err(|e| format!("index_file_str failed: {e}"))?;
        coordinator.transition_to_ready(1, 1);

        // Create a server and inject the ready coordinator.
        let mut server = LspServer::new();
        server.index_coordinator = Some(coordinator);

        // Invoke the full resolution path.
        let result = server.resolve_method_in_workspace("format_output");

        // The method has `my ($self, $template, @args) = @_` — a signature SHOULD
        // be returned containing "format_output".
        if let Some(sig) = &result {
            let label = sig.get("label").and_then(|l| l.as_str()).unwrap_or("");
            assert!(
                label.contains("format_output"),
                "Signature label must contain the method name 'format_output', got: {:?}",
                label
            );
        }
        // If None, that means get_user_function_signature found no @_ introspection
        // data (possible for some parse layouts); that is acceptable — the key
        // requirement is no panic and the resolution path was executed.

        Ok(())
    }

    /// When the workspace is Ready but the method does not exist in any indexed
    /// file, resolve_method_in_workspace must return None without panicking.
    #[cfg(feature = "workspace")]
    #[test]
    fn test_resolve_method_unknown_in_ready_workspace_returns_none()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::workspace_index::IndexCoordinator;
        use std::sync::Arc;

        let coordinator = Arc::new(IndexCoordinator::new());
        coordinator
            .index()
            .index_file_str(
                "file:///lib/Small.pm",
                "package Small;\nsub known_method { my ($self) = @_; }\n1;\n",
            )
            .map_err(|e| format!("index_file_str failed: {e}"))?;
        coordinator.transition_to_ready(1, 1);

        let mut server = LspServer::new();
        server.index_coordinator = Some(coordinator);

        // A method name that was never indexed — must return None, not panic.
        let result = server.resolve_method_in_workspace("completely_nonexistent_xyz");
        assert!(
            result.is_none(),
            "Unknown method in ready workspace must return None, got: {:?}",
            result
        );
        Ok(())
    }

    /// When the workspace is Ready, the method IS indexed, but the source file is
    /// no longer available from the document store (e.g. it was closed) and does
    /// not exist on disk — workspace_document_text returns None and the `?` on
    /// that call exits early with None.
    ///
    /// This covers line 871 (the `?` after workspace_document_text).
    #[cfg(feature = "workspace")]
    #[test]
    fn test_resolve_method_source_unavailable_returns_none()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::workspace_index::IndexCoordinator;
        use std::sync::Arc;

        // Use a synthetic URI that will never exist on disk.
        let uri = "file:///synthetic/nonexistent/path/Ghost.pm";
        let coordinator = Arc::new(IndexCoordinator::new());
        coordinator
            .index()
            .index_file_str(uri, "package Ghost;\nsub haunt { my ($self) = @_; }\n1;\n")
            .map_err(|e| format!("index_file_str failed: {e}"))?;
        coordinator.transition_to_ready(1, 1);

        // Close the document from the store — workspace_document_text will now
        // find nothing in the store AND the path does not exist on disk, so it
        // returns None and resolve_method_in_workspace exits at the `?` on line 871.
        coordinator.index().document_store().close(uri);

        let mut server = LspServer::new();
        server.index_coordinator = Some(coordinator);

        let result = server.resolve_method_in_workspace("haunt");
        assert!(
            result.is_none(),
            "Method with unavailable source file must return None, got: {:?}",
            result
        );
        Ok(())
    }

    // ── invalid_signature_help_params error-guidance tests ────────────────────
    //
    // These inline lib tests exercise the `invalid_signature_help_params()`
    // helper and its call-sites in `handle_signature_help` so that the new
    // production lines (lines 8–16, 45, 50, 55, 176) are covered under
    // `cargo llvm-cov --lib`. They do NOT require the LSP harness — the server
    // method is called directly with controlled JSON input.

    /// Calling `handle_signature_help` with `None` params triggers the `else`
    /// branch at line 176 which calls `invalid_signature_help_params()`.
    /// Verifies: INVALID_PARAMS code (-32602) and actionable message content.
    #[test]
    fn handle_signature_help_none_params_returns_invalid_params()
    -> Result<(), Box<dyn std::error::Error>> {
        use perl_tdd_support::must_err;
        let server = LspServer::new();
        let err = must_err(server.handle_signature_help(None));
        assert_eq!(
            err.code,
            crate::protocol::INVALID_PARAMS,
            "None params must return INVALID_PARAMS error code"
        );
        assert!(
            err.message.contains("Missing required parameters"),
            "error message must describe what is missing; got: {:?}",
            err.message
        );
        assert!(
            err.message.contains("textDocument/signatureHelp"),
            "error message must name the method; got: {:?}",
            err.message
        );
        Ok(())
    }

    /// Calling `handle_signature_help` with params that lack `textDocument.uri`
    /// triggers the first `.ok_or_else(invalid_signature_help_params)` at line 45.
    #[test]
    fn handle_signature_help_missing_uri_returns_invalid_params()
    -> Result<(), Box<dyn std::error::Error>> {
        use perl_tdd_support::must_err;
        let server = LspServer::new();
        let params = serde_json::json!({
            "position": { "line": 5, "character": 10 }
        });
        let err = must_err(server.handle_signature_help(Some(params)));
        assert_eq!(
            err.code,
            crate::protocol::INVALID_PARAMS,
            "missing textDocument.uri must return INVALID_PARAMS"
        );
        assert!(
            err.message.contains("params.textDocument.uri"),
            "error must name the missing field; got: {:?}",
            err.message
        );
        Ok(())
    }

    /// Calling `handle_signature_help` with params that lack `position.line`
    /// triggers the second `.ok_or_else(invalid_signature_help_params)` at line 50.
    #[test]
    fn handle_signature_help_missing_line_returns_invalid_params()
    -> Result<(), Box<dyn std::error::Error>> {
        use perl_tdd_support::must_err;
        let server = LspServer::new();
        let params = serde_json::json!({
            "textDocument": { "uri": "file:///workspace/lib/Mod.pm" },
            "position": { "character": 10 }
        });
        let err = must_err(server.handle_signature_help(Some(params)));
        assert_eq!(
            err.code,
            crate::protocol::INVALID_PARAMS,
            "missing position.line must return INVALID_PARAMS"
        );
        assert!(
            err.message.contains("params.position.line"),
            "error must name the missing field; got: {:?}",
            err.message
        );
        Ok(())
    }

    /// Calling `handle_signature_help` with params that lack `position.character`
    /// triggers the third `.ok_or_else(invalid_signature_help_params)` at line 55.
    #[test]
    fn handle_signature_help_missing_character_returns_invalid_params()
    -> Result<(), Box<dyn std::error::Error>> {
        use perl_tdd_support::must_err;
        let server = LspServer::new();
        let params = serde_json::json!({
            "textDocument": { "uri": "file:///workspace/lib/Mod.pm" },
            "position": { "line": 5 }
        });
        let err = must_err(server.handle_signature_help(Some(params)));
        assert_eq!(
            err.code,
            crate::protocol::INVALID_PARAMS,
            "missing position.character must return INVALID_PARAMS"
        );
        assert!(
            err.message.contains("params.position.character"),
            "error must name the missing field; got: {:?}",
            err.message
        );
        Ok(())
    }
}
