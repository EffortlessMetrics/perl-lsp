//! Perl version compatibility lint checks
//!
//! Detects when code uses features that require a higher Perl version than
//! what is declared via `use v5.XX`, `use 5.0XX`, or `use feature '...'`.
//!
//! # Diagnostic codes
//!
//! | Code | Severity | Description |
//! |------|----------|-------------|
//! | `version-compat` | Warning | Feature used without sufficient version declaration |

use perl_parser_core::ast::{Node, NodeKind};

use super::super::walker::walk_node;
use perl_lsp_diagnostic_types::{Diagnostic, DiagnosticSeverity, RelatedInformation};

/// Minimum Perl version required for a feature, expressed as `(major, minor)`.
///
/// We normalise all version forms to a `(5, minor)` tuple where `minor` is the
/// integer minor version (e.g. 5.10, 5.20, 5.36, 5.38, 5.40).
type PerlVersion = (u32, u32);

/// A feature that requires a minimum Perl version.
struct FeatureRequirement {
    /// Human-readable name shown in diagnostic messages.
    name: &'static str,
    /// Minimum Perl version where the feature is available (possibly experimental).
    min_version: PerlVersion,
}

/// Map of `use feature '...'` pragma names to their minimum versions.
const FEATURE_PRAGMA_MAP: &[(&str, PerlVersion)] = &[
    ("say", (5, 10)),
    ("state", (5, 10)),
    ("switch", (5, 10)),
    ("unicode_strings", (5, 12)),
    ("unicode_eval", (5, 16)),
    ("evalbytes", (5, 16)),
    ("current_sub", (5, 16)),
    ("fc", (5, 16)),
    ("lexical_subs", (5, 18)),
    ("postderef", (5, 20)),
    ("postderef_qq", (5, 20)),
    ("signatures", (5, 20)),
    ("refaliasing", (5, 22)),
    ("bitwise", (5, 22)),
    ("declared_refs", (5, 26)),
    ("isa", (5, 32)),
    ("indirect", (5, 32)),
    ("multidimensional", (5, 34)),
    ("bareword_filehandles", (5, 34)),
    ("try", (5, 34)),
    ("defer", (5, 36)),
    ("extra_paired_delimiters", (5, 36)),
    ("module_true", (5, 38)),
    ("class", (5, 38)),
];

/// AST node kinds that require specific minimum Perl versions.
const NODE_REQUIREMENTS: &[(&str, FeatureRequirement)] = &[
    ("Try", FeatureRequirement { name: "try/catch", min_version: (5, 34) }),
    ("Class", FeatureRequirement { name: "class syntax", min_version: (5, 38) }),
    ("Method", FeatureRequirement { name: "method keyword", min_version: (5, 38) }),
    ("Given", FeatureRequirement { name: "given/when", min_version: (5, 10) }),
    ("When", FeatureRequirement { name: "given/when", min_version: (5, 10) }),
];

/// Check for Perl version compatibility issues.
///
/// Walks the AST to determine the declared Perl version (from `use v5.XX`,
/// `use 5.0XX`, or `use feature '...'`) and then checks for usage of features
/// that require a higher version than declared.
pub fn check_version_compat(node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    // Phase 1: collect version declarations
    let mut declared_version: Option<PerlVersion> = None;
    let mut enabled_features: Vec<String> = Vec::new();

    walk_node(node, &mut |n| {
        if let NodeKind::Use { module, args, .. } = &n.kind {
            // Check for version declarations: `use v5.38`, `use 5.036`
            if let Some(ver) = parse_version_from_module(module) {
                match declared_version {
                    Some(existing) if ver > existing => declared_version = Some(ver),
                    None => declared_version = Some(ver),
                    _ => {}
                }
            }

            // Check for `use feature 'xxx'`
            if module == "feature" {
                for arg in args {
                    let cleaned = arg.trim_matches(|c| c == '\'' || c == '"');
                    enabled_features.push(cleaned.to_string());
                }
            }
        }
    });

    // If no version is declared, we cannot warn about compatibility
    // (the user may be targeting the latest Perl).
    let declared = match declared_version {
        Some(v) => v,
        None => return,
    };

    // Phase 2: check feature pragma usage against declared version
    walk_node(node, &mut |n| {
        if let NodeKind::Use { module, args, .. } = &n.kind
            && module == "feature"
        {
            for arg in args {
                let cleaned = arg.trim_matches(|c| c == '\'' || c == '"');
                if let Some(&(_, required)) =
                    FEATURE_PRAGMA_MAP.iter().find(|&&(name, _)| name == cleaned)
                    && declared < required
                {
                    diagnostics.push(make_diagnostic(
                        n,
                        &format!("feature '{cleaned}'"),
                        required,
                        declared,
                    ));
                }
            }
        }
    });

    // Phase 3: check AST node kinds against declared version
    walk_node(node, &mut |n| {
        let kind_name = n.kind.kind_name();
        for &(node_kind, ref req) in NODE_REQUIREMENTS {
            if kind_name == node_kind {
                // Only warn if the feature is not explicitly enabled via `use feature`
                let feature_name = match node_kind {
                    "Try" => Some("try"),
                    "Class" | "Method" => Some("class"),
                    "Given" | "When" => Some("switch"),
                    _ => None,
                };
                let explicitly_enabled =
                    feature_name.is_some_and(|f| enabled_features.iter().any(|e| e == f));
                if !explicitly_enabled && declared < req.min_version {
                    diagnostics.push(make_diagnostic(n, req.name, req.min_version, declared));
                }
            }
        }

        // Check for `say` as a function call
        if let NodeKind::FunctionCall { name, .. } = &n.kind
            && name == "say"
            && declared < (5, 10)
            && !enabled_features.iter().any(|f| f == "say")
        {
            diagnostics.push(make_diagnostic(n, "say()", (5, 10), declared));
        }

        // Check for `state` declarations
        if let NodeKind::VariableDeclaration { declarator, .. } = &n.kind
            && declarator == "state"
            && declared < (5, 10)
            && !enabled_features.iter().any(|f| f == "state")
        {
            diagnostics.push(make_diagnostic(n, "state variables", (5, 10), declared));
        }

        // Check for subroutine signatures
        if let NodeKind::Subroutine { signature: Some(_), .. } = &n.kind
            && declared < (5, 20)
            && !enabled_features.iter().any(|f| f == "signatures")
        {
            diagnostics.push(make_diagnostic(n, "subroutine signatures", (5, 20), declared));
        }
    });
}

/// Parse a Perl version from a module string.
///
/// Handles forms like:
/// - `v5.38` -> (5, 38)
/// - `v5.38.0` -> (5, 38)
/// - `5.038` -> (5, 38)  (three-digit minor)
/// - `5.38` -> (5, 38)   (two-digit minor)
/// - `5.038000` -> (5, 38)
fn parse_version_from_module(module: &str) -> Option<PerlVersion> {
    let trimmed = module.strip_prefix('v').unwrap_or(module);

    let parts: Vec<&str> = trimmed.split('.').collect();
    if parts.is_empty() {
        return None;
    }

    let major: u32 = parts.first()?.parse().ok()?;
    if major != 5 {
        return None; // Only Perl 5
    }

    let minor_str = parts.get(1)?;
    let minor_raw: u32 = minor_str.parse().ok()?;

    // Normalise: 3-digit+ minors like 036 mean 36, 010 means 10, etc.
    // But 38 just means 38.
    let minor = if minor_str.len() >= 3 {
        // e.g. "036" -> 36, "038" -> 38, "010" -> 10, "010001" -> 10
        // Take first 3 digits and divide by (10^(len-2)) effectively,
        // or more simply: the first N/3 significant digits.
        // Perl convention: 5.036 = 5.36, 5.010 = 5.10, 5.010001 = 5.10
        minor_raw / 10u32.pow((minor_str.len() as u32).saturating_sub(3))
    } else {
        minor_raw
    };

    Some((major, minor))
}

/// Format a version tuple as a human-readable string.
fn format_version(v: PerlVersion) -> String {
    format!("v{}.{}", v.0, v.1)
}

/// Create a version compatibility diagnostic.
fn make_diagnostic(
    node: &Node,
    feature_name: &str,
    required: PerlVersion,
    declared: PerlVersion,
) -> Diagnostic {
    Diagnostic {
        range: (node.location.start, node.location.end),
        severity: DiagnosticSeverity::Warning,
        code: Some("version-compat".to_string()),
        message: format!(
            "{} requires Perl {} but the declared version is {}",
            feature_name,
            format_version(required),
            format_version(declared),
        ),
        related_information: vec![RelatedInformation {
            location: (node.location.start, node.location.end),
            message: format!(
                "Update version declaration to 'use {};' or higher",
                format_version(required),
            ),
        }],
        tags: Vec::new(),
        suggestion: Some(format!(
            "Update version declaration to 'use {};' or higher",
            format_version(required),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_v_string_version() {
        assert_eq!(parse_version_from_module("v5.38"), Some((5, 38)));
        assert_eq!(parse_version_from_module("v5.10"), Some((5, 10)));
        assert_eq!(parse_version_from_module("v5.38.0"), Some((5, 38)));
    }

    #[test]
    fn parse_numeric_version() {
        assert_eq!(parse_version_from_module("5.036"), Some((5, 36)));
        assert_eq!(parse_version_from_module("5.010"), Some((5, 10)));
        assert_eq!(parse_version_from_module("5.038"), Some((5, 38)));
    }

    #[test]
    fn parse_two_digit_version() {
        assert_eq!(parse_version_from_module("5.38"), Some((5, 38)));
        assert_eq!(parse_version_from_module("5.10"), Some((5, 10)));
    }

    #[test]
    fn parse_non_perl5_returns_none() {
        assert_eq!(parse_version_from_module("6.0"), None);
        assert_eq!(parse_version_from_module("Foo::Bar"), None);
    }

    #[test]
    fn parse_six_digit_version() {
        assert_eq!(parse_version_from_module("5.010001"), Some((5, 10)));
        assert_eq!(parse_version_from_module("5.038000"), Some((5, 38)));
    }
}
