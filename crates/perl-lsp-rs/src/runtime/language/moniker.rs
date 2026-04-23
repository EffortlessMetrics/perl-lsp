//! Moniker language feature handler and symbol classification helpers

use super::super::*;
use crate::protocol::{req_position, req_uri};
use perl_module::import::resolve_known_export_tag;

impl LspServer {
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

                        // For subroutines in packages with base/parent,
                        // add monikers pointing to potential parent definitions
                        if key.kind == crate::workspace_index::SymKind::Sub {
                            for parent_pkg in Self::find_base_parents(ast) {
                                let parent_id =
                                    format!("{}.{}", parent_pkg.replace("::", "."), key.name);
                                monikers.push(json!({
                                    "scheme": "perl",
                                    "identifier": parent_id,
                                    "unique": "global",
                                    "kind": "local"
                                }));
                            }
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
        // Check if symbol is exported via @EXPORT or @EXPORT_OK (AST-first, regex fallback)
        let uses_exporter = Self::has_use_exporter(ast);
        let is_exported =
            self.is_symbol_exported_ast(ast, &key.name) || self.is_symbol_exported(text, &key.name);

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
                } else if uses_exporter && key.pkg.as_ref() != "main" {
                    // Module uses Exporter — subs are at least project-visible
                    "project"
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

    /// Check if the AST contains `use Exporter` (or `use parent 'Exporter'`)
    fn has_use_exporter(ast: &crate::ast::Node) -> bool {
        use perl_parser::ast::NodeKind;

        fn check(node: &crate::ast::Node) -> bool {
            match &node.kind {
                NodeKind::Use { module, .. } if module == "Exporter" => true,
                NodeKind::Program { statements } | NodeKind::Block { statements } => {
                    statements.iter().any(check)
                }
                _ => false,
            }
        }
        check(ast)
    }

    /// AST-based export detection: walk Assignment nodes to find
    /// `@EXPORT = (...)` or `@EXPORT_OK = (...)` containing the symbol.
    fn is_symbol_exported_ast(&self, ast: &crate::ast::Node, symbol_name: &str) -> bool {
        use perl_parser::ast::NodeKind;

        fn check(node: &crate::ast::Node, name: &str) -> bool {
            match &node.kind {
                NodeKind::Assignment { lhs, rhs, .. } => {
                    // Check if lhs is @EXPORT or @EXPORT_OK
                    let is_export_var = match &lhs.kind {
                        NodeKind::Variable { name: var_name, sigil } => {
                            sigil.starts_with('@')
                                && (var_name == "EXPORT" || var_name == "EXPORT_OK")
                        }
                        _ => false,
                    };
                    if is_export_var {
                        // Search rhs for the symbol name in string/identifier nodes
                        return contains_symbol_name(rhs, name);
                    }
                    // Recurse into lhs/rhs for nested assignments
                    check(lhs, name) || check(rhs, name)
                }
                NodeKind::Program { statements } | NodeKind::Block { statements } => {
                    statements.iter().any(|s| check(s, name))
                }
                NodeKind::Subroutine { body, .. } => check(body, name),
                NodeKind::ExpressionStatement { expression } => check(expression, name),
                _ => false,
            }
        }

        fn contains_symbol_name(node: &crate::ast::Node, name: &str) -> bool {
            match &node.kind {
                NodeKind::String { value, .. } => {
                    // Check if the string contains the symbol name as a word
                    value.split_whitespace().any(|w| w == name)
                }
                NodeKind::Identifier { name: id } => id == name,
                NodeKind::ArrayLiteral { elements } => {
                    elements.iter().any(|e| contains_symbol_name(e, name))
                }
                _ => {
                    let mut found = false;
                    node.for_each_child(|child| {
                        if !found && contains_symbol_name(child, name) {
                            found = true;
                        }
                    });
                    found
                }
            }
        }

        check(ast, symbol_name)
    }

    /// Detect `use base 'Foo'` or `use parent 'Foo'` and return parent packages
    fn find_base_parents(ast: &crate::ast::Node) -> Vec<String> {
        use perl_parser::ast::NodeKind;

        fn collect(node: &crate::ast::Node, out: &mut Vec<String>) {
            match &node.kind {
                NodeKind::Use { module, args, .. } if module == "base" || module == "parent" => {
                    for arg in args {
                        // Handle qw(...) style: "qw(Foo::Bar Baz::Qux)"
                        if arg.starts_with("qw") {
                            let content = arg
                                .trim_start_matches("qw")
                                .trim_start_matches(|c: char| "([{/<|!".contains(c))
                                .trim_end_matches(|c: char| ")]}/|!>".contains(c));
                            for parent in content.split_whitespace() {
                                if !parent.is_empty() {
                                    out.push(parent.to_string());
                                }
                            }
                        } else if !arg.starts_with('-') && !arg.starts_with("qw") {
                            // Bare string arg: use base 'Foo::Bar'
                            let cleaned = arg.trim_matches(|c: char| c == '\'' || c == '"');
                            if !cleaned.is_empty() {
                                out.push(cleaned.to_string());
                            }
                        }
                    }
                }
                NodeKind::Program { statements } | NodeKind::Block { statements } => {
                    for stmt in statements {
                        collect(stmt, out);
                    }
                }
                _ => {}
            }
        }

        let mut parents = Vec::new();
        collect(ast, &mut parents);
        parents
    }

    /// Check if a symbol name appears in @EXPORT or @EXPORT_OK (regex fallback)
    fn is_symbol_exported(&self, text: &str, symbol_name: &str) -> bool {
        use std::sync::OnceLock;

        static EXPORT_QW_RE: OnceLock<Option<regex::Regex>> = OnceLock::new();
        static EXPORT_ARRAY_RE: OnceLock<Option<regex::Regex>> = OnceLock::new();

        let export_re = EXPORT_QW_RE.get_or_init(|| {
            regex::Regex::new(r"@EXPORT(?:_OK)?\s*=\s*qw[(\[{/<|!]([^)\]}/|!>]+)[)\]}/|!>]").ok()
        });

        if let Some(re) = export_re {
            for cap in re.captures_iter(text) {
                if let Some(content) = cap.get(1) {
                    if content.as_str().split_whitespace().any(|w| w == symbol_name) {
                        return true;
                    }
                }
            }
        }

        let array_re = EXPORT_ARRAY_RE
            .get_or_init(|| regex::Regex::new(r"@EXPORT(?:_OK)?\s*=\s*\(([^)]+)\)").ok());
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
        self.find_import_source(ast, symbol_name).is_some()
    }

    /// Find the source module for an imported symbol
    ///
    /// Searches `use` statements for the symbol name, handling both bare imports
    /// and `qw<...>` style import lists with all delimiter types.
    pub(super) fn find_import_source(
        &self,
        ast: &crate::ast::Node,
        symbol_name: &str,
    ) -> Option<String> {
        use perl_parser::ast::NodeKind;

        fn require_module_name(node: &crate::ast::Node) -> Option<String> {
            let args = match &node.kind {
                NodeKind::FunctionCall { name, args } if name == "require" => args,
                _ => return None,
            };
            let arg = args.first()?;
            match &arg.kind {
                NodeKind::Identifier { name } => Some(name.clone()),
                NodeKind::String { value, .. } => {
                    let cleaned = value.trim_matches('\'').trim_matches('"').trim();
                    Some(cleaned.trim_end_matches(".pm").replace('/', "::"))
                }
                _ => None,
            }
        }

        fn module_runtime_alias(expr: &crate::ast::Node) -> Option<(String, String)> {
            let (alias_name, call_node) = match &expr.kind {
                NodeKind::Assignment { lhs, rhs, op } if op == "=" => {
                    let NodeKind::Variable { name, .. } = &lhs.kind else {
                        return None;
                    };
                    (name.as_str(), rhs.as_ref())
                }
                NodeKind::VariableDeclaration { variable, initializer: Some(rhs), .. } => {
                    let NodeKind::Variable { name, .. } = &variable.kind else {
                        return None;
                    };
                    (name.as_str(), rhs.as_ref())
                }
                _ => return None,
            };
            let NodeKind::FunctionCall { name, args } = &call_node.kind else {
                return None;
            };
            if !matches!(
                name.as_str(),
                "use_module"
                    | "require_module"
                    | "Module::Runtime::use_module"
                    | "Module::Runtime::require_module"
            ) {
                return None;
            }
            let first = args.first()?;
            let NodeKind::String { value, .. } = &first.kind else {
                return None;
            };
            let module = value.trim_matches('\'').trim_matches('"').trim();
            if module.is_empty() {
                return None;
            }
            Some((alias_name.to_string(), module.to_string()))
        }

        fn arg_matches_symbol(module: &str, arg: &crate::ast::Node, symbol: &str) -> bool {
            match &arg.kind {
                NodeKind::String { value, .. } => {
                    let bare = value.trim_matches('\'').trim_matches('"').trim();
                    bare == symbol
                        || (bare.starts_with(':')
                            && resolve_known_export_tag(module, bare)
                                .is_some_and(|expanded| expanded.contains(&symbol)))
                }
                NodeKind::Identifier { name } => {
                    if name == symbol {
                        return true;
                    }
                    if name.starts_with("qw") {
                        let content = name
                            .trim_start_matches("qw")
                            .trim_start_matches(|c: char| "([{/<|!".contains(c))
                            .trim_end_matches(|c: char| ")]}/|!>".contains(c));
                        return content.split_whitespace().any(|word| {
                            word == symbol
                                || (word.starts_with(':')
                                    && resolve_known_export_tag(module, word)
                                        .is_some_and(|expanded| expanded.contains(&symbol)))
                        });
                    }
                    false
                }
                NodeKind::ArrayLiteral { elements } => {
                    elements.iter().any(|el| arg_matches_symbol(module, el, symbol))
                }
                _ => false,
            }
        }

        fn import_call_exports(
            expr: &crate::ast::Node,
            module: &str,
            symbol: &str,
            aliases: &std::collections::HashMap<String, String>,
        ) -> bool {
            let NodeKind::MethodCall { object, method, args } = &expr.kind else {
                return false;
            };
            if method != "import" {
                return false;
            }
            let object_name = match &object.kind {
                NodeKind::Identifier { name } => Some(name.as_str()),
                NodeKind::Variable { name, .. } => aliases.get(name).map(String::as_str),
                _ => return false,
            };
            let Some(object_name) = object_name else {
                return false;
            };
            if object_name != module {
                return false;
            }
            if args.is_empty() {
                return true;
            }
            args.iter().any(|arg| arg_matches_symbol(module, arg, symbol))
        }

        fn inner_expr(node: &crate::ast::Node) -> &crate::ast::Node {
            if let NodeKind::ExpressionStatement { expression } = &node.kind {
                expression.as_ref()
            } else {
                node
            }
        }

        fn find(node: &crate::ast::Node, name: &str) -> Option<String> {
            match &node.kind {
                NodeKind::Use { module, args, .. } => {
                    for arg in args {
                        if arg == name {
                            return Some(module.clone());
                        }
                        if arg.starts_with("qw") {
                            // Support all qw delimiters: (), [], {}, <>, //, ||, !!
                            let content = arg
                                .trim_start_matches("qw")
                                .trim_start_matches(|c: char| "([{/<|!".contains(c))
                                .trim_end_matches(|c: char| ")]}/|!>".contains(c));
                            for word in content.split_whitespace() {
                                if word == name {
                                    return Some(module.clone());
                                }
                                if word.starts_with(':')
                                    && let Some(expanded) = resolve_known_export_tag(module, word)
                                    && expanded.contains(&name)
                                {
                                    return Some(module.clone());
                                }
                            }
                        } else if arg.starts_with(':')
                            && let Some(expanded) = resolve_known_export_tag(module, arg)
                            && expanded.contains(&name)
                        {
                            return Some(module.clone());
                        }
                    }
                }
                NodeKind::Program { statements } | NodeKind::Block { statements } => {
                    let mut required_modules: Vec<String> = statements
                        .iter()
                        .filter_map(|stmt| require_module_name(inner_expr(stmt)))
                        .collect();
                    let mut aliases: std::collections::HashMap<String, String> =
                        std::collections::HashMap::new();
                    for stmt in statements {
                        if let Some((alias, module)) = module_runtime_alias(inner_expr(stmt)) {
                            aliases.insert(alias, module.clone());
                            if !required_modules.contains(&module) {
                                required_modules.push(module);
                            }
                        }
                    }
                    for stmt in statements {
                        let expr = inner_expr(stmt);
                        for module in &required_modules {
                            if import_call_exports(expr, module, name, &aliases) {
                                return Some(module.clone());
                            }
                        }
                    }
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
        use perl_parser::ast::NodeKind;

        fn check(node: &crate::ast::Node, name: &str, sigil: Option<char>) -> bool {
            match &node.kind {
                NodeKind::VariableDeclaration { declarator, variable, .. }
                    if declarator == "our" =>
                {
                    if let NodeKind::Variable { name: n, sigil: s } = &variable.kind {
                        if n == name {
                            return match sigil {
                                None => true,
                                Some(sig) => s.starts_with(sig),
                            };
                        }
                    }
                }
                NodeKind::VariableListDeclaration { declarator, variables, .. }
                    if declarator == "our" =>
                {
                    for var in variables {
                        if let NodeKind::Variable { name: n, sigil: s } = &var.kind {
                            if n == name {
                                return match sigil {
                                    None => true,
                                    Some(sig) => s.starts_with(sig),
                                };
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
}
