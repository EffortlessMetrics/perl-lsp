use color_eyre::eyre::{Context, ContextCompat, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const SCHEMA_VERSION: &str = "1";
const EVENT_PULL_REQUEST: &str = "pull_request";

#[derive(Debug, Clone)]
pub struct AggregateReceiptsConfig {
    pub check: String,
    pub inputs: PathBuf,
    pub output: PathBuf,
    pub allow_noop: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repro {
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subreceipt {
    pub name: String,
    #[serde(default = "default_selected")]
    pub selected: bool,
    #[serde(default)]
    pub required: bool,
    pub verdict: Verdict,
    #[serde(default = "default_classification")]
    pub classification: Classification,
    #[serde(default)]
    pub repro: Option<Repro>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Pass,
    Fail,
    Warn,
    Skipped,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Classification {
    CodeRegression,
    InfraFailure,
    StaleBase,
    Skipped,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatorReceipt {
    pub check: String,
    pub schema_version: String,
    pub event: String,
    pub verdict: Verdict,
    pub classification: Classification,
    pub subreceipts: Vec<Subreceipt>,
    pub missing_receipts: Vec<String>,
    pub repro: Repro,
}

#[derive(Debug, Deserialize)]
struct RequiredManifest {
    required_receipts: Vec<String>,
}

fn default_selected() -> bool {
    true
}

fn default_classification() -> Classification {
    Classification::Unknown
}

pub fn run(config: AggregateReceiptsConfig) -> Result<()> {
    let receipt = build_aggregator_receipt(&config)?;
    let output_parent =
        config.output.parent().context("output path must include a parent directory")?;
    fs::create_dir_all(output_parent)
        .with_context(|| format!("failed to create {}", output_parent.display()))?;
    let json = serde_json::to_string_pretty(&receipt).context("serialize aggregator receipt")?;
    fs::write(&config.output, json)
        .with_context(|| format!("write {}", config.output.display()))?;
    println!("wrote {}", config.output.display());
    Ok(())
}

pub fn build_aggregator_receipt(config: &AggregateReceiptsConfig) -> Result<AggregatorReceipt> {
    let entries = fs::read_dir(&config.inputs)
        .with_context(|| format!("failed to read inputs dir {}", config.inputs.display()))?;

    let mut required_by_manifest = BTreeSet::new();
    let mut subreceipts = Vec::new();

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !is_json_file(&path) {
            continue;
        }

        let body = fs::read_to_string(&path)
            .with_context(|| format!("failed to read fixture file {}", path.display()))?;
        if let Ok(manifest) = serde_json::from_str::<RequiredManifest>(&body) {
            for required in manifest.required_receipts {
                required_by_manifest.insert(required);
            }
            continue;
        }

        let subreceipt: Subreceipt = serde_json::from_str(&body)
            .with_context(|| format!("failed to parse subreceipt {}", path.display()))?;
        subreceipts.push(subreceipt);
    }

    if subreceipts.is_empty() && required_by_manifest.is_empty() {
        bail!("no subreceipt JSON files found in {}", config.inputs.display());
    }

    subreceipts.sort_by(|a, b| a.name.cmp(&b.name));

    let present_names: BTreeSet<String> = subreceipts.iter().map(|s| s.name.clone()).collect();
    let missing_receipts =
        required_by_manifest.difference(&present_names).cloned().collect::<Vec<_>>();

    let (verdict, classification) =
        evaluate_receipt(&subreceipts, &missing_receipts, config.allow_noop);

    Ok(AggregatorReceipt {
        check: config.check.clone(),
        schema_version: SCHEMA_VERSION.to_string(),
        event: EVENT_PULL_REQUEST.to_string(),
        verdict,
        classification,
        subreceipts,
        missing_receipts,
        repro: Repro {
            command: format!(
                "cargo xtask aggregate-receipts --check \"{}\" --inputs {} --output {}",
                config.check,
                config.inputs.display(),
                config.output.display()
            ),
        },
    })
}

pub fn evaluate_receipt(
    subreceipts: &[Subreceipt],
    missing_receipts: &[String],
    allow_noop: bool,
) -> (Verdict, Classification) {
    if !missing_receipts.is_empty() {
        return (Verdict::Fail, Classification::StaleBase);
    }

    let mut required_selected = 0_u64;

    for subreceipt in subreceipts {
        if !subreceipt.required {
            continue;
        }
        if !subreceipt.selected || subreceipt.verdict == Verdict::Skipped {
            continue;
        }
        required_selected += 1;
        if subreceipt.verdict == Verdict::Fail {
            let class = if subreceipt.classification == Classification::Unknown {
                Classification::CodeRegression
            } else {
                subreceipt.classification
            };
            return (Verdict::Fail, class);
        }
    }

    if required_selected == 0 {
        if allow_noop {
            return (Verdict::Pass, Classification::Skipped);
        }
        return (Verdict::Fail, Classification::Skipped);
    }

    (Verdict::Pass, Classification::Unknown)
}

fn is_json_file(path: &Path) -> bool {
    path.is_file() && path.extension().is_some_and(|ext| ext == "json")
}
