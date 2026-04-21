//! Unused import detection lint
//!
//! Detects `use Module` statements where the module name does not appear
//! elsewhere in the source text. Pragmas and known implicit-export modules
//! are excluded to avoid false positives.
//!
//! # Diagnostic codes
//!
//! | Code | Severity | Description |
//! |------|----------|-------------|
//! | `unused-import` | Hint | Module appears to be unused |

use perl_diagnostics::codes::DiagnosticCode;
use perl_parser_core::ast::{Node, NodeKind};

use super::super::internal_types::{Diagnostic, DiagnosticTag, RelatedInformation};
use super::super::walker::walk_node;
use perl_diagnostics::codes::DiagnosticSeverity;

/// Pragmas that should never be flagged as unused (they operate via side effects).
const PRAGMA_SKIP_LIST: &[&str] = &[
    "strict",
    "warnings",
    "utf8",
    "feature",
    "constant",
    "lib",
    "parent",
    "base",
    "Exporter",
    "vars",
    "subs",
    "overload",
    "overloading",
    "integer",
    "bigint",
    "bignum",
    "bigrat",
    "bytes",
    "charnames",
    "encoding",
    "locale",
    "mro",
    "open",
    "ops",
    "re",
    "sigtrap",
    "sort",
    "threads",
    "threads::shared",
    "autodie",
    "autouse",
    "diagnostics",
    "English",
    "experimental",
    "fields",
    "filetest",
    "if",
    "less",
    "POSIX",
];

/// Modules that implicitly export symbols into the caller's namespace.
const IMPLICIT_EXPORT_SKIP_LIST: &[&str] = &[
    "Moose",
    "Moose::Role",
    "Moose::Util::TypeConstraints",
    "MooseX::StrictConstructor",
    "MooseX::Types",
    "Moo",
    "Moo::Role",
    "Mouse",
    "Mouse::Role",
    "Modern::Perl",
    "Dancer",
    "Dancer2",
    "Catalyst",
    "Mojolicious",
    "Mojolicious::Lite",
    "Mojo::Base",
    "Test::More",
    "Test::Most",
    "Test::Simple",
    "Test::Exception",
    "Test::Fatal",
    "Test::Warnings",
    "Test::Deep",
    "Test::Differences",
    "Test::Class",
    "Test::MockModule",
    "Test::MockObject",
    "Test::Spec",
    "Test::Builder",
    "Test2::V0",
    "Test2::Bundle::More",
    "Carp",
    "Carp::Always",
    "Scalar::Util",
    "List::Util",
    "List::MoreUtils",
    "Data::Dumper",
    "Try::Tiny",
    "Getopt::Long",
    "File::Basename",
    "FindBin",
    "Fcntl",
    "UNIVERSAL",
];

/// Check for unused import statements.
///
/// Walks the AST to collect all `use Module` statements, then searches the
/// source text for references to each module name. If no reference is found
/// outside the `use` statement itself, a diagnostic hint is emitted.
pub fn check_unused_imports(node: &Node, source: &str, diagnostics: &mut Vec<Diagnostic>) {
    // Collect all use statements from the AST
    let mut use_statements: Vec<(String, usize, usize)> = Vec::new();

    walk_node(node, &mut |n| {
        if let NodeKind::Use { module, .. } = &n.kind {
            use_statements.push((module.clone(), n.location.start, n.location.end));
        }
    });

    for (module, start, end) in &use_statements {
        let module = module.as_str();
        let start = *start;
        let end = *end;

        // Skip pragmas
        if PRAGMA_SKIP_LIST.contains(&module) {
            continue;
        }

        // Skip implicit exporters
        if IMPLICIT_EXPORT_SKIP_LIST.contains(&module) {
            continue;
        }

        // Skip version-like imports (e.g., `use 5.010;`)
        if module.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            continue;
        }

        // Skip if the use statement has explicit import arguments
        if has_use_args(node, module) {
            continue;
        }

        // Check if the module name appears elsewhere in the source
        if !is_module_referenced(source, module, start, end) {
            diagnostics.push(Diagnostic {
                range: (start, end),
                severity: DiagnosticSeverity::Hint,
                code: Some(DiagnosticCode::UnusedImport.as_str().to_string()),
                message: format!("Module '{}' appears to be unused", module),
                related_information: vec![RelatedInformation {
                    location: (start, end),
                    message: format!(
                        "If '{}' is imported for side effects, you can ignore this hint",
                        module
                    ),
                }],
                tags: vec![DiagnosticTag::Unnecessary],
                suggestion: Some(format!("Remove 'use {};' if it is not needed", module)),
            });
        }
    }
}

/// Returns true if the use statement for `target_module` has explicit import args.
fn has_use_args(node: &Node, target_module: &str) -> bool {
    let mut found = false;
    walk_node(node, &mut |n| {
        if let NodeKind::Use { module, args, .. } = &n.kind
            && module == target_module
            && !args.is_empty()
        {
            found = true;
        }
    });
    found
}

/// Returns true if `module` appears in `source` outside the range `[use_start, use_end)`.
///
/// Uses word-boundary matching to avoid substring false positives (e.g.,
/// `FooBar` should not match a reference to `Foo`).
fn is_module_referenced(source: &str, module: &str, use_start: usize, use_end: usize) -> bool {
    let mut search_start = 0;
    while let Some(pos) = source[search_start..].find(module) {
        let abs_pos = search_start + pos;
        let match_end = abs_pos + module.len();

        // Skip the use statement itself
        if abs_pos >= use_start && abs_pos < use_end {
            search_start = match_end;
            continue;
        }

        // Check word boundaries
        let before_ok = abs_pos == 0 || {
            let ch = source.as_bytes()[abs_pos - 1];
            !ch.is_ascii_alphanumeric() && ch != b'_'
        };
        let after_ok = match_end >= source.len() || {
            let ch = source.as_bytes()[match_end];
            !ch.is_ascii_alphanumeric() && ch != b'_'
        };

        if before_ok && after_ok {
            return true;
        }

        search_start = match_end;
    }
    false
}
