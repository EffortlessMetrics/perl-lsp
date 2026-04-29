use serde::{Deserialize, Serialize};

/// Query kinds supported by semantic shadow comparisons.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SemanticQueryName {
    /// Definition lookup.
    FindDefinition,
    /// References lookup.
    FindReferences,
    /// Usage count lookup.
    CountUsages,
}

/// Comparison verdict for old-vs-new semantic query answers.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShadowVerdict {
    /// Old and new summaries are equivalent.
    Same,
    /// New summary is measurably better.
    Improved,
    /// New summary is measurably worse.
    Regression,
    /// Cannot safely classify as better/worse.
    Ambiguous,
    /// Missing fact-backed answer from one or both sides.
    Unavailable,
}

/// Normalized, deterministic input envelope for a workspace semantic query.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueryInput {
    /// Symbol text passed to the query.
    pub symbol: String,
}

/// Deterministic summary for definition lookups.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DefinitionSummary {
    /// Whether a definition was found.
    pub found: bool,
    /// Canonical location key (`path:line:column`) when available.
    pub location_key: Option<String>,
}

/// Deterministic summary for references lookups.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReferencesSummary {
    /// Number of references discovered.
    pub total: usize,
    /// Sorted canonical location keys (`path:line:column`).
    pub locations: Vec<String>,
}

/// Deterministic summary for usage-count lookups.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageCountSummary {
    /// Usage count result.
    pub count: usize,
}

/// Per-query shadow comparison summaries.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum QuerySummary {
    /// find_definition summary payload.
    FindDefinition(DefinitionSummary),
    /// find_references summary payload.
    FindReferences(ReferencesSummary),
    /// count_usages summary payload.
    CountUsages(UsageCountSummary),
}

/// Receipt row for one semantic query shadow comparison.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticShadowReceipt {
    /// Schema version for future-compatible evolution.
    pub schema_version: u32,
    /// Query operation being compared.
    pub query_name: SemanticQueryName,
    /// Normalized query input.
    pub input: QueryInput,
    /// Old engine summary (or unavailable).
    pub old_result_summary: Option<QuerySummary>,
    /// New engine summary (or unavailable).
    pub new_result_summary: Option<QuerySummary>,
    /// Final quality verdict.
    pub verdict: ShadowVerdict,
    /// Human-readable classification note.
    pub notes: String,
}

impl SemanticShadowReceipt {
    /// Build a deterministic receipt and classify verdict from optional summaries.
    #[must_use]
    pub fn from_summaries(
        query_name: SemanticQueryName,
        input: QueryInput,
        old_result_summary: Option<QuerySummary>,
        new_result_summary: Option<QuerySummary>,
        notes: impl Into<String>,
    ) -> Self {
        let verdict = classify(&old_result_summary, &new_result_summary);
        Self {
            schema_version: 1,
            query_name,
            input,
            old_result_summary,
            new_result_summary,
            verdict,
            notes: notes.into(),
        }
    }
}

fn classify(old_result_summary: &Option<QuerySummary>, new_result_summary: &Option<QuerySummary>) -> ShadowVerdict {
    match (old_result_summary, new_result_summary) {
        (None, _) | (_, None) => ShadowVerdict::Unavailable,
        (Some(old), Some(new)) if old == new => ShadowVerdict::Same,
        (Some(QuerySummary::FindDefinition(old)), Some(QuerySummary::FindDefinition(new))) => {
            match (old.found, new.found) {
                (false, true) => ShadowVerdict::Improved,
                (true, false) => ShadowVerdict::Regression,
                _ => ShadowVerdict::Ambiguous,
            }
        }
        (Some(QuerySummary::FindReferences(old)), Some(QuerySummary::FindReferences(new))) => {
            if new.total > old.total {
                ShadowVerdict::Improved
            } else if new.total < old.total {
                ShadowVerdict::Regression
            } else {
                ShadowVerdict::Ambiguous
            }
        }
        (Some(QuerySummary::CountUsages(old)), Some(QuerySummary::CountUsages(new))) => {
            if new.count > old.count {
                ShadowVerdict::Improved
            } else if new.count < old.count {
                ShadowVerdict::Regression
            } else {
                ShadowVerdict::Same
            }
        }
        _ => ShadowVerdict::Ambiguous,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_deterministic_json_shape_for_find_references() -> Result<(), Box<dyn std::error::Error>> {
        let receipt = SemanticShadowReceipt::from_summaries(
            SemanticQueryName::FindReferences,
            QueryInput { symbol: "Foo::bar".to_string() },
            Some(QuerySummary::FindReferences(ReferencesSummary {
                total: 1,
                locations: vec!["a.pl:1:1".to_string()],
            })),
            Some(QuerySummary::FindReferences(ReferencesSummary {
                total: 2,
                locations: vec!["a.pl:1:1".to_string(), "b.pl:3:5".to_string()],
            })),
            "new facts discovered one extra callsite",
        );

        let json = serde_json::to_string_pretty(&receipt)?;
        let value: serde_json::Value = serde_json::from_str(&json)?;

        assert_eq!(value["query_name"], "find_references");
        assert_eq!(value["verdict"], "improved");
        assert_eq!(value["old_result_summary"]["kind"], "find_references");
        assert_eq!(value["new_result_summary"]["total"], 2);
        Ok(())
    }

    #[test]
    fn missing_summary_is_unavailable_not_panic() {
        let receipt = SemanticShadowReceipt::from_summaries(
            SemanticQueryName::FindDefinition,
            QueryInput { symbol: "Missing::symbol".to_string() },
            None,
            Some(QuerySummary::FindDefinition(DefinitionSummary {
                found: true,
                location_key: Some("lib/Missing.pm:12:1".to_string()),
            })),
            "old engine does not have fact-backed path",
        );

        assert_eq!(receipt.verdict, ShadowVerdict::Unavailable);
    }
}
