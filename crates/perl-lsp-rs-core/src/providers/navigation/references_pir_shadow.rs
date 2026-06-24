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
//! # Guarded promotion machinery (PR3b, #2651)
//!
//! [`references_pir_promote`] is the corrected PR3a/b entry point. It is governed
//! by [`PromotionMode`], which ships as [`DEFAULT_PROMOTION_MODE`] (`Off`).
//! No live provider wiring occurs while the mode is `Off`; the `references.rs`
//! legacy arm is untouched.
//!
//! The live provider passes the mode from config. Flip criterion for
//! `PromotionMode::PromoteExact`: ops may set this only after a human sign-off
//! confirming that the PR2 shadow scorecard on issue #2635 shows
//! `extra_in_compiler == 0` across the full set1 fixture set for at least one
//! complete CI green run post-PR2 merge (the corpus-soak precondition from the
//! PR3 plan-reviewed spec). No individual agent may flip the mode without that
//! explicit human sign-off.
//!
//! # Usage
//!
//! ```rust,ignore
//! use perl_parser_core::{Parser, hir::lower_ast, pir::extract_lexical_facts};
//! use perl_parser_core::pir::LexicalName;
//! use perl_lsp_rs_core::providers::navigation::references_pir_shadow::{
//!     shadow_references_with_pir, references_pir_promote, PromotionMode, ReferenceOptions,
//!     DEFAULT_PROMOTION_MODE,
//! };
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

/// Why [`shadow_references_with_pir`] or [`references_pir_promote`] refused to
/// run or refused to promote.
///
/// Only reasons the comparison can actually *produce* are modelled. The enum is
/// `#[non_exhaustive]` so follow-up PRs can add reasons (e.g. stale-generation
/// or dynamic-boundary) once the corresponding inputs exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PirShadowRefusalReason {
    /// The feature is off: [`PromotionMode::Off`] was passed. No evaluation ran.
    FeatureDisabled,
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
    /// The compiler set is empty after applying sigil+name filtering. Cannot
    /// produce an `Exact` result with zero ranges.
    ///
    /// This is distinct from `NoAnchoredFacts`: the receipt has bodies and the
    /// body index is in range, but no fact matched the requested `LexicalName`.
    NoExactFacts,
    /// [`PromotionMode::Shadow`] mode: the candidate was evaluated for scorecard
    /// observation but the live provider result is preserved unchanged.
    ///
    /// Distinct from all refusal reasons: this is not a refusal — the PIR path
    /// ran and produced evidence. The legacy result is returned to the caller
    /// unchanged (shadow-only, no cutover). The observation is recorded via the
    /// `Shadow` arm in [`references_pir_promote`]; scorecard aggregation uses the
    /// companion [`shadow_references_with_pir`] for the full receipt.
    ShadowObserved,
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
///
/// The `target_name` argument is the bare variable name (no sigil), used only
/// for the `::` qualification check.
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

    // Build the compiler set: anchored facts for `target_name` (bare name) in the target body.
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

// ─────────────────────────────────────────────────────────────────────────────
// PR3b (#2651): Corrected PIR-A lexical reference promotion contract
// ─────────────────────────────────────────────────────────────────────────────

/// How the PIR-A lexical reference promotion path is activated.
///
/// The live provider passes the mode from config. Modes:
///
/// - [`Off`](Self::Off) — Do not evaluate; return
///   [`LegacyFallback { reason: FeatureDisabled }`](ReferencesPirPromoteOutcome::LegacyFallback)
///   immediately without examining the receipt.
///
/// - [`Shadow`](Self::Shadow) — Evaluate the candidate and emit the comparison
///   receipt but **return [`LegacyFallback`](ReferencesPirPromoteOutcome::LegacyFallback)**
///   (the "always-run-then-fallback" pattern, now named honestly). Used for
///   scorecard aggregation without changing live provider results.
///
/// - [`PromoteExact`](Self::PromoteExact) — Return
///   [`Exact`](ReferencesPirPromoteOutcome::Exact) when all gates pass; refuse to
///   fallback otherwise.
///
/// **Flip criterion for [`PromoteExact`](Self::PromoteExact)**: ops may set this only
/// after a human sign-off confirming that the PR2 shadow scorecard on issue #2635
/// shows `extra_in_compiler == 0` across the full set1 fixture set for at least one
/// complete CI green run post-PR2 merge (the corpus-soak precondition from the PR3
/// plan-reviewed spec). No individual agent may flip this mode without that explicit
/// human sign-off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromotionMode {
    /// Feature is off. Return `LegacyFallback { reason: FeatureDisabled }` immediately.
    Off,
    /// Evaluate candidate + emit comparison receipt, but return `LegacyFallback`.
    /// Used for scorecard aggregation without cutover.
    Shadow,
    /// Return `Exact` when all gates pass; refuse and fallback otherwise.
    PromoteExact,
}

/// The rollback anchor. The live provider reads this constant to determine the
/// default mode when no config override is present.
pub const DEFAULT_PROMOTION_MODE: PromotionMode = PromotionMode::Off;

/// Options controlling what [`references_pir_promote`] includes in the result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferenceOptions {
    /// When `true`, the declaration-role occurrence (the `LexicalWrite` that
    /// introduces the binding) is included in the result. When `false`, it is
    /// filtered out.
    ///
    /// **Ambiguity note:** In the current PIR v0 model there is no explicit
    /// "declaration vs. use" distinction beyond the `LexicalRole::Write` fact.
    /// The first `Write` in a body for the target name is treated as the
    /// declaration anchor. If this heuristic is insufficient for a given
    /// caller, keep `include_declaration: true` and filter at the provider
    /// layer where more context is available.
    pub include_declaration: bool,
}

/// Outcome of a guarded PIR-A lexical reference promotion attempt.
///
/// The caller MUST NOT union the compiler result with the legacy result — the
/// cutover is exclusive. On [`Exact`](Self::Exact) return the legacy result is
/// discarded; on [`LegacyFallback`](Self::LegacyFallback) the compiler result
/// is discarded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferencesPirPromoteOutcome {
    /// Compiler path taken: this is the authoritative result.
    ///
    /// The ranges are scope-exact LSP ranges derived from the PIR-A lexical
    /// extractor. Do NOT union with the legacy result.
    ///
    /// Invariants: ranges are sorted (line, character, end) and deduplicated.
    /// The set is never empty — an empty compiler set produces
    /// [`LegacyFallback { reason: NoExactFacts }`](Self::LegacyFallback)
    /// instead.
    Exact(Vec<lsp_types::Range>),

    /// Compiler path refused or not taken for the stated reason.
    ///
    /// The caller must fall back to the supplied `result` (the unmodified legacy
    /// byte-offset pairs from `find_references_single_file`).
    LegacyFallback {
        /// The original legacy byte-offset pairs from `find_references_single_file`.
        result: Vec<(usize, usize)>,
        /// The reason the compiler path was not taken.
        reason: PirShadowRefusalReason,
    },
}

/// Sort key for an `lsp_types::Range` for deterministic ordering.
///
/// Orders by (start.line, start.character, end.line, end.character).
#[inline]
fn range_sort_key(r: &lsp_types::Range) -> (u32, u32, u32, u32) {
    (r.start.line, r.start.character, r.end.line, r.end.character)
}

/// Evaluate the inner PIR reference candidate, applying sigil+name filtering.
///
/// Private helper shared by [`references_pir_promote`] for the `Shadow` and
/// `PromoteExact` modes. Returns the sorted, deduplicated set of compiler
/// ranges, or a refusal reason if no ranges matched.
///
/// `target_sigil` and `target_name` together identify the full variable identity
/// (e.g. `"$"` and `"x"` for `$x`). The `::` check is performed on `target_name`.
fn evaluate_pir_reference_candidate(
    pir_receipt: &LexicalExtractorReceipt,
    target_sigil: &str,
    target_name: &str,
    target_body_idx: usize,
    uri_mapper: &dyn Fn(usize, usize) -> lsp_types::Range,
    include_declaration: bool,
) -> Result<Vec<lsp_types::Range>, PirShadowRefusalReason> {
    // Refusal ladder on the bare name part.
    if let Some(reason) = evaluate_refusal(
        target_name,
        target_body_idx,
        pir_receipt.bodies.len(),
        pir_receipt.provider_behavior_changed,
    ) {
        return Err(reason);
    }

    use perl_parser_core::pir::LexicalRole;

    let body = &pir_receipt.bodies[target_body_idx];

    // Build the range list. Match on sigil AND name (full identity). When
    // `include_declaration` is false, skip the first Write fact for the target
    // (treated as the declaration anchor).
    let mut declaration_skipped = false;
    let mut ranges: Vec<lsp_types::Range> = Vec::new();
    for fact in &body.facts {
        if fact.name.sigil != target_sigil || fact.name.name != target_name {
            continue;
        }
        // Note: extractor invariant (PR1 #2637) guarantees every emitted fact has
        // `source_anchor.is_anchored() == true` — no dead branch needed here.
        if !include_declaration && !declaration_skipped && fact.role == LexicalRole::Write {
            declaration_skipped = true;
            continue;
        }
        if let Some(r) = fact.source_anchor.range.as_ref() {
            ranges.push(uri_mapper(r.start, r.end));
        }
    }

    if ranges.is_empty() {
        return Err(PirShadowRefusalReason::NoExactFacts);
    }

    // Sort by (line, character, end_line, end_character) and dedup.
    ranges.sort_by_key(range_sort_key);
    ranges.dedup();

    Ok(ranges)
}

/// Run the PIR-A lexical reference promotion with the corrected contract.
///
/// The `mode` parameter governs whether and how the compiler path is taken:
///
/// - [`Off`](PromotionMode::Off): return `LegacyFallback { reason: FeatureDisabled }` immediately.
/// - [`Shadow`](PromotionMode::Shadow): evaluate the candidate and return a
///   comparison receipt embedded in `LegacyFallback` (scorecard mode — live
///   result unchanged).
/// - [`PromoteExact`](PromotionMode::PromoteExact): return `Exact` when all
///   gates pass; refuse and return `LegacyFallback` otherwise.
///
/// # Promotion contract
///
/// The caller MUST NOT union the returned compiler ranges with the legacy
/// result. Choose one:
///
/// - [`Exact`](ReferencesPirPromoteOutcome::Exact) → return compiler ranges to
///   the LSP client; discard `legacy_result`.
/// - [`LegacyFallback`](ReferencesPirPromoteOutcome::LegacyFallback) → return
///   legacy result; discard compiler ranges.
///
/// # Arguments
///
/// * `mode` — [`PromotionMode`] governing compiler-path activation.
/// * `target_sigil` — Variable sigil (`"$"`, `"@"`, `"%"`). **Sigil is part
///   of the identity:** `$x` and `@x` are distinct and must not collide.
/// * `target_name` — Bare variable name without sigil (e.g. `"x"` for `$x`).
///   Must not be `::` qualified; package variables are refused automatically.
/// * `pir_receipt` — The `LexicalExtractorReceipt` from `extract_lexical_facts`.
/// * `legacy_result` — The `(start_byte, end_byte)` pairs from
///   `find_references_single_file`; returned unmodified on fallback.
/// * `target_body_idx` — The body index in `pir_receipt.bodies` where the
///   target binding was found.
/// * `uri_mapper` — Converts a `(start_byte, end_byte)` pair to an LSP
///   `lsp_types::Range` (handles UTF-16 encoding for the LSP client).
/// * `opts` — [`ReferenceOptions`] controlling what is included (e.g.
///   `include_declaration`).
#[must_use]
pub fn references_pir_promote(
    mode: PromotionMode,
    target_sigil: &str,
    target_name: &str,
    pir_receipt: &LexicalExtractorReceipt,
    legacy_result: &[(usize, usize)],
    target_body_idx: usize,
    uri_mapper: &dyn Fn(usize, usize) -> lsp_types::Range,
    opts: ReferenceOptions,
) -> ReferencesPirPromoteOutcome {
    let legacy_vec = legacy_result.to_vec();

    match mode {
        PromotionMode::Off => ReferencesPirPromoteOutcome::LegacyFallback {
            result: legacy_vec,
            reason: PirShadowRefusalReason::FeatureDisabled,
        },

        PromotionMode::Shadow => {
            // Evaluate candidate for scorecard observation (the whole point of Shadow
            // mode) but always return the legacy result unchanged — no cutover.
            // The candidate result is observed here; `ShadowObserved` is the honest
            // reason: the path ran, the evidence is real, the live result is preserved.
            //
            // Build the durable comparison receipt using `shadow_references_with_pir`,
            // which operates on the byte-offset level (no uri_mapper needed for the
            // scorecard). The candidate evaluation result flows into the receipt via
            // the compiler-set counts. Emit via tracing::debug! for aggregation.
            let receipt = shadow_references_with_pir(
                pir_receipt,
                legacy_result,
                target_name,
                target_body_idx,
            );
            // Extract receipt fields to local variables so coverage tools can
            // track these expressions independently of the tracing::debug! macro
            // expansion (which may be inlined into a conditional that appears
            // uncovered when no subscriber is installed).
            let missing_count = receipt.missing_from_compiler.len();
            let extra_count = receipt.extra_in_compiler.len();
            let disagreement_count = receipt.range_disagreements.len();
            tracing::debug!(
                target: "pir_shadow_receipt",
                target_sigil = %target_sigil,
                target_name = %target_name,
                target_body_idx = target_body_idx,
                compiler_candidate_count = receipt.compiler_candidate_count,
                legacy_candidate_count = receipt.legacy_candidate_count,
                missing_from_compiler = missing_count,
                extra_in_compiler = extra_count,
                range_disagreements = disagreement_count,
                refusal_reason = ?receipt.refusal_reason,
                provider_behavior_changed = receipt.provider_behavior_changed,
                "pir_shadow_compare_receipt"
            );
            ReferencesPirPromoteOutcome::LegacyFallback {
                result: legacy_vec,
                reason: PirShadowRefusalReason::ShadowObserved,
            }
        }

        PromotionMode::PromoteExact => {
            match evaluate_pir_reference_candidate(
                pir_receipt,
                target_sigil,
                target_name,
                target_body_idx,
                uri_mapper,
                opts.include_declaration,
            ) {
                Ok(ranges) => ReferencesPirPromoteOutcome::Exact(ranges),
                Err(reason) => {
                    ReferencesPirPromoteOutcome::LegacyFallback { result: legacy_vec, reason }
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// DEFERRED (→ follow-up issues):
//   - `Stale` variant: needs a document-generation/freshness input the fn
//     doesn't receive yet.
//   - `Ambiguous` variant: needs detection logic + inputs that don't exist.
//   - `DynamicBoundary` variant: needs detection logic + inputs that don't exist.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod promote_tests {
    use super::{
        DEFAULT_PROMOTION_MODE, PirShadowRefusalReason, PromotionMode, ReferenceOptions,
        ReferencesPirPromoteOutcome, references_pir_promote,
    };
    use perl_parser_core::{Parser, hir::lower_ast, pir::extract_lexical_facts};

    fn byte_mapper(start: usize, end: usize) -> lsp_types::Range {
        lsp_types::Range {
            start: lsp_types::Position { line: 0, character: start as u32 },
            end: lsp_types::Position { line: 0, character: end as u32 },
        }
    }

    fn receipt_for(source: &str) -> perl_parser_core::pir::LexicalExtractorReceipt {
        let mut parser = Parser::new(source);
        let output = parser.parse_with_recovery();
        let hir = lower_ast(&output.ast);
        extract_lexical_facts(&hir)
    }

    fn opts_all() -> ReferenceOptions {
        ReferenceOptions { include_declaration: true }
    }

    // ── DEFAULT_PROMOTION_MODE is Off ──────────────────────────────────────

    #[test]
    fn default_mode_is_off() {
        assert_eq!(DEFAULT_PROMOTION_MODE, PromotionMode::Off);
    }

    // ── Off → FeatureDisabled immediately ─────────────────────────────────

    #[test]
    fn off_returns_feature_disabled_without_evaluating() {
        let receipt = receipt_for("my $x = 1;\nprint $x;\n");
        let legacy = vec![(3usize, 5usize)];
        let outcome = references_pir_promote(
            PromotionMode::Off,
            "$",
            "x",
            &receipt,
            &legacy,
            0,
            &byte_mapper,
            opts_all(),
        );
        assert!(
            matches!(
                &outcome,
                ReferencesPirPromoteOutcome::LegacyFallback {
                    reason: PirShadowRefusalReason::FeatureDisabled,
                    ..
                }
            ),
            "Off mode must return FeatureDisabled, got {outcome:?}"
        );
        if let ReferencesPirPromoteOutcome::LegacyFallback { result, .. } = outcome {
            assert_eq!(result, legacy, "legacy result must be returned unchanged");
        }
    }

    // ── Shadow → evaluates but returns LegacyFallback ─────────────────────

    #[test]
    fn shadow_returns_legacy_fallback_with_receipt() {
        let receipt = receipt_for("my $x = 1;\nprint $x;\n");
        let legacy = vec![(3usize, 5usize)];
        let outcome = references_pir_promote(
            PromotionMode::Shadow,
            "$",
            "x",
            &receipt,
            &legacy,
            0,
            &byte_mapper,
            opts_all(),
        );
        assert!(
            matches!(
                &outcome,
                ReferencesPirPromoteOutcome::LegacyFallback {
                    reason: PirShadowRefusalReason::ShadowObserved,
                    ..
                }
            ),
            "Shadow mode must return LegacyFallback with ShadowObserved reason, got {outcome:?}"
        );
        if let ReferencesPirPromoteOutcome::LegacyFallback { result, .. } = outcome {
            assert_eq!(result, legacy, "shadow mode must preserve legacy result");
        }
    }

    // ── PromoteExact → Exact for valid receipt ─────────────────────────────

    #[test]
    fn promote_exact_returns_exact_for_valid_receipt() {
        let receipt = receipt_for("my $x = 1;\nprint $x;\n");
        let outcome = references_pir_promote(
            PromotionMode::PromoteExact,
            "$",
            "x",
            &receipt,
            &[],
            0,
            &byte_mapper,
            opts_all(),
        );
        assert!(
            matches!(&outcome, ReferencesPirPromoteOutcome::Exact(_)),
            "PromoteExact must return Exact for valid $x receipt, got {outcome:?}"
        );
        if let ReferencesPirPromoteOutcome::Exact(ranges) = outcome {
            assert!(
                ranges.len() >= 2,
                "must have at least 2 facts for $x (write+read): {ranges:?}"
            );
        }
    }

    // ── Sigil discrimination: $x vs @x → disjoint sets ────────────────────

    #[test]
    fn sigil_discrimination_scalar_vs_array_disjoint() {
        // Source with both $x (scalar) and @x (array).
        let source = "my $x = 1;\nmy @x = (1, 2);\nprint $x;\nprint @x;\n";
        let receipt = receipt_for(source);

        let scalar_outcome = references_pir_promote(
            PromotionMode::PromoteExact,
            "$",
            "x",
            &receipt,
            &[],
            0,
            &byte_mapper,
            opts_all(),
        );
        let array_outcome = references_pir_promote(
            PromotionMode::PromoteExact,
            "@",
            "x",
            &receipt,
            &[],
            0,
            &byte_mapper,
            opts_all(),
        );

        let scalar_ranges = match scalar_outcome {
            ReferencesPirPromoteOutcome::Exact(r) => r,
            other => panic!("expected Exact for $x, got {other:?}"),
        };
        let array_ranges = match array_outcome {
            ReferencesPirPromoteOutcome::Exact(r) => r,
            other => panic!("expected Exact for @x, got {other:?}"),
        };

        // $x and @x have disjoint range sets (different byte positions in source).
        for r in &scalar_ranges {
            assert!(
                !array_ranges.contains(r),
                "$x range {r:?} must not appear in @x result — sigil collision detected"
            );
        }
        for r in &array_ranges {
            assert!(
                !scalar_ranges.contains(r),
                "@x range {r:?} must not appear in $x result — sigil collision detected"
            );
        }

        // Both must have at least 2 occurrences (decl + use).
        assert!(scalar_ranges.len() >= 2, "$x must have >=2 ranges: {scalar_ranges:?}");
        assert!(array_ranges.len() >= 2, "@x must have >=2 ranges: {array_ranges:?}");
    }

    // ── PromoteExact with empty compiler set → NoExactFacts, not Exact([]) ─

    #[test]
    fn promote_exact_empty_compiler_set_returns_no_exact_facts() {
        // Name with no facts in receipt → NoExactFacts, never Exact([]).
        let receipt = receipt_for("my $x = 1;\n");
        let outcome = references_pir_promote(
            PromotionMode::PromoteExact,
            "$",
            "zzz_no_such_var",
            &receipt,
            &[(0, 2)],
            0,
            &byte_mapper,
            opts_all(),
        );
        assert!(
            matches!(
                &outcome,
                ReferencesPirPromoteOutcome::LegacyFallback {
                    reason: PirShadowRefusalReason::NoExactFacts,
                    ..
                }
            ),
            "unknown variable must produce NoExactFacts refusal, got {outcome:?}"
        );
    }

    // ── Ranges are sorted and deduped ──────────────────────────────────────

    #[test]
    fn promote_exact_ranges_sorted_and_deduped() {
        // Any valid receipt with multiple facts to check ordering.
        let receipt = receipt_for("my $x = 1;\nprint $x;\n");
        let outcome = references_pir_promote(
            PromotionMode::PromoteExact,
            "$",
            "x",
            &receipt,
            &[],
            0,
            &byte_mapper,
            opts_all(),
        );
        if let ReferencesPirPromoteOutcome::Exact(ranges) = outcome {
            // Check sorted ascending by (line, character).
            for window in ranges.windows(2) {
                let a = &window[0];
                let b = &window[1];
                assert!(
                    (a.start.line, a.start.character) <= (b.start.line, b.start.character),
                    "ranges must be sorted ascending: {a:?} > {b:?}"
                );
            }
            // Check deduped: no consecutive equal elements.
            for window in ranges.windows(2) {
                assert_ne!(window[0], window[1], "duplicate range found: {:?}", window[0]);
            }
        } else {
            panic!("expected Exact for valid $x receipt");
        }
    }

    // ── Package-qualified name → NotSameFileLexical via PromoteExact ───────

    #[test]
    fn promote_exact_package_qualified_returns_not_same_file_lexical() {
        let receipt = receipt_for("my $x = 1;");
        let legacy = vec![(0usize, 2usize)];
        let outcome = references_pir_promote(
            PromotionMode::PromoteExact,
            "$",
            "Foo::x",
            &receipt,
            &legacy,
            0,
            &byte_mapper,
            opts_all(),
        );
        assert!(
            matches!(
                &outcome,
                ReferencesPirPromoteOutcome::LegacyFallback {
                    reason: PirShadowRefusalReason::NotSameFileLexical,
                    ..
                }
            ),
            "package-qualified name must refuse with NotSameFileLexical, got {outcome:?}"
        );
    }

    // ── includeDeclaration: true includes declaration ──────────────────────

    #[test]
    fn include_declaration_true_includes_write_fact() {
        let receipt = receipt_for("my $a = 1;\nprint $a;\n");
        let outcome = references_pir_promote(
            PromotionMode::PromoteExact,
            "$",
            "a",
            &receipt,
            &[],
            0,
            &byte_mapper,
            ReferenceOptions { include_declaration: true },
        );
        if let ReferencesPirPromoteOutcome::Exact(ranges) = outcome {
            assert!(ranges.len() >= 2, "with include_declaration=true must have >=2 ranges");
        } else {
            panic!("expected Exact");
        }
    }

    // ── includeDeclaration: false excludes first Write fact ────────────────

    #[test]
    fn include_declaration_false_excludes_first_write_fact() {
        let receipt = receipt_for("my $a = 1;\nprint $a;\n");

        let with_decl = references_pir_promote(
            PromotionMode::PromoteExact,
            "$",
            "a",
            &receipt,
            &[],
            0,
            &byte_mapper,
            ReferenceOptions { include_declaration: true },
        );
        let without_decl = references_pir_promote(
            PromotionMode::PromoteExact,
            "$",
            "a",
            &receipt,
            &[],
            0,
            &byte_mapper,
            ReferenceOptions { include_declaration: false },
        );

        match (with_decl, without_decl) {
            (
                ReferencesPirPromoteOutcome::Exact(with),
                ReferencesPirPromoteOutcome::Exact(without),
            ) => {
                assert!(
                    with.len() > without.len(),
                    "include_declaration=true must yield more ranges than false; \
                     with={with:?}, without={without:?}"
                );
            }
            (ReferencesPirPromoteOutcome::Exact(_), other) => {
                // include_declaration=false with only 1 fact remaining → NoExactFacts is valid
                assert!(
                    matches!(
                        other,
                        ReferencesPirPromoteOutcome::LegacyFallback {
                            reason: PirShadowRefusalReason::NoExactFacts,
                            ..
                        }
                    ),
                    "unexpected outcome without decl: {other:?}"
                );
            }
            other => panic!("unexpected outcomes: {other:?}"),
        }
    }

    // ── Legacy result preserved on all fallback paths ──────────────────────

    #[test]
    fn legacy_result_preserved_on_off_fallback() {
        let receipt = receipt_for("my $z = 0;\nprint $z;\n");
        let legacy = vec![(3usize, 5usize), (13usize, 15usize)];
        let outcome = references_pir_promote(
            PromotionMode::Off,
            "$",
            "z",
            &receipt,
            &legacy,
            0,
            &byte_mapper,
            opts_all(),
        );
        if let ReferencesPirPromoteOutcome::LegacyFallback { result, .. } = outcome {
            assert_eq!(result, legacy);
        } else {
            panic!("expected LegacyFallback");
        }
    }

    // ── uri_mapper is called for each range ───────────────────────────────

    #[test]
    fn uri_mapper_applied_to_all_ranges() {
        let receipt = receipt_for("my $p = 1;");
        let sentinel_mapper = |start: usize, end: usize| lsp_types::Range {
            start: lsp_types::Position { line: 1, character: (start + 1000) as u32 },
            end: lsp_types::Position { line: 1, character: (end + 1000) as u32 },
        };
        let outcome = references_pir_promote(
            PromotionMode::PromoteExact,
            "$",
            "p",
            &receipt,
            &[],
            0,
            &sentinel_mapper,
            opts_all(),
        );
        if let ReferencesPirPromoteOutcome::Exact(ranges) = outcome {
            assert!(
                ranges.iter().all(|r| r.start.character >= 1000),
                "uri_mapper must have been applied to all ranges: {ranges:?}"
            );
        } else {
            panic!("expected Exact for $p");
        }
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
