use std::collections::HashMap;

use perl_parser_core::ast::Node;

use super::super::super::internal_types::Diagnostic;
use super::diagnostic::duplicate_key_diagnostic;
use super::key::static_key_value;

type FirstOccurrenceByKey = HashMap<String, (usize, usize)>;

/// Check for duplicate keys in a single `HashLiteral` node.
///
/// Each duplicate key beyond the first occurrence produces a `PL408` diagnostic
/// pointing at the duplicate entry. The first occurrence is referenced in the
/// `related_information` field.
pub(super) fn check_hash_literal_pairs(pairs: &[(Node, Node)], diagnostics: &mut Vec<Diagnostic>) {
    let mut seen: FirstOccurrenceByKey = HashMap::new();

    for (key, _value) in pairs {
        let Some(key_text) = static_key_value(key) else {
            continue;
        };

        if let Some(&first_occurrence) = seen.get(&key_text) {
            diagnostics.push(duplicate_key_diagnostic(key, &key_text, first_occurrence));
        } else {
            seen.insert(key_text, (key.location.start, key.location.end));
        }
    }
}
