use super::ImportMap;
use super::symbols::collect_node_import_symbols;
use perl_parser_core::ast::{Node, NodeKind};
use std::collections::{HashMap, HashSet};

pub(super) fn collect_runtime_imports(statements: &[Node], map: &mut ImportMap) {
    let mut required_modules = collect_required_modules(statements);
    let aliases = collect_module_runtime_aliases(statements, &mut required_modules);

    for stmt in statements {
        let expr = inner_expr(stmt);
        let NodeKind::MethodCall { object, method, args } = &expr.kind else {
            continue;
        };
        if method != "import" || args.is_empty() {
            continue;
        }

        let Some(object_name) = resolve_import_receiver(object, &aliases) else {
            continue;
        };
        if !required_modules.iter().any(|module| module == object_name) {
            continue;
        }

        collect_method_import_symbols(object_name, args, map);
    }
}

pub(super) fn inner_expr(node: &Node) -> &Node {
    if let NodeKind::ExpressionStatement { expression } = &node.kind {
        expression.as_ref()
    } else {
        node
    }
}

fn collect_required_modules(statements: &[Node]) -> Vec<String> {
    statements.iter().filter_map(|stmt| require_module_name(inner_expr(stmt))).collect()
}

fn collect_module_runtime_aliases(
    statements: &[Node],
    required_modules: &mut Vec<String>,
) -> HashMap<String, String> {
    let mut aliases = HashMap::new();
    for stmt in statements {
        if let Some((alias, module)) = module_runtime_alias(inner_expr(stmt)) {
            aliases.insert(alias, module.clone());
            if !required_modules.contains(&module) {
                required_modules.push(module);
            }
        }
    }
    aliases
}

fn collect_method_import_symbols(object_name: &str, args: &[Node], map: &mut ImportMap) {
    let mut imported_symbols: HashSet<String> = HashSet::new();
    let mut has_symbols = false;
    let mut has_unresolved_tag = false;
    for arg in args {
        let (arg_has_symbols, arg_unresolved_tag) =
            collect_node_import_symbols(object_name, arg, &mut imported_symbols);
        has_symbols |= arg_has_symbols;
        has_unresolved_tag |= arg_unresolved_tag;
    }
    if !has_unresolved_tag && has_symbols {
        map.entry(object_name.to_string()).or_default().extend(imported_symbols);
    }
}

fn resolve_import_receiver<'a>(
    object: &'a Node,
    aliases: &'a HashMap<String, String>,
) -> Option<&'a str> {
    match &object.kind {
        NodeKind::Identifier { name } => Some(name.as_str()),
        NodeKind::Variable { name, .. } => aliases.get(name).map(String::as_str),
        _ => None,
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
