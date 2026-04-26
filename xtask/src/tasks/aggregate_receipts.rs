use color_eyre::eyre::{Result, bail, eyre};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_SCHEMA_VERSION: u32 = 1;
const REQUIRED_CONFIG_FILE_STEM: &str = "_required";

#[derive(Debug, Clone, Deserialize)]
struct RequiredConfig {
    #[serde(default)]
    required: Vec<String>,
    #[serde(default)]
    allow_noop: bool,
    #[serde(default)]
    advisory_mode: AdvisoryMode,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum AdvisoryMode {
    #[default]
    Pass,
    Warn,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Subreceipt {
    pub check: String,
    #[serde(default = "default_selected")]
    pub selected: bool,
    #[serde(default)]
    pub skipped: bool,
    pub verdict: SubreceiptVerdict,
    #[serde(default)]
    pub classification: Option<FailureClassification>,
    #[serde(default)]
    pub required: bool,
}

fn default_selected() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SubreceiptVerdict {
    Pass,
    Fail,
    Warn,
    Skipped,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FailureClassification {
    CodeRegression,
    InfraFailure,
    StaleBase,
    Skipped,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Repro {
    pub command: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AggregatorReceipt {
    pub check: String,
    pub schema_version: u32,
    pub event: String,
    pub verdict: SubreceiptVerdict,
    pub classification: FailureClassification,
    pub subreceipts: Vec<Subreceipt>,
    pub missing_receipts: Vec<String>,
    pub repro: Repro,
}

pub fn run(check: String, inputs: PathBuf, output: PathBuf) -> Result<()> {
    let (config, subreceipts) = load_inputs(&inputs)?;
    let receipt = aggregate(check, config, subreceipts);

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let payload = serde_json::to_string_pretty(&receipt)?;
    fs::write(&output, format!("{payload}\n"))?;
    println!("Wrote aggregator receipt: {}", output.display());
    Ok(())
}

fn load_inputs(inputs: &Path) -> Result<(RequiredConfig, Vec<Subreceipt>)> {
    if !inputs.exists() {
        bail!("input directory does not exist: {}", inputs.display());
    }

    let mut entries: Vec<PathBuf> = fs::read_dir(inputs)?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect();
    entries.sort();

    let mut config = RequiredConfig {
        required: Vec::new(),
        allow_noop: false,
        advisory_mode: AdvisoryMode::Pass,
    };
    let mut subreceipts = Vec::new();

    for path in entries {
        let raw = fs::read_to_string(&path)?;
        if path.file_stem().and_then(|s| s.to_str()) == Some(REQUIRED_CONFIG_FILE_STEM) {
            config = serde_json::from_str(&raw)
                .map_err(|err| eyre!("failed to parse {}: {err}", path.display()))?;
            continue;
        }

        let parsed: Subreceipt = serde_json::from_str(&raw)
            .map_err(|err| eyre!("failed to parse {}: {err}", path.display()))?;
        subreceipts.push(parsed);
    }

    Ok((config, subreceipts))
}

fn aggregate(
    check: String,
    config: RequiredConfig,
    subreceipts: Vec<Subreceipt>,
) -> AggregatorReceipt {
    let mut required: BTreeSet<String> = config.required.iter().cloned().collect();
    let mut by_check = BTreeMap::new();
    for sub in &subreceipts {
        if sub.required {
            required.insert(sub.check.clone());
        }
        by_check.insert(sub.check.clone(), sub);
    }

    let missing_receipts: Vec<String> =
        required.iter().filter(|name| !by_check.contains_key(*name)).cloned().collect();

    let mut required_fail_classification = None;
    let mut advisory_failures = 0usize;
    let mut required_skipped_or_unselected = 0usize;
    let mut required_count = 0usize;

    for sub in &subreceipts {
        if !is_required(sub, &required) {
            if sub.verdict == SubreceiptVerdict::Fail {
                advisory_failures += 1;
            }
            continue;
        }

        required_count += 1;
        if !sub.selected || sub.skipped || sub.verdict == SubreceiptVerdict::Skipped {
            required_skipped_or_unselected += 1;
            continue;
        }

        if sub.verdict == SubreceiptVerdict::Fail {
            required_fail_classification =
                Some(sub.classification.unwrap_or(FailureClassification::Unknown));
        }
    }

    let (verdict, classification) = if !missing_receipts.is_empty() {
        (SubreceiptVerdict::Fail, FailureClassification::Unknown)
    } else if let Some(classification) = required_fail_classification {
        (SubreceiptVerdict::Fail, classification)
    } else if required_count > 0 && required_skipped_or_unselected == required_count {
        if config.allow_noop {
            (SubreceiptVerdict::Pass, FailureClassification::Skipped)
        } else {
            (SubreceiptVerdict::Fail, FailureClassification::Skipped)
        }
    } else if advisory_failures > 0 {
        match config.advisory_mode {
            AdvisoryMode::Pass => (SubreceiptVerdict::Pass, FailureClassification::Unknown),
            AdvisoryMode::Warn => (SubreceiptVerdict::Warn, FailureClassification::Unknown),
        }
    } else {
        (SubreceiptVerdict::Pass, FailureClassification::Unknown)
    };

    AggregatorReceipt {
        check,
        schema_version: DEFAULT_SCHEMA_VERSION,
        event: "pull_request".to_string(),
        verdict,
        classification,
        subreceipts,
        missing_receipts,
        repro: Repro {
            command: "cargo xtask aggregate-receipts --check \"<check-name>\" --inputs <dir> --output <path>".to_string(),
        },
    }
}

fn is_required(subreceipt: &Subreceipt, required: &BTreeSet<String>) -> bool {
    subreceipt.required || required.contains(&subreceipt.check)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_path(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/aggregator").join(name)
    }

    #[test]
    fn aggregates_pass_fixture() -> Result<()> {
        let (cfg, subs) = load_inputs(&fixture_path("pass"))?;
        let receipt = aggregate("Test Gate".to_string(), cfg, subs);
        assert_eq!(receipt.verdict, SubreceiptVerdict::Pass);
        assert!(receipt.missing_receipts.is_empty());
        Ok(())
    }

    #[test]
    fn aggregates_fail_fixture() -> Result<()> {
        let (cfg, subs) = load_inputs(&fixture_path("fail"))?;
        let receipt = aggregate("Test Gate".to_string(), cfg, subs);
        assert_eq!(receipt.verdict, SubreceiptVerdict::Fail);
        assert_eq!(receipt.classification, FailureClassification::CodeRegression);
        Ok(())
    }

    #[test]
    fn aggregates_missing_required_fixture() -> Result<()> {
        let (cfg, subs) = load_inputs(&fixture_path("missing-required"))?;
        let receipt = aggregate("Test Gate".to_string(), cfg, subs);
        assert_eq!(receipt.verdict, SubreceiptVerdict::Fail);
        assert_eq!(receipt.missing_receipts, vec!["windows-guardrails".to_string()]);
        Ok(())
    }
}
