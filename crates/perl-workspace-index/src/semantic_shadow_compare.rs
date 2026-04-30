use serde::{Deserialize, Serialize};

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
        let verdict = classify_verdict(&old_result, &new_result);
        Self { schema_version: 1, query, input, old_result, new_result, verdict, notes }
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
            "schema_version": 1,
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
            "notes": ["fixture=h1"]
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
        assert_eq!(summary.available, true);
        assert_eq!(summary.match_count, 3);
        assert_eq!(
            summary.identities,
            vec!["a.pm:1:1".to_string(), "b.pm:2:2".to_string(), "c.pm:3:1".to_string()]
        );
    }
}
