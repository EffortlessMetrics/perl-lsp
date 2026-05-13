use super::ImportMap;
use perl_parser_core::ast::{Node, NodeKind};
use std::collections::{HashMap, HashSet};

mod runtime_imports;
mod symbols;
mod used_modules;

use runtime_imports::collect_runtime_imports;
use symbols::collect_import_symbols;
use used_modules::is_importable_module;

/// Walk the top-level AST and build an `ImportMap` from `use` statements.
///
/// Only uppercase-starting module names are included (skips pragmas like
/// `strict`, `warnings`, `feature`, `constant`, `utf8`, `lib`, `parent`, `base`).
pub(super) fn extract_import_map(ast: &Node) -> ImportMap {
    let mut map: ImportMap = HashMap::new();
    collect(ast, &mut map);
    map
}

fn collect(node: &Node, map: &mut ImportMap) {
    match &node.kind {
        NodeKind::Use { module, args, .. } => collect_use_import(module, args, map),
        NodeKind::Program { statements } | NodeKind::Block { statements } => {
            collect_runtime_imports(statements, map);
            for stmt in statements {
                collect(stmt, map);
            }
        }
        _ => {}
    }
}

fn collect_use_import(module: &str, args: &[String], map: &mut ImportMap) {
    if !is_importable_module(module) || args.is_empty() {
        return;
    }

    let mut symbols: HashSet<String> = HashSet::new();
    let mut has_symbol_args = false;
    let mut has_unresolved_tag = false;

    for arg in args.iter().filter(|arg| is_symbol_arg_candidate(arg)) {
        let (has_symbols_in_arg, unresolved_tag) =
            collect_import_symbols(module, arg, &mut symbols);
        has_symbol_args |= has_symbols_in_arg;
        has_unresolved_tag |= unresolved_tag;
    }

    if has_unresolved_tag {
        return;
    }

    if has_symbol_args {
        map.entry(module.to_string()).or_default().extend(symbols);
    } else {
        map.entry(module.to_string()).or_default();
    }
}

fn is_symbol_arg_candidate(arg: &str) -> bool {
    let first_byte = arg.as_bytes().first().copied().unwrap_or(0);
    !first_byte.is_ascii_digit() && !arg.starts_with('-')
}

pub(super) use used_modules::collect_used_module_names;
