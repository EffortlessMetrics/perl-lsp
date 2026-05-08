//! Same-file Moo/Moose/Role::Tiny role conflict diagnostics.
//!
//! This lint checks for roles consumed by a class in the same file that
//! provide overlapping method names. It intentionally ignores workspace-wide
//! indexing and transitive role composition.

use std::collections::{HashMap, HashSet};

use super::super::internal_types::Diagnostic;
use perl_diagnostics::codes::DiagnosticCode;
use perl_diagnostics::codes::DiagnosticSeverity;
use perl_parser_core::ast::Node;
use perl_semantic_analyzer::{
    class_model::{ClassModel, ClassModelBuilder},
    symbol::{SymbolKind, SymbolTable},
};

/// Check for same-file Moo/Moose/Role::Tiny role method conflicts.
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

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser::Parser;
    use perl_semantic_analyzer::analysis::symbol::SymbolExtractor;
    use perl_tdd_support::{must, must_some};

    fn role_conflict_diags(source: &str) -> Vec<Diagnostic> {
        let ast = must(Parser::new(source).parse());
        let symbol_table = SymbolExtractor::new_with_source(source).extract(&ast);
        let mut diagnostics = Vec::new();
        check_role_conflicts(&ast, &symbol_table, &mut diagnostics);
        diagnostics
    }

    fn has_code(diags: &[Diagnostic], code: &str) -> bool {
        diags.iter().any(|d| d.code.as_deref() == Some(code))
    }

    #[test]
    fn two_roles_with_same_method_fires_pl303() {
        let source = r#"
package MyRole::Greet;
use Moo::Role;
sub greet { return "hello" }

package MyRole::Welcome;
use Moo::Role;
sub greet { return "welcome" }

package MyClass;
use Moo;
with 'MyRole::Greet', 'MyRole::Welcome';
"#;
        let diags = role_conflict_diags(source);
        assert!(
            has_code(&diags, "PL303"),
            "two roles with same method should fire PL303: {diags:?}"
        );
    }

    #[test]
    fn class_overriding_conflicting_method_suppresses_pl303() {
        let source = r#"
package MyRole::Greet;
use Moo::Role;
sub greet { return "hello" }

package MyRole::Welcome;
use Moo::Role;
sub greet { return "welcome" }

package MyClass;
use Moo;
with 'MyRole::Greet', 'MyRole::Welcome';
sub greet { return "my custom greeting" }
"#;
        let diags = role_conflict_diags(source);
        assert!(
            !has_code(&diags, "PL303"),
            "class providing its own `greet` should suppress PL303: {diags:?}"
        );
    }

    #[test]
    fn roles_with_non_overlapping_methods_no_pl303() {
        let source = r#"
package MyRole::Greet;
use Moo::Role;
sub greet { return "hello" }

package MyRole::Farewell;
use Moo::Role;
sub farewell { return "goodbye" }

package MyClass;
use Moo;
with 'MyRole::Greet', 'MyRole::Farewell';
"#;
        let diags = role_conflict_diags(source);
        assert!(
            !has_code(&diags, "PL303"),
            "non-overlapping role methods should not fire PL303: {diags:?}"
        );
    }

    #[test]
    fn single_role_consumed_no_pl303() {
        let source = r#"
package MyRole::Greet;
use Moo::Role;
sub greet { return "hello" }

package MyClass;
use Moo;
with 'MyRole::Greet';
"#;
        let diags = role_conflict_diags(source);
        assert!(
            !has_code(&diags, "PL303"),
            "single role consumption should not fire PL303: {diags:?}"
        );
    }

    #[test]
    fn three_roles_all_with_same_method_fires_pl303() {
        let source = r#"
package MyRole::A;
use Moo::Role;
sub process { return "A" }

package MyRole::B;
use Moo::Role;
sub process { return "B" }

package MyRole::C;
use Moo::Role;
sub process { return "C" }

package MyClass;
use Moo;
with 'MyRole::A', 'MyRole::B', 'MyRole::C';
"#;
        let diags = role_conflict_diags(source);
        assert!(
            has_code(&diags, "PL303"),
            "three roles with the same method should fire PL303: {diags:?}"
        );
    }

    #[test]
    fn diagnostic_message_names_conflicting_method() {
        let source = r#"
package MyRole::A;
use Moo::Role;
sub run { 1 }

package MyRole::B;
use Moo::Role;
sub run { 1 }

package MyClass;
use Moo;
with 'MyRole::A', 'MyRole::B';
"#;
        let diags = role_conflict_diags(source);
        let pl303 = must_some(diags.iter().find(|d| d.code.as_deref() == Some("PL303")));
        let msg = &pl303.message;
        assert!(msg.contains("run"), "message should name the conflicting method `run`: {msg}");
    }

    #[test]
    fn class_without_any_roles_no_pl303() {
        let source = r#"
package MyClass;
use Moo;
sub greet { "hello" }
"#;
        let diags = role_conflict_diags(source);
        assert!(!has_code(&diags, "PL303"), "class with no roles should not fire PL303: {diags:?}");
    }

    #[test]
    fn plain_package_without_oo_framework_no_pl303() {
        let source = r#"
package MyPackage;
sub greet { "hello" }
"#;
        let diags = role_conflict_diags(source);
        assert!(
            !has_code(&diags, "PL303"),
            "plain package without Moo/Moose should not fire PL303: {diags:?}"
        );
    }

    #[test]
    fn pl303_diagnostic_includes_suggestion() {
        let source = r#"
package MyRole::A;
use Moo::Role;
sub handle { 1 }

package MyRole::B;
use Moo::Role;
sub handle { 1 }

package MyClass;
use Moo;
with 'MyRole::A', 'MyRole::B';
"#;
        let diags = role_conflict_diags(source);
        let pl303 = must_some(diags.iter().find(|d| d.code.as_deref() == Some("PL303")));
        assert!(pl303.suggestion.is_some(), "PL303 should include a resolution suggestion");
    }

    #[test]
    fn moose_role_conflict_also_fires_pl303() {
        let source = r#"
package MyRole::A;
use Moose::Role;
sub serialize { 1 }

package MyRole::B;
use Moose::Role;
sub serialize { 1 }

package MyClass;
use Moose;
with 'MyRole::A', 'MyRole::B';
"#;
        let diags = role_conflict_diags(source);
        assert!(
            has_code(&diags, "PL303"),
            "Moose::Role conflict should also fire PL303: {diags:?}"
        );
    }
}
