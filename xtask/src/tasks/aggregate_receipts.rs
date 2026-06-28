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

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::{Context, Result};
    use serde_json::json;
    use tempfile::tempdir;

    fn subreceipt(
        name: &str,
        required: bool,
        selected: bool,
        verdict: Verdict,
        classification: Classification,
    ) -> Subreceipt {
        Subreceipt {
            name: name.to_string(),
            selected,
            required,
            verdict,
            classification,
            repro: None,
        }
    }

    fn required_receipt(
        name: &str,
        verdict: Verdict,
        classification: Classification,
    ) -> Subreceipt {
        subreceipt(name, true, true, verdict, classification)
    }

    #[test]
    fn subreceipt_builder_preserves_fields() -> Result<()> {
        let receipt =
            subreceipt("coverage", true, false, Verdict::Warn, Classification::InfraFailure);

        assert_eq!(receipt.name, "coverage");
        assert!(receipt.required);
        assert!(!receipt.selected);
        assert_eq!(receipt.verdict, Verdict::Warn);
        assert_eq!(receipt.classification, Classification::InfraFailure);
        assert!(receipt.repro.is_none());
        assert!(
            matches!(
                &receipt,
                Subreceipt {
                    name,
                    selected: false,
                    required: true,
                    verdict: Verdict::Warn,
                    classification: Classification::InfraFailure,
                    repro: None,
                } if name == "coverage"
            ),
            "Subreceipt literal should preserve every builder field"
        );
        Ok(())
    }

    #[test]
    fn missing_required_receipt_is_stale_base_failure() -> Result<()> {
        let subreceipts = vec![required_receipt("linux", Verdict::Pass, Classification::Unknown)];
        let missing = vec!["coverage".to_string()];

        let (verdict, classification) = evaluate_receipt(&subreceipts, &missing, true);

        assert_eq!(verdict, Verdict::Fail, "missing receipt should fail aggregate check");
        assert_eq!(
            classification,
            Classification::StaleBase,
            "missing receipt should classify as stale base"
        );
        Ok(())
    }

    #[test]
    fn required_failures_default_to_code_regression_and_preserve_explicit_class() -> Result<()> {
        let fallback = vec![required_receipt("rust", Verdict::Fail, Classification::Unknown)];
        let (fallback_verdict, fallback_classification) = evaluate_receipt(&fallback, &[], true);

        assert_eq!(
            fallback_verdict,
            Verdict::Fail,
            "required failed receipt should fail aggregate check"
        );
        assert_eq!(
            fallback_classification,
            Classification::CodeRegression,
            "unknown required failure should default to code regression"
        );

        let explicit =
            vec![required_receipt("coverage", Verdict::Fail, Classification::InfraFailure)];
        let (explicit_verdict, explicit_classification) = evaluate_receipt(&explicit, &[], true);

        assert_eq!(
            explicit_verdict,
            Verdict::Fail,
            "required failed receipt should fail aggregate check"
        );
        assert_eq!(
            explicit_classification,
            Classification::InfraFailure,
            "explicit classification should be preserved"
        );
        Ok(())
    }

    #[test]
    fn required_noop_honors_allow_noop_flag() -> Result<()> {
        let cases = [
            subreceipt("unselected", true, false, Verdict::Pass, Classification::Unknown),
            subreceipt("skipped", true, true, Verdict::Skipped, Classification::Skipped),
        ];

        for subreceipt in cases {
            let subreceipts = vec![subreceipt];

            let (allowed_verdict, allowed_classification) =
                evaluate_receipt(&subreceipts, &[], true);
            assert_eq!(
                allowed_verdict,
                Verdict::Pass,
                "allowed no-op required receipt should pass"
            );
            assert_eq!(
                allowed_classification,
                Classification::Skipped,
                "allowed no-op required receipt should classify as skipped"
            );

            let (blocked_verdict, blocked_classification) =
                evaluate_receipt(&subreceipts, &[], false);
            assert_eq!(
                blocked_verdict,
                Verdict::Fail,
                "blocked no-op required receipt should fail"
            );
            assert_eq!(
                blocked_classification,
                Classification::Skipped,
                "blocked no-op required receipt should classify as skipped"
            );
        }

        Ok(())
    }

    #[test]
    fn advisory_failures_do_not_override_required_pass() -> Result<()> {
        let subreceipts = vec![
            subreceipt("advisory-fail", false, true, Verdict::Fail, Classification::InfraFailure),
            subreceipt("advisory-warn", false, true, Verdict::Warn, Classification::Unknown),
            required_receipt("rust", Verdict::Pass, Classification::Unknown),
        ];

        let (verdict, classification) = evaluate_receipt(&subreceipts, &[], false);

        assert_eq!(verdict, Verdict::Pass, "advisory failures should not fail aggregate check");
        assert_eq!(
            classification,
            Classification::Unknown,
            "required pass should keep aggregate classification unknown"
        );
        Ok(())
    }

    #[test]
    fn build_receipt_reads_manifest_sorts_inputs_and_records_missing_receipts() -> Result<()> {
        let dir = tempdir().context("create aggregate receipt fixture dir")?;
        let inputs = dir.path().join("inputs");
        fs::create_dir(&inputs).context("create aggregate receipt input dir")?;
        fs::write(
            inputs.join("manifest.json"),
            serde_json::to_vec_pretty(&json!({
                "required_receipts": ["rust", "coverage", "missing"]
            }))?,
        )
        .context("write manifest")?;
        fs::write(
            inputs.join("z-rust.json"),
            serde_json::to_vec_pretty(&json!({
                "name": "rust",
                "required": true,
                "verdict": "pass"
            }))?,
        )
        .context("write rust subreceipt")?;
        fs::write(
            inputs.join("a-coverage.json"),
            serde_json::to_vec_pretty(&json!({
                "name": "coverage",
                "required": true,
                "verdict": "pass"
            }))?,
        )
        .context("write coverage subreceipt")?;
        fs::write(inputs.join("notes.txt"), "not a receipt").context("write ignored text file")?;

        let config = AggregateReceiptsConfig {
            check: "quality-gate".to_string(),
            inputs: inputs.clone(),
            output: dir.path().join("out").join("aggregate.json"),
            allow_noop: true,
        };

        let receipt = build_aggregator_receipt(&config)?;

        assert_eq!(receipt.check, "quality-gate", "receipt should preserve check name");
        assert_eq!(receipt.schema_version, SCHEMA_VERSION, "receipt schema version drifted");
        assert_eq!(receipt.event, EVENT_PULL_REQUEST, "receipt event drifted");
        assert_eq!(receipt.verdict, Verdict::Fail, "missing required receipt should fail");
        assert_eq!(
            receipt.classification,
            Classification::StaleBase,
            "missing required receipt should classify stale base"
        );
        assert_eq!(
            receipt.missing_receipts,
            vec!["missing".to_string()],
            "receipt should list missing manifest entry"
        );
        let first = receipt.subreceipts.first().context("missing first subreceipt")?;
        let second = receipt.subreceipts.get(1).context("missing second subreceipt")?;
        assert_eq!(first.name, "coverage", "subreceipts should sort by name");
        assert_eq!(second.name, "rust", "subreceipts should sort by name");
        assert!(
            receipt.repro.command.contains("cargo xtask aggregate-receipts"),
            "receipt should include aggregate-receipts repro command"
        );
        assert!(
            receipt.repro.command.contains("quality-gate"),
            "receipt repro command should include check name"
        );
        Ok(())
    }

    #[test]
    fn run_writes_receipt_to_output_parent() -> Result<()> {
        let dir = tempdir().context("create aggregate receipt fixture dir")?;
        let inputs = dir.path().join("inputs");
        fs::create_dir(&inputs).context("create aggregate receipt input dir")?;
        fs::write(
            inputs.join("rust.json"),
            serde_json::to_vec_pretty(&json!({
                "name": "rust",
                "required": true,
                "verdict": "pass"
            }))?,
        )
        .context("write rust subreceipt")?;
        let output = dir.path().join("nested").join("receipt.json");
        let config = AggregateReceiptsConfig {
            check: "quality-gate".to_string(),
            inputs,
            output: output.clone(),
            allow_noop: true,
        };

        run(config)?;

        let body =
            fs::read_to_string(&output).with_context(|| format!("read {}", output.display()))?;
        let receipt: AggregatorReceipt =
            serde_json::from_str(&body).context("parse aggregate receipt output")?;
        assert_eq!(receipt.verdict, Verdict::Pass, "written receipt should pass");
        assert_eq!(
            receipt.classification,
            Classification::Unknown,
            "written receipt classification should be unknown"
        );
        Ok(())
    }
}
