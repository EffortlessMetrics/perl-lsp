use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const SCHEMA_VERSION: &str = "1";

#[derive(Debug, Deserialize)]
struct InputSubreceipt {
    check: String,
    #[serde(default)]
    required: bool,
    #[serde(default = "default_selected")]
    selected: bool,
    verdict: String,
    #[serde(default)]
    classification: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RequiredManifest {
    required_receipts: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggregatorReceipt {
    pub check: String,
    pub schema_version: String,
    pub event: String,
    pub verdict: String,
    pub classification: String,
    pub subreceipts: Vec<Subreceipt>,
    pub missing_receipts: Vec<String>,
    pub repro: Repro,
    pub allow_noop: bool,
    pub advisory_mode: AdvisoryMode,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Subreceipt {
    pub check: String,
    pub required: bool,
    pub selected: bool,
    pub verdict: String,
    pub classification: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Repro {
    pub command: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AdvisoryMode {
    Warn,
    Fail,
}

impl Default for AdvisoryMode {
    fn default() -> Self {
        Self::Warn
    }
}

pub fn run(check: String, inputs: PathBuf, output: PathBuf) -> Result<()> {
    let receipt = aggregate_from_dir(&check, &inputs)?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(&receipt).context("Failed to serialize receipt")?;
    fs::write(&output, json).with_context(|| format!("Failed to write {}", output.display()))?;
    println!("Wrote aggregator receipt: {}", output.display());
    Ok(())
}

pub fn aggregate_from_dir(check: &str, inputs: &Path) -> Result<AggregatorReceipt> {
    let mut subreceipts = Vec::new();
    let mut required = HashSet::new();

    for entry in
        fs::read_dir(inputs).with_context(|| format!("Failed to read {}", inputs.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }

        let data = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read input receipt {}", path.display()))?;
        if let Ok(manifest) = serde_json::from_str::<RequiredManifest>(&data) {
            required.extend(manifest.required_receipts);
            continue;
        }

        let input: InputSubreceipt = serde_json::from_str(&data)
            .with_context(|| format!("Failed to parse subreceipt {}", path.display()))?;

        if input.required {
            required.insert(input.check.clone());
        }

        subreceipts.push(Subreceipt {
            check: input.check,
            required: input.required,
            selected: input.selected,
            verdict: input.verdict,
            classification: input.classification,
        });
    }

    subreceipts.sort_by(|a, b| a.check.cmp(&b.check));

    let present: HashSet<String> = subreceipts.iter().map(|item| item.check.clone()).collect();
    let mut missing_receipts: Vec<String> =
        required.into_iter().filter(|name| !present.contains(name)).collect();
    missing_receipts.sort();

    let receipt = AggregatorReceipt {
        check: check.to_string(),
        schema_version: SCHEMA_VERSION.to_string(),
        event: "pull_request".to_string(),
        verdict: "unknown".to_string(),
        classification: "unknown".to_string(),
        subreceipts,
        missing_receipts,
        repro: Repro {
            command: format!(
                "cargo xtask aggregate-receipts --check \"{}\" --inputs {} --output <path>",
                check,
                inputs.display()
            ),
        },
        allow_noop: true,
        advisory_mode: AdvisoryMode::Warn,
    };

    Ok(super::finalize_check::finalize_receipt(receipt))
}

fn default_selected() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::Result;

    fn fixture_dir(path: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)
    }

    #[test]
    fn aggregates_passing_fixture() -> Result<()> {
        let receipt =
            aggregate_from_dir("Test Gate", &fixture_dir("tests/fixtures/aggregator/pass"))?;
        assert_eq!(receipt.verdict, "pass");
        assert!(receipt.missing_receipts.is_empty());
        Ok(())
    }

    #[test]
    fn aggregates_missing_required_fixture() -> Result<()> {
        let receipt = aggregate_from_dir(
            "Test Gate",
            &fixture_dir("tests/fixtures/aggregator/missing-required"),
        )?;
        assert_eq!(receipt.verdict, "fail");
        assert_eq!(receipt.classification, "stale_base");
        assert!(!receipt.missing_receipts.is_empty());
        Ok(())
    }
}
