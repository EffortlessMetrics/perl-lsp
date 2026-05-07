//! Package and subroutine diagnostic lint checks
//!
//! This module implements diagnostic checks for package and subroutine declarations.
//!
//! # Diagnostic codes
//!
//! | Code  | Severity | Description                                      | Status      |
//! |-------|----------|--------------------------------------------------|-------------|
//! | PL200 | Warning  | Missing package declaration in file              | Implemented |
//! | PL201 | Warning  | Package name declared more than once in file     | Implemented |
//! | PL300 | Warning  | Subroutine name defined more than once in file   | Implemented |
//! | PL301 | Warning  | Subroutine has no explicit return statement      | Deferred    |
//! | PL402 | Warning  | Return value of expression used implicitly       | Deferred    |

// PL301 (MissingReturn): Deferred. Correct implementation requires full
// control-flow analysis of every branch in a subroutine body. In idiomatic
// Perl, implicit return of the last expression is correct style; emitting
// on every sub without an explicit `return` would be extremely noisy.
// Revisit when a control-flow graph is available in the AST.

// PL402 (ImplicitReturn): Deferred. In Perl, every expression is an
// implicit return value. This lint would fire on virtually every subroutine
// body. The code is reserved for future use with a narrower trigger condition.

use std::collections::HashMap;
use std::path::Path;

use super::super::internal_types::Diagnostic;
use perl_diagnostics::codes::DiagnosticCode;
use perl_diagnostics::codes::DiagnosticSeverity;
use perl_parser_core::ast::{Node, NodeKind};

use super::super::walker::walk_node;

/// Check for missing package declaration (PL200).
///
/// Walks the top-level statements of the `Program` node only (not recursive).
/// If no `Package` node is found at the top level, emits a warning at position `(0, 0)`.
///
/// Only the `Program` node's direct children are examined. Package declarations
/// inside `eval` blocks or other nested constructs are not counted — they do not
/// establish the file's package namespace in the same way.
pub fn check_missing_package_declaration(
    node: &Node,
    source: &str,
    source_path: Option<&Path>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let statements = match &node.kind {
        NodeKind::Program { statements } => statements,
        _ => return,
    };

    if should_skip_missing_package_declaration(source, source_path) {
        return;
    }

    let has_package = statements.iter().any(|stmt| matches!(&stmt.kind, NodeKind::Package { .. }));

    if !has_package {
        diagnostics.push(Diagnostic {
            range: (0, 0),
            severity: DiagnosticSeverity::Warning,
            code: Some(DiagnosticCode::MissingPackageDeclaration.as_str().to_string()),
            message: "This file has no package declaration. \
                      Add 'package MyModule;' to declare the package namespace."
                .to_string(),
            related_information: Vec::new(),
            tags: Vec::new(),
            suggestion: Some("Add 'package MyModule;' at the top of the file".to_string()),
        });
    }
}

fn should_skip_missing_package_declaration(source: &str, source_path: Option<&Path>) -> bool {
    if let Some(extension) =
        source_path.and_then(|path| path.extension()).and_then(|ext| ext.to_str())
    {
        let extension = extension.to_ascii_lowercase();
        if matches!(extension.as_str(), "pl" | "t" | "cgi" | "psgi" | "plx") {
            return true;
        }
    }

    source.trim_start().starts_with("#!")
}

/// Check for duplicate package declarations (PL201).
///
/// Walks the entire AST. For each package name seen more than once,
/// emits a warning on the second and every subsequent occurrence.
/// The first declaration is always clean.
pub fn check_duplicate_package(node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    let mut seen: HashMap<String, usize> = HashMap::new();

    walk_node(node, &mut |n| {
        if let NodeKind::Package { name, name_span, .. } = &n.kind {
            let count = seen.entry(name.clone()).or_insert(0);
            *count += 1;
            if *count > 1 {
                diagnostics.push(Diagnostic {
                    range: (name_span.start, name_span.end),
                    severity: DiagnosticSeverity::Warning,
                    code: Some(DiagnosticCode::DuplicatePackage.as_str().to_string()),
                    message: format!("Package '{}' is declared more than once in this file", name),
                    related_information: Vec::new(),
                    tags: Vec::new(),
                    suggestion: Some(format!(
                        "Remove the duplicate 'package {};' declaration",
                        name
                    )),
                });
            }
        }
    });
}

/// Check for duplicate named subroutine definitions (PL300).
///
/// Walks the entire AST. For each fully-qualified subroutine name seen more than once,
/// emits a warning on the second and every subsequent occurrence.
/// Anonymous subroutines (`name: None`) are excluded.
/// `Method` nodes are excluded — class method redefinition semantics differ.
///
/// Subroutine names are qualified by the current package context so that
/// `package Foo; sub new { }` and `package Bar; sub new { }` are treated as
/// distinct subroutines (`Foo::new` vs `Bar::new`) and do not trigger PL300.
pub fn check_duplicate_subroutine(node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    // Collect all (qualified_name, span) pairs in source order, tracking
    // the current package as we encounter Package nodes.
    let mut subs: Vec<(String, (usize, usize))> = Vec::new();
    let mut current_package = String::from("main");

    walk_node(node, &mut |n| {
        match &n.kind {
            NodeKind::Package { name, .. } => {
                current_package = name.clone();
            }
            NodeKind::Subroutine { name: Some(name), name_span: Some(span), .. } => {
                // Build a fully-qualified key so that Foo::new and Bar::new are distinct.
                // If the name already contains "::" it is explicitly qualified by the author.
                let qualified = if name.contains("::") {
                    name.clone()
                } else {
                    format!("{}::{}", current_package, name)
                };
                subs.push((qualified, (span.start, span.end)));
            }
            _ => {}
        }
    });

    // Second pass: find duplicates in the collected list.
    let mut seen: HashMap<String, usize> = HashMap::new();
    for (qualified, span) in subs {
        let count = seen.entry(qualified.clone()).or_insert(0);
        *count += 1;
        if *count > 1 {
            // Display only the bare name in the message (after the last "::").
            let display_name = qualified.rsplit("::").next().unwrap_or(&qualified).to_string();
            diagnostics.push(Diagnostic {
                range: span,
                severity: DiagnosticSeverity::Warning,
                code: Some(DiagnosticCode::DuplicateSubroutine.as_str().to_string()),
                message: format!("Subroutine '{}' is defined more than once", display_name),
                related_information: Vec::new(),
                tags: Vec::new(),
                suggestion: Some(format!(
                    "Remove or rename the duplicate 'sub {}' definition",
                    display_name
                )),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser::Parser;
    use perl_tdd_support::must;

    fn missing_pkg_diags(source: &str, path: Option<&Path>) -> Vec<Diagnostic> {
        let ast = must(Parser::new(source).parse());
        let mut diags = Vec::new();
        check_missing_package_declaration(&ast, source, path, &mut diags);
        diags
    }

    fn dup_pkg_diags(source: &str) -> Vec<Diagnostic> {
        let ast = must(Parser::new(source).parse());
        let mut diags = Vec::new();
        check_duplicate_package(&ast, &mut diags);
        diags
    }

    fn dup_sub_diags(source: &str) -> Vec<Diagnostic> {
        let ast = must(Parser::new(source).parse());
        let mut diags = Vec::new();
        check_duplicate_subroutine(&ast, &mut diags);
        diags
    }

    // --- PL200: missing package declaration ---

    #[test]
    fn pm_file_without_package_flagged() {
        let source = "sub foo { 1; }\n";
        let path = Path::new("MyModule.pm");
        let diags = missing_pkg_diags(source, Some(path));
        assert!(
            diags.iter().any(|d| d.code.as_deref() == Some("PL200")),
            "pm file without package should be flagged as PL200: {diags:?}"
        );
    }

    #[test]
    fn pm_file_with_package_not_flagged() {
        let source = "package MyModule;\nsub foo { 1; }\n";
        let path = Path::new("MyModule.pm");
        let diags = missing_pkg_diags(source, Some(path));
        assert!(
            diags.iter().all(|d| d.code.as_deref() != Some("PL200")),
            "pm file with package declaration should not be flagged: {diags:?}"
        );
    }

    #[test]
    fn pl_script_without_package_not_flagged() {
        let source = "print 'hello';\n";
        let path = Path::new("script.pl");
        let diags = missing_pkg_diags(source, Some(path));
        assert!(
            diags.iter().all(|d| d.code.as_deref() != Some("PL200")),
            "pl script without package should not be flagged: {diags:?}"
        );
    }

    #[test]
    fn test_file_without_package_not_flagged() {
        let source = "use Test::More;\nok(1);\n";
        let path = Path::new("my_test.t");
        let diags = missing_pkg_diags(source, Some(path));
        assert!(
            diags.iter().all(|d| d.code.as_deref() != Some("PL200")),
            "test .t file should not be flagged for missing package: {diags:?}"
        );
    }

    #[test]
    fn shebang_file_without_package_not_flagged() {
        let source = "#!/usr/bin/perl\nprint 'hi';\n";
        let diags = missing_pkg_diags(source, None);
        assert!(
            diags.iter().all(|d| d.code.as_deref() != Some("PL200")),
            "shebang script should not be flagged for missing package: {diags:?}"
        );
    }

    // --- PL201: duplicate package ---

    #[test]
    fn duplicate_package_is_flagged() {
        let diags = dup_pkg_diags("package Foo;\nsub a { }\npackage Foo;\nsub b { }\n");
        assert!(
            diags.iter().any(|d| d.code.as_deref() == Some("PL201")),
            "duplicate package declaration should be flagged as PL201: {diags:?}"
        );
    }

    #[test]
    fn single_package_not_flagged() {
        let diags = dup_pkg_diags("package Bar;\nsub x { }\n");
        assert!(
            diags.iter().all(|d| d.code.as_deref() != Some("PL201")),
            "single package declaration should not be flagged: {diags:?}"
        );
    }

    #[test]
    fn different_packages_not_flagged() {
        let diags = dup_pkg_diags("package Foo;\nsub a { }\npackage Bar;\nsub b { }\n");
        assert!(
            diags.iter().all(|d| d.code.as_deref() != Some("PL201")),
            "different package names should not trigger PL201: {diags:?}"
        );
    }

    #[test]
    fn duplicate_package_message_names_it() {
        let diags = dup_pkg_diags("package Baz;\npackage Baz;\n");
        let diag = diags.iter().find(|d| d.code.as_deref() == Some("PL201")).unwrap();
        assert!(
            diag.message.contains("Baz"),
            "PL201 message should name the duplicate package: {}", diag.message
        );
    }

    // --- PL300: duplicate subroutine ---

    #[test]
    fn duplicate_sub_is_flagged() {
        let diags = dup_sub_diags("package Pkg;\nsub process { 1; }\nsub process { 2; }\n");
        assert!(
            diags.iter().any(|d| d.code.as_deref() == Some("PL300")),
            "duplicate subroutine should be flagged as PL300: {diags:?}"
        );
    }

    #[test]
    fn unique_subs_not_flagged() {
        let diags = dup_sub_diags("package Pkg;\nsub foo { 1; }\nsub bar { 2; }\n");
        assert!(
            diags.iter().all(|d| d.code.as_deref() != Some("PL300")),
            "unique subroutines should not be flagged: {diags:?}"
        );
    }

    #[test]
    fn same_name_different_packages_not_flagged() {
        let diags = dup_sub_diags(
            "package Alpha;\nsub new { 1; }\npackage Beta;\nsub new { 2; }\n",
        );
        assert!(
            diags.iter().all(|d| d.code.as_deref() != Some("PL300")),
            "same sub name in different packages should not be flagged: {diags:?}"
        );
    }

    #[test]
    fn duplicate_sub_message_names_it() {
        let diags = dup_sub_diags("package X;\nsub helper { 1; }\nsub helper { 2; }\n");
        let diag = diags.iter().find(|d| d.code.as_deref() == Some("PL300")).unwrap();
        assert!(
            diag.message.contains("helper"),
            "PL300 message should name the duplicate subroutine: {}", diag.message
        );
    }

    #[test]
    fn duplicate_sub_has_suggestion() {
        let diags = dup_sub_diags("package X;\nsub run { 1; }\nsub run { 2; }\n");
        let diag = diags.iter().find(|d| d.code.as_deref() == Some("PL300")).unwrap();
        assert!(diag.suggestion.is_some(), "PL300 should carry a suggestion");
    }
}
