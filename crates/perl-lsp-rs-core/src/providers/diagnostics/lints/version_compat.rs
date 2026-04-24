//! Perl version compatibility lint (PL900)
//!
//! Warns when code uses features not available in the declared Perl version.
//!
//! # How it works
//!
//! 1. First pass over top-level statements: collect the declared version (`use vN.NN`
//!    or `use N.NNN`) and any builtin imports that affect version checks.
//! 2. Build lexical pragma state with `PragmaTracker`, so explicit `use feature`
//!    and `no feature` pragmas are honored at each AST node.
//! 3. Second pass (via walker): detect version-gated AST constructs and emit
//!    `PL900` warnings for those not covered by the effective feature set.
//!
//! When no version is declared at all, the check emits nothing — undeclared
//! version is ambiguous (the file may be targeting the system Perl).

use perl_diagnostics::codes::DiagnosticCode;
use perl_parser_core::ast::{Node, NodeKind};
use perl_pragma::{PerlVersion, PragmaTracker, parse_perl_version};

use super::super::internal_types::Diagnostic;
use super::super::walker::walk_node;
use perl_diagnostics::codes::DiagnosticSeverity;

/// Feature → minimum (major, minor) version table.
///
/// If a feature's minimum version is met by the declared version (either
/// directly or via bundle implication), no warning is emitted.
const FEATURE_VERSIONS: &[(&str, u32, u32)] = &[
    ("say", 5, 10),
    ("state", 5, 10),
    // switch: the feature bundle name for given/when/default constructs (Perl 5.10+)
    ("switch", 5, 10),
    ("postfix_deref", 5, 20),
    ("try", 5, 34),
    // signatures: experimental since v5.20 but only stable-bundled at v5.36.
    // We use 5.36 as the effective minimum to match features_enabled_by_version,
    // preventing false-positive warnings on `use v5.20` files that rely on the
    // experimental pragma (`use feature 'signatures'`).
    ("signatures", 5, 36),
    // defer block: experimental since v5.36.
    // Detected only when the AST matches the parser's `defer { ... }` shape,
    // not for arbitrary helpers/imports named `defer`.
    ("defer", 5, 36),
    ("class", 5, 38),
    ("field", 5, 38),
    // isa: experimental in v5.32, stable-bundled at v5.36.
    // `$obj isa 'ClassName'` — infix operator for class membership testing.
    ("isa", 5, 36),
    ("builtin", 5, 40),
];

/// `builtin` bundle and import minimums.
///
/// The namespace-level bundle still gates at 5.40, but individual functions
/// were introduced across multiple releases.
const BUILTIN_BUNDLE_MIN_VERSION: PerlVersion = PerlVersion::new(5, 40);

const BUILTIN_FUNCTION_VERSIONS: &[(&str, u32, u32)] = &[
    ("true", 5, 36),
    ("false", 5, 36),
    ("is_bool", 5, 36),
    ("inf", 5, 40),
    ("nan", 5, 40),
    ("weaken", 5, 36),
    ("unweaken", 5, 36),
    ("is_weak", 5, 36),
    ("blessed", 5, 36),
    ("refaddr", 5, 36),
    ("reftype", 5, 36),
    ("created_as_string", 5, 36),
    ("created_as_number", 5, 36),
    ("stringify", 5, 36),
    ("ceil", 5, 36),
    ("floor", 5, 36),
    ("indexed", 5, 36),
    ("trim", 5, 36),
    ("is_tainted", 5, 38),
    ("export_lexically", 5, 38),
    ("load_module", 5, 40),
];

const GIVEN_WHEN_DEPRECATION_VERSION: PerlVersion = PerlVersion::new(5, 38);
const GIVEN_WHEN_REMOVAL_VERSION: PerlVersion = PerlVersion::new(5, 42);
const SMARTMATCH_DEPRECATION_VERSION: PerlVersion = PerlVersion::new(5, 38);
const SMARTMATCH_REMOVAL_VERSION: PerlVersion = PerlVersion::new(5, 42);

/// Check for Perl version compatibility issues.
///
/// Walks the AST looking for uses of version-gated features and emits
/// `PL900` warnings when the declared version does not support them.
pub fn check_version_compat(node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    // Collect the declared version and builtin imports from top-level statements.
    let statements = match &node.kind {
        NodeKind::Program { statements } => statements,
        _ => return,
    };

    let mut declared_version: Option<PerlVersion> = None;
    let mut builtin_imports: Vec<String> = Vec::new();
    let mut builtin_bundle_declared = false;

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
            if module == "builtin" {
                if args.is_empty() {
                    builtin_bundle_declared = true;
                }

                for arg in args {
                    for name in builtin_import_names(arg) {
                        if name.is_empty() {
                            continue;
                        }

                        if name.starts_with(':') {
                            builtin_bundle_declared = true;
                            continue;
                        }

                        if !builtin_imports.contains(&name) {
                            builtin_imports.push(name);
                        }
                    }
                }
            }
        }
    }

    // If no version was declared, skip all checks.
    let declared_version = match declared_version {
        Some(v) => v,
        None => return,
    };

    let pragma_map = PragmaTracker::build(node);

    // Second pass: walk AST for version-gated constructs.
    walk_node(node, &mut |n| {
        let pragma_state = PragmaTracker::state_for_offset(&pragma_map, n.location.start);

        match &n.kind {
            // `class Foo { }` — requires v5.38
            NodeKind::Class { .. } => {
                if !pragma_state.has_feature("class") {
                    let min = feature_min_version("class");
                    diagnostics.push(make_diagnostic(n, "class", declared_version, min));
                }
            }

            // `given` / `when` / `default` need the `switch` feature (v5.10+),
            // are deprecated in v5.38, and removed in v5.42.
            NodeKind::Given { .. } | NodeKind::When { .. } | NodeKind::Default { .. } => {
                let construct = if matches!(&n.kind, NodeKind::Given { .. }) {
                    "given"
                } else if matches!(&n.kind, NodeKind::When { .. }) {
                    "when"
                } else {
                    "default"
                };

                if declared_version >= GIVEN_WHEN_REMOVAL_VERSION {
                    diagnostics.push(make_given_when_default_diagnostic(
                        n,
                        declared_version,
                        DiagnosticSeverity::Error,
                    ));
                } else if declared_version >= GIVEN_WHEN_DEPRECATION_VERSION {
                    diagnostics.push(make_given_when_default_diagnostic(
                        n,
                        declared_version,
                        DiagnosticSeverity::Warning,
                    ));
                } else if !pragma_state.has_feature("switch") {
                    let min = feature_min_version("switch");
                    diagnostics.push(make_diagnostic(n, construct, declared_version, min));
                }
            }

            // `try { } catch { }` — requires v5.34
            NodeKind::Try { .. } => {
                if !pragma_state.has_feature("try") {
                    let min = feature_min_version("try");
                    diagnostics.push(make_diagnostic(n, "try/catch", declared_version, min));
                }
            }

            // `say` function call — requires v5.10
            NodeKind::FunctionCall { name, .. } if name == "say" => {
                if !pragma_state.has_feature("say") {
                    let min = feature_min_version("say");
                    diagnostics.push(make_diagnostic(n, "say", declared_version, min));
                }
            }

            // `defer { }` block — requires v5.36 (`use feature 'defer'`).
            NodeKind::Defer { .. } => {
                if !pragma_state.has_feature("defer") {
                    let min = feature_min_version("defer");
                    diagnostics.push(make_diagnostic(n, "defer", declared_version, min));
                }
            }

            NodeKind::FunctionCall { name, .. } if name.starts_with("builtin::") => {
                let builtin_name = name.trim_start_matches("builtin::");
                let min = builtin_min_version(builtin_name);
                let imported = builtin_imports.iter().any(|import| import == builtin_name);

                if declared_version < min && !builtin_bundle_declared && !imported {
                    diagnostics.push(make_diagnostic(
                        n,
                        name,
                        declared_version,
                        (min.major, min.minor),
                    ));
                }
            }

            NodeKind::Use { module, args, .. } if module == "builtin" => {
                if args.is_empty() {
                    if declared_version < BUILTIN_BUNDLE_MIN_VERSION {
                        diagnostics.push(make_diagnostic(
                            n,
                            "use builtin",
                            declared_version,
                            (BUILTIN_BUNDLE_MIN_VERSION.major, BUILTIN_BUNDLE_MIN_VERSION.minor),
                        ));
                    }
                    return;
                }

                for arg in args {
                    for name in builtin_import_names(arg) {
                        let min = builtin_import_min_version(&name);
                        if declared_version < min {
                            let display = format!("use builtin {}", arg);
                            diagnostics.push(make_diagnostic(
                                n,
                                &display,
                                declared_version,
                                (min.major, min.minor),
                            ));
                        }
                    }
                }
            }

            // `state $x` declaration — requires v5.10
            NodeKind::VariableDeclaration { declarator, .. } if declarator == "state" => {
                if !pragma_state.has_feature("state") {
                    let min = feature_min_version("state");
                    diagnostics.push(make_diagnostic(n, "state", declared_version, min));
                }
            }

            // Postfix dereference `$x->@*`, `$x->%*`, `$x->$*` — requires v5.20
            NodeKind::Unary { op, .. }
                if op == "->@*" || op == "->%*" || op == "->$*" || op == "->@[" || op == "->@{" =>
            {
                if !pragma_state.has_feature("postfix_deref") {
                    let min = feature_min_version("postfix_deref");
                    diagnostics.push(make_diagnostic(n, "postfix deref", declared_version, min));
                }
            }

            // Subroutine with a signature — requires v5.20
            NodeKind::Subroutine { signature: Some(_), .. } => {
                if !pragma_state.has_feature("signatures") {
                    let min = feature_min_version("signatures");
                    diagnostics.push(make_diagnostic(
                        n,
                        "subroutine signatures",
                        declared_version,
                        min,
                    ));
                }
            }

            // `$obj isa 'ClassName'` — infix operator; stable at v5.36
            NodeKind::Binary { op, .. } if op == "isa" => {
                if !pragma_state.has_feature("isa") {
                    let min = feature_min_version("isa");
                    diagnostics.push(make_diagnostic(n, "isa", declared_version, min));
                }
            }

            // Smartmatch operator `~~` — enabled by `use feature 'switch'` in v5.10+,
            // deprecated in v5.38, and removed in v5.42.
            NodeKind::Binary { op, .. } if op == "~~" => {
                if declared_version >= SMARTMATCH_REMOVAL_VERSION {
                    diagnostics.push(make_smartmatch_diagnostic(
                        n,
                        declared_version,
                        DiagnosticSeverity::Error,
                    ));
                } else if declared_version >= SMARTMATCH_DEPRECATION_VERSION {
                    diagnostics.push(make_smartmatch_diagnostic(
                        n,
                        declared_version,
                        DiagnosticSeverity::Warning,
                    ));
                } else if !pragma_state.has_feature("switch") {
                    diagnostics.push(make_smartmatch_feature_diagnostic(n, declared_version));
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

/// Return the minimum Perl version for a `builtin::name` call or named import.
fn builtin_min_version(name: &str) -> PerlVersion {
    BUILTIN_FUNCTION_VERSIONS
        .iter()
        .find(|(builtin_name, _, _)| *builtin_name == name)
        .map(|(_, maj, min)| PerlVersion::new(*maj, *min))
        .unwrap_or(BUILTIN_BUNDLE_MIN_VERSION)
}

fn builtin_import_min_version(name: &str) -> PerlVersion {
    if let Some(bundle) = name.strip_prefix(':') {
        return parse_perl_version(bundle).unwrap_or(BUILTIN_BUNDLE_MIN_VERSION);
    }

    builtin_min_version(name)
}

fn builtin_import_names(arg: &str) -> Vec<String> {
    let trimmed = arg.trim();

    if let Some(inner) = trimmed.strip_prefix("qw(").and_then(|s| s.strip_suffix(')')) {
        return inner
            .split_whitespace()
            .filter(|name| !name.is_empty())
            .map(|name| name.to_string())
            .collect();
    }

    vec![trimmed.trim_matches(|c| c == '\'' || c == '"').to_string()]
}

/// Build a PL900 diagnostic for a version-incompatible feature use.
fn make_diagnostic(
    node: &Node,
    display: &str,
    declared_version: PerlVersion,
    min_version: (u32, u32),
) -> Diagnostic {
    make_diagnostic_with_details(
        node,
        display,
        declared_version,
        min_version,
        DiagnosticSeverity::Warning,
        Some(format!(
            "Update 'use v{}.{}' to 'use v{}.{}' or add 'use feature \"{}\";'",
            declared_version.major, declared_version.minor, min_version.0, min_version.1, display,
        )),
    )
}

fn make_given_when_default_diagnostic(
    node: &Node,
    declared_version: PerlVersion,
    severity: DiagnosticSeverity,
) -> Diagnostic {
    let (message, min_version) = match severity {
        DiagnosticSeverity::Error => (
            format!(
                "'given/when/default' was removed in Perl v5.42; declared version is v{}.{}",
                declared_version.major, declared_version.minor
            ),
            (5, 42),
        ),
        _ => (
            format!(
                "'given/when/default' is deprecated starting in Perl v5.38; declared version is v{}.{}",
                declared_version.major, declared_version.minor
            ),
            (5, 38),
        ),
    };

    Diagnostic {
        range: (node.location.start, node.location.end),
        severity,
        code: Some(DiagnosticCode::VersionIncompatFeature.as_str().to_string()),
        message,
        related_information: vec![],
        tags: vec![],
        suggestion: Some(format!(
            "Refactor `given` / `when` / `default` to `if` / `elsif` or another supported control-flow form; this feature is {} in v{}.{}.",
            if severity == DiagnosticSeverity::Error { "removed" } else { "deprecated" },
            min_version.0,
            min_version.1
        )),
    }
}

fn make_smartmatch_diagnostic(
    node: &Node,
    declared_version: PerlVersion,
    severity: DiagnosticSeverity,
) -> Diagnostic {
    let (message, min_version) = match severity {
        DiagnosticSeverity::Error => (
            format!(
                "smartmatch operator `~~` was removed in Perl v5.42; declared version is v{}.{}",
                declared_version.major, declared_version.minor
            ),
            (5, 42),
        ),
        _ => (
            format!(
                "smartmatch operator `~~` is deprecated starting in Perl v5.38; declared version is v{}.{}",
                declared_version.major, declared_version.minor
            ),
            (5, 38),
        ),
    };

    Diagnostic {
        range: (node.location.start, node.location.end),
        severity,
        code: Some(DiagnosticCode::VersionIncompatFeature.as_str().to_string()),
        message,
        related_information: vec![],
        tags: vec![],
        suggestion: Some(format!(
            "Replace smartmatch `~~` with `if` / `elsif`, `grep`, or `any` from List::Util; this operator is {} in v{}.{}.",
            if severity == DiagnosticSeverity::Error { "removed" } else { "deprecated" },
            min_version.0,
            min_version.1
        )),
    }
}

fn make_smartmatch_feature_diagnostic(node: &Node, declared_version: PerlVersion) -> Diagnostic {
    make_diagnostic_with_details(
        node,
        "smartmatch operator `~~`",
        declared_version,
        (5, 10),
        DiagnosticSeverity::Warning,
        Some(format!(
            "Update 'use v{}.{}' to 'use v5.10' or add 'use feature \"switch\";'",
            declared_version.major, declared_version.minor
        )),
    )
}

fn make_diagnostic_with_details(
    node: &Node,
    display: &str,
    declared_version: PerlVersion,
    min_version: (u32, u32),
    severity: DiagnosticSeverity,
    suggestion: Option<String>,
) -> Diagnostic {
    let message = format!(
        "'{}' requires Perl v{}.{}+; declared version is v{}.{}",
        display, min_version.0, min_version.1, declared_version.major, declared_version.minor,
    );

    Diagnostic {
        range: (node.location.start, node.location.end),
        severity,
        code: Some(DiagnosticCode::VersionIncompatFeature.as_str().to_string()),
        message,
        related_information: vec![],
        tags: vec![],
        suggestion,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser::Parser;
    use perl_tdd_support::must;

    fn version_compat_diags(source: &str) -> Vec<Diagnostic> {
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        check_version_compat(&ast, &mut diags);
        diags
    }

    #[test]
    fn version_bundle_state_is_visible_to_downstream_feature_checks() {
        let diags = version_compat_diags("use v5.36;\nsub greet ($name) { say $name; }\n");
        assert!(
            diags.is_empty(),
            "use v5.36 feature bundle should satisfy signatures/say checks; got: {diags:?}"
        );
    }

    #[test]
    fn explicit_feature_state_is_visible_to_downstream_feature_checks() {
        let diags =
            version_compat_diags("use v5.20;\nuse feature 'signatures';\nsub greet ($name) { }\n");
        assert!(
            diags.is_empty(),
            "explicit feature pragma should satisfy signatures check under lower declared versions; got: {diags:?}"
        );
    }

    #[test]
    fn builtin_named_import_is_distinct_from_builtin_bundle() {
        let only_named_import = version_compat_diags(
            "use v5.36;\nuse builtin 'true';\nmy $x = builtin::true();\nmy $y = builtin::inf();\n",
        );
        assert!(
            only_named_import.iter().any(|d| d.message.contains("builtin::inf")),
            "named import should not act like the full builtin bundle; expected builtin::inf warning, got: {only_named_import:?}"
        );
        assert!(
            only_named_import.iter().all(|d| !d.message.contains("builtin::true")),
            "named import should satisfy builtin::true call; got: {only_named_import:?}"
        );

        let bundle_decl =
            version_compat_diags("use v5.36;\nuse builtin;\nmy $x = builtin::inf();\n");
        assert!(
            bundle_decl.iter().any(|d| d.message.contains("'use builtin' requires Perl v5.40+")),
            "bundle import should be validated separately and warn below v5.40; got: {bundle_decl:?}"
        );
    }
}
