//! Hover and signature help handlers
//!
//! Provides hover information and function signature help for Perl code.

use super::super::*;
use crate::cancellation::RequestCleanupGuard;
use crate::protocol::{req_position, req_uri};
mod hover_cards;
mod hover_extracted;
#[cfg(test)]
mod hover_tests;
mod live_compiler_hover;
mod regex_hover;
mod signature_help;

use hover_extracted::HoverExtracted;

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
            let (extracted, live_compiler_context) = {
                let documents = self.documents_guard();
                if let Some(doc) = self.get_document(&documents, uri) {
                    let offset = self.pos16_to_offset(doc, line, character);
                    let live_compiler_context =
                        Self::live_hover_compiler_context(uri, &doc.text, offset);
                    if let Some(ast) = &doc.ast {
                        // Check for `use Module` at this offset first
                        let extracted = if let Some(module_name) =
                            Self::find_use_module_at_offset(ast, offset)
                        {
                            // If the module is a known pragma, return pragma docs immediately
                            // without doing module file resolution.
                            if let Some(pragma_hover) = Self::build_pragma_hover(&module_name) {
                                HoverExtracted::Complete(pragma_hover)
                            } else {
                                HoverExtracted::UseModule(
                                    module_name,
                                    doc.text.clone(),
                                    uri.to_string(),
                                    offset,
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
                                offset,
                            )
                        } else {
                            self.extract_symbol_hover(uri, ast, &doc.text, offset)
                        };
                        (extracted, live_compiler_context)
                    } else {
                        (Self::extract_token_hover(uri, &doc.text, offset), live_compiler_context)
                    }
                } else {
                    (HoverExtracted::None, None)
                }
            };
            // Document lock released here

            // Phase 2: Resolve module or return pre-built hover
            match extracted {
                HoverExtracted::Complete(value) => {
                    if let Some(compiler_hover) =
                        self.try_live_compiler_hover(Some(&value), live_compiler_context.as_ref())
                    {
                        return Ok(Some(compiler_hover));
                    }
                    return Ok(Some(value));
                }
                HoverExtracted::UseModule(module_name, doc_text, doc_uri, doc_offset) => {
                    return Ok(Some(self.build_module_hover(
                        &module_name,
                        &doc_text,
                        &doc_uri,
                        Some(doc_offset),
                    )));
                }
                HoverExtracted::PossiblePackage(pkg_name, doc_text, doc_uri, doc_offset) => {
                    // Package names with `::` always get module hover (file path + MetaCPAN link).
                    // For unresolved names this still shows the MetaCPAN link which is more
                    // useful than the bare-token fallback.
                    return Ok(Some(self.build_module_hover(
                        &pkg_name,
                        &doc_text,
                        &doc_uri,
                        Some(doc_offset),
                    )));
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
                HoverExtracted::None => {
                    if let Some(compiler_hover) =
                        self.try_live_compiler_hover(None, live_compiler_context.as_ref())
                    {
                        return Ok(Some(compiler_hover));
                    }
                }
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
            let doc = symbol_info.documentation.as_deref().unwrap_or("");
            return HoverExtracted::Complete(hover_cards::method_modifier_hover(
                modifier_kind,
                method_name,
                doc,
            ));
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
                let doc = symbol_info.documentation.as_deref().unwrap_or("");
                return HoverExtracted::Complete(hover_cards::method_modifier_hover(
                    modifier_kind,
                    method_name,
                    doc,
                ));
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
                    let receiver_pkg =
                        Self::resolve_receiver_package_name(ast, offset, &raw_receiver);

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

            // Before the bare-token fallback, check if the cursor is on a package name
            // (an identifier that spans `::` separators, e.g. `File::Path`, `DBI`).
            // Defer resolution to Phase 2 via `build_module_hover` so no workspace lock
            // is held here.
            if let Some(pkg_name) = Self::get_package_name_at_position(text, offset) {
                // Namespaced builtins (e.g. utf8::encode, utf8::decode) must take
                // priority over module resolution: there is no utf8/encode.pm to
                // find, and the correct response is the builtin hover card.
                if let Some(builtin_doc) = crate::semantic::get_builtin_documentation(&pkg_name) {
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
                return HoverExtracted::PossiblePackage(
                    pkg_name,
                    text.to_string(),
                    uri.to_string(),
                    offset,
                );
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

    fn resolve_receiver_package_name(ast: &Node, offset: usize, raw_receiver: &str) -> String {
        let bare_receiver = raw_receiver.trim_start_matches(['$', '@', '%']);
        if bare_receiver == "self" || bare_receiver == "this" || bare_receiver == "class" {
            return crate::declaration::current_package_at(ast, offset).to_string();
        }

        if bare_receiver.starts_with(|ch: char| ch.is_uppercase()) {
            return bare_receiver.to_string();
        }

        // Variable receiver whose type we cannot statically resolve here.
        // Phase 2 will not be called; fall through to token hover.
        String::new()
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

    /// Extract a package name at `offset`, spanning `::` separators.
    ///
    /// Returns `Some(name)` only when the extracted token contains `::`,
    /// indicating it is a qualified package name (e.g. `File::Path`, `Foo::Bar::Baz`).
    /// Returns `None` for bare single-component identifiers to avoid misidentifying
    /// function names or variables.
    fn get_package_name_at_position(text: &str, offset: usize) -> Option<String> {
        let bytes = text.as_bytes();
        let len = bytes.len();
        if offset >= len {
            return None;
        }

        // Scan left over alphanumeric, underscore, and `::` sequences.
        let mut start = offset;
        while start > 0 {
            let prev = start - 1;
            if bytes[prev].is_ascii_alphanumeric() || bytes[prev] == b'_' {
                start -= 1;
            } else if prev >= 1 && bytes[prev] == b':' && bytes[prev - 1] == b':' {
                start -= 2;
            } else {
                break;
            }
        }

        // Scan right over alphanumeric, underscore, and `::` sequences.
        let mut end = offset;
        while end < len {
            if bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_' {
                end += 1;
            } else if end + 1 < len && bytes[end] == b':' && bytes[end + 1] == b':' {
                end += 2;
            } else {
                break;
            }
        }

        // Trim any trailing `::` (e.g. cursor right after the separator).
        let candidate = text[start..end].trim_end_matches(':');
        if candidate.contains("::") { Some(candidate.to_string()) } else { None }
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
    fn build_module_hover(
        &self,
        module_name: &str,
        doc_text: &str,
        doc_uri: &str,
        doc_offset: Option<usize>,
    ) -> Value {
        // MetaCPAN link is included in every branch — compute once up front.
        let metacpan_link = format!("[View on MetaCPAN](https://metacpan.org/pod/{module_name})");
        let perldoc_virtual_link = Self::perldoc_virtual_link(module_name);
        let docs_links = format!("{metacpan_link} \u{2022} {perldoc_virtual_link}");

        // Try URI resolution (handles open docs + workspace folders)
        if let Some(uri) = self.resolve_module_to_path_with_doc_at_offset(
            module_name,
            Some(doc_text),
            Some(doc_uri),
            doc_offset,
        ) {
            let display_path = uri.strip_prefix("file://").unwrap_or(&uri);
            let fs_path = url::Url::parse(&uri).ok().and_then(|u| u.to_file_path().ok());
            let pod_section =
                fs_path.as_deref().map(|p| self.format_pod_for_hover(p)).unwrap_or_default();
            return json!({
                "contents": {
                    "kind": "markdown",
                    "value": format!(
                        "**{module_name}**\n\n`{display_path}`\n\n[Go to module]({uri}) \u{2022} {docs_links}{pod_section}"
                    ),
                },
            });
        }

        // Try filesystem resolution as a legacy fallback only when the caller
        // has no position. Position-aware callers must not bypass active
        // `@INC` state such as `no lib` cancellation.
        if doc_offset.is_none()
            && let Some(path) =
                self.resolve_module_path_with_uri(module_name, Some(doc_text), Some(doc_uri))
        {
            let pod_section = self.format_pod_for_hover(&path);
            let display = path.display().to_string();
            if let Ok(file_uri) = url::Url::from_file_path(&path) {
                return json!({
                    "contents": {
                        "kind": "markdown",
                        "value": format!(
                            "**{module_name}**\n\n`{display}`\n\n[Go to module]({file_uri}) \u{2022} {docs_links}{pod_section}"
                        ),
                    },
                });
            }
            return json!({
                "contents": {
                    "kind": "markdown",
                    "value": format!(
                        "**{module_name}**\n\n`{display}`\n\n{docs_links}{pod_section}"
                    ),
                },
            });
        }

        // Not found — explain exactly which configured roots were considered and
        // give the user a next step instead of a bare "not found" card.
        let config =
            self.config_for_doc(doc_uri).unwrap_or_else(|| self.workspace_config.lock().clone());
        let perl5lib_paths = std::env::var("PERL5LIB")
            .map(|value| perl_lsp_rs_core::config::WorkspaceConfig::parse_perl5lib(&value))
            .unwrap_or_default();
        let include_paths = config.effective_include_paths(&perl5lib_paths);
        let searched_paths = Self::format_missing_module_search_paths(&include_paths);
        let system_inc_status = if config.use_system_inc { "enabled" } else { "disabled" };

        json!({
            "contents": {
                "kind": "markdown",
                "value": format!(
                    "**{module_name}**

Not found in workspace or configured include paths.

**Searched paths**:
{searched_paths}

**System `@INC`**: {system_inc_status}

**Next steps**: install `{module_name}` (for example, `cpanm {module_name}`) or add the directory that contains it to `.perl-lsp.toml` `include_paths`.

{docs_links}"
                ),
            },
        })
    }

    fn perldoc_virtual_link(module_name: &str) -> String {
        format!("[Open virtual perldoc](perldoc://{module_name})")
    }

    fn format_missing_module_search_paths(include_paths: &[String]) -> String {
        if include_paths.is_empty() {
            return "- No include paths configured".to_string();
        }

        include_paths.iter().map(|path| format!("- `{path}`")).collect::<Vec<_>>().join("\n")
    }

    /// Build a hover response for a known Perl pragma (e.g. `strict`, `warnings`).
    ///
    /// Returns `Some(Value)` when `module_name` is a recognized pragma with inline
    /// documentation, or `None` when it should fall through to regular module resolution.
    fn build_pragma_hover(module_name: &str) -> Option<Value> {
        let doc = crate::semantic::get_pragma_documentation(module_name)?;

        let version_line =
            doc.version_required.map(|v| format!("\n\n**Requires**: Perl {v}")).unwrap_or_default();

        let perldoc_web_link =
            format!("[perldoc {module_name}](https://perldoc.perl.org/{module_name})");
        let perldoc_virtual_link = Self::perldoc_virtual_link(module_name);
        let perldoc_links = format!("{perldoc_web_link} | {perldoc_virtual_link}");

        Some(json!({
            "contents": {
                "kind": "markdown",
                "value": format!(
                    "**Pragma: `{module_name}`**\n\n_{summary}_\n\n{description}{version_line}\n\n{perldoc_links}",
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
        // Soft cap on pod_cache size. The cache is keyed by filesystem path
        // (not the open document), so it accumulates entries for every module
        // ever hovered during the session — the open/close lifecycle does not
        // shrink it. Without this cap a session that hovers many modules grows
        // unboundedly. When the cap is reached we drain to half capacity using
        // an arbitrary-victim policy (HashMap iteration order); precision is
        // unimportant — re-extracting POD is cheap.
        const POD_CACHE_SOFT_CAP: usize = 1024;
        const POD_CACHE_PRUNE_TARGET: usize = 512;

        let pod = {
            let mut cache = self.pod_cache.lock();
            if let Some(cached) = cache.get(path) {
                cached.clone()
            } else {
                if cache.len() >= POD_CACHE_SOFT_CAP {
                    let drop_count = cache.len().saturating_sub(POD_CACHE_PRUNE_TARGET);
                    let mut dropped = 0usize;
                    cache.retain(|_, _| {
                        if dropped < drop_count {
                            dropped += 1;
                            false
                        } else {
                            true
                        }
                    });
                }
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
        // Convert raw Value ID to typed ID at the boundary.
        let typed_id = request_id.and_then(JsonRpcId::try_from_value);
        // RAII guard ensures cleanup on all exit paths (early returns, errors, panics)
        let _cleanup_guard = RequestCleanupGuard::from_ref(typed_id.as_ref());

        if let Some(params) = params {
            // Create or get cancellation token for this request
            if let Some(ref tid) = typed_id {
                let token = GLOBAL_CANCELLATION_REGISTRY.get_token(tid).unwrap_or_else(|| {
                    let token = PerlLspCancellationToken::new(
                        tid.clone(),
                        "textDocument/hover".to_string(),
                    );
                    let _ = GLOBAL_CANCELLATION_REGISTRY.register_token(token.clone());
                    token
                });

                // Early cancellation check with relaxed read
                if token.is_cancelled_relaxed() {
                    return Err(JsonRpcError {
                        code: REQUEST_CANCELLED,
                        message: "Request cancelled - hover provider".to_string(),
                        data: None,
                    });
                }
            }

            // Delegate to original handler
            self.handle_hover(Some(params))
        } else {
            self.handle_hover(params)
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
}
