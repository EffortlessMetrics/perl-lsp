//! Shared inheritance graph traversal helpers for LSP consumers.
//!
//! Centralizes parent/role walking used by completion and navigation so both
//! providers resolve inherited methods consistently.

use perl_workspace::workspace_index::{
    Location, SymbolKind, WorkspaceIndex, WorkspaceSymbol, uri_to_fs_path,
};
use std::collections::{HashMap, HashSet, VecDeque};

/// Collect method symbols accessible from `package_name`, including inherited
/// parents and composed roles.
pub fn collect_accessible_package_members(
    index: &WorkspaceIndex,
    package_name: &str,
) -> Vec<WorkspaceSymbol> {
    let mut seen_names: HashSet<String> = HashSet::new();
    let mut result: Vec<WorkspaceSymbol> = Vec::new();

    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    let mut related_cache: HashMap<String, Vec<String>> = HashMap::new();

    queue.push_back(package_name.to_string());
    visited.insert(package_name.to_string());

    while let Some(pkg) = queue.pop_front() {
        for symbol in index.get_package_members(&pkg) {
            match symbol.kind {
                SymbolKind::Subroutine | SymbolKind::Method => {}
                _ => continue,
            }

            if seen_names.insert(symbol.name.clone()) {
                result.push(symbol);
            }
        }

        for related in collect_related_packages(index, &pkg, &mut related_cache) {
            if visited.insert(related.clone()) {
                queue.push_back(related);
            }
        }
    }

    result
}

/// Resolve `method_name` on ancestors of `receiver_pkg`.
pub fn find_inherited_method_definition(
    index: &WorkspaceIndex,
    receiver_pkg: &str,
    method_name: &str,
) -> Option<Location> {
    collect_accessible_package_members(index, receiver_pkg)
        .into_iter()
        .find_map(|symbol| {
            if symbol.name != method_name {
                return None;
            }
            let defining_pkg = symbol.container_name.as_deref()?;
            if defining_pkg == receiver_pkg {
                return None;
            }

            index.find_definition(&format!("{defining_pkg}::{method_name}"))
        })
}

fn collect_related_packages(
    index: &WorkspaceIndex,
    package_name: &str,
    related_cache: &mut HashMap<String, Vec<String>>,
) -> Vec<String> {
    related_cache
        .entry(package_name.to_string())
        .or_insert_with(|| {
            let Some(pkg_location) = index.find_definition(package_name) else {
                return Vec::new();
            };

            let text = index.document_store().get_text(&pkg_location.uri).or_else(|| {
                uri_to_fs_path(&pkg_location.uri).and_then(|path| std::fs::read_to_string(path).ok())
            });
            let Some(text) = text else {
                return Vec::new();
            };

            let mut parser = perl_semantic_analyzer::Parser::new(&text);
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
