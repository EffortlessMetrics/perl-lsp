//! Same-file Moo/Moose role conflict diagnostics.
//!
//! This lint checks for roles consumed by a class in the same file that
//! provide overlapping method names. It intentionally ignores workspace-wide
//! indexing, Role::Tiny, and transitive role composition.

use std::collections::{HashMap, HashSet};

use super::super::internal_types::Diagnostic;
use perl_diagnostics::codes::DiagnosticCode;
use perl_diagnostics::codes::DiagnosticSeverity;
use perl_parser_core::ast::Node;
use perl_semantic_analyzer::{
    class_model::{ClassModel, ClassModelBuilder},
    symbol::{SymbolKind, SymbolTable},
};

/// Check for same-file Moo/Moose role method conflicts.
pub fn check_role_conflicts(
    node: &Node,
    symbol_table: &SymbolTable,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut role_models: HashMap<String, ClassModel> = HashMap::new();
    let mut class_models: Vec<ClassModel> = Vec::new();

    for model in ClassModelBuilder::new().build(node) {
        match package_kind(symbol_table, &model.name) {
            Some(SymbolKind::Role) => {
                role_models.insert(model.name.clone(), model);
            }
            Some(SymbolKind::Class) => {
                class_models.push(model);
            }
            _ => {}
        }
    }

    for class_model in class_models {
        if class_model.roles.is_empty() {
            continue;
        }

        let class_methods = provided_method_names(&class_model);
        let mut method_providers: HashMap<String, Vec<String>> = HashMap::new();
        let mut seen_roles = HashSet::new();

        for role_name in &class_model.roles {
            if !seen_roles.insert(role_name.clone()) {
                continue;
            }

            let Some(role_model) = role_models.get(role_name) else {
                continue;
            };

            for method_name in provided_method_names(role_model) {
                method_providers.entry(method_name).or_default().push(role_name.clone());
            }
        }

        for (method_name, providers) in method_providers {
            if providers.len() < 2 || class_methods.contains(&method_name) {
                continue;
            }

            let Some(location) = role_anchor_location(symbol_table, &providers) else {
                continue;
            };

            diagnostics.push(Diagnostic {
                range: location,
                severity: DiagnosticSeverity::Warning,
                code: Some(DiagnosticCode::RoleConflict.as_str().to_string()),
                message: build_message(&class_model.name, &method_name, &providers),
                related_information: Vec::new(),
                tags: Vec::new(),
                suggestion: Some(format!(
                    "Define `{method_name}` in `{}` or remove one of the conflicting roles.",
                    class_model.name
                )),
            });
        }
    }
}

fn package_kind(symbol_table: &SymbolTable, package_name: &str) -> Option<SymbolKind> {
    symbol_table.symbols.get(package_name)?.iter().find_map(|symbol| match symbol.kind {
        SymbolKind::Class | SymbolKind::Role => Some(symbol.kind),
        _ => None,
    })
}

fn provided_method_names(model: &ClassModel) -> HashSet<String> {
    model.methods.iter().chain(model.adjusts.iter()).map(|method| method.name.clone()).collect()
}

fn role_anchor_location(
    symbol_table: &SymbolTable,
    role_names: &[String],
) -> Option<(usize, usize)> {
    for role_name in role_names {
        if let Some(reference) = symbol_table.references.get(role_name).and_then(|references| {
            references.iter().find(|reference| reference.kind == SymbolKind::Role)
        }) {
            return Some((reference.location.start, reference.location.end));
        }
    }

    None
}

fn build_message(class_name: &str, method_name: &str, role_names: &[String]) -> String {
    let role_list = format_role_list(role_names);
    let provider_verb = if role_names.len() == 2 { "both provide" } else { "all provide" };
    format!("Roles {role_list} {provider_verb} method `{method_name}` consumed by `{class_name}`")
}

fn format_role_list(role_names: &[String]) -> String {
    match role_names {
        [] => String::from(""),
        [single] => format!("`{single}`"),
        [first, second] => format!("`{first}` and `{second}`"),
        many => {
            let mut parts: Vec<String> =
                many[..many.len() - 1].iter().map(|name| format!("`{name}`")).collect();
            parts.push(format!("and `{}`", many[many.len() - 1]));
            parts.join(", ")
        }
    }
}
