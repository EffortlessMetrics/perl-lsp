use serde::{Deserialize, Serialize};

use perl_semantic_facts::ProviderFactTrace;

/// Current semantic shadow-compare receipt schema version.
pub const SEMANTIC_SHADOW_COMPARE_RECEIPT_SCHEMA_VERSION: u32 = 2;

/// Deterministic verdict for semantic shadow compare receipts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShadowCompareVerdict {
    /// Old and new query answers are semantically equivalent.
    Same,
    /// New answer is strictly better than old answer.
    Improved,
    /// New answer is strictly worse than old answer.
    Regression,
    /// Comparison cannot be decisively classified.
    Ambiguous,
    /// Required fact-backed result is missing; comparison unavailable.
    Unavailable,
}

/// Query names covered by semantic shadow compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShadowQueryName {
    /// `find_definition` query.
    FindDefinition,
    /// `find_references` query.
    FindReferences,
    /// `count_usages` query.
    CountUsages,
    /// `visible_symbols_at` query (SemanticQueries facade).
    VisibleSymbols,
    /// `method_candidates` query (SemanticQueries facade).
    MethodCandidates,
    /// `symbol_at` query (SemanticQueries facade).
    SymbolAt,
    /// `rename_plan` query (SemanticQueries facade).
    RenamePlan,
    /// `safe_delete_plan` query (SemanticQueries facade).
    SafeDeletePlan,
    /// Completion provider visibility query (SemanticQueries facade).
    CompletionVisibility,
    /// Diagnostics provider check query (SemanticQueries facade).
    DiagnosticsCheck,
    /// Hover provider query (SemanticQueries facade).
    Hover,
}

/// Canonical query input payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShadowQueryInput {
    /// Symbol text sent to the query.
    pub symbol: String,
}

/// Compact deterministic summary for query outputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShadowResultSummary {
    /// Whether a fact-backed result exists.
    pub available: bool,
    /// Number of matching items (0/1 for definition, N for references/usages).
    pub match_count: u64,
    /// Stable identity set for deterministic diffing.
    pub identities: Vec<String>,
}

/// Full semantic shadow-compare receipt record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticShadowCompareReceipt {
    /// Schema version for forward-compatible evolution.
    pub schema_version: u32,
    /// Query name.
    pub query: ShadowQueryName,
    /// Query input.
    pub input: ShadowQueryInput,
    /// Old-path summary.
    pub old_result: ShadowResultSummary,
    /// New-path summary.
    pub new_result: ShadowResultSummary,
    /// Comparison verdict.
    pub verdict: ShadowCompareVerdict,
    /// Additional notes for operators.
    pub notes: Vec<String>,
    /// Typed fact-source traces for provider cutover proof.
    pub fact_source_traces: Vec<ProviderFactTrace>,
}

impl SemanticShadowCompareReceipt {
    /// Build a deterministic receipt and compute verdict from summaries.
    pub fn from_summaries(
        query: ShadowQueryName,
        input: ShadowQueryInput,
        old_result: ShadowResultSummary,
        new_result: ShadowResultSummary,
        notes: Vec<String>,
    ) -> Self {
        Self::from_summaries_with_fact_source_traces(
            query,
            input,
            old_result,
            new_result,
            notes,
            Vec::new(),
        )
    }

    /// Build a deterministic receipt with typed provider fact-source traces.
    pub fn from_summaries_with_fact_source_traces(
        query: ShadowQueryName,
        input: ShadowQueryInput,
        old_result: ShadowResultSummary,
        new_result: ShadowResultSummary,
        notes: Vec<String>,
        fact_source_traces: Vec<ProviderFactTrace>,
    ) -> Self {
        let verdict = classify_verdict(&old_result, &new_result);
        Self {
            schema_version: SEMANTIC_SHADOW_COMPARE_RECEIPT_SCHEMA_VERSION,
            query,
            input,
            old_result,
            new_result,
            verdict,
            notes,
            fact_source_traces,
        }
    }
}

fn classify_verdict(
    old_result: &ShadowResultSummary,
    new_result: &ShadowResultSummary,
) -> ShadowCompareVerdict {
    if !old_result.available || !new_result.available {
        return ShadowCompareVerdict::Unavailable;
    }

    if old_result == new_result {
        return ShadowCompareVerdict::Same;
    }

    // Check count direction first so that count-only queries (e.g. CountUsages) that produce
    // no identity strings but a non-zero match_count still get Improved/Regression rather
    // than falling into the identity-equality Ambiguous arm below.
    if new_result.match_count > old_result.match_count {
        return ShadowCompareVerdict::Improved;
    }
    if new_result.match_count < old_result.match_count {
        return ShadowCompareVerdict::Regression;
    }

    // Counts are equal but structs differ: identity sets must differ (available is already
    // asserted true for both). Different identities at the same count is ambiguous —
    // we cannot decide which answer is better without domain context.
    ShadowCompareVerdict::Ambiguous
}

/// Build a stable summary from an optional set of identities.
pub fn summarize_identities(identities: Option<Vec<String>>) -> ShadowResultSummary {
    match identities {
        Some(mut values) => {
            values.sort();
            values.dedup();
            let match_count = u64::try_from(values.len()).unwrap_or(u64::MAX);
            ShadowResultSummary { available: true, match_count, identities: values }
        }
        None => ShadowResultSummary { available: false, match_count: 0, identities: Vec::new() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use proptest::test_runner::Config as ProptestConfig;

    #[test]
    fn receipt_json_shape_is_stable() -> Result<(), Box<dyn std::error::Error>> {
        let receipt = SemanticShadowCompareReceipt::from_summaries(
            ShadowQueryName::FindReferences,
            ShadowQueryInput { symbol: "My::pkg::f".to_string() },
            summarize_identities(Some(vec!["b.pm:3:2".to_string(), "a.pm:1:1".to_string()])),
            summarize_identities(Some(vec!["a.pm:1:1".to_string(), "c.pm:9:9".to_string()])),
            vec!["fixture=h1".to_string()],
        );

        let got: serde_json::Value = serde_json::to_value(&receipt)?;
        let expected = serde_json::json!({
            "schema_version": 2,
            "query": "find_references",
            "input": {"symbol": "My::pkg::f"},
            "old_result": {
                "available": true,
                "match_count": 2,
                "identities": ["a.pm:1:1", "b.pm:3:2"]
            },
            "new_result": {
                "available": true,
                "match_count": 2,
                "identities": ["a.pm:1:1", "c.pm:9:9"]
            },
            "verdict": "ambiguous",
            "notes": ["fixture=h1"],
            "fact_source_traces": []
        });
        assert_eq!(got, expected);
        Ok(())
    }

    #[test]
    fn receipt_json_shape_includes_provider_fact_source_traces()
    -> Result<(), Box<dyn std::error::Error>> {
        let trace = ProviderFactTrace::new(
            perl_semantic_facts::ProviderSurface::Definition,
            perl_semantic_facts::ProviderFactSourceKind::CompilerFact,
            perl_semantic_facts::Provenance::SemanticAnalyzer,
            perl_semantic_facts::Confidence::High,
            perl_semantic_facts::ProviderFactFreshness::Fresh,
            perl_semantic_facts::ProviderFallbackState::Shadow,
            Some("fixture-source-sha".to_string()),
            Some(perl_semantic_facts::AnchorId(10)),
            Some(1),
        );
        let receipt = SemanticShadowCompareReceipt::from_summaries_with_fact_source_traces(
            ShadowQueryName::FindDefinition,
            ShadowQueryInput { symbol: "Foo::bar".to_string() },
            summarize_identities(Some(vec!["lib/Foo.pm:10:5".to_string()])),
            summarize_identities(Some(vec!["lib/Foo.pm:10:5".to_string()])),
            vec!["fact-source trace fixture".to_string()],
            vec![trace],
        );

        let got: serde_json::Value = serde_json::to_value(&receipt)?;
        let expected = serde_json::json!({
            "schema_version": 2,
            "query": "find_definition",
            "input": {"symbol": "Foo::bar"},
            "old_result": {
                "available": true,
                "match_count": 1,
                "identities": ["lib/Foo.pm:10:5"]
            },
            "new_result": {
                "available": true,
                "match_count": 1,
                "identities": ["lib/Foo.pm:10:5"]
            },
            "verdict": "same",
            "notes": ["fact-source trace fixture"],
            "fact_source_traces": [{
                "surface": "Definition",
                "source": "CompilerFact",
                "provenance": "SemanticAnalyzer",
                "confidence": "High",
                "freshness": "Fresh",
                "fallback_state": "Shadow",
                "source_hash": "fixture-source-sha",
                "anchor_id": 10,
                "model_version": 1
            }]
        });
        assert_eq!(got, expected);
        Ok(())
    }

    #[test]
    fn unavailable_when_fact_path_missing() {
        let receipt = SemanticShadowCompareReceipt::from_summaries(
            ShadowQueryName::FindDefinition,
            ShadowQueryInput { symbol: "X::y".to_string() },
            summarize_identities(None),
            summarize_identities(Some(vec!["x.pm:1:1".to_string()])),
            vec!["old path missing fact-backed answer".to_string()],
        );
        assert_eq!(receipt.verdict, ShadowCompareVerdict::Unavailable);
    }

    #[test]
    fn same_verdict_when_results_identical() {
        let summary = summarize_identities(Some(vec!["a.pm:1:1".to_string()]));
        let receipt = SemanticShadowCompareReceipt::from_summaries(
            ShadowQueryName::FindDefinition,
            ShadowQueryInput { symbol: "Foo::bar".to_string() },
            summary.clone(),
            summary,
            vec![],
        );
        assert_eq!(receipt.verdict, ShadowCompareVerdict::Same);
    }

    #[test]
    fn improved_verdict_when_new_has_more_matches() {
        let receipt = SemanticShadowCompareReceipt::from_summaries(
            ShadowQueryName::FindReferences,
            ShadowQueryInput { symbol: "Foo::bar".to_string() },
            summarize_identities(Some(vec!["a.pm:1:1".to_string()])),
            summarize_identities(Some(vec!["a.pm:1:1".to_string(), "b.pm:2:2".to_string()])),
            vec![],
        );
        assert_eq!(receipt.verdict, ShadowCompareVerdict::Improved);
    }

    #[test]
    fn regression_verdict_when_new_has_fewer_matches() {
        let receipt = SemanticShadowCompareReceipt::from_summaries(
            ShadowQueryName::FindReferences,
            ShadowQueryInput { symbol: "Foo::bar".to_string() },
            summarize_identities(Some(vec!["a.pm:1:1".to_string(), "b.pm:2:2".to_string()])),
            summarize_identities(Some(vec!["a.pm:1:1".to_string()])),
            vec![],
        );
        assert_eq!(receipt.verdict, ShadowCompareVerdict::Regression);
    }

    /// `CountUsages` produces identity-free summaries; verdict must be based on
    /// `match_count` alone. The old logic incorrectly returned `Ambiguous` here
    /// because the identity-equality check (`[] == []`) fired before the count
    /// comparison, hiding the numeric difference.
    #[test]
    fn count_usages_improved_with_empty_identities() {
        let old_summary =
            ShadowResultSummary { available: true, match_count: 3, identities: vec![] };
        let new_summary =
            ShadowResultSummary { available: true, match_count: 5, identities: vec![] };
        let receipt = SemanticShadowCompareReceipt::from_summaries(
            ShadowQueryName::CountUsages,
            ShadowQueryInput { symbol: "Foo::bar".to_string() },
            old_summary,
            new_summary,
            vec![],
        );
        assert_eq!(receipt.verdict, ShadowCompareVerdict::Improved);
    }

    /// Symmetric regression case for `CountUsages`.
    #[test]
    fn count_usages_regression_with_empty_identities() {
        let old_summary =
            ShadowResultSummary { available: true, match_count: 5, identities: vec![] };
        let new_summary =
            ShadowResultSummary { available: true, match_count: 2, identities: vec![] };
        let receipt = SemanticShadowCompareReceipt::from_summaries(
            ShadowQueryName::CountUsages,
            ShadowQueryInput { symbol: "Foo::bar".to_string() },
            old_summary,
            new_summary,
            vec![],
        );
        assert_eq!(receipt.verdict, ShadowCompareVerdict::Regression);
    }

    /// Both paths unavailable: verdict must still be `Unavailable`, not `Same`.
    #[test]
    fn both_unavailable_yields_unavailable() {
        let receipt = SemanticShadowCompareReceipt::from_summaries(
            ShadowQueryName::FindDefinition,
            ShadowQueryInput { symbol: "Foo::bar".to_string() },
            summarize_identities(None),
            summarize_identities(None),
            vec![],
        );
        assert_eq!(receipt.verdict, ShadowCompareVerdict::Unavailable);
    }

    #[test]
    fn summarize_identities_sorts_and_deduplicates() {
        let summary = summarize_identities(Some(vec![
            "c.pm:3:1".to_string(),
            "a.pm:1:1".to_string(),
            "a.pm:1:1".to_string(),
            "b.pm:2:2".to_string(),
        ]));
        assert!(summary.available);
        assert_eq!(summary.match_count, 3);
        assert_eq!(
            summary.identities,
            vec!["a.pm:1:1".to_string(), "b.pm:2:2".to_string(), "c.pm:3:1".to_string()]
        );
    }

    /// All `ShadowQueryName` variants round-trip through JSON with stable
    /// snake_case keys.
    #[test]
    fn shadow_query_name_json_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let variants = [
            (ShadowQueryName::FindDefinition, "\"find_definition\""),
            (ShadowQueryName::FindReferences, "\"find_references\""),
            (ShadowQueryName::CountUsages, "\"count_usages\""),
            (ShadowQueryName::VisibleSymbols, "\"visible_symbols\""),
            (ShadowQueryName::MethodCandidates, "\"method_candidates\""),
            (ShadowQueryName::SymbolAt, "\"symbol_at\""),
            (ShadowQueryName::RenamePlan, "\"rename_plan\""),
            (ShadowQueryName::SafeDeletePlan, "\"safe_delete_plan\""),
            (ShadowQueryName::CompletionVisibility, "\"completion_visibility\""),
            (ShadowQueryName::DiagnosticsCheck, "\"diagnostics_check\""),
        ];
        for (variant, expected_json) in variants {
            let json = serde_json::to_string(&variant)?;
            assert_eq!(json, expected_json, "serialization mismatch for {variant:?}");
            let deserialized: ShadowQueryName = serde_json::from_str(&json)?;
            assert_eq!(deserialized, variant, "round-trip mismatch for {variant:?}");
        }
        Ok(())
    }

    /// Receipt JSON shape is stable for new semantic query variants.
    #[test]
    fn receipt_json_shape_stable_for_visible_symbols() -> Result<(), Box<dyn std::error::Error>> {
        let receipt = SemanticShadowCompareReceipt::from_summaries(
            ShadowQueryName::VisibleSymbols,
            ShadowQueryInput { symbol: "my_func".to_string() },
            summarize_identities(Some(vec!["a.pm:1:1".to_string()])),
            summarize_identities(Some(vec!["a.pm:1:1".to_string(), "b.pm:2:2".to_string()])),
            vec![],
        );

        let got: serde_json::Value = serde_json::to_value(&receipt)?;
        let expected = serde_json::json!({
            "schema_version": 2,
            "query": "visible_symbols",
            "input": {"symbol": "my_func"},
            "old_result": {
                "available": true,
                "match_count": 1,
                "identities": ["a.pm:1:1"]
            },
            "new_result": {
                "available": true,
                "match_count": 2,
                "identities": ["a.pm:1:1", "b.pm:2:2"]
            },
            "verdict": "improved",
            "notes": [],
            "fact_source_traces": []
        });
        assert_eq!(got, expected);
        Ok(())
    }

    /// Receipt JSON shape is stable for method_candidates query.
    #[test]
    fn receipt_json_shape_stable_for_method_candidates() -> Result<(), Box<dyn std::error::Error>> {
        let receipt = SemanticShadowCompareReceipt::from_summaries(
            ShadowQueryName::MethodCandidates,
            ShadowQueryInput { symbol: "new".to_string() },
            summarize_identities(Some(vec!["Foo.pm:10:5".to_string()])),
            summarize_identities(Some(vec!["Foo.pm:10:5".to_string()])),
            vec![],
        );

        let got: serde_json::Value = serde_json::to_value(&receipt)?;
        assert_eq!(got["query"], "method_candidates");
        assert_eq!(receipt.verdict, ShadowCompareVerdict::Same);
        Ok(())
    }

    /// Receipt JSON shape is stable for symbol_at query.
    #[test]
    fn receipt_json_shape_stable_for_symbol_at() -> Result<(), Box<dyn std::error::Error>> {
        let receipt = SemanticShadowCompareReceipt::from_summaries(
            ShadowQueryName::SymbolAt,
            ShadowQueryInput { symbol: "$var".to_string() },
            summarize_identities(None),
            summarize_identities(Some(vec!["main.pm:5:3".to_string()])),
            vec!["legacy path unavailable".to_string()],
        );

        let got: serde_json::Value = serde_json::to_value(&receipt)?;
        assert_eq!(got["query"], "symbol_at");
        assert_eq!(receipt.verdict, ShadowCompareVerdict::Unavailable);
        Ok(())
    }

    /// Receipt JSON shape is stable for rename_plan query.
    #[test]
    fn receipt_json_shape_stable_for_rename_plan() -> Result<(), Box<dyn std::error::Error>> {
        let receipt = SemanticShadowCompareReceipt::from_summaries(
            ShadowQueryName::RenamePlan,
            ShadowQueryInput { symbol: "old_name".to_string() },
            summarize_identities(Some(vec!["a.pm:1:1".to_string()])),
            summarize_identities(Some(vec!["a.pm:1:1".to_string()])),
            vec![],
        );

        let got: serde_json::Value = serde_json::to_value(&receipt)?;
        assert_eq!(got["query"], "rename_plan");
        assert_eq!(receipt.verdict, ShadowCompareVerdict::Same);
        Ok(())
    }

    /// Receipt JSON shape is stable for safe_delete_plan query.
    #[test]
    fn receipt_json_shape_stable_for_safe_delete_plan() -> Result<(), Box<dyn std::error::Error>> {
        let receipt = SemanticShadowCompareReceipt::from_summaries(
            ShadowQueryName::SafeDeletePlan,
            ShadowQueryInput { symbol: "unused_sub".to_string() },
            summarize_identities(Some(vec![])),
            summarize_identities(Some(vec![])),
            vec![],
        );

        let got: serde_json::Value = serde_json::to_value(&receipt)?;
        assert_eq!(got["query"], "safe_delete_plan");
        assert_eq!(receipt.verdict, ShadowCompareVerdict::Same);
        Ok(())
    }

    /// Receipt JSON shape is stable for completion_visibility query.
    #[test]
    fn receipt_json_shape_stable_for_completion_visibility()
    -> Result<(), Box<dyn std::error::Error>> {
        let receipt = SemanticShadowCompareReceipt::from_summaries(
            ShadowQueryName::CompletionVisibility,
            ShadowQueryInput { symbol: "use Foo".to_string() },
            summarize_identities(Some(vec!["bar".to_string(), "baz".to_string()])),
            summarize_identities(Some(vec![
                "bar".to_string(),
                "baz".to_string(),
                "qux".to_string(),
            ])),
            vec![],
        );

        let got: serde_json::Value = serde_json::to_value(&receipt)?;
        assert_eq!(got["query"], "completion_visibility");
        assert_eq!(receipt.verdict, ShadowCompareVerdict::Improved);
        Ok(())
    }

    /// Receipt JSON shape is stable for diagnostics_check query.
    #[test]
    fn receipt_json_shape_stable_for_diagnostics_check() -> Result<(), Box<dyn std::error::Error>> {
        let receipt = SemanticShadowCompareReceipt::from_summaries(
            ShadowQueryName::DiagnosticsCheck,
            ShadowQueryInput { symbol: "undef_sym".to_string() },
            summarize_identities(Some(vec!["warn:a.pm:3:1".to_string()])),
            summarize_identities(Some(vec![])),
            vec!["false positive suppressed".to_string()],
        );

        let got: serde_json::Value = serde_json::to_value(&receipt)?;
        assert_eq!(got["query"], "diagnostics_check");
        assert_eq!(receipt.verdict, ShadowCompareVerdict::Regression);
        Ok(())
    }

    /// Strategy to generate an arbitrary `ShadowResultSummary`.
    fn arb_shadow_result_summary() -> impl Strategy<Value = ShadowResultSummary> {
        (any::<bool>(), 0u64..256, prop::collection::vec("[a-z0-9_.:/]{1,20}", 0..16)).prop_map(
            |(available, match_count, mut identities)| {
                identities.sort();
                identities.dedup();
                ShadowResultSummary { available, match_count, identities }
            },
        )
    }

    // **Validates: Requirements 10.2**
    //
    // Property 17: Shadow Compare Verdict Determinism — For any pair of
    // old-path and new-path summaries, `classify_verdict` produces the same
    // verdict when called with the same inputs. The verdict is `Unavailable`
    // when either path is unavailable, `Same` when summaries are equal,
    // `Improved` when the new path has more matches, and `Regression` when
    // the new path has fewer matches.
    proptest! {
        #![proptest_config(ProptestConfig {
            failure_persistence: None,
            ..ProptestConfig::default()
        })]

        #[test]
        fn prop_shadow_compare_verdict_determinism(
            old_summary in arb_shadow_result_summary(),
            new_summary in arb_shadow_result_summary(),
        ) {
            // Determinism: calling classify_verdict twice with the same inputs
            // must produce the same verdict.
            let verdict_a = classify_verdict(&old_summary, &new_summary);
            let verdict_b = classify_verdict(&old_summary, &new_summary);
            prop_assert_eq!(
                verdict_a, verdict_b,
                "classify_verdict must be deterministic for the same inputs"
            );

            // Verify verdict classification rules.
            if !old_summary.available || !new_summary.available {
                prop_assert_eq!(
                    verdict_a,
                    ShadowCompareVerdict::Unavailable,
                    "verdict must be Unavailable when either path is unavailable"
                );
            } else if old_summary == new_summary {
                prop_assert_eq!(
                    verdict_a,
                    ShadowCompareVerdict::Same,
                    "verdict must be Same when summaries are equal"
                );
            } else if new_summary.match_count > old_summary.match_count {
                prop_assert_eq!(
                    verdict_a,
                    ShadowCompareVerdict::Improved,
                    "verdict must be Improved when new path has more matches"
                );
            } else if new_summary.match_count < old_summary.match_count {
                prop_assert_eq!(
                    verdict_a,
                    ShadowCompareVerdict::Regression,
                    "verdict must be Regression when new path has fewer matches"
                );
            } else {
                // Equal counts but different content → Ambiguous
                prop_assert_eq!(
                    verdict_a,
                    ShadowCompareVerdict::Ambiguous,
                    "verdict must be Ambiguous when counts are equal but content differs"
                );
            }
        }
    }
}
