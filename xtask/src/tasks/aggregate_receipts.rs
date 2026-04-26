use clap::ValueEnum;
use color_eyre::eyre::{Result, WrapErr, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum AdvisoryMode {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone)]
pub struct AggregateReceiptsConfig {
    pub check: String,
    pub inputs: PathBuf,
    pub output: PathBuf,
    pub event: String,
    pub allow_noop: bool,
    pub advisory_mode: AdvisoryMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repro {
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subreceipt {
    pub name: String,
    #[serde(default = "default_required")]
    pub required: bool,
    #[serde(default = "default_selected")]
    pub selected: bool,
    #[serde(default = "default_verdict")]
    pub verdict: String,
    #[serde(default)]
    pub classification: Option<String>,
    #[serde(default)]
    pub missing: bool,
    #[serde(default)]
    pub repro: Option<Repro>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatorReceipt {
    pub check: String,
    pub schema_version: u32,
    pub event: String,
    pub verdict: String,
    pub classification: String,
    pub subreceipts: Vec<Subreceipt>,
    pub missing_receipts: Vec<String>,
    pub repro: Repro,
}

fn default_required() -> bool {
    true
}

fn default_selected() -> bool {
    true
}

fn default_verdict() -> String {
    "unknown".to_string()
}

pub fn run(config: AggregateReceiptsConfig) -> Result<()> {
    if config.check.trim().is_empty() {
        bail!("--check cannot be empty");
    }

    let subreceipts = load_subreceipts(&config.inputs)?;
    let (verdict, classification, missing_receipts) =
        finalize_gate_state(&subreceipts, config.allow_noop, config.advisory_mode);

    let receipt = AggregatorReceipt {
        check: config.check.clone(),
        schema_version: 1,
        event: config.event,
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
    };

    if let Some(parent) = config.output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating output directory {}", parent.display()))?;
    }

    let payload =
        serde_json::to_string_pretty(&receipt).context("serializing aggregator receipt")?;
    fs::write(&config.output, format!("{payload}\n"))
        .with_context(|| format!("writing {}", config.output.display()))?;

    println!(
        "aggregated {} subreceipts -> {} ({})",
        receipt.subreceipts.len(),
        receipt.verdict,
        receipt.classification
    );

    Ok(())
}

fn load_subreceipts(inputs_dir: &Path) -> Result<Vec<Subreceipt>> {
    if !inputs_dir.exists() {
        bail!("inputs directory does not exist: {}", inputs_dir.display());
    }

    let mut entries: Vec<PathBuf> = fs::read_dir(inputs_dir)
        .with_context(|| format!("reading {}", inputs_dir.display()))?
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();

    entries.sort();

    if entries.is_empty() {
        bail!("no .json subreceipts found in {}", inputs_dir.display());
    }

    entries
        .into_iter()
        .map(|path| {
            let raw = fs::read_to_string(&path)
                .with_context(|| format!("reading subreceipt {}", path.display()))?;
            serde_json::from_str(&raw)
                .with_context(|| format!("parsing subreceipt {}", path.display()))
        })
        .collect()
}

pub fn finalize_gate_state(
    subreceipts: &[Subreceipt],
    allow_noop: bool,
    advisory_mode: AdvisoryMode,
) -> (String, String, Vec<String>) {
    let mut missing_required = Vec::new();
    let mut required_failed = false;
    let mut advisory_problem = false;

    for receipt in subreceipts {
        if receipt.required {
            if receipt.missing {
                missing_required.push(receipt.name.clone());
                continue;
            }

            if !receipt.selected {
                if !allow_noop {
                    missing_required.push(receipt.name.clone());
                }
                continue;
            }

            if receipt.verdict == "fail" {
                required_failed = true;
            }
        } else if matches!(receipt.verdict.as_str(), "fail" | "warn") {
            advisory_problem = true;
        }
    }

    if !missing_required.is_empty() {
        return ("fail".to_string(), "unknown".to_string(), missing_required);
    }

    if required_failed {
        let classification = first_required_failure_classification(subreceipts)
            .unwrap_or_else(|| "code_regression".to_string());
        return ("fail".to_string(), classification, Vec::new());
    }

    if advisory_problem {
        return match advisory_mode {
            AdvisoryMode::Pass => ("pass".to_string(), "unknown".to_string(), Vec::new()),
            AdvisoryMode::Warn => ("warn".to_string(), "infra_failure".to_string(), Vec::new()),
            AdvisoryMode::Fail => ("fail".to_string(), "infra_failure".to_string(), Vec::new()),
        };
    }

    let any_selected_required =
        subreceipts.iter().any(|receipt| receipt.required && receipt.selected);
    if !any_selected_required {
        return ("pass".to_string(), "skipped".to_string(), Vec::new());
    }

    ("pass".to_string(), "unknown".to_string(), Vec::new())
}

fn first_required_failure_classification(subreceipts: &[Subreceipt]) -> Option<String> {
    subreceipts
        .iter()
        .find(|receipt| receipt.required && receipt.verdict == "fail")
        .and_then(|receipt| receipt.classification.clone())
}
