//! Diagnostic deduplication
//!
//! This module provides functionality for removing duplicate diagnostics
//! to avoid reporting the same issue multiple times.

use super::internal_types::Diagnostic;
use perl_diagnostics::codes::DiagnosticSeverity;

/// Byte-distance threshold for cascade suppression.
///
/// Parse-error diagnostics whose start offsets fall within this many bytes
/// of the current cluster head are treated as downstream cascades and
/// suppressed.  Only the first diagnostic in each cluster is kept.
///
/// # Rationale for 10 bytes
///
/// This threshold targets **intra-statement cascades**: when the parser
/// encounters a syntax error inside an expression it may emit several tightly-
/// clustered errors (e.g. `my $x = = 1;` can trigger two `UnexpectedToken`
/// errors both pointing at the same offset).  After exact-duplicate removal
/// (pass 1) the remaining nearby errors are typically within a few bytes of
/// each other — well inside this threshold.  Tokens in a single syntactic unit
/// rarely span more than 10 bytes in practice, so the threshold catches
/// same-expression noise without suppressing genuinely independent errors on
/// separate lines.
///
/// **Design note — why suppress entirely rather than downgrade to `Information`:**
/// An alternative approach would demote cascade errors to `Information` severity
/// instead of dropping them.  This was considered and rejected: `Information`
/// diagnostics still appear as blue underlines in the editor gutter at positions
/// the user did not make a mistake, which creates confusing noise without adding
/// actionable context.  Complete suppression is the right choice for cascade
/// errors — the root-cause error (the cluster head) is the only marker the
/// user needs.  Lint and scope-analysis diagnostics are never suppressed.
const CASCADE_THRESHOLD_BYTES: usize = 10;

/// Parse-error diagnostic codes emitted by the parser layer.
///
/// Only diagnostics with these codes are considered candidates for cascade
/// suppression.  Scope-analysis warnings and lint hints are never suppressed.
const PARSE_ERROR_CODES: &[&str] = &["PL001", "PL002", "PL003"];

/// Returns `true` if this diagnostic was produced by the parser (not by a lint
/// or scope-analysis pass).
fn is_parse_error_diagnostic(d: &Diagnostic) -> bool {
    d.severity == DiagnosticSeverity::Error
        && d.code.as_deref().map(|c| PARSE_ERROR_CODES.contains(&c)).unwrap_or(false)
}

/// Suppress cascading parse-error diagnostics using location clustering.
///
/// After exact-duplicate removal the parser may still emit several tightly-
/// grouped diagnostics that all stem from a single syntax error (e.g. an
/// unclosed delimiter causes "unexpected token" errors on every subsequent
/// statement).  This pass groups adjacent parse-error diagnostics by byte
/// proximity and retains only the first diagnostic in each cluster.
///
/// Algorithm:
/// 1. Separate diagnostics into parse-errors and everything else.
/// 2. Sort parse-errors by start byte offset.
/// 3. Walk the sorted list; start a new cluster whenever the current
///    diagnostic's start offset exceeds the cluster-head's start offset by
///    more than [`CASCADE_THRESHOLD_BYTES`].
/// 4. Recombine kept parse-errors with the untouched non-parse-error
///    diagnostics.
fn suppress_cascades(diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    if diagnostics.is_empty() {
        return diagnostics;
    }

    // Partition: parse-errors (cascade candidates) vs. everything else.
    let (mut parse_errors, others): (Vec<Diagnostic>, Vec<Diagnostic>) =
        diagnostics.into_iter().partition(is_parse_error_diagnostic);

    if parse_errors.len() <= 1 {
        // Nothing to suppress — fast path.
        parse_errors.extend(others);
        return parse_errors;
    }

    // Sort parse-errors by start position so adjacent errors are neighbours.
    parse_errors.sort_by_key(|d| d.range.0);

    // Walk the sorted list, keeping only cluster heads.
    let mut kept: Vec<Diagnostic> = Vec::with_capacity(parse_errors.len());

    for diag in parse_errors {
        match kept.last() {
            None => kept.push(diag),
            Some(head) => {
                let gap = diag.range.0.saturating_sub(head.range.0);
                if gap > CASCADE_THRESHOLD_BYTES {
                    // New cluster — this diagnostic is a fresh primary error.
                    kept.push(diag);
                }
                // else: within threshold → cascade, drop it.
            }
        }
    }

    kept.extend(others);
    kept
}

/// De-duplicate diagnostics to avoid reporting the same issue twice.
///
/// Performs two passes:
///
/// 1. **Exact deduplication** — removes diagnostics that share the same
///    range, severity, code, and message.
/// 2. **Cascade suppression** — groups adjacent parse-error diagnostics by
///    byte proximity and retains only the first in each cluster, suppressing
///    downstream noise caused by error-recovery.
pub fn deduplicate_diagnostics(diagnostics: &mut Vec<Diagnostic>) {
    // Pass 1: sort by range, severity, code, and message, then remove exact duplicates.
    diagnostics.sort_by(|a, b| {
        a.range
            .0
            .cmp(&b.range.0)
            .then(a.range.1.cmp(&b.range.1))
            .then(a.severity.cmp(&b.severity))
            .then(a.code.cmp(&b.code))
            .then(a.message.cmp(&b.message))
    });

    diagnostics.dedup_by(|a, b| {
        a.range == b.range && a.severity == b.severity && a.code == b.code && a.message == b.message
    });

    // Pass 2: cascade suppression — collapse clusters of nearby parse-errors.
    let after = suppress_cascades(std::mem::take(diagnostics));
    *diagnostics = after;
}
