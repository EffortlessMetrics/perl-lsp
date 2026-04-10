//! Duplicate hash key detection lint (PL408)
//!
//! Detects hash literals and hash reference constructors where the same key
//! string appears more than once. The last value silently wins at runtime,
//! so duplicate keys almost always indicate a copy-paste bug.
//!
//! Only statically-known keys (string literals and auto-quoted bareword
//! identifiers that became strings via `=>`) are compared. Variable-valued
//! keys (`$key => ...`) are skipped to avoid false positives.
//!
//! # Diagnostic codes
//!
//! | Code | Severity | Description |
//! |------|----------|-------------|
//! | `PL408` | Warning | Hash key appears more than once in the same literal |

use std::collections::HashMap;

use perl_diagnostics_codes::DiagnosticCode;
use perl_parser_core::ast::{Node, NodeKind};

use super::super::walker::walk_node;
use perl_lsp_diagnostic_types::{Diagnostic, DiagnosticSeverity, RelatedInformation};

/// Extract the static string value of a hash key node, if statically known.
///
/// Returns `Some(key)` for `NodeKind::String` and `NodeKind::Number` keys.
/// Returns `None` for variable keys and other dynamic expressions.
fn static_key_value(key: &Node) -> Option<String> {
    match &key.kind {
        NodeKind::String { value, .. } => Some(value.clone()),
        NodeKind::Number { value } => Some(value.clone()),
        _ => None,
    }
}

/// Check for duplicate keys in a single `HashLiteral` node.
///
/// Each duplicate key beyond the first occurrence produces a `PL408` diagnostic
/// pointing at the duplicate entry. The first occurrence is referenced in the
/// `related_information` field.
fn check_hash_literal_pairs(pairs: &[(Node, Node)], diagnostics: &mut Vec<Diagnostic>) {
    // Map: static_key_string -> (first_occurrence_start, first_occurrence_end)
    let mut seen: HashMap<String, (usize, usize)> = HashMap::new();

    for (key, _value) in pairs {
        let Some(key_text) = static_key_value(key) else {
            continue;
        };

        if let Some(&(first_start, first_end)) = seen.get(&key_text) {
            diagnostics.push(Diagnostic {
                range: (key.location.start, key.location.end),
                severity: DiagnosticSeverity::Warning,
                code: Some(DiagnosticCode::DuplicateHashKey.as_str().to_string()),
                message: format!(
                    "Duplicate hash key '{}' -- only the last value will be used",
                    key_text
                ),
                related_information: vec![RelatedInformation {
                    location: (first_start, first_end),
                    message: format!("Key '{}' first defined here", key_text),
                }],
                tags: Vec::new(),
                suggestion: Some(format!(
                    "Remove the earlier '{}' entry or rename this key",
                    key_text
                )),
            });
        } else {
            seen.insert(key_text, (key.location.start, key.location.end));
        }
    }
}

/// Check for duplicate keys in hash literals throughout the AST.
///
/// Walks the entire AST and checks every `HashLiteral` node for repeated
/// static keys. Emits a `PL408` warning for each duplicate occurrence.
pub fn check_duplicate_hash_keys(node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    walk_node(node, &mut |n| {
        if let NodeKind::HashLiteral { pairs } = &n.kind {
            check_hash_literal_pairs(pairs, diagnostics);
        }
    });
}
