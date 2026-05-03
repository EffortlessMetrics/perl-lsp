//! Scorecard aggregation for semantic shadow-compare receipts.
//!
//! A [`Scorecard`] collects [`SemanticShadowCompareReceipt`] entries across
//! providers and fixture suites, then reports per-query verdict counts.
//!
//! Three operating modes control how the scorecard result is interpreted:
//!
//! - [`ScorecardMode::Emit`] — always succeeds; used for informational reporting.
//! - [`ScorecardMode::Check`] — deterministic artifact freshness check; fails
//!   when any regression is detected.
//! - [`ScorecardMode::Gate`] — future hard gate, opt-in by CI lane. For RC2,
//!   this behaves identically to `Check` but is a separate variant so that
//!   callers can distinguish intent.
//!
//! # Requirements
//!
//! - **Req 11.1**: Aggregate shadow-compare receipts across all migrated
//!   providers and fixture suites.
//! - **Req 11.2**: Report per-query verdicts (Same, Improved, Regression,
//!   Ambiguous, Unavailable) with counts.
//! - **Req 11.6**: Report rename unsafe-edit count as zero.
//! - **Req 11.8**: Support three modes: emit, --check, --gate.

use crate::semantic_shadow_compare::{
    SemanticShadowCompareReceipt, ShadowCompareVerdict, ShadowQueryName,
};
use perl_semantic_facts::{PlannedEditCategory, RenamePlan};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Operating mode for scorecard evaluation.
///
/// Controls whether the scorecard result is advisory or enforced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScorecardMode {
    /// Always succeeds — informational reporting only.
    Emit,
    /// Deterministic artifact freshness check. Fails when any regression is
    /// detected.
    Check,
    /// Future hard gate, opt-in by CI lane. For RC2, behaves identically to
    /// `Check` but is a separate variant so callers can distinguish intent.
    Gate,
}

/// Per-verdict counts for a single query name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerdictCounts {
    /// Number of receipts with verdict `Same`.
    pub same: u64,
    /// Number of receipts with verdict `Improved`.
    pub improved: u64,
    /// Number of receipts with verdict `Regression`.
    pub regression: u64,
    /// Number of receipts with verdict `Ambiguous`.
    pub ambiguous: u64,
    /// Number of receipts with verdict `Unavailable`.
    pub unavailable: u64,
}

impl VerdictCounts {
    /// Total number of receipts aggregated into these counts.
    pub fn total(&self) -> u64 {
        self.same
            .saturating_add(self.improved)
            .saturating_add(self.regression)
            .saturating_add(self.ambiguous)
            .saturating_add(self.unavailable)
    }

    /// Record a single verdict.
    fn record(&mut self, verdict: ShadowCompareVerdict) {
        match verdict {
            ShadowCompareVerdict::Same => self.same = self.same.saturating_add(1),
            ShadowCompareVerdict::Improved => self.improved = self.improved.saturating_add(1),
            ShadowCompareVerdict::Regression => self.regression = self.regression.saturating_add(1),
            ShadowCompareVerdict::Ambiguous => self.ambiguous = self.ambiguous.saturating_add(1),
            ShadowCompareVerdict::Unavailable => {
                self.unavailable = self.unavailable.saturating_add(1);
            }
        }
    }
}

/// Aggregated scorecard report.
///
/// Contains per-query verdict counts, latency measurements, rename safety
/// metrics, and an overall pass/fail outcome determined by the
/// [`ScorecardMode`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScorecardReport {
    /// The mode used for evaluation.
    pub mode: ScorecardMode,
    /// Per-query-name verdict counts.
    pub by_query: HashMap<String, VerdictCounts>,
    /// Aggregate verdict counts across all queries.
    pub totals: VerdictCounts,
    /// Query latency measurements, keyed by query name.
    pub latency: HashMap<String, LatencyMeasurement>,
    /// Latency threshold violations (query names that exceeded their target).
    pub latency_violations: Vec<LatencyViolation>,
    /// Number of rename edits that are not properly classified into a known
    /// [`PlannedEditCategory`] (Definition, Reference, ImportList, ExportList).
    ///
    /// Req 11.6 requires this count to be zero for the scorecard to pass in
    /// Check or Gate mode.
    pub rename_unsafe_edit_count: u64,
    /// Whether the scorecard passes under the configured mode.
    pub passed: bool,
}

/// A single query latency measurement with p95 and threshold metadata.
///
/// Captures timing samples for a specific query method and computes the
/// p95 latency for comparison against the target threshold.
///
/// # Requirements
///
/// - **Req 19.5**: Report query latency p95 measurements and flag threshold
///   violations.
/// - **Req 11.7**: Scorecard reports query latency p95 within target threshold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LatencyMeasurement {
    /// Human-readable query name (e.g. "symbol_at").
    pub query_name: String,
    /// Number of samples collected.
    pub sample_count: usize,
    /// p95 latency in microseconds.
    pub p95_micros: u64,
    /// Target threshold in microseconds.
    pub threshold_micros: u64,
    /// Whether the p95 exceeds the target threshold.
    pub exceeded: bool,
}

/// A latency threshold violation record.
///
/// Emitted when a query's p95 latency exceeds its target threshold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LatencyViolation {
    /// Query name that violated the threshold.
    pub query_name: String,
    /// Measured p95 latency in microseconds.
    pub p95_micros: u64,
    /// Target threshold in microseconds.
    pub threshold_micros: u64,
}

/// Target latency thresholds for semantic queries on a 1000-file workspace.
///
/// Values from Requirement 19.
pub struct LatencyThresholds;

impl LatencyThresholds {
    /// `symbol_at` target: 5ms p95 (Req 19.1).
    pub const SYMBOL_AT_MICROS: u64 = 5_000;
    /// `definitions` target: 10ms p95 (Req 19.2).
    pub const DEFINITIONS_MICROS: u64 = 10_000;
    /// `references` target: 20ms p95 (Req 19.3).
    pub const REFERENCES_MICROS: u64 = 20_000;
    /// `visible_symbols_at` target: 15ms p95 (Req 19.4).
    pub const VISIBLE_SYMBOLS_AT_MICROS: u64 = 15_000;

    /// Return the threshold in microseconds for a given query name, if known.
    pub fn for_query(query_name: &str) -> Option<u64> {
        match query_name {
            "symbol_at" => Some(Self::SYMBOL_AT_MICROS),
            "definitions" => Some(Self::DEFINITIONS_MICROS),
            "references" => Some(Self::REFERENCES_MICROS),
            "visible_symbols_at" => Some(Self::VISIBLE_SYMBOLS_AT_MICROS),
            _ => None,
        }
    }
}

/// Compute the p95 value from a sorted slice of durations.
///
/// Returns `Duration::ZERO` for an empty slice.
pub fn compute_p95(sorted_durations: &[Duration]) -> Duration {
    if sorted_durations.is_empty() {
        return Duration::ZERO;
    }
    let idx = (sorted_durations.len() as f64 * 0.95).ceil() as usize;
    let clamped = idx.min(sorted_durations.len()).saturating_sub(1);
    sorted_durations[clamped]
}

/// Build a [`LatencyMeasurement`] from a set of duration samples.
///
/// Sorts the samples, computes p95, and checks against the threshold.
pub fn build_latency_measurement(
    query_name: &str,
    samples: &mut [Duration],
    threshold_micros: u64,
) -> LatencyMeasurement {
    samples.sort();
    let p95 = compute_p95(samples);
    let p95_micros = p95.as_micros() as u64;
    LatencyMeasurement {
        query_name: query_name.to_string(),
        sample_count: samples.len(),
        p95_micros,
        threshold_micros,
        exceeded: p95_micros > threshold_micros,
    }
}

/// Scorecard aggregator for semantic shadow-compare receipts.
///
/// Collects receipts and produces a [`ScorecardReport`] summarizing
/// per-query verdicts with counts.
#[derive(Debug)]
pub struct Scorecard {
    mode: ScorecardMode,
    receipts: Vec<SemanticShadowCompareReceipt>,
    latency_measurements: Vec<LatencyMeasurement>,
    rename_plans: Vec<RenamePlan>,
}

impl Scorecard {
    /// Create a new scorecard with the given operating mode.
    pub fn new(mode: ScorecardMode) -> Self {
        Self {
            mode,
            receipts: Vec::new(),
            latency_measurements: Vec::new(),
            rename_plans: Vec::new(),
        }
    }

    /// Add a single shadow-compare receipt to the scorecard.
    pub fn add_receipt(&mut self, receipt: SemanticShadowCompareReceipt) {
        self.receipts.push(receipt);
    }

    /// Add multiple shadow-compare receipts to the scorecard.
    pub fn add_receipts(
        &mut self,
        receipts: impl IntoIterator<Item = SemanticShadowCompareReceipt>,
    ) {
        self.receipts.extend(receipts);
    }

    /// Add a latency measurement to the scorecard.
    pub fn add_latency(&mut self, measurement: LatencyMeasurement) {
        self.latency_measurements.push(measurement);
    }

    /// Add multiple latency measurements to the scorecard.
    pub fn add_latencies(&mut self, measurements: impl IntoIterator<Item = LatencyMeasurement>) {
        self.latency_measurements.extend(measurements);
    }

    /// Add a rename plan for unsafe-edit counting (Req 11.6).
    ///
    /// Each plan's edits are inspected: any edit whose category is not one of
    /// the known [`PlannedEditCategory`] variants (Definition, Reference,
    /// ImportList, ExportList) is counted as an unsafe edit.
    pub fn add_rename_plan(&mut self, plan: RenamePlan) {
        self.rename_plans.push(plan);
    }

    /// Add multiple rename plans for unsafe-edit counting.
    pub fn add_rename_plans(&mut self, plans: impl IntoIterator<Item = RenamePlan>) {
        self.rename_plans.extend(plans);
    }

    /// Return the number of receipts collected so far.
    pub fn receipt_count(&self) -> usize {
        self.receipts.len()
    }

    /// Return the configured operating mode.
    pub fn mode(&self) -> ScorecardMode {
        self.mode
    }

    /// Produce the aggregated scorecard report.
    ///
    /// Verdict counts are grouped by query name and also aggregated into
    /// totals. Latency measurements are included with threshold violation
    /// flags. Rename plans are inspected for unsafe (unclassified) edits.
    /// The `passed` field is determined by the operating mode:
    ///
    /// - `Emit` — always `true`.
    /// - `Check` — `true` only when `totals.regression == 0` **and**
    ///   `rename_unsafe_edit_count == 0`.
    /// - `Gate` — same as `Check` for RC2.
    pub fn report(&self) -> ScorecardReport {
        let mut by_query: HashMap<String, VerdictCounts> = HashMap::new();
        let mut totals = VerdictCounts::default();

        for receipt in &self.receipts {
            let query_key = query_name_key(receipt.query);
            by_query.entry(query_key).or_default().record(receipt.verdict);
            totals.record(receipt.verdict);
        }

        let mut latency: HashMap<String, LatencyMeasurement> = HashMap::new();
        let mut latency_violations: Vec<LatencyViolation> = Vec::new();

        for m in &self.latency_measurements {
            if m.exceeded {
                latency_violations.push(LatencyViolation {
                    query_name: m.query_name.clone(),
                    p95_micros: m.p95_micros,
                    threshold_micros: m.threshold_micros,
                });
            }
            latency.insert(m.query_name.clone(), m.clone());
        }

        // Count rename unsafe edits: any edit not classified into a known
        // PlannedEditCategory is considered unsafe (Req 11.6).
        let rename_unsafe_edit_count = count_rename_unsafe_edits(&self.rename_plans);

        let passed = match self.mode {
            ScorecardMode::Emit => true,
            ScorecardMode::Check | ScorecardMode::Gate => {
                totals.regression == 0 && rename_unsafe_edit_count == 0
            }
        };

        ScorecardReport {
            mode: self.mode,
            by_query,
            totals,
            latency,
            latency_violations,
            rename_unsafe_edit_count,
            passed,
        }
    }
}

/// Convert a [`ShadowQueryName`] to a stable string key for the report map.
fn query_name_key(query: ShadowQueryName) -> String {
    match query {
        ShadowQueryName::FindDefinition => "find_definition".to_string(),
        ShadowQueryName::FindReferences => "find_references".to_string(),
        ShadowQueryName::CountUsages => "count_usages".to_string(),
        ShadowQueryName::VisibleSymbols => "visible_symbols".to_string(),
        ShadowQueryName::MethodCandidates => "method_candidates".to_string(),
        ShadowQueryName::SymbolAt => "symbol_at".to_string(),
        ShadowQueryName::RenamePlan => "rename_plan".to_string(),
        ShadowQueryName::SafeDeletePlan => "safe_delete_plan".to_string(),
        ShadowQueryName::CompletionVisibility => "completion_visibility".to_string(),
        ShadowQueryName::DiagnosticsCheck => "diagnostics_check".to_string(),
        ShadowQueryName::Hover => "hover".to_string(),
    }
}

/// Count the number of unsafe (unclassified) edits across all rename plans.
///
/// An edit is considered "unsafe" if its category is not one of the known
/// [`PlannedEditCategory`] variants. In practice, the current exhaustive
/// enum means all edits are classified, but this function guards against
/// future additions and validates that every edit in every plan carries a
/// recognized category.
///
/// Additionally, any rename plan that has `UnclassifiedOccurrence` blockers
/// contributes those blockers to the unsafe count, since they represent
/// occurrences the rename could not safely classify.
///
/// # Requirements
///
/// - **Req 11.6**: Rename unsafe-edit count is zero.
fn count_rename_unsafe_edits(plans: &[RenamePlan]) -> u64 {
    let mut count: u64 = 0;
    for plan in plans {
        // Count edits that are not properly classified.
        for edit in &plan.edits {
            let is_classified = matches!(
                edit.category,
                PlannedEditCategory::Definition
                    | PlannedEditCategory::Reference
                    | PlannedEditCategory::ImportList
                    | PlannedEditCategory::ExportList
            );
            if !is_classified {
                count = count.saturating_add(1);
            }
        }
        // Count UnclassifiedOccurrence blockers as unsafe edits.
        for blocker in &plan.blockers {
            if blocker.reason == perl_semantic_facts::PlanBlockerReason::UnclassifiedOccurrence {
                count = count.saturating_add(1);
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic_shadow_compare::{
        ShadowQueryInput, ShadowResultSummary, summarize_identities,
    };

    /// Helper: build a receipt with the given query name and verdict.
    fn make_receipt(
        query: ShadowQueryName,
        verdict: ShadowCompareVerdict,
    ) -> SemanticShadowCompareReceipt {
        // Build old/new summaries that produce the desired verdict.
        let (old_result, new_result) = summaries_for_verdict(verdict);
        // Override the verdict directly via from_summaries — the summaries
        // are crafted to produce the correct verdict.
        let receipt = SemanticShadowCompareReceipt::from_summaries(
            query,
            ShadowQueryInput { symbol: "test::sym".to_string() },
            old_result,
            new_result,
            vec![],
        );
        // Sanity-check that the crafted summaries produce the expected verdict.
        assert_eq!(receipt.verdict, verdict);
        receipt
    }

    /// Produce old/new summary pairs that yield the requested verdict from
    /// `classify_verdict`.
    fn summaries_for_verdict(
        verdict: ShadowCompareVerdict,
    ) -> (ShadowResultSummary, ShadowResultSummary) {
        match verdict {
            ShadowCompareVerdict::Same => {
                let s = summarize_identities(Some(vec!["a.pm:1:1".to_string()]));
                (s.clone(), s)
            }
            ShadowCompareVerdict::Improved => (
                summarize_identities(Some(vec!["a.pm:1:1".to_string()])),
                summarize_identities(Some(vec!["a.pm:1:1".to_string(), "b.pm:2:2".to_string()])),
            ),
            ShadowCompareVerdict::Regression => (
                summarize_identities(Some(vec!["a.pm:1:1".to_string(), "b.pm:2:2".to_string()])),
                summarize_identities(Some(vec!["a.pm:1:1".to_string()])),
            ),
            ShadowCompareVerdict::Ambiguous => (
                summarize_identities(Some(vec!["a.pm:1:1".to_string()])),
                summarize_identities(Some(vec!["z.pm:9:9".to_string()])),
            ),
            ShadowCompareVerdict::Unavailable => {
                (summarize_identities(None), summarize_identities(None))
            }
        }
    }

    // ── VerdictCounts ──

    #[test]
    fn verdict_counts_default_is_all_zero() -> Result<(), Box<dyn std::error::Error>> {
        let counts = VerdictCounts::default();
        assert_eq!(counts.same, 0);
        assert_eq!(counts.improved, 0);
        assert_eq!(counts.regression, 0);
        assert_eq!(counts.ambiguous, 0);
        assert_eq!(counts.unavailable, 0);
        assert_eq!(counts.total(), 0);
        Ok(())
    }

    #[test]
    fn verdict_counts_record_increments_correct_field() -> Result<(), Box<dyn std::error::Error>> {
        let mut counts = VerdictCounts::default();
        counts.record(ShadowCompareVerdict::Same);
        counts.record(ShadowCompareVerdict::Same);
        counts.record(ShadowCompareVerdict::Improved);
        counts.record(ShadowCompareVerdict::Regression);
        counts.record(ShadowCompareVerdict::Ambiguous);
        counts.record(ShadowCompareVerdict::Unavailable);

        assert_eq!(counts.same, 2);
        assert_eq!(counts.improved, 1);
        assert_eq!(counts.regression, 1);
        assert_eq!(counts.ambiguous, 1);
        assert_eq!(counts.unavailable, 1);
        assert_eq!(counts.total(), 6);
        Ok(())
    }

    // ── Scorecard — Emit mode ──

    #[test]
    fn emit_mode_always_passes() -> Result<(), Box<dyn std::error::Error>> {
        let mut sc = Scorecard::new(ScorecardMode::Emit);
        sc.add_receipt(make_receipt(
            ShadowQueryName::FindDefinition,
            ShadowCompareVerdict::Regression,
        ));
        sc.add_receipt(make_receipt(
            ShadowQueryName::FindReferences,
            ShadowCompareVerdict::Regression,
        ));

        let report = sc.report();
        assert!(report.passed, "Emit mode should always pass");
        assert_eq!(report.mode, ScorecardMode::Emit);
        assert_eq!(report.totals.regression, 2);
        Ok(())
    }

    // ── Scorecard — Check mode ──

    #[test]
    fn check_mode_passes_with_no_regressions() -> Result<(), Box<dyn std::error::Error>> {
        let mut sc = Scorecard::new(ScorecardMode::Check);
        sc.add_receipt(make_receipt(ShadowQueryName::FindDefinition, ShadowCompareVerdict::Same));
        sc.add_receipt(make_receipt(
            ShadowQueryName::FindReferences,
            ShadowCompareVerdict::Improved,
        ));
        sc.add_receipt(make_receipt(ShadowQueryName::CountUsages, ShadowCompareVerdict::Ambiguous));

        let report = sc.report();
        assert!(report.passed, "Check mode should pass with no regressions");
        assert_eq!(report.totals.regression, 0);
        Ok(())
    }

    #[test]
    fn check_mode_fails_with_regression() -> Result<(), Box<dyn std::error::Error>> {
        let mut sc = Scorecard::new(ScorecardMode::Check);
        sc.add_receipt(make_receipt(ShadowQueryName::FindDefinition, ShadowCompareVerdict::Same));
        sc.add_receipt(make_receipt(
            ShadowQueryName::FindReferences,
            ShadowCompareVerdict::Regression,
        ));

        let report = sc.report();
        assert!(!report.passed, "Check mode should fail with regressions");
        assert_eq!(report.totals.regression, 1);
        Ok(())
    }

    // ── Scorecard — Gate mode ──

    #[test]
    fn gate_mode_passes_with_no_regressions() -> Result<(), Box<dyn std::error::Error>> {
        let mut sc = Scorecard::new(ScorecardMode::Gate);
        sc.add_receipt(make_receipt(ShadowQueryName::FindDefinition, ShadowCompareVerdict::Same));

        let report = sc.report();
        assert!(report.passed, "Gate mode should pass with no regressions");
        assert_eq!(report.mode, ScorecardMode::Gate);
        Ok(())
    }

    #[test]
    fn gate_mode_fails_with_regression() -> Result<(), Box<dyn std::error::Error>> {
        let mut sc = Scorecard::new(ScorecardMode::Gate);
        sc.add_receipt(make_receipt(
            ShadowQueryName::FindDefinition,
            ShadowCompareVerdict::Regression,
        ));

        let report = sc.report();
        assert!(!report.passed, "Gate mode should fail with regressions");
        Ok(())
    }

    // ── Per-query grouping ──

    #[test]
    fn report_groups_by_query_name() -> Result<(), Box<dyn std::error::Error>> {
        let mut sc = Scorecard::new(ScorecardMode::Emit);
        sc.add_receipt(make_receipt(ShadowQueryName::FindDefinition, ShadowCompareVerdict::Same));
        sc.add_receipt(make_receipt(
            ShadowQueryName::FindDefinition,
            ShadowCompareVerdict::Improved,
        ));
        sc.add_receipt(make_receipt(
            ShadowQueryName::FindReferences,
            ShadowCompareVerdict::Regression,
        ));
        sc.add_receipt(make_receipt(
            ShadowQueryName::CountUsages,
            ShadowCompareVerdict::Unavailable,
        ));

        let report = sc.report();

        let def_counts = report.by_query.get("find_definition").ok_or("missing find_definition")?;
        assert_eq!(def_counts.same, 1);
        assert_eq!(def_counts.improved, 1);
        assert_eq!(def_counts.total(), 2);

        let ref_counts = report.by_query.get("find_references").ok_or("missing find_references")?;
        assert_eq!(ref_counts.regression, 1);
        assert_eq!(ref_counts.total(), 1);

        let usage_counts = report.by_query.get("count_usages").ok_or("missing count_usages")?;
        assert_eq!(usage_counts.unavailable, 1);
        assert_eq!(usage_counts.total(), 1);

        // Totals should be the sum across all queries.
        assert_eq!(report.totals.total(), 4);
        assert_eq!(report.totals.same, 1);
        assert_eq!(report.totals.improved, 1);
        assert_eq!(report.totals.regression, 1);
        assert_eq!(report.totals.unavailable, 1);
        Ok(())
    }

    // ── Empty scorecard ──

    #[test]
    fn empty_scorecard_passes_in_all_modes() -> Result<(), Box<dyn std::error::Error>> {
        for mode in [ScorecardMode::Emit, ScorecardMode::Check, ScorecardMode::Gate] {
            let sc = Scorecard::new(mode);
            let report = sc.report();
            assert!(report.passed, "empty scorecard should pass in {mode:?}");
            assert_eq!(report.totals.total(), 0);
            assert!(report.by_query.is_empty());
        }
        Ok(())
    }

    // ── add_receipts batch ──

    #[test]
    fn add_receipts_batch_works() -> Result<(), Box<dyn std::error::Error>> {
        let mut sc = Scorecard::new(ScorecardMode::Check);
        let batch = vec![
            make_receipt(ShadowQueryName::FindDefinition, ShadowCompareVerdict::Same),
            make_receipt(ShadowQueryName::FindReferences, ShadowCompareVerdict::Improved),
        ];
        sc.add_receipts(batch);

        assert_eq!(sc.receipt_count(), 2);
        let report = sc.report();
        assert!(report.passed);
        assert_eq!(report.totals.total(), 2);
        Ok(())
    }

    // ── ScorecardReport JSON round-trip ──

    #[test]
    fn scorecard_report_json_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let mut sc = Scorecard::new(ScorecardMode::Check);
        sc.add_receipt(make_receipt(ShadowQueryName::FindDefinition, ShadowCompareVerdict::Same));
        sc.add_receipt(make_receipt(
            ShadowQueryName::FindReferences,
            ShadowCompareVerdict::Improved,
        ));

        let report = sc.report();
        let json = serde_json::to_string(&report)?;
        let deserialized: ScorecardReport = serde_json::from_str(&json)?;
        assert_eq!(report, deserialized);
        Ok(())
    }

    // ── ScorecardMode JSON round-trip ──

    #[test]
    fn scorecard_mode_json_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        for mode in [ScorecardMode::Emit, ScorecardMode::Check, ScorecardMode::Gate] {
            let json = serde_json::to_string(&mode)?;
            let deserialized: ScorecardMode = serde_json::from_str(&json)?;
            assert_eq!(mode, deserialized);
        }
        Ok(())
    }

    // ── New semantic query names in scorecard ──

    #[test]
    fn report_groups_new_semantic_query_names() -> Result<(), Box<dyn std::error::Error>> {
        let mut sc = Scorecard::new(ScorecardMode::Check);
        sc.add_receipt(make_receipt(ShadowQueryName::VisibleSymbols, ShadowCompareVerdict::Same));
        sc.add_receipt(make_receipt(
            ShadowQueryName::MethodCandidates,
            ShadowCompareVerdict::Improved,
        ));
        sc.add_receipt(make_receipt(ShadowQueryName::SymbolAt, ShadowCompareVerdict::Same));
        sc.add_receipt(make_receipt(ShadowQueryName::RenamePlan, ShadowCompareVerdict::Ambiguous));
        sc.add_receipt(make_receipt(ShadowQueryName::SafeDeletePlan, ShadowCompareVerdict::Same));
        sc.add_receipt(make_receipt(
            ShadowQueryName::CompletionVisibility,
            ShadowCompareVerdict::Improved,
        ));
        sc.add_receipt(make_receipt(ShadowQueryName::DiagnosticsCheck, ShadowCompareVerdict::Same));

        let report = sc.report();
        assert!(report.passed, "no regressions should pass");
        assert_eq!(report.totals.total(), 7);

        let vis = report.by_query.get("visible_symbols").ok_or("missing visible_symbols")?;
        assert_eq!(vis.same, 1);

        let mc = report.by_query.get("method_candidates").ok_or("missing method_candidates")?;
        assert_eq!(mc.improved, 1);

        let sa = report.by_query.get("symbol_at").ok_or("missing symbol_at")?;
        assert_eq!(sa.same, 1);

        let rp = report.by_query.get("rename_plan").ok_or("missing rename_plan")?;
        assert_eq!(rp.ambiguous, 1);

        let sdp = report.by_query.get("safe_delete_plan").ok_or("missing safe_delete_plan")?;
        assert_eq!(sdp.same, 1);

        let cv =
            report.by_query.get("completion_visibility").ok_or("missing completion_visibility")?;
        assert_eq!(cv.improved, 1);

        let dc = report.by_query.get("diagnostics_check").ok_or("missing diagnostics_check")?;
        assert_eq!(dc.same, 1);

        Ok(())
    }

    // ── Latency measurement helpers ──

    #[test]
    fn compute_p95_empty_returns_zero() -> Result<(), Box<dyn std::error::Error>> {
        let p95 = super::compute_p95(&[]);
        assert_eq!(p95, Duration::ZERO);
        Ok(())
    }

    #[test]
    fn compute_p95_single_sample() -> Result<(), Box<dyn std::error::Error>> {
        let samples = [Duration::from_micros(100)];
        let p95 = super::compute_p95(&samples);
        assert_eq!(p95, Duration::from_micros(100));
        Ok(())
    }

    #[test]
    fn compute_p95_twenty_samples() -> Result<(), Box<dyn std::error::Error>> {
        // 20 samples: 1..=20 ms. p95 index = ceil(20 * 0.95) - 1 = 19 - 1 = 18 → 19ms.
        let mut samples: Vec<Duration> = (1..=20).map(Duration::from_millis).collect();
        samples.sort();
        let p95 = super::compute_p95(&samples);
        assert_eq!(p95, Duration::from_millis(19));
        Ok(())
    }

    #[test]
    fn compute_p95_hundred_samples() -> Result<(), Box<dyn std::error::Error>> {
        // 100 samples: 1..=100 µs. p95 index = ceil(100 * 0.95) - 1 = 95 - 1 = 94 → 95µs.
        let mut samples: Vec<Duration> = (1..=100).map(Duration::from_micros).collect();
        samples.sort();
        let p95 = super::compute_p95(&samples);
        assert_eq!(p95, Duration::from_micros(95));
        Ok(())
    }

    #[test]
    fn build_latency_measurement_within_threshold() -> Result<(), Box<dyn std::error::Error>> {
        let mut samples: Vec<Duration> = (1..=100).map(Duration::from_micros).collect();
        let m = super::build_latency_measurement("symbol_at", &mut samples, 5_000);
        assert_eq!(m.query_name, "symbol_at");
        assert_eq!(m.sample_count, 100);
        assert_eq!(m.p95_micros, 95);
        assert_eq!(m.threshold_micros, 5_000);
        assert!(!m.exceeded);
        Ok(())
    }

    #[test]
    fn build_latency_measurement_exceeds_threshold() -> Result<(), Box<dyn std::error::Error>> {
        // All samples at 10ms → p95 = 10ms, threshold = 5ms → exceeded.
        let mut samples: Vec<Duration> = (0..100).map(|_| Duration::from_millis(10)).collect();
        let m = super::build_latency_measurement("symbol_at", &mut samples, 5_000);
        assert!(m.exceeded);
        assert_eq!(m.p95_micros, 10_000);
        Ok(())
    }

    #[test]
    fn latency_thresholds_for_known_queries() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(LatencyThresholds::for_query("symbol_at"), Some(5_000));
        assert_eq!(LatencyThresholds::for_query("definitions"), Some(10_000));
        assert_eq!(LatencyThresholds::for_query("references"), Some(20_000));
        assert_eq!(LatencyThresholds::for_query("visible_symbols_at"), Some(15_000));
        assert_eq!(LatencyThresholds::for_query("unknown_query"), None);
        Ok(())
    }

    // ── Scorecard latency integration ──

    #[test]
    fn scorecard_report_includes_latency_measurements() -> Result<(), Box<dyn std::error::Error>> {
        let mut sc = Scorecard::new(ScorecardMode::Emit);
        sc.add_latency(LatencyMeasurement {
            query_name: "symbol_at".to_string(),
            sample_count: 100,
            p95_micros: 2_000,
            threshold_micros: 5_000,
            exceeded: false,
        });
        sc.add_latency(LatencyMeasurement {
            query_name: "definitions".to_string(),
            sample_count: 100,
            p95_micros: 8_000,
            threshold_micros: 10_000,
            exceeded: false,
        });

        let report = sc.report();
        assert_eq!(report.latency.len(), 2);
        assert!(report.latency_violations.is_empty());

        let sa = report.latency.get("symbol_at").ok_or("missing symbol_at latency")?;
        assert_eq!(sa.p95_micros, 2_000);
        assert!(!sa.exceeded);

        let def = report.latency.get("definitions").ok_or("missing definitions latency")?;
        assert_eq!(def.p95_micros, 8_000);
        assert!(!def.exceeded);
        Ok(())
    }

    #[test]
    fn scorecard_report_flags_latency_violations() -> Result<(), Box<dyn std::error::Error>> {
        let mut sc = Scorecard::new(ScorecardMode::Check);
        sc.add_latency(LatencyMeasurement {
            query_name: "symbol_at".to_string(),
            sample_count: 100,
            p95_micros: 2_000,
            threshold_micros: 5_000,
            exceeded: false,
        });
        sc.add_latency(LatencyMeasurement {
            query_name: "references".to_string(),
            sample_count: 100,
            p95_micros: 25_000,
            threshold_micros: 20_000,
            exceeded: true,
        });

        let report = sc.report();
        assert_eq!(report.latency_violations.len(), 1);
        assert_eq!(report.latency_violations[0].query_name, "references");
        assert_eq!(report.latency_violations[0].p95_micros, 25_000);
        assert_eq!(report.latency_violations[0].threshold_micros, 20_000);
        Ok(())
    }

    #[test]
    fn scorecard_add_latencies_batch() -> Result<(), Box<dyn std::error::Error>> {
        let mut sc = Scorecard::new(ScorecardMode::Emit);
        let measurements = vec![
            LatencyMeasurement {
                query_name: "symbol_at".to_string(),
                sample_count: 50,
                p95_micros: 1_000,
                threshold_micros: 5_000,
                exceeded: false,
            },
            LatencyMeasurement {
                query_name: "definitions".to_string(),
                sample_count: 50,
                p95_micros: 7_000,
                threshold_micros: 10_000,
                exceeded: false,
            },
        ];
        sc.add_latencies(measurements);

        let report = sc.report();
        assert_eq!(report.latency.len(), 2);
        assert!(report.latency_violations.is_empty());
        Ok(())
    }

    #[test]
    fn scorecard_report_with_latency_json_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let mut sc = Scorecard::new(ScorecardMode::Check);
        sc.add_receipt(make_receipt(ShadowQueryName::FindDefinition, ShadowCompareVerdict::Same));
        sc.add_latency(LatencyMeasurement {
            query_name: "symbol_at".to_string(),
            sample_count: 100,
            p95_micros: 3_000,
            threshold_micros: 5_000,
            exceeded: false,
        });
        sc.add_latency(LatencyMeasurement {
            query_name: "references".to_string(),
            sample_count: 100,
            p95_micros: 25_000,
            threshold_micros: 20_000,
            exceeded: true,
        });

        let report = sc.report();
        let json = serde_json::to_string(&report)?;
        let deserialized: ScorecardReport = serde_json::from_str(&json)?;
        assert_eq!(report, deserialized);
        Ok(())
    }

    #[test]
    fn empty_scorecard_has_empty_latency() -> Result<(), Box<dyn std::error::Error>> {
        let sc = Scorecard::new(ScorecardMode::Emit);
        let report = sc.report();
        assert!(report.latency.is_empty());
        assert!(report.latency_violations.is_empty());
        Ok(())
    }

    // ── Rename unsafe-edit count (Req 11.6) ──

    /// Helper: build a rename plan with all edits properly classified.
    fn make_safe_rename_plan() -> RenamePlan {
        use perl_semantic_facts::{AnchorId, EntityId, FileId, PlannedEdit, PlannedEditCategory};
        RenamePlan::new(
            EntityId(100),
            "old_name".to_string(),
            "new_name".to_string(),
            vec![
                PlannedEdit::new(
                    AnchorId(1),
                    FileId(1),
                    PlannedEditCategory::Definition,
                    "old_name".to_string(),
                    "new_name".to_string(),
                ),
                PlannedEdit::new(
                    AnchorId(2),
                    FileId(1),
                    PlannedEditCategory::Reference,
                    "old_name".to_string(),
                    "new_name".to_string(),
                ),
                PlannedEdit::new(
                    AnchorId(3),
                    FileId(2),
                    PlannedEditCategory::ImportList,
                    "old_name".to_string(),
                    "new_name".to_string(),
                ),
                PlannedEdit::new(
                    AnchorId(4),
                    FileId(2),
                    PlannedEditCategory::ExportList,
                    "old_name".to_string(),
                    "new_name".to_string(),
                ),
            ],
            vec![],
            vec![],
        )
    }

    /// Helper: build a rename plan with an UnclassifiedOccurrence blocker.
    fn make_rename_plan_with_unclassified_blocker() -> RenamePlan {
        use perl_semantic_facts::{EntityId, PlanBlocker, PlanBlockerReason};
        RenamePlan::new(
            EntityId(200),
            "problematic".to_string(),
            "renamed".to_string(),
            vec![],
            vec![PlanBlocker::new(
                PlanBlockerReason::UnclassifiedOccurrence,
                None,
                "occurrence could not be classified".to_string(),
            )],
            vec![],
        )
    }

    #[test]
    fn rename_unsafe_edit_count_zero_for_classified_plans() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut sc = Scorecard::new(ScorecardMode::Check);
        sc.add_rename_plan(make_safe_rename_plan());

        let report = sc.report();
        assert_eq!(
            report.rename_unsafe_edit_count, 0,
            "all edits are classified, unsafe count should be zero"
        );
        assert!(report.passed, "scorecard should pass with zero unsafe edits and no regressions");
        Ok(())
    }

    #[test]
    fn rename_unsafe_edit_count_nonzero_blocks_check_mode() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut sc = Scorecard::new(ScorecardMode::Check);
        sc.add_rename_plan(make_rename_plan_with_unclassified_blocker());

        let report = sc.report();
        assert_eq!(
            report.rename_unsafe_edit_count, 1,
            "unclassified occurrence blocker should count as unsafe"
        );
        assert!(!report.passed, "Check mode should fail when rename_unsafe_edit_count > 0");
        Ok(())
    }

    #[test]
    fn rename_unsafe_edit_count_nonzero_blocks_gate_mode() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut sc = Scorecard::new(ScorecardMode::Gate);
        sc.add_rename_plan(make_rename_plan_with_unclassified_blocker());

        let report = sc.report();
        assert_eq!(report.rename_unsafe_edit_count, 1);
        assert!(!report.passed, "Gate mode should fail when rename_unsafe_edit_count > 0");
        Ok(())
    }

    #[test]
    fn emit_mode_passes_even_with_unsafe_rename_edits() -> Result<(), Box<dyn std::error::Error>> {
        let mut sc = Scorecard::new(ScorecardMode::Emit);
        sc.add_rename_plan(make_rename_plan_with_unclassified_blocker());

        let report = sc.report();
        assert_eq!(report.rename_unsafe_edit_count, 1);
        assert!(report.passed, "Emit mode should always pass regardless of unsafe edits");
        Ok(())
    }

    #[test]
    fn rename_plans_batch_add() -> Result<(), Box<dyn std::error::Error>> {
        let mut sc = Scorecard::new(ScorecardMode::Check);
        sc.add_rename_plans(vec![make_safe_rename_plan(), make_safe_rename_plan()]);

        let report = sc.report();
        assert_eq!(report.rename_unsafe_edit_count, 0);
        assert!(report.passed);
        Ok(())
    }

    #[test]
    fn empty_scorecard_has_zero_rename_unsafe_edits() -> Result<(), Box<dyn std::error::Error>> {
        let sc = Scorecard::new(ScorecardMode::Check);
        let report = sc.report();
        assert_eq!(report.rename_unsafe_edit_count, 0);
        Ok(())
    }

    // ── Req 11.1: Aggregate across all providers ──

    #[test]
    fn aggregate_receipts_across_all_providers_and_fixtures()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut sc = Scorecard::new(ScorecardMode::Check);

        // Simulate receipts from multiple providers and fixture suites.
        // Goto-definition provider
        sc.add_receipt(make_receipt(ShadowQueryName::FindDefinition, ShadowCompareVerdict::Same));
        sc.add_receipt(make_receipt(
            ShadowQueryName::FindDefinition,
            ShadowCompareVerdict::Improved,
        ));
        // Find-references provider
        sc.add_receipt(make_receipt(ShadowQueryName::FindReferences, ShadowCompareVerdict::Same));
        // Completion provider
        sc.add_receipt(make_receipt(
            ShadowQueryName::CompletionVisibility,
            ShadowCompareVerdict::Same,
        ));
        // Diagnostics provider
        sc.add_receipt(make_receipt(
            ShadowQueryName::DiagnosticsCheck,
            ShadowCompareVerdict::Improved,
        ));
        // Rename provider
        sc.add_receipt(make_receipt(ShadowQueryName::RenamePlan, ShadowCompareVerdict::Same));
        // Safe-delete provider
        sc.add_receipt(make_receipt(ShadowQueryName::SafeDeletePlan, ShadowCompareVerdict::Same));
        // Hover provider
        sc.add_receipt(make_receipt(ShadowQueryName::Hover, ShadowCompareVerdict::Same));

        let report = sc.report();
        assert!(report.passed, "aggregate scorecard should pass with no regressions");
        assert_eq!(report.totals.total(), 8, "all 8 receipts should be counted");
        assert_eq!(report.totals.same, 6);
        assert_eq!(report.totals.improved, 2);
        assert_eq!(report.totals.regression, 0);

        // Verify all provider query names are represented.
        assert!(report.by_query.contains_key("find_definition"));
        assert!(report.by_query.contains_key("find_references"));
        assert!(report.by_query.contains_key("completion_visibility"));
        assert!(report.by_query.contains_key("diagnostics_check"));
        assert!(report.by_query.contains_key("rename_plan"));
        assert!(report.by_query.contains_key("safe_delete_plan"));
        assert!(report.by_query.contains_key("hover"));
        Ok(())
    }

    // ── Req 11.2: Per-query verdicts with counts ──

    #[test]
    fn per_query_verdicts_report_all_five_categories() -> Result<(), Box<dyn std::error::Error>> {
        let mut sc = Scorecard::new(ScorecardMode::Emit);

        // Add one of each verdict for find_definition.
        sc.add_receipt(make_receipt(ShadowQueryName::FindDefinition, ShadowCompareVerdict::Same));
        sc.add_receipt(make_receipt(
            ShadowQueryName::FindDefinition,
            ShadowCompareVerdict::Improved,
        ));
        sc.add_receipt(make_receipt(
            ShadowQueryName::FindDefinition,
            ShadowCompareVerdict::Regression,
        ));
        sc.add_receipt(make_receipt(
            ShadowQueryName::FindDefinition,
            ShadowCompareVerdict::Ambiguous,
        ));
        sc.add_receipt(make_receipt(
            ShadowQueryName::FindDefinition,
            ShadowCompareVerdict::Unavailable,
        ));

        let report = sc.report();
        let def = report.by_query.get("find_definition").ok_or("missing find_definition")?;
        assert_eq!(def.same, 1, "Same count");
        assert_eq!(def.improved, 1, "Improved count");
        assert_eq!(def.regression, 1, "Regression count");
        assert_eq!(def.ambiguous, 1, "Ambiguous count");
        assert_eq!(def.unavailable, 1, "Unavailable count");
        assert_eq!(def.total(), 5, "total count");

        // Totals should match.
        assert_eq!(report.totals.same, 1);
        assert_eq!(report.totals.improved, 1);
        assert_eq!(report.totals.regression, 1);
        assert_eq!(report.totals.ambiguous, 1);
        assert_eq!(report.totals.unavailable, 1);
        Ok(())
    }

    // ── Req 11.8: Three modes ──

    #[test]
    fn check_mode_fails_on_regression_and_unsafe_edits() -> Result<(), Box<dyn std::error::Error>> {
        // Regression alone fails.
        let mut sc = Scorecard::new(ScorecardMode::Check);
        sc.add_receipt(make_receipt(
            ShadowQueryName::FindDefinition,
            ShadowCompareVerdict::Regression,
        ));
        assert!(!sc.report().passed, "Check mode should fail on regression");

        // Unsafe edits alone fail.
        let mut sc2 = Scorecard::new(ScorecardMode::Check);
        sc2.add_rename_plan(make_rename_plan_with_unclassified_blocker());
        assert!(!sc2.report().passed, "Check mode should fail on unsafe edits");

        // Both regression and unsafe edits fail.
        let mut sc3 = Scorecard::new(ScorecardMode::Check);
        sc3.add_receipt(make_receipt(
            ShadowQueryName::FindDefinition,
            ShadowCompareVerdict::Regression,
        ));
        sc3.add_rename_plan(make_rename_plan_with_unclassified_blocker());
        assert!(!sc3.report().passed, "Check mode should fail on both");
        Ok(())
    }

    #[test]
    fn scorecard_report_with_rename_plans_json_round_trip() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut sc = Scorecard::new(ScorecardMode::Check);
        sc.add_receipt(make_receipt(ShadowQueryName::FindDefinition, ShadowCompareVerdict::Same));
        sc.add_rename_plan(make_safe_rename_plan());

        let report = sc.report();
        let json = serde_json::to_string(&report)?;
        let deserialized: ScorecardReport = serde_json::from_str(&json)?;
        assert_eq!(report, deserialized);
        Ok(())
    }
}
