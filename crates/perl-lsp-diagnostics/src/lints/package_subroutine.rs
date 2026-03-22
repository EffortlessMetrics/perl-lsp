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

use perl_diagnostics_codes::DiagnosticCode;
use perl_lsp_diagnostic_types::{Diagnostic, DiagnosticSeverity};
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
pub fn check_missing_package_declaration(node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    let statements = match &node.kind {
        NodeKind::Program { statements } => statements,
        _ => return,
    };

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
/// Walks the entire AST. For each subroutine name seen more than once,
/// emits a warning on the second and every subsequent occurrence.
/// Anonymous subroutines (`name: None`) are excluded.
/// `Method` nodes are excluded — class method redefinition semantics differ.
pub fn check_duplicate_subroutine(node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    let mut seen: HashMap<String, usize> = HashMap::new();

    walk_node(node, &mut |n| {
        if let NodeKind::Subroutine { name: Some(name), name_span: Some(span), .. } = &n.kind {
            let count = seen.entry(name.clone()).or_insert(0);
            *count += 1;
            if *count > 1 {
                diagnostics.push(Diagnostic {
                    range: (span.start, span.end),
                    severity: DiagnosticSeverity::Warning,
                    code: Some(DiagnosticCode::DuplicateSubroutine.as_str().to_string()),
                    message: format!("Subroutine '{}' is defined more than once", name),
                    related_information: Vec::new(),
                    tags: Vec::new(),
                    suggestion: Some(format!(
                        "Remove or rename the duplicate 'sub {}' definition",
                        name
                    )),
                });
            }
        }
    });
}
