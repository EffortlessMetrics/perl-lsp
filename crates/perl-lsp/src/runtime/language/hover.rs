//! Hover and signature help handlers
//!
//! Provides hover information and function signature help for Perl code.

use super::super::*;
use crate::cancellation::RequestCleanupGuard;
use crate::protocol::{req_position, req_uri};

/// Intermediate result from phase-1 hover extraction (under document lock).
///
/// The document lock must be released before calling module resolution to avoid
/// deadlock, so we extract what we need first and resolve afterwards.
enum HoverExtracted {
    /// Hover content fully built (symbol, builtin, or token hover).
    Complete(Value),
    /// A `use Module` was found; module name needs resolution without lock.
    /// Carries (module_name, doc_text, doc_uri) for use lib / FindBin wiring.
    UseModule(String, String, String),
    /// Cursor is on a `->method()` call where the method belongs to an inherited or
    /// role-composed ancestor class. Carries (receiver_pkg, method_name, doc_uri).
    /// Phase 2 resolves the hover using the workspace index BFS (same logic as
    /// `inherited_method_definition_location` in navigation.rs).
    InheritedMethod(String, String, String),
    /// Nothing hoverable at this position.
    None,
}

#[cfg(test)]
mod tests {
    use super::LspServer;

    #[test]
    fn test_internal_pl_sv_yes_hover_from_sigiled_token() {
        let text = "print $PL_sv_yes;\n";
        let offset = text.find('$').expect("sigil should exist");

        assert_eq!(
            LspServer::extract_special_variable(text, offset).as_deref(),
            Some("$PL_sv_yes")
        );

        let hover = LspServer::get_special_variable_hover("$PL_sv_yes")
            .expect("hover should exist for $PL_sv_yes");
        let value = hover["contents"]["value"].as_str().expect("markdown hover text");
        assert!(
            value.contains("true scalar"),
            "hover should describe the shared true scalar: {value}"
        );
    }
}

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

            // Phase 1: Extract hover info under document lock
            let extracted = {
                let documents = self.documents_guard();
                if let Some(doc) = self.get_document(&documents, uri) {
                    if let Some(ast) = &doc.ast {
                        let offset = self.pos16_to_offset(doc, line, character);

                        // Check for `use Module` at this offset first
                        if let Some(module_name) = Self::find_use_module_at_offset(ast, offset) {
                            // If the module is a known pragma, return pragma docs immediately
                            // without doing module file resolution.
                            if let Some(pragma_hover) = Self::build_pragma_hover(&module_name) {
                                HoverExtracted::Complete(pragma_hover)
                            } else {
                                HoverExtracted::UseModule(
                                    module_name,
                                    doc.text.clone(),
                                    uri.to_string(),
                                )
                            }
                        } else if let Some(module_name) =
                            Self::find_with_module_at_offset(ast, offset)
                        {
                            // Check for `with 'Role'` / `extends 'Parent'` at this offset
                            HoverExtracted::UseModule(
                                module_name,
                                doc.text.clone(),
                                uri.to_string(),
                            )
                        } else {
                            self.extract_symbol_hover(uri, ast, &doc.text, offset)
                        }
                    } else {
                        let offset = self.pos16_to_offset(doc, line, character);
                        Self::extract_token_hover(uri, &doc.text, offset)
                    }
                } else {
                    HoverExtracted::None
                }
            };
            // Document lock released here

            // Phase 2: Resolve module or return pre-built hover
            match extracted {
                HoverExtracted::Complete(value) => return Ok(Some(value)),
                HoverExtracted::UseModule(module_name, doc_text, doc_uri) => {
                    return Ok(Some(self.build_module_hover(&module_name, &doc_text, &doc_uri)));
                }
                #[cfg(feature = "workspace")]
                HoverExtracted::InheritedMethod(receiver_pkg, method_name, doc_uri) => {
                    if let Some(hover_value) =
                        self.build_inherited_method_hover(&receiver_pkg, &method_name, &doc_uri)
                    {
                        return Ok(Some(hover_value));
                    }
                }
                #[cfg(not(feature = "workspace"))]
                HoverExtracted::InheritedMethod(..) => {}
                HoverExtracted::None => {}
            }
        }

        Ok(Some(json!(null)))
    }

    /// Extract hover information from semantic analysis (called under document lock).
    ///
    /// Uses `get_or_build_analyzer` so repeated hovers on the same document version
    /// share a single cached `SemanticAnalyzer` rather than re-traversing the AST.
    fn extract_symbol_hover(
        &self,
        uri: &str,
        ast: &Node,
        text: &str,
        offset: usize,
    ) -> HoverExtracted {
        if let Some(xs_hover) = Self::extract_xs_api_hover(uri, text, offset) {
            return HoverExtracted::Complete(xs_hover);
        }

        let analyzer = self.get_or_build_analyzer(uri, text, ast);

        if let Some(symbol_info) =
            analyzer.symbol_at(crate::SourceLocation { start: offset, end: offset })
            && let Some(modifier_kind) =
                symbol_info.attributes.iter().find_map(|a| a.strip_prefix("modifier="))
        {
            let method_name = &symbol_info.name;
            let kind_label = match modifier_kind {
                "before" => "runs **before** the method — use for preconditions and logging",
                "after" => "runs **after** the method — use for postconditions and cleanup",
                "around" => {
                    "wraps the method — receives `$orig` as first arg, must call `$orig->($self, @_)`"
                }
                "override" => "overrides the parent method — use to replace inherited behavior",
                "augment" => "extends the parent method — call `inner()` to invoke the next layer",
                _ => "modifies the method",
            };
            let doc = symbol_info.documentation.as_deref().unwrap_or("");
            return HoverExtracted::Complete(json!({
                "contents": {
                    "kind": "markdown",
                    "value": format!(
                        "**Method Modifier (`{modifier_kind}`)**\n\n`{method_name}` — {kind_label}\n\n{doc}"
                    ),
                },
            }));
        }

        if let Some(symbol_info) = analyzer.find_definition(offset) {
            // Detect Moo/Moose attribute accessors (declaration == "has") early and
            // render a dedicated card that shows the attribute metadata clearly,
            // instead of the generic "Subroutine" label which is misleading for accessors.
            if symbol_info.declaration.as_deref() == Some("has") {
                let accessor_name = &symbol_info.name;
                let doc = Self::format_moo_accessor_hover(accessor_name, &symbol_info.attributes);
                return HoverExtracted::Complete(json!({
                    "contents": {
                        "kind": "markdown",
                        "value": doc,
                    },
                }));
            }

            // Detect method modifier symbols (before/after/around/override/augment) early and render
            // a dedicated card instead of the generic "Subroutine" label.
            if let Some(modifier_kind) =
                symbol_info.attributes.iter().find_map(|a| a.strip_prefix("modifier="))
            {
                let method_name = &symbol_info.name;
                let kind_label = match modifier_kind {
                    "before" => "runs **before** the method — use for preconditions and logging",
                    "after" => "runs **after** the method — use for postconditions and cleanup",
                    "around" => {
                        "wraps the method — receives `$orig` as first arg, must call `$orig->($self, @_)`"
                    }
                    "override" => "overrides the parent method — use to replace inherited behavior",
                    "augment" => {
                        "extends the parent method — call `inner()` to invoke the next layer"
                    }
                    _ => "modifies the method",
                };
                let doc = symbol_info.documentation.as_deref().unwrap_or("");
                return HoverExtracted::Complete(json!({
                    "contents": {
                        "kind": "markdown",
                        "value": format!(
                            "**Method Modifier (`{modifier_kind}`)**\n\n`{method_name}` — {kind_label}\n\n{doc}"
                        ),
                    },
                }));
            }

            use crate::symbol::VarKind;
            let kind_str = match symbol_info.kind {
                crate::symbol::SymbolKind::Variable(VarKind::Scalar) => "Scalar Variable",
                crate::symbol::SymbolKind::Variable(VarKind::Array) => "Array Variable",
                crate::symbol::SymbolKind::Variable(VarKind::Hash) => "Hash Variable",
                crate::symbol::SymbolKind::Subroutine => "Subroutine",
                crate::symbol::SymbolKind::Method => "Method",
                crate::symbol::SymbolKind::Package => "Package",
                crate::symbol::SymbolKind::Constant => "Constant",
                crate::symbol::SymbolKind::Label => "Label",
                crate::symbol::SymbolKind::Format => "Format",
                _ => "Symbol",
            };

            let (display_name, complexity_info) = if matches!(
                symbol_info.kind,
                crate::symbol::SymbolKind::Subroutine | crate::symbol::SymbolKind::Method
            ) {
                let is_method = symbol_info.kind == crate::symbol::SymbolKind::Method;
                let prefix = if is_method { "method" } else { "sub" };
                let mut params = Vec::new();
                let mut complexity = String::new();
                if let Some(sub_node) = self.find_subroutine_definition(ast, &symbol_info.name) {
                    if let NodeKind::Subroutine { signature: sub_sig, body, .. } = &sub_node.kind {
                        if let Some(sig) = sub_sig {
                            if let NodeKind::Signature { parameters } = &sig.kind {
                                for param in parameters {
                                    self.extract_signature_params(param, &mut params);
                                }
                            }
                        } else {
                            self.extract_params_from_body(body, &mut params);
                        }
                    } else if let NodeKind::Method { signature: method_sig, .. } = &sub_node.kind {
                        if let Some(sig) = method_sig {
                            if let NodeKind::Signature { parameters } = &sig.kind {
                                for param in parameters {
                                    self.extract_signature_params(param, &mut params);
                                }
                            }
                        }
                    }
                    complexity = Self::build_complexity_info(sub_node, text);
                }
                let name = if params.is_empty() {
                    format!("{} {}", prefix, symbol_info.name)
                } else {
                    format!("{} {}({})", prefix, symbol_info.name, params.join(", "))
                };
                (name, complexity)
            } else {
                let sigil = symbol_info.kind.sigil().unwrap_or("");
                (format!("{}{}", sigil, symbol_info.name), String::new())
            };

            let decl_info = symbol_info
                .declaration
                .as_ref()
                .map(|d| format!("\n**Declaration**: `{}`", d))
                .unwrap_or_default();

            // For variables, show declaration line and scope context.
            let (decl_line_info, scope_context_info) = if symbol_info.kind.is_variable() {
                let decl_offset = symbol_info.location.start;
                let (line_0based, _col) = byte_to_line_col(text, decl_offset);
                let decl_line = format!("\n**Declared at**: line {}", line_0based + 1);
                let scope_ctx = Self::build_variable_scope_context(&analyzer, symbol_info);
                (decl_line, scope_ctx)
            } else {
                (String::new(), String::new())
            };

            // Check if this variable is tied — scan AST for a matching Tie node.
            let tied_info = if symbol_info.kind.is_variable() {
                let sigil = symbol_info.kind.sigil().unwrap_or("");
                Self::find_tied_class(ast, sigil, &symbol_info.name)
                    .map(|cls| format!("\n**Tied to**: `{}`", cls))
                    .unwrap_or_default()
            } else {
                String::new()
            };

            // Infer type for variables using TypeInferenceEngine
            let type_info = if symbol_info.kind.is_variable() {
                let var_name = &symbol_info.name; // already without sigil
                let mut type_engine = crate::type_inference::TypeInferenceEngine::new();
                let _ = type_engine.infer(ast); // ignore errors, just build env
                type_engine
                    .hover_label_for(var_name)
                    .filter(|label| label != "Any")
                    .map(|label| format!("\n**Type**: `{}`", label))
                    .unwrap_or_default()
            } else {
                String::new()
            };

            let attrs_info = if symbol_info.attributes.is_empty() {
                String::new()
            } else {
                format!("\n**Attributes**: {}", symbol_info.attributes.join(", "))
            };

            let complexity_section = if complexity_info.is_empty() {
                String::new()
            } else {
                format!("\n\n{}", complexity_info)
            };

            let doc_info = symbol_info
                .documentation
                .as_ref()
                .map(|d| format!("\n\n{}", d))
                .unwrap_or_default();

            return HoverExtracted::Complete(json!({
                "contents": {
                    "kind": "markdown",
                    "value": format!("**{}**\n\n`{}`{}{}{}{}{}{}{}{}",
                        kind_str,
                        display_name,
                        decl_info,
                        decl_line_info,
                        scope_context_info,
                        type_info,
                        tied_info,
                        attrs_info,
                        complexity_section,
                        doc_info
                    ),
                },
            }));
        }

        // Inherited method hover: cursor is on a `->method()` call but find_definition
        // found nothing in the current file.  Try the in-file class model first
        // (resolve_inherited_method_hover handles same-file parent/role chains), then
        // emit InheritedMethod for Phase 2 (workspace index BFS).
        #[cfg(feature = "workspace")]
        {
            if let Some(raw_receiver) = Self::extract_arrow_receiver(text, offset) {
                // Extract the method name token at the cursor
                let method_name = Self::get_token_at_position_static(text, offset);
                if !method_name.is_empty() && !method_name.starts_with(['$', '@', '%']) {
                    // Resolve receiver to a package name.
                    // `$self`, `$this`, `$class` map to current_package; bare identifiers
                    // starting with uppercase are treated as package names.
                    let bare_receiver =
                        raw_receiver.trim_start_matches(['$', '@', '%']).to_string();
                    let receiver_pkg = if bare_receiver == "self"
                        || bare_receiver == "this"
                        || bare_receiver == "class"
                    {
                        crate::declaration::current_package_at(ast, offset).to_string()
                    } else if bare_receiver.starts_with(|c: char| c.is_uppercase()) {
                        bare_receiver
                    } else {
                        // Variable receiver whose type we cannot statically resolve here.
                        // Phase 2 will not be called; fall through to token hover.
                        String::new()
                    };

                    if !receiver_pkg.is_empty() {
                        // Try in-file ancestors first (no workspace lock needed)
                        if let Some(hover_info) =
                            analyzer.resolve_inherited_method_hover(&receiver_pkg, &method_name)
                        {
                            let details = hover_info.details.join("\n");
                            return HoverExtracted::Complete(json!({
                                "contents": {
                                    "kind": "markdown",
                                    "value": format!(
                                        "**Method**\n\n`{}`\n\n{}",
                                        hover_info.signature,
                                        details
                                    ),
                                },
                            }));
                        }

                        // No in-file ancestor found — defer to Phase 2 workspace BFS
                        return HoverExtracted::InheritedMethod(
                            receiver_pkg,
                            method_name,
                            uri.to_string(),
                        );
                    }
                }
            }
        }

        Self::extract_token_hover(uri, text, offset)
    }

    /// Extract hover information from the token fallback path.
    fn extract_token_hover(uri: &str, text: &str, offset: usize) -> HoverExtracted {
        // Check if the cursor is inside a regex literal and provide explanation.
        if let Some(regex_hover) = Self::extract_regex_hover(text, offset) {
            return HoverExtracted::Complete(regex_hover);
        }

        // Check for special/punctuation variables (e.g. $!, $/, $$, $^W)
        // before falling back to the normal tokenizer which misses them.
        if let Some(special_var) = Self::extract_special_variable(text, offset) {
            if let Some(hover) = Self::get_special_variable_hover(&special_var) {
                return HoverExtracted::Complete(hover);
            }
        }

        // Handle file test operators (`-e`, `-f`, `-M`, etc.) before the
        // general token fallback, because the token scanner does not include
        // the leading `-`.
        if let Some(op) = Self::extract_file_test_operator(text, offset) {
            if let Some(op_doc) = crate::semantic::get_operator_documentation(&op) {
                return HoverExtracted::Complete(json!({
                    "contents": {
                        "kind": "markdown",
                        "value": format!(
                            "**File Test Operator**\n\n```\n{}\n```\n\n{}",
                            op_doc.signature,
                            op_doc.description
                        ),
                    },
                }));
            }
        }

        // Fall back to simple token display, with builtin docs.
        let hover_text = {
            // The normal tokenizer only captures `[$@%]` + alphanumeric/underscore,
            // so it misses punctuation variables handled above.
            Self::get_token_at_position_static(text, offset)
        };

        if !hover_text.is_empty() {
            // Check for special variable hover (handles $_, @_, @ISA, %ENV, etc.)
            if let Some(hover) = Self::get_special_variable_hover(&hover_text) {
                return HoverExtracted::Complete(hover);
            }

            let bare = hover_text.trim_start_matches(['$', '@', '%']);
            if let Some(builtin_doc) = crate::semantic::get_builtin_documentation(bare) {
                return HoverExtracted::Complete(json!({
                    "contents": {
                        "kind": "markdown",
                        "value": format!(
                            "**Built-in Function**\n\n```\n{}\n```\n\n{}",
                            builtin_doc.signature,
                            builtin_doc.description
                        ),
                    },
                }));
            }

            if let Some(xs_hover) = Self::extract_xs_api_hover(uri, text, offset) {
                return HoverExtracted::Complete(xs_hover);
            }

            // Check Test::More/Test2 function hover when source imports a test framework
            let is_test_source = text.contains("use Test::More") || text.contains("use Test2");
            if is_test_source {
                if let Some((sig, desc)) = crate::completion::get_test_more_documentation(bare) {
                    return HoverExtracted::Complete(json!({
                        "contents": {
                            "kind": "markdown",
                            "value": format!(
                                "**Test::More**\n\n```perl\n{}\n```\n\n{}",
                                sig, desc
                            ),
                        },
                    }));
                }
            }

            // Check DBI method hover: token preceded by -> in a DBI-importing file.
            // Guard on `use DBI` to avoid false positives for common method names like
            // `execute`, `fetch`, `rows`, `commit`, `rollback` in non-DBI code.
            let is_dbi_source = text.contains("use DBI") || text.contains("use DBIx");
            if is_dbi_source && !bare.is_empty() && !hover_text.starts_with(['$', '@', '%']) {
                if let Some(receiver) = Self::extract_arrow_receiver(text, offset) {
                    if let Some((sig, desc)) =
                        crate::completion::get_dbi_method_documentation(&receiver, bare)
                    {
                        return HoverExtracted::Complete(json!({
                            "contents": {
                                "kind": "markdown",
                                "value": format!(
                                    "**DBI Method**\n\n```perl\n{}\n```\n\n{}",
                                    sig, desc
                                ),
                            },
                        }));
                    }
                }
            }

            return HoverExtracted::Complete(json!({
                "contents": {
                    "kind": "markdown",
                    "value": format!("**Perl**: `{}`", hover_text),
                },
            }));
        }

        HoverExtracted::None
    }

    /// Build a scope context string for a variable hover card.
    ///
    /// Finds the innermost subroutine whose byte span contains the variable's
    /// declaration offset, and returns a formatted string like
    /// `\n**Scope**: lexical in subroutine `foo`` or `\n**Scope**: file scope`.
    fn build_variable_scope_context(
        analyzer: &crate::semantic::SemanticAnalyzer,
        symbol: &crate::symbol::Symbol,
    ) -> String {
        let decl_offset = symbol.location.start;
        let table = analyzer.symbol_table();

        // Find the innermost (smallest span) subroutine that contains decl_offset.
        let mut best_sub_name: Option<String> = None;
        let mut best_span = usize::MAX;

        for syms in table.symbols.values() {
            for sym in syms {
                if sym.kind == crate::symbol::SymbolKind::Subroutine
                    && sym.location.start < decl_offset
                    && sym.location.end > decl_offset
                {
                    let span = sym.location.end - sym.location.start;
                    if span < best_span {
                        best_sub_name = Some(sym.name.clone());
                        best_span = span;
                    }
                }
            }
        }

        if let Some(sub_name) = best_sub_name {
            format!("\n**Scope**: lexical in subroutine `{sub_name}`")
        } else {
            "\n**Scope**: file scope".to_string()
        }
    }

    fn format_moo_accessor_hover(name: &str, attributes: &[String]) -> String {
        let isa = Self::moo_attribute_value(attributes, "isa");
        let access = Self::moo_attribute_value(attributes, "is").map(Self::describe_access_mode);
        let required = Self::moo_attribute_value(attributes, "required").map(Self::describe_truthy);
        let predicate = Self::moo_accessor_method_name(name, attributes, "predicate", "has_");
        let builder = Self::moo_accessor_method_name(name, attributes, "builder", "_build_");
        let clearer = Self::moo_accessor_method_name(name, attributes, "clearer", "clear_");
        let reader = Self::moo_attribute_value(attributes, "reader");
        let writer = Self::moo_attribute_value(attributes, "writer");
        let accessor = Self::moo_attribute_value(attributes, "accessor");
        let lazy = Self::moo_attribute_value(attributes, "lazy").map(Self::describe_truthy);
        let default = Self::moo_attribute_value(attributes, "default");

        let mut lines = vec!["**Moo/Moose Attribute Accessor**".to_string(), String::new()];
        lines.push(format!("**Attribute**: `{name}`"));

        if let Some(isa) = isa {
            lines.push(format!("**Type**: `{isa}`"));
        }
        if let Some(access) = access {
            lines.push(format!("**Access**: {access}"));
        }
        if let Some(required) = required {
            lines.push(format!("**Required**: {required}"));
        }
        if let Some(predicate) = predicate {
            lines.push(format!("**Predicate**: `{predicate}`"));
        }
        if let Some(builder) = builder {
            lines.push(format!("**Builder**: `{builder}`"));
        }
        if let Some(clearer) = clearer {
            lines.push(format!("**Clearer**: `{clearer}`"));
        }
        if let Some(reader) = reader {
            lines.push(format!("**Reader**: `{reader}`"));
        }
        if let Some(writer) = writer {
            lines.push(format!("**Writer**: `{writer}`"));
        }
        if let Some(accessor) = accessor {
            lines.push(format!("**Accessor**: `{accessor}`"));
        }
        if let Some(lazy) = lazy {
            lines.push(format!("**Lazy**: {lazy}"));
        }
        if let Some(default) = default {
            lines.push(format!("**Default**: `{default}`"));
        }

        let extras: Vec<String> = attributes
            .iter()
            .filter_map(|attr| {
                let (key, _) = attr.split_once('=')?;
                if matches!(
                    key,
                    "isa"
                        | "is"
                        | "required"
                        | "predicate"
                        | "builder"
                        | "clearer"
                        | "reader"
                        | "writer"
                        | "accessor"
                        | "lazy"
                        | "default"
                ) {
                    None
                } else {
                    Some(attr.clone())
                }
            })
            .collect();
        if !extras.is_empty() {
            lines.push(format!("**Options**: {}", extras.join(", ")));
        }

        lines.join("\n")
    }

    fn moo_attribute_value<'a>(attributes: &'a [String], key: &str) -> Option<&'a str> {
        attributes.iter().find_map(|attr| {
            let (attr_key, value) = attr.split_once('=')?;
            if attr_key == key { Some(value) } else { None }
        })
    }

    fn describe_access_mode(value: &str) -> String {
        match value {
            "ro" => "read-only".to_string(),
            "rw" => "read-write".to_string(),
            "rwp" => "read-write private".to_string(),
            "lazy" => "lazy".to_string(),
            other => other.to_string(),
        }
    }

    fn describe_truthy(value: &str) -> String {
        match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" => "yes".to_string(),
            "0" | "false" | "no" => "no".to_string(),
            other => other.to_string(),
        }
    }

    fn moo_accessor_method_name(
        name: &str,
        attributes: &[String],
        key: &str,
        default_prefix: &str,
    ) -> Option<String> {
        let value = Self::moo_attribute_value(attributes, key)?;
        if Self::is_truthy(value) {
            Some(format!("{default_prefix}{name}"))
        } else {
            Some(value.to_string())
        }
    }

    fn is_truthy(value: &str) -> bool {
        matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes")
    }

    /// Get a token using the same simple fallback as rename, without requiring `&self`.
    fn get_token_at_position_static(content: &str, offset: usize) -> String {
        let chars: Vec<char> = content.chars().collect();
        if offset >= chars.len() {
            return String::new();
        }

        let mut start = offset;
        while start > 0
            && (chars[start - 1].is_alphanumeric()
                || chars[start - 1] == '_'
                || chars[start - 1] == '$'
                || chars[start - 1] == '@'
                || chars[start - 1] == '%')
        {
            start -= 1;
        }

        let mut end = offset;
        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
            end += 1;
        }

        chars[start..end].iter().collect()
    }

    fn extract_xs_api_hover(uri: &str, text: &str, offset: usize) -> Option<Value> {
        if !crate::completion::is_xs_source(text, Some(uri)) {
            return None;
        }

        let token = Self::get_token_at_position_static(text, offset);
        if token.is_empty() {
            return None;
        }

        let bare = token.trim_start_matches(['$', '@', '%']);
        let (sig, desc) = crate::completion::get_xs_api_documentation(bare)?;
        Some(json!({
            "contents": {
                "kind": "markdown",
                "value": format!(
                    "**XS / Perl C API**\n\n```c\n{}\n```\n\n{}",
                    sig, desc
                ),
            },
        }))
    }

    /// Extract the receiver token immediately before `->` at `offset`.
    ///
    /// Given `$dbh->prepare` with `offset` pointing anywhere in `prepare`,
    /// scans left to find `->` and returns the identifier/variable before it
    /// (e.g. `"$dbh"`). Returns `None` when there is no `->` before the token.
    ///
    /// Handles whitespace around `->`, e.g. `$dbh -> prepare`.
    fn extract_arrow_receiver(text: &str, offset: usize) -> Option<String> {
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        if len == 0 {
            return None;
        }

        // Walk to the start of the current token
        let mut tok_start = offset.min(len.saturating_sub(1));
        while tok_start > 0
            && (chars[tok_start - 1].is_alphanumeric() || chars[tok_start - 1] == '_')
        {
            tok_start -= 1;
        }

        // Skip whitespace before the token
        let mut i = tok_start.saturating_sub(1);
        while i > 0 && chars[i].is_whitespace() {
            i -= 1;
        }

        // Expect `>`
        if chars[i] != '>' {
            return None;
        }
        if i == 0 || chars[i - 1] != '-' {
            return None;
        }

        // Skip past `->`
        i = i.saturating_sub(2); // point before '-'
        while i > 0 && chars[i].is_whitespace() {
            i -= 1;
        }

        // Collect identifier/variable backwards (include sigil `$`)
        let rec_end = i + 1;
        while i > 0
            && (chars[i - 1].is_alphanumeric()
                || chars[i - 1] == '_'
                || chars[i - 1] == '$'
                || chars[i - 1] == ':')
        {
            i -= 1;
        }
        let rec: String = chars[i..rec_end].iter().collect();
        if rec.is_empty() { None } else { Some(rec) }
    }

    /// Walk the AST to find a `use Module` node whose location spans `offset`.
    fn find_use_module_at_offset(node: &Node, offset: usize) -> Option<String> {
        if offset < node.location.start || offset > node.location.end {
            return None;
        }

        if let NodeKind::Use { module, .. } = &node.kind {
            if !module.is_empty() {
                return Some(module.clone());
            }
        }

        // Recurse into container nodes
        match &node.kind {
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                for stmt in statements {
                    if let Some(m) = Self::find_use_module_at_offset(stmt, offset) {
                        return Some(m);
                    }
                }
            }
            NodeKind::Package { block, .. } => {
                if let Some(b) = block {
                    if let Some(m) = Self::find_use_module_at_offset(b, offset) {
                        return Some(m);
                    }
                }
            }
            NodeKind::PhaseBlock { block, .. } => {
                if let Some(m) = Self::find_use_module_at_offset(block, offset) {
                    return Some(m);
                }
            }
            _ => {}
        }

        None
    }

    /// Walk the AST to find a `with 'Role'` or `extends 'Parent'` name at `offset`.
    ///
    /// Handles two AST forms produced by the parser:
    ///
    /// 1. **FunctionCall form**: `ExpressionStatement { FunctionCall { name: "with"/"extends", args } }`
    ///    where args contains `String { value }` or `ArrayLiteral { elements: [String, ...] }`.
    ///
    /// 2. **Two-statement form**: consecutive `ExpressionStatement { Identifier { name: "with"/"extends" } }`
    ///    followed by `ExpressionStatement { String/ArrayLiteral }` within the same `Block`.
    ///
    /// Returns the role/parent module name only when `offset` falls within the **name string node**,
    /// not when the cursor is on the `with`/`extends` keyword itself.
    fn find_with_module_at_offset(node: &Node, offset: usize) -> Option<String> {
        // Recurse into container nodes, and handle with/extends patterns at Block level.
        // NOTE: We do NOT use the ExpressionStatement's outer location to gate entry because
        // the parser captures only the keyword span (e.g. "with" at 30-34) for the
        // ExpressionStatement, not the full statement including its arguments. We instead
        // walk into each ExpressionStatement unconditionally when looking for with/extends calls.
        match &node.kind {
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                for (idx, stmt) in statements.iter().enumerate() {
                    // FunctionCall form: `with 'Role'` or `with 'A', 'B'` parsed as a call.
                    // Check the inner FunctionCall's args directly — do NOT gate on the outer
                    // ExpressionStatement's location which only spans the keyword.
                    if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
                        if let NodeKind::FunctionCall { name, args } = &expression.kind {
                            if matches!(name.as_str(), "with" | "extends") {
                                for arg in args {
                                    if let Some(role) = Self::role_name_at_offset(arg, offset) {
                                        return Some(role);
                                    }
                                }
                            }
                        }
                    }

                    // Two-statement form: Identifier("with"/"extends") then String/ArrayLiteral
                    if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
                        if let NodeKind::Identifier { name } = &expression.kind {
                            if matches!(name.as_str(), "with" | "extends") {
                                if let Some(next) = statements.get(idx + 1) {
                                    if let NodeKind::ExpressionStatement { expression: next_expr } =
                                        &next.kind
                                    {
                                        if let Some(role) =
                                            Self::role_name_at_offset(next_expr, offset)
                                        {
                                            return Some(role);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Recurse deeper for nested blocks/packages
                    if let Some(m) = Self::find_with_module_at_offset(stmt, offset) {
                        return Some(m);
                    }
                }
            }
            NodeKind::Package { block, .. } => {
                if let Some(b) = block {
                    if let Some(m) = Self::find_with_module_at_offset(b, offset) {
                        return Some(m);
                    }
                }
            }
            NodeKind::PhaseBlock { block, .. } => {
                if let Some(m) = Self::find_with_module_at_offset(block, offset) {
                    return Some(m);
                }
            }
            _ => {}
        }

        None
    }

    /// Extract a role/module name from a node if `offset` falls within it.
    ///
    /// Handles `String { value }` (single role) and `ArrayLiteral { elements }`
    /// (multi-role `with 'A', 'B'`). Returns `None` if the offset is not within
    /// any string node in the argument.
    fn role_name_at_offset(node: &Node, offset: usize) -> Option<String> {
        match &node.kind {
            NodeKind::String { value, .. } => {
                if offset >= node.location.start && offset <= node.location.end {
                    let trimmed = value.trim().trim_matches('\'').trim_matches('"').trim();
                    if !trimmed.is_empty() {
                        return Some(trimmed.to_string());
                    }
                }
                None
            }
            NodeKind::ArrayLiteral { elements } => {
                for elem in elements {
                    if let Some(role) = Self::role_name_at_offset(elem, offset) {
                        return Some(role);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Build a hover response for an inherited or role-composed method call.
    ///
    /// Called in Phase 2 (outside document lock) when Phase 1 detected a `->method()`
    /// call but the method was not found in the current file's class models. Performs
    /// a BFS over the workspace index following the same parent/role chains as
    /// `inherited_method_definition_location` in navigation.rs.
    ///
    /// Returns `None` when no ancestor defines the method (hover falls through to token
    /// display).
    #[cfg(feature = "workspace")]
    fn build_inherited_method_hover(
        &self,
        receiver_pkg: &str,
        method_name: &str,
        _doc_uri: &str,
    ) -> Option<Value> {
        use std::collections::{HashMap, HashSet, VecDeque};

        let coord = self.coordinator()?;
        let workspace_index = coord.index();

        let mut visited = HashSet::from([receiver_pkg.to_string()]);
        let mut queue = VecDeque::new();
        let mut related_package_cache: HashMap<String, Vec<String>> = HashMap::new();

        let build_package_hover = |package_name: &str| -> Option<Value> {
            let members = workspace_index.get_package_members(package_name);
            if members.iter().any(|symbol| symbol.name == method_name) {
                let detail = if package_name == receiver_pkg {
                    format!("Defined in `{package_name}`")
                } else {
                    format!("Inherited from `{package_name}`")
                };
                return Some(json!({
                    "contents": {
                        "kind": "markdown",
                        "value": format!(
                            "**Method**\n\n`sub {}::{}`\n\n{}",
                            package_name, method_name, detail
                        ),
                    },
                }));
            }

            if members.iter().any(|symbol| symbol.name == "AUTOLOAD") {
                let detail = if package_name == receiver_pkg {
                    format!("Resolved via `AUTOLOAD` in `{package_name}`")
                } else {
                    format!("Resolved via inherited `AUTOLOAD` in `{package_name}`")
                };
                return Some(json!({
                    "contents": {
                        "kind": "markdown",
                        "value": format!(
                            "**Method**\n\n`sub {}::AUTOLOAD`\n\n{}\n\nRequested method: `{}`",
                            package_name, detail, method_name
                        ),
                    },
                }));
            }

            None
        };

        if let Some(hover) = build_package_hover(receiver_pkg) {
            return Some(hover);
        }

        // Inner closure: enqueue parent and role packages not yet visited.
        // Mirrors the logic in `inherited_method_definition_location` (navigation.rs)
        // but also includes model.roles so that composed roles are traversed.
        let mut enqueue_related =
            |package_name: &str, queue: &mut VecDeque<String>, visited: &HashSet<String>| {
                let related = related_package_cache
                    .entry(package_name.to_string())
                    .or_insert_with(|| {
                        use crate::semantic::SemanticAnalyzer;
                        let Some(package_location) = workspace_index.find_definition(package_name)
                        else {
                            return Vec::new();
                        };
                        let Some(text) = super::navigation::workspace_document_text(
                            workspace_index,
                            &package_location.uri,
                        ) else {
                            return Vec::new();
                        };

                        let mut parser = crate::Parser::new(&text);
                        let Ok(ast) = parser.parse() else {
                            return Vec::new();
                        };

                        SemanticAnalyzer::analyze_with_source(&ast, &text)
                            .class_models
                            .into_iter()
                            .find(|model| model.name == package_name)
                            .map(|model| {
                                model
                                    .parents
                                    .iter()
                                    .chain(model.roles.iter())
                                    .cloned()
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default()
                    })
                    .clone();

                for pkg in related {
                    if !visited.contains(&pkg) {
                        queue.push_back(pkg);
                    }
                }
            };

        enqueue_related(receiver_pkg, &mut queue, &visited);

        while let Some(package_name) = queue.pop_front() {
            if !visited.insert(package_name.clone()) {
                continue;
            }

            if let Some(hover) = build_package_hover(&package_name) {
                return Some(hover);
            }

            enqueue_related(&package_name, &mut queue, &visited);
        }

        None
    }

    /// Build a hover response for a `use Module` statement.
    ///
    /// Tries URI-based resolution first, then filesystem-based resolution.
    /// When a module file is found, extracts POD documentation and includes
    /// it in the hover display. Results are cached per file path.
    fn build_module_hover(&self, module_name: &str, doc_text: &str, doc_uri: &str) -> Value {
        // MetaCPAN link is included in every branch — compute once up front.
        let metacpan_link = format!("[View on MetaCPAN](https://metacpan.org/pod/{module_name})");

        // Try URI resolution (handles open docs + workspace folders)
        if let Some(uri) =
            self.resolve_module_to_path_with_doc(module_name, Some(doc_text), Some(doc_uri))
        {
            let display_path = uri.strip_prefix("file://").unwrap_or(&uri);
            let fs_path = url::Url::parse(&uri).ok().and_then(|u| u.to_file_path().ok());
            let pod_section =
                fs_path.as_deref().map(|p| self.format_pod_for_hover(p)).unwrap_or_default();
            return json!({
                "contents": {
                    "kind": "markdown",
                    "value": format!(
                        "**{module_name}**\n\n`{display_path}`\n\n[Go to module]({uri}) \u{2022} {metacpan_link}{pod_section}"
                    ),
                },
            });
        }

        // Try filesystem resolution as fallback
        if let Some(path) =
            self.resolve_module_path_with_uri(module_name, Some(doc_text), Some(doc_uri))
        {
            let pod_section = self.format_pod_for_hover(&path);
            let display = path.display().to_string();
            if let Ok(file_uri) = url::Url::from_file_path(&path) {
                return json!({
                    "contents": {
                        "kind": "markdown",
                        "value": format!(
                            "**{module_name}**\n\n`{display}`\n\n[Go to module]({file_uri}) \u{2022} {metacpan_link}{pod_section}"
                        ),
                    },
                });
            }
            return json!({
                "contents": {
                    "kind": "markdown",
                    "value": format!(
                        "**{module_name}**\n\n`{display}`\n\n{metacpan_link}{pod_section}"
                    ),
                },
            });
        }

        // Not found — show search paths and MetaCPAN link
        let include_paths = self
            .config_for_doc(doc_uri)
            .unwrap_or_else(|| self.workspace_config.lock().clone())
            .include_paths
            .join(", ");

        json!({
            "contents": {
                "kind": "markdown",
                "value": format!(
                    "**{}**\n\nNot found in workspace\n\nSearch paths: {}\n\n{}",
                    module_name, include_paths, metacpan_link
                ),
            },
        })
    }

    /// Build a hover response for a known Perl pragma (e.g. `strict`, `warnings`).
    ///
    /// Returns `Some(Value)` when `module_name` is a recognized pragma with inline
    /// documentation, or `None` when it should fall through to regular module resolution.
    fn build_pragma_hover(module_name: &str) -> Option<Value> {
        let doc = crate::semantic::get_pragma_documentation(module_name)?;

        let version_line =
            doc.version_required.map(|v| format!("\n\n**Requires**: Perl {v}")).unwrap_or_default();

        let perldoc_link =
            format!("[perldoc {module_name}](https://perldoc.perl.org/{module_name})");

        Some(json!({
            "contents": {
                "kind": "markdown",
                "value": format!(
                    "**Pragma: `{module_name}`**\n\n_{summary}_\n\n{description}{version_line}\n\n{perldoc_link}",
                    summary = doc.summary,
                    description = doc.description,
                ),
            },
        }))
    }

    /// Extract POD documentation from a module file and format it for hover display.
    ///
    /// Uses a per-path cache to avoid re-parsing on every hover request.
    /// Returns an empty string if no POD is found or the file cannot be read.
    fn format_pod_for_hover(&self, path: &Path) -> String {
        let pod = {
            let mut cache = self.pod_cache.lock();
            if let Some(cached) = cache.get(path) {
                cached.clone()
            } else {
                let doc = perl_pod::extract_pod_from_file(path).unwrap_or_default();
                cache.insert(path.to_path_buf(), doc.clone());
                doc
            }
        };

        if pod.is_empty() {
            return String::new();
        }

        let mut parts = Vec::new();

        if let Some(ref synopsis) = pod.synopsis {
            parts.push(format!("## Synopsis\n\n```perl\n{synopsis}\n```"));
        }

        if let Some(ref description) = pod.description {
            parts.push(format!("## Description\n\n{description}"));
        }

        if parts.is_empty() {
            return String::new();
        }

        format!("\n\n---\n\n{}", parts.join("\n\n"))
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
                                        "activeSignature": 0,
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
            NodeKind::Method { name: method_name, .. } if method_name == name => {
                return Some(node);
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
    fn find_tied_class(node: &Node, sigil: &str, var_name: &str) -> Option<String> {
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
    fn extract_signature_params(&self, params_node: &Node, params: &mut Vec<String>) {
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

    /// Build a complexity summary string for a subroutine node.
    fn build_complexity_info(node: &Node, text: &str) -> String {
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

    /// Extract a special/punctuation variable name at the given byte offset.
    ///
    /// The normal tokenizer (`get_token_at_position`) only captures `[$@%]` +
    /// alphanumeric/underscore, so it misses punctuation variables like `$!`,
    /// `$/`, `$$`, and caret variables like `$^W`.  This function handles those.
    fn extract_special_variable(text: &str, offset: usize) -> Option<String> {
        let bytes = text.as_bytes();
        let len = bytes.len();
        if offset >= len {
            return None;
        }

        // Find the sigil: either at offset or one position before
        let sigil_pos = if matches!(bytes[offset], b'$' | b'@' | b'%') {
            Some(offset)
        } else if offset > 0 && matches!(bytes[offset - 1], b'$' | b'@' | b'%') {
            Some(offset - 1)
        } else {
            None
        };
        let sigil_pos = sigil_pos?;
        let sigil = bytes[sigil_pos] as char;
        let next_pos = sigil_pos + 1;
        if next_pos >= len {
            return None;
        }
        let next_ch = bytes[next_pos];

        // $^X pattern (caret variables like $^W, $^O)
        if sigil == '$' && next_ch == b'^' && next_pos + 1 < len {
            let caret_ch = bytes[next_pos + 1];
            if caret_ch.is_ascii_alphabetic() {
                return Some(format!("$^{}", caret_ch as char));
            }
        }

        // Internal Perl values used by XS/C code, e.g. $PL_sv_yes.
        if sigil == '$' && bytes[next_pos..].starts_with(b"PL_sv_") {
            let mut end = next_pos + "PL_sv_".len();
            while end < len && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
                end += 1;
            }
            return Some(text[sigil_pos..end].to_string());
        }

        // Single punctuation character after $ (e.g. $!, $?, $/, $\, $$, $;, etc.)
        if sigil == '$' && !next_ch.is_ascii_alphanumeric() && next_ch != b'_' {
            let punct = next_ch as char;
            if matches!(
                punct,
                '!' | '@' | '?' | '/' | '\\' | '$' | ';' | ',' | '.' | '&' | '\'' | '`' | '+' | '|'
            ) {
                return Some(format!("${}", punct));
            }
        }

        None
    }

    /// Extract a file test operator at the given byte offset.
    ///
    /// Recognizes operators like `-e`, `-f`, and `-M` when the cursor is on
    /// either the `-` or the operator letter.
    fn extract_file_test_operator(text: &str, offset: usize) -> Option<String> {
        let bytes = text.as_bytes();
        if bytes.is_empty() || offset >= bytes.len() {
            return None;
        }

        for start in [offset, offset.saturating_sub(1)] {
            if bytes.get(start) != Some(&b'-') {
                continue;
            }

            if let Some(op_char) = bytes.get(start + 1) {
                let op = format!("-{}", *op_char as char);
                if crate::semantic::SemanticAnalyzer::is_file_test_operator(&op) {
                    return Some(op);
                }
            }
        }

        None
    }

    /// Return educational hover documentation for Perl special variables.
    ///
    /// Covers the common special variables every Perl developer encounters,
    /// plus a few internal `PL_sv_*` constants used by XS/C code. Returns a
    /// JSON hover response with markdown content, or `None` if the variable is
    /// not in the known set.
    fn get_internal_special_variable_hover(name: &str) -> Option<Value> {
        let (heading, description) = match name {
            "$PL_sv_yes" | "PL_sv_yes" => (
                "Internal Special Variable",
                "The canonical true scalar used by Perl internals and XS/C code. It is an immutable shared value, so extensions can return or compare against it without allocating a fresh true scalar.",
            ),
            "$PL_sv_no" | "PL_sv_no" => (
                "Internal Special Variable",
                "The canonical false scalar used by Perl internals and XS/C code. It is an immutable shared value representing Perl's shared false value.",
            ),
            "$PL_sv_undef" | "PL_sv_undef" => (
                "Internal Special Variable",
                "The canonical undefined scalar used by Perl internals and XS/C code. It represents Perl's shared `undef` value.",
            ),
            _ => return None,
        };

        Some(json!({
            "contents": {
                "kind": "markdown",
                "value": format!(
                    "**`{name}` \u{2014} {heading}**\n\n{description}\n\n```perl\n# XS/C internals typically treat this as a shared value\n```"
                ),
            },
        }))
    }

    fn get_special_variable_hover(name: &str) -> Option<Value> {
        if let Some(hover) = Self::get_internal_special_variable_hover(name) {
            return Some(hover);
        }

        // Handle $1-$9 capture group variables with dynamic content.
        if let Some(digit) = name
            .strip_prefix('$')
            .filter(|s| s.len() == 1 && matches!(s.as_bytes().first(), Some(b'1'..=b'9')))
        {
            let n: u8 = digit.as_bytes()[0] - b'0';
            let desc = format!(
                "**`${n}` \u{2014} Regex Capture Group {n}**\n\n\
                 Contains the text matched by the {n}{ord} set of parentheses in the \
                 last successful regex match.  Only valid until the next regex match \
                 or the end of the enclosing scope.\n\n\
                 ```perl\n\"2024-03-15\" =~ /(\\d{{4}})-(\\d{{2}})-(\\d{{2}})/;\
                 \nprint $1;  # \"2024\"  (capture group 1)\n```",
                ord = match n {
                    1 => "st",
                    2 => "nd",
                    3 => "rd",
                    _ => "th",
                }
            );
            return Some(json!({
                "contents": {
                    "kind": "markdown",
                    "value": desc,
                },
            }));
        }

        let text: &str = match name {
            "$_" => {
                "**`$_` \u{2014} The Default Variable**\n\n\
                 Used implicitly by many builtins: `foreach`, `map`, `grep`, \
                 `print`, `chomp`, and more.  The \"it\" of Perl.\n\n\
                 ```perl\nfor (@items) {\n    print;  # prints $_\n}\n```"
            }
            "@_" => {
                "**`@_` \u{2014} Subroutine Arguments**\n\n\
                 Contains all arguments passed to the current function. \
                 Use `shift`, `pop`, or list assignment to unpack.\n\n\
                 ```perl\nsub greet {\n    my ($name) = @_;\n\
                     print \"Hello, $name\\n\";\n}\n```"
            }
            "$!" => {
                "**`$!` \u{2014} OS Error (errno)**\n\n\
                 In numeric context returns the current `errno` value. \
                 In string context returns the corresponding system error \
                 message (like `strerror`).\n\n\
                 ```perl\nopen my $fh, '<', $file\n    or die \"Cannot open $file: $!\";\n```"
            }
            "$@" => {
                "**`$@` \u{2014} Eval Error**\n\n\
                 Set to the error message when `eval { }` or `eval EXPR` \
                 catches an exception. Empty string when no error occurred.\n\n\
                 ```perl\neval { risky_operation() };\nif ($@) {\n    warn \"Caught: $@\";\n}\n```"
            }
            "$/" => {
                "**`$/` \u{2014} Input Record Separator**\n\n\
                 Controls what constitutes a \"line\" when reading from a \
                 filehandle.  Defaults to `\\n`.  Set to `undef` to slurp \
                 an entire file at once.\n\n\
                 ```perl\nlocal $/;  # enable slurp mode\nmy $content = <$fh>;\n```"
            }
            "$\\" => {
                "**`$\\` \u{2014} Output Record Separator**\n\n\
                 Appended after every `print` statement.  Defaults to empty \
                 string (no separator).\n\n\
                 ```perl\nlocal $\\ = \"\\n\";\nprint \"first\";  # prints \"first\\n\"\n```"
            }
            "$$" => {
                "**`$$` \u{2014} Process ID**\n\n\
                 The PID of the currently running Perl process.  Read-only.\n\n\
                 ```perl\nprint \"PID: $$\\n\";\n```"
            }
            "$0" => {
                "**`$0` \u{2014} Program Name**\n\n\
                 Contains the name of the script being executed.  Assigning \
                 to it changes the process name visible in `ps`.\n\n\
                 ```perl\nprint \"Running: $0\\n\";\n$0 = \"my-daemon\";\n```"
            }
            "$;" => {
                "**`$;` \u{2014} Subscript Separator**\n\n\
                 Used in emulating multidimensional hashes: \
                 `$hash{$a,$b}` is really `$hash{join($;, $a, $b)}`. \
                 Defaults to `\\034` (SUBSEP).\n\n\
                 ```perl\n$h{\"x\",\"y\"} = 1;  # key is \"x\\034y\"\n```"
            }
            "$," => {
                "**`$,` \u{2014} Output Field Separator**\n\n\
                 Inserted between arguments in a `print` list.  Defaults \
                 to empty string.\n\n\
                 ```perl\nlocal $, = \", \";\nprint \"a\", \"b\", \"c\";  # a, b, c\n```"
            }
            "$." => {
                "**`$.` \u{2014} Current Line Number**\n\n\
                 The line number of the last line read from the most \
                 recently accessed filehandle.\n\n\
                 ```perl\nwhile (<$fh>) {\n    print \"Line $.: $_\";\n}\n```"
            }
            "$&" => {
                "**`$&` \u{2014} Matched String**\n\n\
                 Contains the text matched by the last successful pattern \
                 match.  Using it anywhere in a program imposes a performance \
                 penalty on all regexes (mitigated in Perl 5.20+).\n\n\
                 ```perl\n\"Hello World\" =~ /Wo\\w+/;\nprint $&;  # \"World\"\n```"
            }
            "$'" => {
                "**`$'` \u{2014} Postmatch String**\n\n\
                 Contains the string following the last successful pattern \
                 match.\n\n\
                 ```perl\n\"Hello World\" =~ /\\s/;\nprint $';  # \"World\"\n```"
            }
            "$`" => {
                "**`$\\`` \u{2014} Prematch String**\n\n\
                 Contains the string preceding the last successful pattern \
                 match.\n\n\
                 ```perl\n\"Hello World\" =~ /\\s/;\nprint $`;  # \"Hello\"\n```"
            }
            "$+" => {
                "**`$+` \u{2014} Last Bracket Matched**\n\n\
                 Contains the last bracket (capture group) that actually matched \
                 in the last successful regex. Useful when alternation makes it \
                 unknown which branch matched.\n\n\
                 ```perl\n\"1999-12-31\" =~ /(\\d{4})-(\\d{2})-(\\d{2})/;\nprint $+;  # \"31\" (last group)\n```"
            }
            "@ISA" => {
                "**`@ISA` \u{2014} Inheritance List**\n\n\
                 Defines the parent classes for method resolution. Perl \
                 searches `@ISA` (depth-first by default, C3 with `use mro \
                 'c3'`) when a method is not found in the current package.\n\n\
                 ```perl\npackage Dog;\nour @ISA = ('Animal');\n```"
            }
            "%ENV" => {
                "**`%ENV` \u{2014} Environment Variables**\n\n\
                 Hash containing the current environment variables. Changes \
                 to `%ENV` are inherited by child processes.\n\n\
                 ```perl\nmy $home = $ENV{HOME};\n$ENV{PATH} .= \":/opt/bin\";\n```"
            }
            "@INC" => {
                "**`@INC` \u{2014} Module Search Paths**\n\n\
                 List of directories (and code refs) searched when loading \
                 modules via `use` or `require`.  Modify with `use lib` or \
                 `PERL5LIB`.  Note: `.` was removed from `@INC` in Perl 5.26.\n\n\
                 ```perl\nuse lib '/my/modules';\nprint join(\"\\n\", @INC);\n```"
            }
            "%INC" => {
                "**`%INC` \u{2014} Loaded Modules**\n\n\
                 Records every file loaded by `use`, `require`, or `do`. \
                 Keys are the module filenames (e.g. `Foo/Bar.pm`), values \
                 are the full paths.\n\n\
                 ```perl\nuse Data::Dumper;\nprint $INC{'Data/Dumper.pm'};\n```"
            }
            "$^W" => {
                "**`$^W` \u{2014} Warning Flag**\n\n\
                 Global flag that enables or disables warnings at runtime. \
                 Prefer `use warnings` for lexical scoping.\n\n\
                 ```perl\nlocal $^W = 1;  # enable warnings temporarily\n```"
            }
            "$^O" => {
                "**`$^O` \u{2014} Operating System Name**\n\n\
                 Contains the OS name the Perl binary was built for \
                 (e.g. `linux`, `darwin`, `MSWin32`).  Useful for \
                 platform-specific code paths.\n\n\
                 ```perl\nif ($^O eq 'MSWin32') {\n    # Windows-specific\n}\n```"
            }
            "$?" => {
                "**`$?` \u{2014} Child Process Status**\n\n\
                 Set after `system()`, backtick execution (`` ` ` ``), `wait()`, \
                 or `waitpid()`. The value is the raw wait status: the exit code \
                 is `$? >> 8` and the signal number (if any) is `$? & 127`.\n\n\
                 ```perl\nsystem('ls');\nif ($? == -1) {\n    warn \"fork failed: $!\";\n} elsif ($? >> 8) {\n    warn \"exit status: \", $? >> 8;\n}\n```"
            }
            "$^V" => {
                "**`$^V` \u{2014} Perl Version**\n\n\
                 The Perl interpreter version as a v-string (e.g. `v5.38.0`). \
                 Use `use v5.10;` syntax for version requirements or compare \
                 with `$^V ge v5.10.0`.\n\n\
                 ```perl\nprint \"Perl \", $^V, \"\\n\";  # e.g. Perl v5.38.0\n```"
            }
            "@ARGV" => {
                "**`@ARGV` \u{2014} Command-Line Arguments**\n\n\
                 Contains the command-line arguments passed to the script \
                 (not including the script name, which is in `$0`). \
                 `shift` without arguments removes and returns the first element.\n\n\
                 ```perl\nmy $file = shift @ARGV // die \"Usage: $0 <file>\\n\";\n```"
            }
            "%SIG" => {
                "**`%SIG` \u{2014} Signal Handlers**\n\n\
                 Hash mapping signal names to handler code refs (or `'IGNORE'` / \
                 `'DEFAULT'`). Use `local %SIG` to temporarily override handlers.\n\n\
                 ```perl\n$SIG{INT}  = sub { print \"Interrupted\\n\"; exit 1 };\n$SIG{TERM} = 'IGNORE';\n```"
            }
            "$^A" => {
                "**`$^A` \u{2014} Accumulator for `format()`**\n\n\
                 The write accumulator for `format()` and `write()` output. \
                 Normally you do not access this directly; the `formline()` \
                 builtin writes into it and `write()` flushes it to the \
                 current output filehandle.\n\n\
                 ```perl\nformline(\"@<<<\", \"hi\");\nprint $^A;  # \"hi \"\n```"
            }
            "$^T" => {
                "**`$^T` \u{2014} Script Start Time**\n\n\
                 The time (in seconds since the epoch, like `time()`) at which \
                 the script began running. Used for age calculations relative to \
                 script startup and for the `-M`, `-A`, `-C` file-test operators.\n\n\
                 ```perl\nprint \"Running for \", time() - $^T, \" seconds\\n\";\n```"
            }
            "$|" => {
                "**`$|` \u{2014} Output Autoflush**\n\n\
                 If set to a non-zero value, Perl flushes the output buffer of the \
                 currently selected filehandle after every `print` or `write`. \
                 Set to `1` to enable autoflush (useful for real-time progress output \
                 or when writing to pipes).\n\n\
                 ```perl\n$| = 1;  # enable autoflush on STDOUT\nprint \"Progress: 50%\\n\";\n```"
            }
            "__FILE__" => {
                "**`__FILE__`** \u{2014} Compile-time constant: the current source file name"
            }
            "__LINE__" => "**`__LINE__`** \u{2014} Compile-time constant: the current line number",
            "__PACKAGE__" => {
                "**`__PACKAGE__`** \u{2014} Compile-time constant: the current package name \
                 (`\"main\"` at top level; `undef` inside `package BLOCK` with no name)"
            }
            "__SUB__" => {
                "**`__SUB__`** \u{2014} Compile-time constant: a reference to the current \
                 subroutine (Perl 5.16+, requires `use feature 'current_sub'`)"
            }
            _ => return None,
        };

        Some(json!({
            "contents": {
                "kind": "markdown",
                "value": text,
            },
        }))
    }

    /// Build a hover response when the cursor is inside a Perl regex literal.
    ///
    /// Detects `/pattern/`, `m/pattern/`, `s/pattern/repl/`, and `qr/pattern/`
    /// operators (including paired-delimiter variants) and returns a Markdown
    /// table explaining each metacharacter in the pattern.
    fn extract_regex_hover(text: &str, offset: usize) -> Option<Value> {
        let pattern = Self::find_regex_at_offset(text, offset)?;
        let entries = Self::explain_regex(&pattern);
        if entries.is_empty() {
            return None;
        }

        let mut table = "**Regex Pattern**\n\n".to_string();
        table.push_str("| Token | Meaning |\n|-------|-------|\n");
        for (tok, desc) in &entries {
            table.push_str(&format!("| `{}` | {} |\n", tok, desc));
        }

        Some(json!({
            "contents": {
                "kind": "markdown",
                "value": table,
            },
        }))
    }

    /// Return the inner pattern string if `offset` falls inside a regex literal.
    fn find_regex_at_offset(text: &str, offset: usize) -> Option<String> {
        // Find which line contains the offset and compute the column within it.
        let mut line_start = 0usize;
        for line in text.split('\n') {
            let line_end = line_start + line.len();
            if offset <= line_end {
                let col = offset - line_start;
                return Self::find_regex_in_line(line, col);
            }
            line_start = line_end + 1; // +1 for the '\n'
        }
        None
    }

    /// Scan `line` for Perl regex operators and return the inner pattern if
    /// `col` (0-based byte index into `line`) falls inside the pattern.
    fn find_regex_in_line(line: &str, col: usize) -> Option<String> {
        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut i = 0usize;

        while i < len {
            // --- bare /pattern/ ---
            if bytes[i] == b'/' {
                // Reject division: preceded by alphanumeric, `_`, `)`, `]`, `}`, `'`, `"`
                // e.g. `$x / 2` or `$hash{key}/2` should not trigger regex detection.
                let is_division = i > 0 && {
                    let prev = bytes[i - 1];
                    prev.is_ascii_alphanumeric()
                        || prev == b'_'
                        || prev == b')'
                        || prev == b']'
                        || prev == b'}'
                        || prev == b'\''
                        || prev == b'"'
                };
                if !is_division {
                    let open_delim = i;
                    let pattern_start = open_delim + 1;
                    let mut j = pattern_start;
                    while j < len {
                        if bytes[j] == b'\\' {
                            j += 2;
                            continue;
                        }
                        if bytes[j] == b'/' {
                            // col inside [pattern_start, j)?
                            if col >= pattern_start && col < j {
                                return Some(line[pattern_start..j].to_string());
                            }
                            i = j + 1;
                            break;
                        }
                        j += 1;
                    }
                    if j >= len {
                        // unterminated regex — skip
                        break;
                    }
                    continue;
                }
            }

            // --- m/.../, m{...}, m(...), m[...], m<...> ---
            // --- qr/.../, qr{...}, etc. ---
            // --- s/.../.../, s{...}{...}, etc. ---
            if i + 1 < len {
                let is_m = bytes[i] == b'm';
                let is_qr = bytes[i] == b'q' && i + 2 < len && bytes[i + 1] == b'r';
                let is_s = bytes[i] == b's';

                // Operator must be followed by a non-word character to avoid
                // matching variable names / identifiers like `$str`, `some`.
                let op_end = if is_qr { i + 2 } else { i + 1 };
                let delim_pos = op_end;

                if (is_m || is_qr || is_s)
                    && delim_pos < len
                    && !bytes[delim_pos].is_ascii_alphanumeric()
                    && bytes[delim_pos] != b'_'
                    // also make sure the operator itself starts a token
                    && (i == 0
                        || (!bytes[i - 1].is_ascii_alphanumeric()
                            && bytes[i - 1] != b'_'))
                {
                    let open = bytes[delim_pos];
                    let close = Self::matching_close(open);
                    let paired = open != close;
                    let pattern_start = delim_pos + 1;
                    let mut j = pattern_start;
                    let mut depth = 1usize;
                    while j < len {
                        if bytes[j] == b'\\' {
                            j += 2;
                            continue;
                        }
                        if paired {
                            if bytes[j] == open {
                                depth += 1;
                            } else if bytes[j] == close {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                        } else if bytes[j] == close {
                            break;
                        }
                        j += 1;
                    }
                    if j <= len && col >= pattern_start && col < j {
                        return Some(line[pattern_start..j].to_string());
                    }
                    i = j + 1;
                    continue;
                }
            }

            i += 1;
        }
        None
    }

    /// For paired delimiters return the matching close; otherwise return `open`.
    fn matching_close(open: u8) -> u8 {
        match open {
            b'{' => b'}',
            b'(' => b')',
            b'[' => b']',
            b'<' => b'>',
            other => other,
        }
    }

    /// Walk `pattern` and return `(token, description)` pairs for each
    /// recognisable metacharacter or metacharacter sequence.
    fn explain_regex(pattern: &str) -> Vec<(String, String)> {
        let bytes = pattern.as_bytes();
        let len = bytes.len();
        let mut entries: Vec<(String, String)> = Vec::new();
        let mut i = 0usize;

        while i < len {
            let b = bytes[i];

            match b {
                b'\\' if i + 1 < len => {
                    let next = bytes[i + 1];
                    // Handle \p{...} and \P{...} Unicode property escapes.
                    if (next == b'p' || next == b'P') && i + 2 < len && bytes[i + 2] == b'{' {
                        let prop_start = i + 3;
                        let mut k = prop_start;
                        while k < len && bytes[k] != b'}' {
                            k += 1;
                        }
                        let prop_end = if k < len { k + 1 } else { k };
                        let prop_str = pattern.get(i..prop_end).unwrap_or(if next == b'p' {
                            r"\p{}"
                        } else {
                            r"\P{}"
                        });
                        let desc = if next == b'p' {
                            "Unicode property — matches characters with this property"
                        } else {
                            "Unicode property complement — matches characters WITHOUT this property"
                        };
                        i = prop_end;
                        let prop_owned = prop_str.to_string();
                        let (final_tok, final_desc) =
                            Self::apply_quantifier(prop_owned, desc.to_string(), bytes, &mut i);
                        entries.push((final_tok, final_desc));
                        continue;
                    }
                    // Handle \N{name} — named Unicode character.
                    if next == b'N' && i + 2 < len && bytes[i + 2] == b'{' {
                        let name_start = i + 3;
                        let mut k = name_start;
                        while k < len && bytes[k] != b'}' {
                            k += 1;
                        }
                        let name_end = if k < len { k + 1 } else { k };
                        let name_str = pattern.get(i..name_end).unwrap_or(r"\N{}");
                        i = name_end;
                        let name_owned = name_str.to_string();
                        entries.push((name_owned, "Named Unicode character".to_string()));
                        continue;
                    }
                    // Handle \g{name} and \g{n} — named/numbered backreference.
                    if next == b'g' && i + 2 < len && bytes[i + 2] == b'{' {
                        let ref_start = i + 3;
                        let mut k = ref_start;
                        while k < len && bytes[k] != b'}' {
                            k += 1;
                        }
                        let ref_end = if k < len { k + 1 } else { k };
                        let ref_str = pattern.get(i..ref_end).unwrap_or(r"\g{}");
                        i = ref_end;
                        let ref_owned = ref_str.to_string();
                        entries.push((ref_owned, "Named or numbered backreference".to_string()));
                        continue;
                    }
                    // Handle \k<name> — named backreference (angle-bracket form).
                    if next == b'k' && i + 2 < len && bytes[i + 2] == b'<' {
                        let name_start = i + 3;
                        let mut k = name_start;
                        while k < len && bytes[k] != b'>' {
                            k += 1;
                        }
                        let name_end = if k < len { k + 1 } else { k };
                        let ref_str = pattern.get(i..name_end).unwrap_or(r"\k<>");
                        i = name_end;
                        let ref_owned = ref_str.to_string();
                        entries.push((
                            ref_owned,
                            "Named backreference (angle-bracket form)".to_string(),
                        ));
                        continue;
                    }
                    let (tok, desc) = match next {
                        b'd' => (r"\d", "Any decimal digit (0-9)"),
                        b'D' => (r"\D", "Any non-digit character"),
                        b'w' => (r"\w", "Any word character (alphanumeric + underscore)"),
                        b'W' => (r"\W", "Any non-word character"),
                        b's' => (r"\s", "Any whitespace character (space, tab, newline, etc.)"),
                        b'S' => (r"\S", "Any non-whitespace character"),
                        b'b' => (r"\b", "Word boundary"),
                        b'B' => (r"\B", "Non-word boundary"),
                        b'A' => (r"\A", "Start of string (unaffected by multiline mode)"),
                        b'Z' => (r"\Z", "End of string (allows optional trailing newline)"),
                        b'z' => (r"\z", "Absolute end of string"),
                        b'G' => (r"\G", "Where the previous match left off (pos())"),
                        b'n' => (r"\n", "Newline character"),
                        b't' => (r"\t", "Tab character"),
                        b'r' => (r"\r", "Carriage return character"),
                        b'f' => (r"\f", "Form feed character"),
                        b'e' => (r"\e", "Escape character"),
                        b'a' => (r"\a", "Alarm (bell) character"),
                        b'0' => (r"\0", "Null character"),
                        b'h' => (r"\h", "Horizontal whitespace (space or tab)"),
                        b'H' => (r"\H", "Non-horizontal-whitespace character"),
                        b'v' => (r"\v", "Vertical whitespace character"),
                        b'V' => (r"\V", "Non-vertical-whitespace character"),
                        b'X' => (r"\X", "Extended Unicode grapheme cluster"),
                        b'1'..=b'9' => {
                            let n = (next - b'0') as usize;
                            let tok_s = format!("\\{}", n);
                            let desc_s = format!("Backreference to capture group {}", n);
                            i += 2;
                            let (final_tok, final_desc) =
                                Self::apply_quantifier(tok_s, desc_s, bytes, &mut i);
                            entries.push((final_tok, final_desc));
                            continue;
                        }
                        _ => {
                            // Escaped literal or unrecognised — skip silently.
                            i += 2;
                            continue;
                        }
                    };
                    let tok_s = tok.to_string();
                    let desc_s = desc.to_string();
                    i += 2;
                    let (final_tok, final_desc) =
                        Self::apply_quantifier(tok_s, desc_s, bytes, &mut i);
                    entries.push((final_tok, final_desc));
                }
                b'^' => {
                    // Outside a character class (handled separately by `[`), `^`
                    // is always an anchor — at position 0 it anchors to the start
                    // of the string/line, and after `(?` it can appear inside
                    // alternatives such as `(?:^foo|^bar)`.
                    i += 1;
                    entries.push((r"^".to_string(), "Start of string/line anchor".to_string()));
                }
                b'$' => {
                    i += 1;
                    entries.push((r"$".to_string(), "End of string/line anchor".to_string()));
                }
                b'.' => {
                    let tok_s = ".".to_string();
                    let desc_s = "Any character except newline".to_string();
                    i += 1;
                    let (final_tok, final_desc) =
                        Self::apply_quantifier(tok_s, desc_s, bytes, &mut i);
                    entries.push((final_tok, final_desc));
                }
                b'(' => {
                    // Check for non-capturing group, lookaround, or named capture.
                    if i + 2 < len && bytes[i + 1] == b'?' {
                        let kind = bytes[i + 2];
                        let (prefix, desc, advance) = match kind {
                            b':' => ("(?:", "Non-capturing group", 3),
                            b'=' => ("(?=", "Positive lookahead assertion", 3),
                            b'!' => ("(?!", "Negative lookahead assertion", 3),
                            b'<' if i + 3 < len && bytes[i + 3] == b'=' => {
                                ("(?<=", "Positive lookbehind assertion", 4)
                            }
                            b'<' if i + 3 < len && bytes[i + 3] == b'!' => {
                                ("(?<!", "Negative lookbehind assertion", 4)
                            }
                            b'<' => ("(?<name>", "Named capture group (angle-bracket form)", 3),
                            b'\'' => ("(?'name'", "Named capture group (single-quote form)", 3),
                            b'#' => ("(?#", "Comment — ignored by the regex engine", 3),
                            b'|' => (
                                "(?|",
                                "Branch reset group — resets capture numbering per branch",
                                3,
                            ),
                            b'>' => ("(?>", "Atomic group (no backtracking into this group)", 3),
                            _ => ("(?", "Special group (inline modifier or other extension)", 2),
                        };
                        // Advance past the full group-open prefix so we don't
                        // re-process `?`, `:`, `=`, etc. as standalone tokens.
                        i += advance;
                        entries.push((prefix.to_string(), desc.to_string()));
                    } else {
                        i += 1;
                        entries.push((
                            "(".to_string(),
                            "Capture group — captures matched text".to_string(),
                        ));
                    }
                }
                b'[' => {
                    // Collect up to the closing `]` to show the class
                    let start = i;
                    i += 1;
                    if i < len && bytes[i] == b'^' {
                        i += 1;
                    }
                    while i < len && bytes[i] != b']' {
                        if bytes[i] == b'\\' {
                            i += 1;
                        }
                        i += 1;
                    }
                    let end = if i < len { i + 1 } else { i };
                    let cls = pattern[start..end.min(len)].to_string();
                    let desc = if cls.starts_with("[^") {
                        "Negated character class"
                    } else {
                        "Character class"
                    };
                    i = end;
                    let (final_tok, final_desc) =
                        Self::apply_quantifier(cls, desc.to_string(), bytes, &mut i);
                    entries.push((final_tok, final_desc));
                }
                b'+' => {
                    i += 1;
                    entries.push(("+".to_string(), "Quantifier: one or more".to_string()));
                }
                b'*' => {
                    i += 1;
                    entries.push(("*".to_string(), "Quantifier: zero or more".to_string()));
                }
                b'?' => {
                    i += 1;
                    entries
                        .push(("?".to_string(), "Quantifier: zero or one (optional)".to_string()));
                }
                b'|' => {
                    i += 1;
                    entries.push(("| ".to_string(), "Alternation (OR)".to_string()));
                }
                _ => {
                    i += 1;
                }
            }
        }

        entries
    }

    /// If the next byte(s) in `bytes` starting at `*pos` are a quantifier
    /// (`+`, `*`, `?`, `{n,m}`), consume them and fold into the description.
    fn apply_quantifier(
        tok: String,
        desc: String,
        bytes: &[u8],
        pos: &mut usize,
    ) -> (String, String) {
        let len = bytes.len();
        if *pos >= len {
            return (tok, desc);
        }
        let suffix = match bytes[*pos] {
            b'+' => {
                *pos += 1;
                if *pos < len && bytes[*pos] == b'+' {
                    // `++` possessive quantifier — no backtracking
                    *pos += 1;
                    ", one or more (possessive — no backtracking)"
                } else if *pos < len && bytes[*pos] == b'?' {
                    // `+?` lazy quantifier — matches as few as possible
                    *pos += 1;
                    ", one or more (lazy — matches as few as possible)"
                } else {
                    ", one or more (greedy)"
                }
            }
            b'*' => {
                *pos += 1;
                if *pos < len && bytes[*pos] == b'?' {
                    // `*?` lazy quantifier
                    *pos += 1;
                    ", zero or more (lazy — matches as few as possible)"
                } else if *pos < len && bytes[*pos] == b'+' {
                    // `*+` possessive quantifier
                    *pos += 1;
                    ", zero or more (possessive — no backtracking)"
                } else {
                    ", zero or more (greedy)"
                }
            }
            b'?' => {
                *pos += 1;
                if *pos < len && bytes[*pos] == b'?' {
                    // `??` lazy optional
                    *pos += 1;
                    ", zero or one (lazy — prefers zero)"
                } else {
                    ", zero or one (optional, greedy)"
                }
            }
            b'{' => {
                // {n} or {n,m} — collect the quantifier text and fold it in.
                let brace_start = *pos;
                *pos += 1;
                while *pos < len && bytes[*pos] != b'}' {
                    *pos += 1;
                }
                if *pos < len {
                    *pos += 1; // consume '}'
                }
                let brace_end = *pos;
                // Check for lazy ({n,m}?) or possessive ({n,m}+) suffix.
                let counted_suffix = if *pos < len && bytes[*pos] == b'?' {
                    *pos += 1;
                    ", counted repetition (lazy)"
                } else if *pos < len && bytes[*pos] == b'+' {
                    *pos += 1;
                    ", counted repetition (possessive)"
                } else {
                    ", counted repetition"
                };
                // All bytes in {…} are ASCII so from_utf8 is infallible here.
                let range = std::str::from_utf8(&bytes[brace_start..brace_end]).unwrap_or("{n}");
                return (format!("{}{}", tok, range), format!("{}{}", desc, counted_suffix));
            }
            _ => return (tok, desc),
        };
        (tok, format!("{}{}", desc, suffix))
    }
}
