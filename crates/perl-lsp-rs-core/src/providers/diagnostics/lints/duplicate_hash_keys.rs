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

use perl_diagnostics::codes::DiagnosticCode;
use perl_parser_core::ast::{Node, NodeKind};

use super::super::internal_types::{Diagnostic, RelatedInformation};
use super::super::walker::walk_node;
use perl_diagnostics::codes::DiagnosticSeverity;

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

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser::Parser;
    use perl_tdd_support::{must, must_some};

    fn dup_key_diags(source: &str) -> Vec<Diagnostic> {
        let ast = must(Parser::new(source).parse());
        let mut diags = Vec::new();
        check_duplicate_hash_keys(&ast, &mut diags);
        diags
    }

    fn has_pl408(diags: &[Diagnostic]) -> bool {
        diags.iter().any(|d| d.code.as_deref() == Some("PL408"))
    }

    #[test]
    fn duplicate_string_key_is_flagged() {
        let diags = dup_key_diags(r#"my %h = (foo => 1, foo => 2);"#);
        assert!(
            has_pl408(&diags),
            "duplicate string key 'foo' should be flagged as PL408: {diags:?}"
        );
    }

    #[test]
    fn unique_keys_not_flagged() {
        let diags = dup_key_diags(r#"my %h = (foo => 1, bar => 2, baz => 3);"#);
        assert!(!has_pl408(&diags), "unique keys should not be flagged: {diags:?}");
    }

    #[test]
    fn three_occurrences_two_diagnostics() {
        let diags = dup_key_diags(r#"my %h = (x => 1, x => 2, x => 3);"#);
        let count = diags.iter().filter(|d| d.code.as_deref() == Some("PL408")).count();
        assert_eq!(
            count, 2,
            "three occurrences of same key should produce two PL408 diagnostics: {diags:?}"
        );
    }

    #[test]
    fn duplicate_numeric_key_is_flagged() {
        let diags = dup_key_diags(r#"my %h = (1 => "a", 1 => "b");"#);
        assert!(has_pl408(&diags), "duplicate numeric key should be flagged: {diags:?}");
    }

    #[test]
    fn dynamic_variable_key_not_flagged() {
        let diags = dup_key_diags(r#"my $k = "foo"; my %h = ($k => 1, $k => 2);"#);
        assert!(!has_pl408(&diags), "dynamic variable keys should not be flagged: {diags:?}");
    }

    #[test]
    fn duplicate_message_names_the_key() {
        let diags = dup_key_diags(r#"my %h = (alpha => 1, alpha => 2);"#);
        let diag = must_some(diags.iter().find(|d| d.code.as_deref() == Some("PL408")));
        assert!(
            diag.message.contains("alpha"),
            "PL408 message should name the duplicate key: {}",
            diag.message
        );
    }

    #[test]
    fn duplicate_diagnostic_has_related_info_for_first_occurrence() {
        let diags = dup_key_diags(r#"my %h = (name => "Alice", name => "Bob");"#);
        let diag = must_some(diags.iter().find(|d| d.code.as_deref() == Some("PL408")));
        assert!(
            !diag.related_information.is_empty(),
            "PL408 should include related information pointing to first occurrence"
        );
    }

    #[test]
    fn nested_hash_inner_duplicate_flagged() {
        let diags = dup_key_diags(r#"my %outer = (inner => { x => 1, x => 2 });"#);
        assert!(
            has_pl408(&diags),
            "duplicate key inside nested hash ref should be flagged: {diags:?}"
        );
    }

    #[test]
    fn empty_hash_not_flagged() {
        let diags = dup_key_diags(r#"my %h = ();"#);
        assert!(!has_pl408(&diags), "empty hash should not be flagged: {diags:?}");
    }

    #[test]
    fn single_pair_hash_not_flagged() {
        let diags = dup_key_diags(r#"my %h = (key => "value");"#);
        assert!(!has_pl408(&diags), "single-pair hash should not be flagged: {diags:?}");
    }
}
