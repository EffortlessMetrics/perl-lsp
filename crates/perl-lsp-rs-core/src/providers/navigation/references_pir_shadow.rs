//! PIR-A references shadow compare beside the legacy find-references provider.
//!
//! This module implements shadow comparison of the PIR-A lexical extractor path
//! against the legacy [`find_references_single_file`](super::find_references_single_file)
//! provider. It runs the PIR path **beside** the legacy path and records a
//! [`PirShadowCompareReceipt`] for scorecard aggregation.
//!
//! **Shadow-only — no cutover.** The live provider result is never changed by
//! this module. PR3 (#2635) performs the cutover. The legacy slice is passed in
//! and returned to the caller untouched; this module only *observes* it.
//!
//! # Usage
//!
//! ```rust,ignore
//! use perl_parser_core::{Parser, hir::lower_ast, pir::extract_lexical_facts};
//! use perl_lsp_rs_core::providers::navigation::references_pir_shadow::shadow_references_with_pir;
//!
//! let mut parser = Parser::new(source);
//! let output = parser.parse_with_recovery();
//! let hir = lower_ast(&output.ast);
//! let receipt = extract_lexical_facts(&hir);
//!
//! // `legacy` is the byte-offset ranges the live provider already computed.
//! let compare = shadow_references_with_pir(&receipt, &legacy, "x", 0);
//! assert!(!compare.provider_behavior_changed); // always false in PR2
//! ```

use std::collections::BTreeSet;

use perl_parser_core::pir::LexicalExtractorReceipt;

/// Latency sample from one shadow-compare run.
///
/// Populated by the *caller* (which owns the two timing spans), not by
/// [`shadow_references_with_pir`] itself — this struct only models the schema.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PirShadowLatency {
    /// Nanoseconds spent in the PIR-A (`extract_lexical_facts`) path.
    pub compiler_ns: u64,
    /// Nanoseconds spent in the legacy `find_references_single_file` path.
    pub legacy_ns: u64,
}

/// A range disagreement: the two paths agree a site exists near a position but
/// disagree on its exact byte offsets.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RangeDisagreement {
    /// Bare name of the variable (no sigil), for diagnostic purposes.
    pub variable: String,
    /// Compiler byte-offset range (from the PIR-A source anchor).
    pub compiler_range: (usize, usize),
    /// Legacy byte-offset range (from `find_references_single_file`).
    pub legacy_range: (usize, usize),
}

/// Why [`shadow_references_with_pir`] refused to run the comparison.
///
/// Only reasons the comparison can actually *produce* are modelled. The enum is
/// `#[non_exhaustive]` so PR3 can add reasons (e.g. stale-generation or
/// dynamic-boundary) once the corresponding inputs exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PirShadowRefusalReason {
    /// The target resolves to a package/global (`::`-qualified) name, which is
    /// not a same-file lexical and is out of scope for the lexical shadow.
    NotSameFileLexical,
    /// No anchored lexical facts are reachable for the request: the receipt has
    /// no bodies, or `target_body_idx` is out of range.
    NoAnchoredFacts,
    /// The upstream extractor flagged a behavior change. PR1/PR2 guarantee this
    /// is always `false`; if it is ever `true` the shadow refuses (never panics)
    /// rather than compare against an untrusted receipt.
    ProviderBehaviorChanged,
}

/// Receipt from one shadow-compare run.
///
/// Always produced. When the comparison was refused, `refusal_reason` carries
/// the reason and all counts are zero. `provider_behavior_changed` is always
/// `false` in PR2 (shadow-only).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PirShadowCompareReceipt {
    /// How many distinct reference sites the compiler (PIR-A) path found.
    pub compiler_candidate_count: usize,
    /// How many distinct reference sites the legacy path found.
    pub legacy_candidate_count: usize,
    /// Sites present in the legacy result but absent from the compiler result,
    /// excluding near-matches recorded in `range_disagreements`. Sorted ascending.
    pub missing_from_compiler: Vec<(usize, usize)>,
    /// Sites present in the compiler result but absent from the legacy result,
    /// excluding near-matches recorded in `range_disagreements`. Sorted ascending.
    pub extra_in_compiler: Vec<(usize, usize)>,
    /// Near-matches: one legacy-only site paired with one compiler-only site
    /// whose start is within [`RANGE_NEAR_MATCH_BYTES`] bytes. Disjoint from
    /// `missing_from_compiler` and `extra_in_compiler`.
    pub range_disagreements: Vec<RangeDisagreement>,
    /// Why the comparison was refused, or `None` when the comparison ran.
    pub refusal_reason: Option<PirShadowRefusalReason>,
    /// Whether the live provider result changed. Always `false` in PR2.
    pub provider_behavior_changed: bool,
    /// Optional latency sample (`None` unless the caller supplies one).
    pub latency: Option<PirShadowLatency>,
}

/// Maximum byte distance between two range starts for them to be treated as a
/// near-match (range disagreement) rather than independent missing/extra sites.
const RANGE_NEAR_MATCH_BYTES: usize = 2;

impl PirShadowCompareReceipt {
    /// Construct a refusal receipt with all counts zeroed.
    fn refused(reason: PirShadowRefusalReason) -> Self {
        Self {
            compiler_candidate_count: 0,
            legacy_candidate_count: 0,
            missing_from_compiler: Vec::new(),
            extra_in_compiler: Vec::new(),
            range_disagreements: Vec::new(),
            refusal_reason: Some(reason),
            provider_behavior_changed: false,
            latency: None,
        }
    }
}

/// Evaluate the ordered refusal guards for a shadow-compare request.
///
/// Pure over primitive inputs so the full guard ladder — including the two
/// guards (`bodies_len == 0`, `provider_behavior_changed == true`) that the
/// PR1 pipeline never reaches with real data — is observable and unit-testable
/// with literal arguments. Returns `Some(reason)` if the comparison must be
/// refused, `None` if it may proceed.
///
/// Guard order is significant and asserted by tests:
/// 1. `::`-qualified name → [`PirShadowRefusalReason::NotSameFileLexical`]
/// 2. `bodies_len == 0` → [`PirShadowRefusalReason::NoAnchoredFacts`]
/// 3. `target_body_idx >= bodies_len` → [`PirShadowRefusalReason::NoAnchoredFacts`]
/// 4. `provider_behavior_changed` → [`PirShadowRefusalReason::ProviderBehaviorChanged`]
fn evaluate_refusal(
    target_name: &str,
    target_body_idx: usize,
    bodies_len: usize,
    provider_behavior_changed: bool,
) -> Option<PirShadowRefusalReason> {
    if target_name.contains("::") {
        return Some(PirShadowRefusalReason::NotSameFileLexical);
    }
    if bodies_len == 0 {
        return Some(PirShadowRefusalReason::NoAnchoredFacts);
    }
    if target_body_idx >= bodies_len {
        return Some(PirShadowRefusalReason::NoAnchoredFacts);
    }
    if provider_behavior_changed {
        return Some(PirShadowRefusalReason::ProviderBehaviorChanged);
    }
    None
}

/// Run PIR-A reference extraction beside the legacy find-references result for
/// the narrow same-file lexical slice.
///
/// The legacy result is supplied as byte-offset pairs and is never mutated —
/// callers continue to return it to the LSP client unchanged.
///
/// # Refusal
///
/// See [`evaluate_refusal`] for the ordered guards. On refusal the returned
/// receipt has the reason set and all counts zeroed.
///
/// # Comparison algorithm
///
/// 1. Build the compiler set: anchored facts in `receipt.bodies[target_body_idx]`
///    whose bare name equals `target_name`, projected to `(start, end)` byte pairs.
/// 2. Build the legacy set from `legacy_result`.
/// 3. Sites in exactly one set are *candidates* for disagreement. Greedily pair a
///    legacy-only site with the first unused compiler-only site whose start is
///    within [`RANGE_NEAR_MATCH_BYTES`]; paired sites become `range_disagreements`.
/// 4. Unpaired legacy-only sites → `missing_from_compiler`; unpaired compiler-only
///    sites → `extra_in_compiler`. The three categories are disjoint.
///
/// Ordering is deterministic: both sets are [`BTreeSet`]s, so all output vectors
/// are sorted ascending and the pairing is reproducible.
///
/// # Invariants
///
/// - `provider_behavior_changed` is always `false`.
/// - The legacy result is not mutated.
#[must_use]
pub fn shadow_references_with_pir(
    receipt: &LexicalExtractorReceipt,
    legacy_result: &[(usize, usize)],
    target_name: &str,
    target_body_idx: usize,
) -> PirShadowCompareReceipt {
    if let Some(reason) = evaluate_refusal(
        target_name,
        target_body_idx,
        receipt.bodies.len(),
        receipt.provider_behavior_changed,
    ) {
        return PirShadowCompareReceipt::refused(reason);
    }

    // Build the compiler set: anchored facts for `target_name` in the target body.
    let compiler_ranges: BTreeSet<(usize, usize)> = receipt.bodies[target_body_idx]
        .facts
        .iter()
        .filter(|f| f.name.name == target_name && f.source_anchor.is_anchored())
        .filter_map(|f| f.source_anchor.range.as_ref().map(|r| (r.start, r.end)))
        .collect();

    let legacy_set: BTreeSet<(usize, usize)> = legacy_result.iter().copied().collect();

    let compiler_candidate_count = compiler_ranges.len();
    let legacy_candidate_count = legacy_set.len();

    // Sites in exactly one set (BTreeSet::difference yields sorted-ascending order).
    let legacy_only: Vec<(usize, usize)> =
        legacy_set.difference(&compiler_ranges).copied().collect();
    let compiler_only: Vec<(usize, usize)> =
        compiler_ranges.difference(&legacy_set).copied().collect();

    // Greedily pair near-matches; keep the three categories disjoint.
    let mut compiler_paired = vec![false; compiler_only.len()];
    let mut range_disagreements: Vec<RangeDisagreement> = Vec::new();
    let mut missing_from_compiler: Vec<(usize, usize)> = Vec::new();

    for &(ls, le) in &legacy_only {
        let matched = compiler_only.iter().enumerate().find(|&(i, &(cs, _))| {
            !compiler_paired[i] && cs.abs_diff(ls) <= RANGE_NEAR_MATCH_BYTES
        });

        match matched {
            Some((i, &(cs, ce))) => {
                compiler_paired[i] = true;
                range_disagreements.push(RangeDisagreement {
                    variable: target_name.to_string(),
                    compiler_range: (cs, ce),
                    legacy_range: (ls, le),
                });
            }
            None => missing_from_compiler.push((ls, le)),
        }
    }

    let extra_in_compiler: Vec<(usize, usize)> = compiler_only
        .iter()
        .enumerate()
        .filter(|&(i, _)| !compiler_paired[i])
        .map(|(_, &r)| r)
        .collect();

    PirShadowCompareReceipt {
        compiler_candidate_count,
        legacy_candidate_count,
        missing_from_compiler,
        extra_in_compiler,
        range_disagreements,
        refusal_reason: None,
        provider_behavior_changed: false,
        latency: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{PirShadowRefusalReason, evaluate_refusal};

    // The four refusal guards, exercised with literal arguments so the two
    // guards the real PR1 pipeline never reaches (empty bodies, behavior change)
    // are still observable.

    #[test]
    fn refusal_package_qualified_name() {
        assert_eq!(
            evaluate_refusal("Foo::bar", 0, 3, false),
            Some(PirShadowRefusalReason::NotSameFileLexical),
        );
    }

    #[test]
    fn refusal_empty_bodies() {
        assert_eq!(
            evaluate_refusal("x", 0, 0, false),
            Some(PirShadowRefusalReason::NoAnchoredFacts),
        );
    }

    #[test]
    fn refusal_body_index_out_of_range() {
        // idx beyond len, and the boundary case idx == len, both refuse.
        assert_eq!(
            evaluate_refusal("x", 5, 3, false),
            Some(PirShadowRefusalReason::NoAnchoredFacts),
        );
        assert_eq!(
            evaluate_refusal("x", 3, 3, false),
            Some(PirShadowRefusalReason::NoAnchoredFacts),
        );
    }

    #[test]
    fn refusal_provider_behavior_changed() {
        assert_eq!(
            evaluate_refusal("x", 0, 3, true),
            Some(PirShadowRefusalReason::ProviderBehaviorChanged),
        );
    }

    #[test]
    fn proceed_when_request_is_valid() {
        assert_eq!(evaluate_refusal("x", 0, 3, false), None);
        // last in-range index proceeds
        assert_eq!(evaluate_refusal("x", 2, 3, false), None);
    }

    #[test]
    fn guard_order_package_qualified_wins() {
        // `::` is checked first, ahead of empty/oob/changed.
        assert_eq!(
            evaluate_refusal("A::b", 99, 0, true),
            Some(PirShadowRefusalReason::NotSameFileLexical),
        );
    }

    #[test]
    fn guard_order_empty_bodies_before_behavior_change() {
        // Guard 2 (empty) precedes guard 4 (changed).
        assert_eq!(
            evaluate_refusal("x", 0, 0, true),
            Some(PirShadowRefusalReason::NoAnchoredFacts),
        );
    }
}
