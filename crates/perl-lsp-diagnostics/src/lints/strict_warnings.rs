//! Strict and warnings pragma lint checks
//!
//! This module provides functionality for checking if 'use strict' and 'use warnings'
//! pragmas are present in Perl code, and detecting misspelled pragma names.
//!
//! # Diagnostic codes
//!
//! | Code | Severity | Description |
//! |------|----------|-------------|
//! | `missing-strict` | Information | `use strict` pragma not found |
//! | `missing-warnings` | Information | `use warnings` pragma not found |
//! | `misspelled-pragma` | Warning | Pragma name appears misspelled |

use perl_diagnostics_codes::DiagnosticCode;
use perl_parser_core::ast::{Node, NodeKind};
use perl_pragma::PragmaTracker;

use super::super::walker::walk_node;
use perl_lsp_diagnostic_types::{Diagnostic, DiagnosticSeverity, RelatedInformation};

/// Known pragma names and their common misspellings.
///
/// Each entry maps a correct pragma to a list of known typos.
const PRAGMA_TYPOS: &[(&str, &[&str])] = &[
    ("strict", &["structs", "strickt", "stricts", "stirct", "stict", "strct", "srict"]),
    ("warnings", &["warning", "warningss", "warnigns", "warrnings", "warnins", "warnnigs"]),
    ("utf8", &["utf-8", "uft8", "utf88"]),
    ("feature", &["feaure", "featrue", "feture"]),
    ("constant", &["constanst", "contstant", "costant", "consant"]),
    ("parent", &["parrent", "parnet"]),
    ("base", &["basse", "bace"]),
    ("lib", &["lbi", "libb"]),
    ("Carp", &["Carb", "Crap"]),
];

/// Check for common strict/warnings issues
///
/// This function checks if 'use strict' and 'use warnings' pragmas are present
/// in the code and generates informational diagnostics if they are missing.
/// It also detects misspelled pragma names and provides "Did you mean?" suggestions.
pub fn check_strict_warnings(node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    // Do not suggest strict/warnings for empty, whitespace-only, comment-only,
    // or shebang-only files — the file has no executable content yet.
    if let NodeKind::Program { statements } = &node.kind
        && statements.is_empty()
    {
        return;
    }

    let pragma_map = PragmaTracker::build(node);
    let mut has_strict = pragma_map.iter().any(|(_, state)| {
        state.strict_vars || state.strict_subs || state.strict_refs || state.signatures_strict
    });
    let mut has_warnings = pragma_map.iter().any(|(_, state)| state.warnings);

    // OO frameworks that implicitly provide strict+warnings
    const IMPLICIT_STRICT_MODULES: &[&str] = &[
        "Moo",
        "Moose",
        "MooseX::StrictConstructor",
        "Modern::Perl",
        "Dancer2",
        "Catalyst",
        "Mojolicious",
        "Mojo::Base",
    ];

    // Check if 'use strict' and 'use warnings' are present,
    // and detect misspelled pragmas
    walk_node(node, &mut |n| {
        if let NodeKind::Use { module, .. } = &n.kind {
            if module == "strict" {
                has_strict = true;
            } else if module == "warnings" {
                has_warnings = true;
            } else if module.starts_with('v')
                || module.chars().next().is_some_and(|c| c.is_ascii_digit())
            {
                // Version pragmas are already reflected in the shared pragma map.
            } else if IMPLICIT_STRICT_MODULES.contains(&module.as_str()) {
                has_strict = true;
                has_warnings = true;
            } else {
                // Check for misspelled pragmas
                check_misspelled_pragma(module, n, diagnostics);
            }
        }
    });

    // Add diagnostics if missing
    if !has_strict {
        diagnostics.push(Diagnostic {
            range: (0, 0),
            severity: DiagnosticSeverity::Information,
            code: Some(DiagnosticCode::MissingStrict.as_str().to_string()),
            message: "Consider adding 'use strict;' for better error checking".to_string(),
            related_information: vec![
                RelatedInformation {
                    location: (0, 0),
                    message: "💡 Add 'use strict;' at the beginning of your script".to_string(),
                },
                RelatedInformation {
                    location: (0, 0),
                    message: "ℹ️ The 'use strict' pragma enforces good coding practices by requiring variable declarations, disabling barewords, and preventing symbolic references.".to_string(),
                }
            ],
            tags: Vec::new(),
            suggestion: Some("Add 'use strict;' at the top of the file".to_string()),
        });
    }

    if !has_warnings {
        diagnostics.push(Diagnostic {
            range: (0, 0),
            severity: DiagnosticSeverity::Information,
            code: Some(DiagnosticCode::MissingWarnings.as_str().to_string()),
            message: "Consider adding 'use warnings;' for better error detection".to_string(),
            related_information: vec![
                RelatedInformation {
                    location: (0, 0),
                    message: "💡 Add 'use warnings;' at the beginning of your script".to_string(),
                },
                RelatedInformation {
                    location: (0, 0),
                    message: "ℹ️ The 'use warnings' pragma enables helpful warning messages about questionable constructs, uninitialized values, and deprecated features.".to_string(),
                }
            ],
            tags: Vec::new(),
            suggestion: Some("Add 'use warnings;' at the top of the file".to_string()),
        });
    }
}

/// Check if a module name is a misspelling of a known pragma.
///
/// Produces a `misspelled-pragma` warning with a "Did you mean?" suggestion
/// when the module name matches a known typo.
fn check_misspelled_pragma(module: &str, node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    for &(correct, typos) in PRAGMA_TYPOS {
        if typos.contains(&module) {
            diagnostics.push(Diagnostic {
                range: (node.location.start, node.location.end),
                severity: DiagnosticSeverity::Warning,
                code: Some(DiagnosticCode::MisspelledPragma.as_str().to_string()),
                message: format!(
                    "Did you mean 'use {};'? '{}' is not a known pragma",
                    correct, module
                ),
                related_information: vec![RelatedInformation {
                    location: (node.location.start, node.location.end),
                    message: format!("Replace '{}' with '{}'", module, correct),
                }],
                tags: Vec::new(),
                suggestion: Some(format!("Replace 'use {};' with 'use {};'", module, correct)),
            });
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser::Parser;
    use perl_tdd_support::must;

    fn strict_warnings_diags(source: &str) -> Vec<Diagnostic> {
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        check_strict_warnings(&ast, &mut diags);
        diags
    }

    #[test]
    fn empty_file_no_strict_warnings_diagnostic() {
        assert!(
            strict_warnings_diags("").is_empty(),
            "empty file should not get strict/warnings diagnostics"
        );
    }

    #[test]
    fn whitespace_only_no_strict_warnings_diagnostic() {
        assert!(
            strict_warnings_diags("   \n\t\n").is_empty(),
            "whitespace-only file should not get strict/warnings diagnostics"
        );
    }

    #[test]
    fn comment_only_no_strict_warnings_diagnostic() {
        assert!(
            strict_warnings_diags("# just a comment\n").is_empty(),
            "comment-only file should not get strict/warnings diagnostics"
        );
    }

    #[test]
    fn shebang_only_no_strict_warnings_diagnostic() {
        assert!(
            strict_warnings_diags("#!/usr/bin/perl\n").is_empty(),
            "shebang-only file should not get strict/warnings diagnostics"
        );
    }

    #[test]
    fn non_empty_file_without_strict_still_gets_diagnostic() {
        let diags = strict_warnings_diags("my $x = 1;\n");
        assert!(
            diags.iter().any(|d| d.code.as_deref() == Some("PL100")),
            "non-empty file without strict should still get missing-strict diagnostic"
        );
    }

    #[test]
    fn file_with_strict_and_warnings_no_diagnostic() {
        let diags = strict_warnings_diags("use strict;\nuse warnings;\nmy $x = 1;\n");
        let has_strict_warn =
            diags.iter().any(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")));
        assert!(
            !has_strict_warn,
            "file with both pragmas should get no strict/warnings diagnostic"
        );
    }

    #[test]
    fn version_pragma_suppresses_strict_warnings_diagnostic() {
        let diags = strict_warnings_diags("use v5.40;\nmy $x = 1;\n");
        let has_strict_warn =
            diags.iter().any(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")));
        assert!(!has_strict_warn, "use v5.40 should suppress strict/warnings diagnostics");
    }

    #[test]
    fn numeric_version_pragma_suppresses_strict_warnings_diagnostic() {
        let diags = strict_warnings_diags("use 5.040;\nmy $x = 1;\n");
        let has_strict_warn =
            diags.iter().any(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")));
        assert!(!has_strict_warn, "use 5.040 should suppress strict/warnings diagnostics");
    }

    #[test]
    fn developer_version_pragma_suppresses_strict_warnings_diagnostic() {
        let diags = strict_warnings_diags("use 5.040_001;\nmy $x = 1;\n");
        let has_strict_warn =
            diags.iter().any(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")));
        assert!(!has_strict_warn, "use 5.040_001 should suppress strict/warnings diagnostics");
    }

    #[test]
    fn v5_36_suppresses_both_strict_and_warnings_diagnostics() {
        // use v5.36 enables both strict and warnings via the feature bundle.
        // Neither PL100 (missing-strict) nor PL101 (missing-warnings) should fire.
        let diags = strict_warnings_diags("use v5.36;\nsub foo ($x) { my $y = $x; }\n");
        let has_strict_warn =
            diags.iter().any(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")));
        assert!(
            !has_strict_warn,
            "use v5.36 should suppress both missing-strict and missing-warnings diagnostics"
        );
    }

    #[test]
    fn v5_36_numeric_form_suppresses_both_strict_and_warnings() {
        // use 5.036 is the numeric form of use v5.36.
        let diags = strict_warnings_diags("use 5.036;\nmy $x = 1;\n");
        let has_strict_warn =
            diags.iter().any(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")));
        assert!(
            !has_strict_warn,
            "use 5.036 should suppress both missing-strict and missing-warnings diagnostics"
        );
    }

    #[test]
    fn v5_12_suppresses_strict_but_not_missing_warnings() {
        let diags = strict_warnings_diags("use v5.12;\nmy $x = 1;\n");
        assert!(
            diags.iter().all(|d| d.code.as_deref() != Some("PL100")),
            "use v5.12 should imply strict"
        );
        assert!(
            diags.iter().any(|d| d.code.as_deref() == Some("PL101")),
            "use v5.12 should not imply warnings"
        );
    }

    #[test]
    fn crlf_only_no_strict_warnings_diagnostic() {
        // Windows CRLF line endings in an otherwise-empty file — both \r and \n
        // are whitespace-skipped by the lexer, so statements remains empty.
        assert!(
            strict_warnings_diags("\r\n\r\n").is_empty(),
            "CRLF-only file should not get strict/warnings diagnostics"
        );
    }

    #[test]
    fn shebang_plus_comment_no_strict_warnings_diagnostic() {
        // Combined: shebang line followed by a comment — both are skipped as trivia.
        assert!(
            strict_warnings_diags("#!/usr/bin/perl\n# a comment\n").is_empty(),
            "shebang + comment file should not get strict/warnings diagnostics"
        );
    }

    #[test]
    fn misspelled_pragma_in_non_empty_file_still_detected() {
        // The guard must not suppress misspelled-pragma detection in real files.
        // MisspelledPragma = PL111; the guard only fires for empty statements vec.
        let diags = strict_warnings_diags("use structs;\nmy $x = 1;\n");
        assert!(
            diags.iter().any(|d| d.code.as_deref() == Some("PL111")),
            "misspelled pragma should still be detected in non-empty files"
        );
    }

    #[test]
    fn pod_only_no_strict_warnings_diagnostic() {
        // POD blocks are consumed as trivia by the lexer, so a POD-only file
        // produces Program { statements: [] }.  The empty-file guard fires and
        // suppresses PL100/PL101 — the same as comment-only files.
        // EDGE_CASES.md documents this behaviour.
        let pod_only = "=head1 NAME\n\nMy::Module - description\n\n=cut\n";
        assert!(
            strict_warnings_diags(pod_only).is_empty(),
            "POD-only file should not get strict/warnings diagnostics — POD is trivia"
        );
    }
}
