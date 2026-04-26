//! Same-file Moo/Moose/Role::Tiny role conflict diagnostics (PL303).
//!
//! This lint checks for roles consumed by a class in the same file that
//! provide overlapping method names. Supports Moo, Moose, Mouse, and Role::Tiny
//! role composition via `with()`.
//!
//! Detection relies on framework detection in [`FrameworkKind`] and
//! [`Framework`] enums, and symbol classification via [`SymbolKind::Role`].
//! See [`perl_semantic_analyzer::analysis::symbol`] and
//! [`perl_semantic_analyzer::analysis::class_model`] for details.

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

/// Look up the [`SymbolKind`] for a package in the symbol table.
///
/// Returns `SymbolKind::Class` or `SymbolKind::Role` if the package has been
/// upgraded from `SymbolKind::Package` by the framework detection pass in
/// [`upgrade_package_symbols_from_framework_flags`](crate::symbol::SymbolExtractor::upgrade_package_symbols_from_framework_flags).
///
/// Returns `None` if the package is not in the symbol table or has not been
/// classified as a class or role.
fn package_kind(symbol_table: &SymbolTable, package_name: &str) -> Option<SymbolKind> {
    symbol_table.symbols.get(package_name)?.iter().find_map(|symbol| match symbol.kind {
        SymbolKind::Class | SymbolKind::Role => Some(symbol.kind),
        _ => None,
    })
}

/// Collect all method names provided by a class model.
///
/// Combines both regular methods and `BUILD`/`DEMOLISH` adjustment methods
/// since both contribute to method visibility in role composition.
fn provided_method_names(model: &ClassModel) -> HashSet<String> {
    model.methods.iter().chain(model.adjusts.iter()).map(|method| method.name.clone()).collect()
}

/// Find the source location of a role reference for use as a diagnostic anchor.
///
/// The anchor is the `with()` call that consumes the conflicting roles.
/// Returns the location of the first role reference found in the symbol table
/// that has `SymbolKind::Role`. Returns `None` if no role references are found.
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

/// Build a human-readable diagnostic message for a role method conflict.
///
/// Uses "both provide" for two roles, "all provide" for three or more.
fn build_message(class_name: &str, method_name: &str, role_names: &[String]) -> String {
    let role_list = format_role_list(role_names);
    let provider_verb = if role_names.len() == 2 { "both provide" } else { "all provide" };
    format!("Roles {role_list} {provider_verb} method `{method_name}` consumed by `{class_name}`")
}

/// Format a list of role names as a human-readable string.
///
/// Examples:
/// - `[]` → `""`
/// - `["RoleA"]` → `` `RoleA` ``
/// - `["RoleA", "RoleB"]` → `` `RoleA` and `RoleB` ``
/// - `["RoleA", "RoleB", "RoleC"]` → `` `RoleA`, `RoleB`, and `RoleC` ``
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
