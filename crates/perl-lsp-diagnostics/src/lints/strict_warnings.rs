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
    let mut has_strict = false;
    let mut has_warnings = false;

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
