use super::ImportMap;
use perl_module::import::resolve_known_export_tag;
use perl_parser_core::ast::{Node, NodeKind};
use std::collections::{HashMap, HashSet};

/// Walk the top-level AST and build an `ImportMap` from `use` statements.
///
/// Only uppercase-starting module names are included (skips pragmas like
/// `strict`, `warnings`, `feature`, `constant`, `utf8`, `lib`, `parent`, `base`).
pub(super) fn extract_import_map(ast: &Node) -> ImportMap {
    let mut map: ImportMap = HashMap::new();

    fn collect_import_symbols(
        module: &str,
        arg: &str,
        symbols: &mut HashSet<String>,
    ) -> (bool, bool) {
        let trimmed = arg.trim();
        if trimmed.is_empty() {
            return (false, false);
        }
        if matches!(trimmed, "=>" | "," | "(" | ")" | "[" | "]" | "{" | "}") {
            return (false, false);
        }

        let mut content = trimmed;
        if let Some(inner) = content.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            content = inner.trim();
        }

        if content.starts_with("qw") {
            content = content
                .trim_start_matches("qw")
                .trim_start_matches(|c: char| "([{/<|!".contains(c))
                .trim_end_matches(|c: char| ")]}/|!>".contains(c))
                .trim();

            let mut unresolved_tag = false;
            for word in content.split_whitespace() {
                if word.is_empty() {
                    continue;
                }
                if word.starts_with(':') {
                    if let Some(expanded) = resolve_known_export_tag(module, word) {
                        symbols.extend(expanded.iter().map(|name| (*name).to_string()));
                    } else {
                        unresolved_tag = true;
                    }
                } else {
                    symbols.insert(word.to_string());
                }
            }
            return (!content.is_empty(), unresolved_tag);
        }

        let cleaned = content.trim_matches(|c: char| c == '\'' || c == '"');
        if cleaned.is_empty() {
            return (false, false);
        }

        let mut unresolved_tag = false;
        for word in cleaned.split_whitespace() {
            if word.is_empty() {
                continue;
            }
            if word.starts_with(':') {
                if let Some(expanded) = resolve_known_export_tag(module, word) {
                    symbols.extend(expanded.iter().map(|name| (*name).to_string()));
                } else {
                    unresolved_tag = true;
                }
            } else {
                symbols.insert(word.to_string());
            }
        }
        (true, unresolved_tag)
    }

    fn collect_node_import_symbols(
        module: &str,
        arg: &Node,
        symbols: &mut HashSet<String>,
    ) -> (bool, bool) {
        match &arg.kind {
            NodeKind::String { value, .. } => {
                collect_import_symbols(module, value.trim_matches('\'').trim_matches('"'), symbols)
            }
            NodeKind::Identifier { name } => collect_import_symbols(module, name, symbols),
            NodeKind::ArrayLiteral { elements } => {
                let mut has_symbols = false;
                let mut has_unresolved_tag = false;
                for element in elements {
                    let (element_has_symbols, element_unresolved_tag) =
                        collect_node_import_symbols(module, element, symbols);
                    if element_has_symbols {
                        has_symbols = true;
                    }
                    if element_unresolved_tag {
                        has_unresolved_tag = true;
                    }
                }
                (has_symbols, has_unresolved_tag)
            }
            _ => (false, false),
        }
    }

    fn require_module_name(expr: &Node) -> Option<String> {
        let NodeKind::FunctionCall { name, args } = &expr.kind else {
            return None;
        };
        if name != "require" {
            return None;
        }
        let first = args.first()?;
        match &first.kind {
            NodeKind::Identifier { name } => Some(name.clone()),
            NodeKind::String { value, .. } => {
                let cleaned = value.trim_matches('\'').trim_matches('"').trim();
                Some(cleaned.trim_end_matches(".pm").replace('/', "::"))
            }
            _ => None,
        }
    }

    fn module_runtime_alias(expr: &Node) -> Option<(String, String)> {
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

    fn inner_expr(node: &Node) -> &Node {
        if let NodeKind::ExpressionStatement { expression } = &node.kind {
            expression.as_ref()
        } else {
            node
        }
    }

    fn collect(node: &Node, map: &mut ImportMap) {
        match &node.kind {
            NodeKind::Use { module, args, .. } => {
                let first_char: Option<char> = module.chars().next();
                if !first_char.is_some_and(|c: char| c.is_ascii_uppercase()) {
                    return;
                }

                if args.is_empty() {
                    return;
                }

                let mut symbols: HashSet<String> = HashSet::new();
                let mut has_symbol_args = false;
                let mut has_unresolved_tag = false;

                for arg in args {
                    let first_byte = arg.as_bytes().first().copied().unwrap_or(0);
                    if first_byte.is_ascii_digit() {
                        continue;
                    }
                    if arg.starts_with('-') {
                        continue;
                    }
                    let (has_symbols_in_arg, unresolved_tag) =
                        collect_import_symbols(module, arg, &mut symbols);
                    if has_symbols_in_arg {
                        has_symbol_args = true;
                    }
                    if unresolved_tag {
                        has_unresolved_tag = true;
                    }
                }

                if has_unresolved_tag {
                    return;
                }

                if has_symbol_args {
                    map.entry(module.clone()).or_default().extend(symbols);
                } else {
                    map.entry(module.clone()).or_default();
                }
            }
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                let mut required_modules: Vec<String> = statements
                    .iter()
                    .filter_map(|stmt| require_module_name(inner_expr(stmt)))
                    .collect();
                let mut aliases: HashMap<String, String> = HashMap::new();
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
                    let NodeKind::MethodCall { object, method, args } = &expr.kind else {
                        continue;
                    };
                    if method != "import" {
                        continue;
                    }
                    let object_name = match &object.kind {
                        NodeKind::Identifier { name } => Some(name.as_str()),
                        NodeKind::Variable { name, .. } => aliases.get(name).map(String::as_str),
                        _ => None,
                    };
                    let Some(object_name) = object_name else {
                        continue;
                    };
                    if !required_modules.iter().any(|module| module == object_name) {
                        continue;
                    }

                    if args.is_empty() {
                        continue;
                    }

                    let mut imported_symbols: HashSet<String> = HashSet::new();
                    let mut has_symbols = false;
                    let mut has_unresolved_tag = false;
                    for arg in args {
                        let (arg_has_symbols, arg_unresolved_tag) =
                            collect_node_import_symbols(object_name, arg, &mut imported_symbols);
                        if arg_has_symbols {
                            has_symbols = true;
                        }
                        if arg_unresolved_tag {
                            has_unresolved_tag = true;
                        }
                    }
                    if has_unresolved_tag || !has_symbols {
                        continue;
                    }
                    map.entry(object_name.to_string()).or_default().extend(imported_symbols);
                }

                for stmt in statements {
                    collect(stmt, map);
                }
            }
            _ => {}
        }
    }

    collect(ast, &mut map);
    map
}
