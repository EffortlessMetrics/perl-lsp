//! POD coverage lint for exported subroutines
//!
//! This module provides functionality for checking that all exported subroutines
//! have corresponding POD documentation (via `=head2 subroutine_name`).
//! The check only applies to files that use Exporter.
//!
//! # Diagnostic codes
//!
//! | Code  | Severity | Description                                      |
//! |-------|----------|--------------------------------------------------|
//! | PL304 | Warning  | Exported subroutine without POD documentation    |

use std::collections::HashSet;

use perl_diagnostics_codes::DiagnosticCode;
use perl_lsp_diagnostic_types::{Diagnostic, DiagnosticSeverity, RelatedInformation};
use perl_parser_core::ast::{Node, NodeKind};

use super::super::walker::walk_node;

/// Check for exported subroutines without POD documentation (PL304).
///
/// This function walks the AST to find:
/// 1. Whether the file uses Exporter (via `use Exporter 'import'`)
/// 2. Subroutine names exported via `@EXPORT` and `@EXPORT_OK`
/// 3. Subroutine definitions in the file
///
/// It then scans the source text for POD sections (`=head2 subroutine_name`)
/// and reports PL304 for any exported subroutine that lacks documentation.
///
/// Scripts (files starting with `#!`) are skipped as they are not modules.
pub fn check_pod_coverage(node: &Node, source: &str, diagnostics: &mut Vec<Diagnostic>) {
    // Skip scripts - they don't have the same documentation expectations as modules
    if source.trim_start().starts_with("#!") {
        return;
    }

    // Check if the file uses Exporter
    let uses_exporter = check_uses_exporter(node);
    if !uses_exporter {
        return;
    }

    // Collect exported subroutine names
    let exported_subs = collect_exported_subs(node);

    // If no exported subs, nothing to check
    if exported_subs.is_empty() {
        return;
    }

    // Collect defined subroutine names (for locating where to report diagnostics)
    let defined_subs = collect_defined_subs(node);

    // Scan source for documented subroutine names via =head2
    let documented_subs = scan_pod_documentation(source);

    // Find exported subs without documentation
    for sub_name in &exported_subs {
        if !documented_subs.contains(sub_name) {
            // Find the subroutine definition location for better diagnostic placement
            let sub_location = defined_subs.get(sub_name);

            let (range, message_suffix) = if let Some((start, end)) = sub_location {
                (
                    (*start, *end),
                    format!(
                        "Exported subroutine '{}' is not documented with '=head2 {}'",
                        sub_name, sub_name
                    ),
                )
            } else {
                // Exported but not defined in this file - use a generic range
                (
                    find_package_declaration_range(node),
                    format!(
                        "Exported subroutine '{}' (defined elsewhere) is not documented with '=head2 {}'",
                        sub_name, sub_name
                    ),
                )
            };

            diagnostics.push(Diagnostic {
                range,
                severity: DiagnosticSeverity::Warning,
                code: Some(DiagnosticCode::ExportedSubroutineWithoutPodDocs.as_str().to_string()),
                message: message_suffix,
                related_information: vec![
                    RelatedInformation {
                        location: range,
                        message: format!(
                            "Add '=head2 {}' followed by documentation to suppress this warning",
                            sub_name
                        ),
                    },
                    RelatedInformation {
                        location: range,
                        message: "POD documentation helps users understand the public API"
                            .to_string(),
                    },
                ],
                tags: Vec::new(),
                suggestion: Some(format!(
                    "Add '=head2 {}' section before or after the subroutine definition",
                    sub_name
                )),
            });
        }
    }
}

/// Check if the AST uses Exporter with 'import' (e.g., `use Exporter 'import';`)
fn check_uses_exporter(node: &Node) -> bool {
    let mut uses_exporter = false;

    walk_node(node, &mut |n| {
        if let NodeKind::Use { module, args, .. } = &n.kind {
            // Both quoted ("'import'") and bareword ("import") forms are valid in Perl's
            // use statement. The Exporter module is only active when 'import' is requested.
            if module == "Exporter" && args.iter().any(|arg| arg == "import" || arg == "'import'") {
                uses_exporter = true;
            }
        }
    });

    uses_exporter
}

/// Returns true if the variable name is EXPORT or EXPORT_OK
fn is_export_variable(name: &str) -> bool {
    name == "EXPORT" || name == "EXPORT_OK"
}

/// Collect all subroutine names exported via @EXPORT and @EXPORT_OK
///
/// We intentionally do NOT collapse the nested ifs into a single condition because
/// the outer check (`is_export_variable`) filters out most nodes early, making
/// the two-level structure more readable and easier to maintain.
#[allow(clippy::collapsible_if)]
fn collect_exported_subs(node: &Node) -> HashSet<String> {
    let mut exported = HashSet::new();

    walk_node(node, &mut |n| {
        // Check for package variable declarations: our @EXPORT = qw(...) or our @EXPORT_OK = qw(...)
        if let NodeKind::VariableListDeclaration { variables, initializer: Some(init), .. } =
            &n.kind
        {
            for var in variables {
                if let NodeKind::Variable { name, .. } = &var.kind {
                    if is_export_variable(name) {
                        collect_qw_items(init, &mut exported);
                    }
                }
            }
        }

        // Also check single-variable our declarations: our $EXPORT = qw(...)
        if let NodeKind::VariableDeclaration { variable, initializer: Some(init), .. } = &n.kind {
            if let NodeKind::Variable { name, .. } = &variable.kind {
                if is_export_variable(name) {
                    // Collect the qw() list items
                    collect_qw_items(init, &mut exported);
                }
            }
        }
    });

    exported
}

/// Extract items from a qw() quoted word list
fn collect_qw_items(node: &Node, output: &mut HashSet<String>) {
    match &node.kind {
        // Array literal: qw(foo bar baz) becomes ArrayLiteral with String elements
        NodeKind::ArrayLiteral { elements } => {
            for elem in elements {
                if let NodeKind::String { value, .. } = &elem.kind {
                    output.insert(value.clone());
                }
            }
        }
        // Recurse into children for nested structures
        _ => {
            for child in node.children() {
                collect_qw_items(child, output);
            }
        }
    }
}

/// Collect defined subroutine names and their source locations
fn collect_defined_subs(node: &Node) -> std::collections::HashMap<String, (usize, usize)> {
    let mut subs = std::collections::HashMap::new();

    walk_node(node, &mut |n| {
        if let NodeKind::Subroutine { name: Some(name), name_span: Some(span), .. } = &n.kind {
            // Only add if not already present (first definition wins for duplicates)
            subs.entry(name.clone()).or_insert((span.start, span.end));
        }
    });

    subs
}

/// Scan POD documentation in source text for `=head2 subroutine_name` sections.
///
/// Returns a set of subroutine names that have POD documentation.
fn scan_pod_documentation(source: &str) -> HashSet<String> {
    let mut documented = HashSet::new();

    // Find all =head2 sections and extract the subroutine name that follows
    // POD format: =head2 subroutine_name
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("=head2") {
            let after_head2 = trimmed.strip_prefix("=head2").unwrap_or("").trim();
            // The name is the first word (subroutine name)
            if let Some(first_word) = after_head2.split_whitespace().next() {
                documented.insert(first_word.to_string());
            }
        }
    }

    documented
}

/// Find the package declaration range for fallback diagnostic placement
fn find_package_declaration_range(node: &Node) -> (usize, usize) {
    if let NodeKind::Program { statements } = &node.kind {
        for stmt in statements {
            if let NodeKind::Package { name_span, .. } = &stmt.kind {
                return (name_span.start, name_span.end);
            }
        }
    }
    // Fallback to the start of the file
    (0, 0)
}
