//! Perl version compatibility lint (PL900)
//!
//! Warns when code uses features not available in the declared Perl version.
//!
//! # How it works
//!
//! 1. First pass over top-level statements: collect declared version (`use vN.NN`
//!    or `use N.NNN`) and any explicit `use feature 'X'` calls.
//! 2. Derive the effective feature set from the declared version (bundle
//!    implication — `use v5.36` implicitly enables all features available in 5.36).
//! 3. Second pass (via walker): detect version-gated AST constructs and emit
//!    `PL900` warnings for those not covered by the effective feature set.
//!
//! When no version is declared at all, the check emits nothing — undeclared
//! version is ambiguous (the file may be targeting the system Perl).

use perl_diagnostics_codes::DiagnosticCode;
use perl_parser_core::ast::{Node, NodeKind};
use perl_pragma::{PerlVersion, features_enabled_by_version, parse_perl_version};

use super::super::walker::walk_node;
use perl_lsp_diagnostic_types::{Diagnostic, DiagnosticSeverity};

/// Feature → minimum (major, minor) version table.
///
/// If a feature's minimum version is met by the declared version (either
/// directly or via bundle implication), no warning is emitted.
const FEATURE_VERSIONS: &[(&str, u32, u32)] = &[
    ("say", 5, 10),
    ("state", 5, 10),
    ("postfix_deref", 5, 20),
    // signatures: experimental since v5.20 but only stable-bundled at v5.36.
    // We use 5.36 as the effective minimum to match features_enabled_by_version,
    // preventing false-positive warnings on `use v5.20` files that rely on the
    // experimental pragma (`use feature 'signatures'`).
    ("signatures", 5, 36),
    ("try", 5, 34),
    ("class", 5, 38),
    ("field", 5, 38),
];

/// Check for Perl version compatibility issues.
///
/// Walks the AST looking for uses of version-gated features and emits
/// `PL900` warnings when the declared version does not support them.
pub fn check_version_compat(node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    // Collect version declaration and explicit `use feature` calls from top-level statements.
    let statements = match &node.kind {
        NodeKind::Program { statements } => statements,
        _ => return,
    };

    let mut declared_version: Option<PerlVersion> = None;
    let mut explicit_features: Vec<String> = Vec::new();

    for stmt in statements {
        if let NodeKind::Use { module, args, .. } = &stmt.kind {
            // Check for `use vN.NN` or `use N.NNN`
            if let Some(version) = parse_perl_version(module) {
                // Take the highest declared version if multiple appear
                match declared_version {
                    None => declared_version = Some(version),
                    Some(existing) if version > existing => declared_version = Some(version),
                    _ => {}
                }
            }
            // Check for `use feature 'X'` or `use feature qw(X Y)`
            if module == "feature" {
                for arg in args {
                    // Args may be bare names or quoted: 'say', "say"
                    let name = arg.trim_matches(|c| c == '\'' || c == '"');
                    explicit_features.push(name.to_string());
                }
            }
        }
    }

    // If no version was declared, skip all checks.
    let declared_version = match declared_version {
        Some(v) => v,
        None => return,
    };

    // Derive effective feature set from declared version.
    let mut effective_features = features_enabled_by_version(declared_version);

    // Explicit `use feature 'X'` additions override version.
    for feat in &explicit_features {
        if !effective_features.contains(&feat.as_str()) {
            // We only need to track features we check for.
            // Store them as references to our known list if possible.
            for (known, _, _) in FEATURE_VERSIONS {
                if *known == feat.as_str() && !effective_features.contains(known) {
                    effective_features.push(known);
                }
            }
        }
    }

    // Second pass: walk AST for version-gated constructs.
    walk_node(node, &mut |n| {
        match &n.kind {
            // `class Foo { }` — requires v5.38
            NodeKind::Class { .. } => {
                if !effective_features.contains(&"class") {
                    let min = feature_min_version("class");
                    diagnostics.push(make_diagnostic(n, "class", declared_version, min));
                }
            }

            // `try { } catch { }` — requires v5.34
            NodeKind::Try { .. } => {
                if !effective_features.contains(&"try") {
                    let min = feature_min_version("try");
                    diagnostics.push(make_diagnostic(n, "try/catch", declared_version, min));
                }
            }

            // `say` function call — requires v5.10
            NodeKind::FunctionCall { name, .. } if name == "say" => {
                if !effective_features.contains(&"say") {
                    let min = feature_min_version("say");
                    diagnostics.push(make_diagnostic(n, "say", declared_version, min));
                }
            }

            // `state $x` declaration — requires v5.10
            NodeKind::VariableDeclaration { declarator, .. } if declarator == "state" => {
                if !effective_features.contains(&"state") {
                    let min = feature_min_version("state");
                    diagnostics.push(make_diagnostic(n, "state", declared_version, min));
                }
            }

            // Postfix dereference `$x->@*`, `$x->%*`, `$x->$*` — requires v5.20
            NodeKind::Unary { op, .. }
                if op == "->@*" || op == "->%*" || op == "->$*" || op == "->@[" || op == "->@{" =>
            {
                if !effective_features.contains(&"postfix_deref") {
                    let min = feature_min_version("postfix_deref");
                    diagnostics.push(make_diagnostic(n, "postfix deref", declared_version, min));
                }
            }

            // Subroutine with a signature — requires v5.20
            NodeKind::Subroutine { signature: Some(_), .. } => {
                if !effective_features.contains(&"signatures") {
                    let min = feature_min_version("signatures");
                    diagnostics.push(make_diagnostic(
                        n,
                        "subroutine signatures",
                        declared_version,
                        min,
                    ));
                }
            }

            _ => {}
        }
    });
}

/// Return the minimum (major, minor) for a named feature from the table.
fn feature_min_version(feature: &str) -> (u32, u32) {
    FEATURE_VERSIONS
        .iter()
        .find(|(name, _, _)| *name == feature)
        .map(|(_, maj, min)| (*maj, *min))
        .unwrap_or((5, 0))
}

/// Build a PL900 diagnostic for a version-incompatible feature use.
fn make_diagnostic(
    node: &Node,
    display: &str,
    declared_version: PerlVersion,
    min_version: (u32, u32),
) -> Diagnostic {
    let message = format!(
        "'{}' requires Perl v{}.{}+; declared version is v{}.{}",
        display, min_version.0, min_version.1, declared_version.major, declared_version.minor,
    );

    Diagnostic {
        range: (node.location.start, node.location.end),
        severity: DiagnosticSeverity::Warning,
        code: Some(DiagnosticCode::VersionIncompatFeature.as_str().to_string()),
        message,
        related_information: vec![],
        tags: vec![],
        suggestion: Some(format!(
            "Update 'use v{}.{}' to 'use v{}.{}' or add 'use feature \"{}\";'",
            declared_version.major, declared_version.minor, min_version.0, min_version.1, display,
        )),
    }
}
