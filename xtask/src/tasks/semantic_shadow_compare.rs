use crate::utils::project_root;
use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_INPUT: &str =
    "crates/perl-workspace-index/tests/fixtures/semantic_scorecard/shadow_compare.json";
const DEFAULT_OUTPUT: &str = "target/receipts/semantic_shadow_compare.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShadowVerdict {
    Same,
    Improved,
    Regression,
    Ambiguous,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QueryName {
    FindDefinition,
    FindReferences,
    CountUsages,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueryResultSummary {
    pub present: bool,
    pub item_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShadowCompareReceipt {
    pub schema_version: u32,
    pub query: QueryName,
    pub input: Value,
    pub old_result_summary: QueryResultSummary,
    pub new_result_summary: QueryResultSummary,
    pub verdict: ShadowVerdict,
    pub notes: Vec<String>,
}

pub fn run(input: Option<PathBuf>, output: Option<PathBuf>) -> Result<()> {
    let root = project_root()?;
    let input_path = root.join(input.unwrap_or_else(|| PathBuf::from(DEFAULT_INPUT)));
    let output_path = root.join(output.unwrap_or_else(|| PathBuf::from(DEFAULT_OUTPUT)));

    let source = fs::read_to_string(&input_path)
        .with_context(|| format!("reading {}", input_path.display()))?;
    let receipts: Vec<ShadowCompareReceipt> = serde_json::from_str(&source)
        .with_context(|| format!("parsing {}", input_path.display()))?;

    write_receipts(&output_path, &receipts)?;
    println!("semantic shadow-compare receipts written: {}", output_path.display());
    Ok(())
}

fn write_receipts(path: &Path, receipts: &[ShadowCompareReceipt]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(receipts)?;
    fs::write(path, format!("{json}\n")).with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_json_shape_is_stable() -> Result<()> {
        let receipt = ShadowCompareReceipt {
            schema_version: 1,
            query: QueryName::FindDefinition,
            input: serde_json::json!({"symbol": "Foo::bar"}),
            old_result_summary: QueryResultSummary {
                present: true,
                item_count: 1,
                preview: Some(vec!["file:///old.pm:4:1".to_string()]),
            },
            new_result_summary: QueryResultSummary { present: false, item_count: 0, preview: None },
            verdict: ShadowVerdict::Unavailable,
            notes: vec!["fact-backed path unavailable".to_string()],
        };

        let encoded = serde_json::to_string_pretty(&receipt)?;
        let expected = r#"{
  "schema_version": 1,
  "query": "find_definition",
  "input": {
    "symbol": "Foo::bar"
  },
  "old_result_summary": {
    "present": true,
    "item_count": 1,
    "preview": [
      "file:///old.pm:4:1"
    ]
  },
  "new_result_summary": {
    "present": false,
    "item_count": 0
  },
  "verdict": "unavailable",
  "notes": [
    "fact-backed path unavailable"
  ]
}"#;
        assert_eq!(encoded, expected);
        Ok(())
    }

    #[test]
    fn serializes_all_query_names() -> Result<()> {
        let queries =
            [QueryName::FindDefinition, QueryName::FindReferences, QueryName::CountUsages];
        let encoded = serde_json::to_string(&queries)?;
        assert_eq!(encoded, r#"["find_definition","find_references","count_usages"]"#);
        Ok(())
    }
}
