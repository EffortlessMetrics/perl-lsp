//! Strict and warnings pragma lint checks
//!
//! This module provides functionality for checking if 'use strict' and 'use warnings'
//! pragmas are present in Perl code.

use perl_parser_core::ast::{Node, NodeKind};

use super::super::types::{Diagnostic, DiagnosticSeverity, RelatedInformation};
use super::super::walker::walk_node;

/// Check for common strict/warnings issues
///
/// This function checks if 'use strict' and 'use warnings' pragmas are present
/// in the code and generates informational diagnostics if they are missing.
pub fn check_strict_warnings(node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    let mut has_strict = false;
    let mut has_warnings = false;

    // Check if 'use strict' and 'use warnings' are present
    walk_node(node, &mut |n| {
        if let NodeKind::Use { module, .. } = &n.kind {
            if module == "strict" {
                has_strict = true;
            } else if module == "warnings" {
                has_warnings = true;
            }
        }
    });

    // Add diagnostics if missing
    if !has_strict {
        diagnostics.push(Diagnostic {
            range: (0, 0),
            severity: DiagnosticSeverity::Information,
            code: Some("missing-strict".to_string()),
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
        });
    }

    if !has_warnings {
        diagnostics.push(Diagnostic {
            range: (0, 0),
            severity: DiagnosticSeverity::Information,
            code: Some("missing-warnings".to_string()),
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
        });
    }
}
