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

mod diagnostic;
mod key;
mod pairs;

use perl_parser_core::ast::{Node, NodeKind};

use super::super::internal_types::Diagnostic;
use super::super::walker::walk_node;
use pairs::check_hash_literal_pairs;

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
mod tests;
