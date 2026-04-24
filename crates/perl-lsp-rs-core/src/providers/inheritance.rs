//! Shared inheritance/role resolution helpers for navigation and completion.

use perl_semantic_analyzer::Parser;
use perl_workspace::workspace_index::{
    SymbolKind as WsSymbolKind, WorkspaceIndex, WorkspaceSymbol,
};
use std::collections::{HashMap, HashSet, VecDeque};

/// Return receiver-first BFS package order including ancestors and composed roles.
///
/// The first entry is always `package_name`.
pub fn package_resolution_order(index: &WorkspaceIndex, package_name: &str) -> Vec<String> {
    let mut order: Vec<String> = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    let mut related_cache: HashMap<String, Vec<String>> = HashMap::new();

    queue.push_back(package_name.to_string());
    visited.insert(package_name.to_string());

    while let Some(pkg) = queue.pop_front() {
        order.push(pkg.clone());

        for related in related_packages(index, &pkg, &mut related_cache) {
            if visited.insert(related.clone()) {
                queue.push_back(related);
            }
        }
    }

    order
}

/// Collect method-like symbols visible from `package_name`, including inherited methods.
///
/// Child-defined methods shadow ancestors: first occurrence wins according to
/// [`package_resolution_order`].
pub fn collect_accessible_method_symbols(
    index: &WorkspaceIndex,
    package_name: &str,
) -> Vec<WorkspaceSymbol> {
    let mut seen_names: HashSet<String> = HashSet::new();
    let mut result: Vec<WorkspaceSymbol> = Vec::new();

    for pkg in package_resolution_order(index, package_name) {
        for symbol in index.get_package_members(&pkg) {
            match symbol.kind {
                WsSymbolKind::Subroutine | WsSymbolKind::Method => {}
                _ => continue,
            }

            if seen_names.insert(symbol.name.clone()) {
                result.push(symbol);
            }
        }
    }

    result
}

fn related_packages(
    index: &WorkspaceIndex,
    package_name: &str,
    cache: &mut HashMap<String, Vec<String>>,
) -> Vec<String> {
    cache
        .entry(package_name.to_string())
        .or_insert_with(|| {
            let Some(package_location) = index.find_definition(package_name) else {
                return Vec::new();
            };

            let text = index.document_store().get_text(&package_location.uri).or_else(|| {
                perl_workspace::workspace_index::uri_to_fs_path(&package_location.uri)
                    .and_then(|path| std::fs::read_to_string(path).ok())
            });

            let Some(text) = text else {
                return Vec::new();
            };

            let mut parser = Parser::new(&text);
            let Ok(ast) = parser.parse() else {
                return Vec::new();
            };

            perl_semantic_analyzer::semantic::SemanticAnalyzer::analyze_with_source(&ast, &text)
                .class_models
                .into_iter()
                .find(|model| model.name == package_name)
                .map(|model| {
                    model.parents.iter().chain(model.roles.iter()).cloned().collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
        .clone()
}
