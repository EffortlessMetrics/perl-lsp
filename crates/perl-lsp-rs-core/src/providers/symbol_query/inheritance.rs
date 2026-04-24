//! Shared inherited-method resolution helpers for workspace-backed providers.
//!
//! This module centralizes package lineage traversal (parent classes + roles)
//! so completion and navigation consumers do not duplicate inheritance walking.

use perl_semantic_analyzer::semantic::SemanticAnalyzer;
use perl_semantic_analyzer::Parser;
use perl_workspace::workspace_index::{
    uri_to_fs_path, SymbolKind as WsSymbolKind, WorkspaceIndex, WorkspaceSymbol,
};
use std::collections::{HashMap, HashSet, VecDeque};

/// Collect all method/subroutine symbols accessible from `package_name`.
///
/// Traverses the package itself first, then parent/role ancestors breadth-first.
/// Child definitions shadow inherited methods with the same name.
#[must_use]
pub fn collect_accessible_method_symbols(
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
        let members = index.get_package_members(&pkg);
        for symbol in members {
            if !matches!(symbol.kind, WsSymbolKind::Subroutine | WsSymbolKind::Method) {
                continue;
            }
            if seen_names.insert(symbol.name.clone()) {
                result.push(symbol);
            }
        }

        for ancestor in collect_related_packages(index, &pkg, &mut related_cache) {
            if visited.insert(ancestor.clone()) {
                queue.push_back(ancestor);
            }
        }
    }

    result
}

/// Find `method_name` in the receiver package ancestry chain.
///
/// Returns the symbol that should be considered visible for method dispatch.
#[must_use]
pub fn find_accessible_method_symbol(
    index: &WorkspaceIndex,
    receiver_package: &str,
    method_name: &str,
) -> Option<WorkspaceSymbol> {
    collect_accessible_method_symbols(index, receiver_package)
        .into_iter()
        .find(|symbol| symbol.name == method_name)
}

fn collect_related_packages(
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
                uri_to_fs_path(&package_location.uri)
                    .and_then(|path| std::fs::read_to_string(path).ok())
            });
            let Some(text) = text else {
                return Vec::new();
            };

            let mut parser = Parser::new(&text);
            let Ok(ast) = parser.parse() else {
                return Vec::new();
            };

            SemanticAnalyzer::analyze_with_source(&ast, &text)
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

#[cfg(test)]
mod tests {
    use super::{collect_accessible_method_symbols, find_accessible_method_symbol};
    use perl_workspace::workspace_index::WorkspaceIndex;
    use std::sync::Arc;
    use url::Url;

    #[test]
    fn inherited_method_lookup_finds_parent_method() -> Result<(), Box<dyn std::error::Error>> {
        let index = Arc::new(WorkspaceIndex::new());
        index.index_file(
            Url::parse("file:///workspace/Parent.pm")?,
            "package Parent;\nsub inherited { 1 }\n1;\n".to_string(),
        )?;
        index.index_file(
            Url::parse("file:///workspace/Child.pm")?,
            "package Child;\nuse parent 'Parent';\nsub local_only { 1 }\n1;\n".to_string(),
        )?;

        let found = find_accessible_method_symbol(&index, "Child", "inherited");
        assert!(found.is_some(), "inherited method should be reachable from Child");

        let symbol = found.ok_or("missing inherited method symbol")?;
        assert_eq!(symbol.container_name.as_deref(), Some("Parent"));
        Ok(())
    }

    #[test]
    fn own_method_shadows_inherited_method() -> Result<(), Box<dyn std::error::Error>> {
        let index = Arc::new(WorkspaceIndex::new());
        index.index_file(
            Url::parse("file:///workspace/Parent.pm")?,
            "package Parent;\nsub ping { 1 }\n1;\n".to_string(),
        )?;
        index.index_file(
            Url::parse("file:///workspace/Child.pm")?,
            "package Child;\nuse parent 'Parent';\nsub ping { 2 }\n1;\n".to_string(),
        )?;

        let symbols = collect_accessible_method_symbols(&index, "Child");
        let ping = symbols.iter().find(|symbol| symbol.name == "ping");
        assert!(ping.is_some(), "ping should be discoverable");
        assert_eq!(
            ping.and_then(|symbol| symbol.container_name.as_deref()),
            Some("Child"),
            "closest (child) implementation must win",
        );
        Ok(())
    }
}
