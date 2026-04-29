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

fn classify_verdict(old_result: &ShadowResultSummary, new_result: &ShadowResultSummary) -> ShadowCompareVerdict {
    if !old_result.available || !new_result.available {
        return ShadowCompareVerdict::Unavailable;
    }

    if old_result == new_result {
        return ShadowCompareVerdict::Same;
    }

    if old_result.identities == new_result.identities {
        return ShadowCompareVerdict::Ambiguous;
    }

    if new_result.match_count > old_result.match_count {
        ShadowCompareVerdict::Improved
    } else if new_result.match_count < old_result.match_count {
        ShadowCompareVerdict::Regression
    } else {
        ShadowCompareVerdict::Ambiguous
    }
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
}
