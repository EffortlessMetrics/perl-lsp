use super::workspace_index::Location;
use serde::{Deserialize, Serialize};

/// Deterministic verdict for old-vs-new semantic shadow comparisons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShadowVerdict {
    /// Old and new summaries are identical.
    Same,
    /// New summary indicates strictly better outcome.
    Improved,
    /// New summary indicates a worse outcome.
    Regression,
    /// Old and new cannot be cleanly ordered.
    Ambiguous,
    /// One or both sides are unavailable (missing fact-backed path).
    Unavailable,
}

/// Stable summary for find_definition results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinitionSummary {
    /// Whether a definition location was found.
    pub found: bool,
    /// URI of the definition location.
    pub uri: Option<String>,
    /// Zero-based line of the definition location.
    pub line: Option<u32>,
}

/// Stable summary for find_references results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferencesSummary {
    /// Total number of reference locations.
    pub count: usize,
    /// URI for the first reference in deterministic order.
    pub first_uri: Option<String>,
    /// Zero-based line for the first reference in deterministic order.
    pub first_line: Option<u32>,
}

/// Stable summary for count_usages results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageCountSummary {
    /// Total number of non-definition usages.
    pub count: usize,
}

/// Deterministic JSON receipt for semantic shadow comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryShadowReceipt<TSummary>
where
    TSummary: Serialize,
{
    /// Queried operation name.
    pub query_name: &'static str,
    /// Query input payload (currently symbol name).
    pub input: String,
    /// Old path summary when available.
    pub old_result_summary: Option<TSummary>,
    /// New path summary when available.
    pub new_result_summary: Option<TSummary>,
    /// Deterministic old-vs-new verdict.
    pub verdict: ShadowVerdict,
    /// Optional comparator notes.
    pub notes: Vec<String>,
}

fn location_line(location: &Location) -> u32 {
    location.range.start.line
}

/// Summarize `find_definition` output into a stable receipt shape.
pub fn summarize_definition(result: Option<&Location>) -> Option<DefinitionSummary> {
    result.map(|location| DefinitionSummary {
        found: true,
        uri: Some(location.uri.clone()),
        line: Some(location_line(location)),
    })
}

/// Summarize `find_references` output into a stable receipt shape.
pub fn summarize_references(result: Option<&[Location]>) -> Option<ReferencesSummary> {
    result.map(|locations| ReferencesSummary {
        count: locations.len(),
        first_uri: locations.first().map(|location| location.uri.clone()),
        first_line: locations.first().map(location_line),
    })
}

/// Summarize `count_usages` output into a stable receipt shape.
pub fn summarize_usage_count(result: Option<usize>) -> Option<UsageCountSummary> {
    result.map(|count| UsageCountSummary { count })
}

/// Compare old-vs-new `find_definition` outputs and emit a receipt row.
pub fn compare_definition(
    input: impl Into<String>,
    old_result: Option<&Location>,
    new_result: Option<&Location>,
) -> QueryShadowReceipt<DefinitionSummary> {
    let old_summary = summarize_definition(old_result);
    let new_summary = summarize_definition(new_result);
    let (verdict, notes) =
        compare_presence_and_equality(old_summary.as_ref(), new_summary.as_ref());

    QueryShadowReceipt {
        query_name: "find_definition",
        input: input.into(),
        old_result_summary: old_summary,
        new_result_summary: new_summary,
        verdict,
        notes,
    }
}

/// Compare old-vs-new `find_references` outputs and emit a receipt row.
pub fn compare_references(
    input: impl Into<String>,
    old_result: Option<&[Location]>,
    new_result: Option<&[Location]>,
) -> QueryShadowReceipt<ReferencesSummary> {
    let old_summary = summarize_references(old_result);
    let new_summary = summarize_references(new_result);
    let (verdict, notes) =
        compare_presence_and_equality(old_summary.as_ref(), new_summary.as_ref());

    QueryShadowReceipt {
        query_name: "find_references",
        input: input.into(),
        old_result_summary: old_summary,
        new_result_summary: new_summary,
        verdict,
        notes,
    }
}

/// Compare old-vs-new `count_usages` outputs and emit a receipt row.
pub fn compare_usage_count(
    input: impl Into<String>,
    old_result: Option<usize>,
    new_result: Option<usize>,
) -> QueryShadowReceipt<UsageCountSummary> {
    let old_summary = summarize_usage_count(old_result);
    let new_summary = summarize_usage_count(new_result);
    let (verdict, notes) =
        compare_presence_and_equality(old_summary.as_ref(), new_summary.as_ref());

    QueryShadowReceipt {
        query_name: "count_usages",
        input: input.into(),
        old_result_summary: old_summary,
        new_result_summary: new_summary,
        verdict,
        notes,
    }
}

fn compare_presence_and_equality<T: PartialEq>(
    old_summary: Option<&T>,
    new_summary: Option<&T>,
) -> (ShadowVerdict, Vec<String>) {
    match (old_summary, new_summary) {
        (None, None) => {
            (ShadowVerdict::Unavailable, vec!["old and new paths unavailable".to_string()])
        }
        (Some(_), None) => (ShadowVerdict::Unavailable, vec!["new path unavailable".to_string()]),
        (None, Some(_)) => (ShadowVerdict::Improved, vec!["old path unavailable".to_string()]),
        (Some(old), Some(new)) if old == new => {
            (ShadowVerdict::Same, vec!["summaries match".to_string()])
        }
        (Some(_), Some(_)) => (
            ShadowVerdict::Ambiguous,
            vec!["summary mismatch requires fixture-level adjudication".to_string()],
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::{Position, Range};

    fn location(uri: &str, line: u32) -> Location {
        Location {
            uri: uri.to_string(),
            range: Range::new(Position::new(0, line, 0), Position::new(0, line, 1)),
        }
    }

    #[test]
    fn definition_receipt_json_shape_is_stable() -> Result<(), Box<dyn std::error::Error>> {
        let old = location("file:///a.pl", 3);
        let new = location("file:///a.pl", 3);
        let receipt = compare_definition("Pkg::sym", Some(&old), Some(&new));
        let json = serde_json::to_string_pretty(&receipt)?;
        let expected = r#"{
  "query_name": "find_definition",
  "input": "Pkg::sym",
  "old_result_summary": {
    "found": true,
    "uri": "file:///a.pl",
    "line": 3
  },
  "new_result_summary": {
    "found": true,
    "uri": "file:///a.pl",
    "line": 3
  },
  "verdict": "same",
  "notes": [
    "summaries match"
  ]
}"#;
        assert_eq!(json, expected);
        Ok(())
    }

    #[test]
    fn references_receipt_uses_unavailable_when_new_missing() {
        let old = vec![location("file:///a.pl", 1), location("file:///b.pl", 2)];
        let receipt = compare_references("helper", Some(&old), None);
        assert_eq!(receipt.verdict, ShadowVerdict::Unavailable);
        assert_eq!(receipt.notes, vec!["new path unavailable".to_string()]);
    }

    #[test]
    fn usage_receipt_marks_old_missing_as_improved() {
        let receipt = compare_usage_count("helper", None, Some(2));
        assert_eq!(receipt.verdict, ShadowVerdict::Improved);
    }
}
