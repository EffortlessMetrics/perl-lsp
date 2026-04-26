use color_eyre::eyre::{Context, Result, bail, eyre};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::utils::project_root;

#[derive(Debug, Clone, Deserialize)]
struct ProjectionConfig {
    state: BTreeMap<String, ProjectionRule>,
}

#[derive(Debug, Clone, Deserialize)]
struct ProjectionRule {
    #[serde(default)]
    apply: Vec<String>,
    #[serde(default)]
    remove: Vec<String>,
}

#[derive(Debug, Serialize)]
struct LabelProjectionReceipt {
    current_labels: Vec<String>,
    projected_apply: Vec<String>,
    projected_remove: Vec<String>,
    skipped: bool,
    reason: String,
    dry_run: bool,
    verdict: String,
}

pub fn run_project_labels(
    state_path: PathBuf,
    dry_run: bool,
    apply: bool,
    receipt: Option<PathBuf>,
    config: Option<PathBuf>,
    create_missing_labels: bool,
) -> Result<()> {
    if dry_run && apply {
        bail!("--dry-run and --apply are mutually exclusive");
    }

    let effective_dry_run = !apply;
    let root = project_root()?;
    let config_path = config.unwrap_or_else(|| root.join(".ci/state/label-projection.toml"));
    let projection_config = load_projection_config(&config_path)?;
    let state_payload = load_state_json(&state_path)?;

    let canonical_state = extract_canonical_state(&state_payload).ok_or_else(|| {
        eyre!("state receipt is missing canonical state (expected `canonical_state` or `state`)")
    })?;

    let current_labels = extract_labels(&state_payload, "current_labels");

    let Some(rule) = projection_config.state.get(&canonical_state) else {
        let receipt_payload = LabelProjectionReceipt {
            current_labels,
            projected_apply: Vec::new(),
            projected_remove: Vec::new(),
            skipped: true,
            reason: format!("no label projection rule for state `{canonical_state}`"),
            dry_run: effective_dry_run,
            verdict: "skipped".to_string(),
        };
        write_receipt_if_requested(receipt, &receipt_payload)?;
        print_receipt(&receipt_payload)?;
        return Ok(());
    };

    if canonical_state == "MERGE_READY" && !has_valid_merge_ready_receipt(&state_payload) {
        let receipt_payload = LabelProjectionReceipt {
            current_labels,
            projected_apply: Vec::new(),
            projected_remove: Vec::new(),
            skipped: true,
            reason: "MERGE_READY projection refused: missing valid merge-ready receipt".to_string(),
            dry_run: effective_dry_run,
            verdict: "refused".to_string(),
        };
        write_receipt_if_requested(receipt, &receipt_payload)?;
        print_receipt(&receipt_payload)?;
        return Ok(());
    }

    let projected_apply = difference(&rule.apply, &current_labels);
    let projected_remove = intersection(&rule.remove, &current_labels);

    let mut receipt_payload = LabelProjectionReceipt {
        current_labels,
        projected_apply,
        projected_remove,
        skipped: false,
        reason: format!("state `{canonical_state}` projection"),
        dry_run: effective_dry_run,
        verdict: if effective_dry_run { "dry-run".to_string() } else { "applied".to_string() },
    };

    if apply {
        apply_projection(&state_payload, &receipt_payload, create_missing_labels)?;
    }

    write_receipt_if_requested(receipt, &receipt_payload)?;
    print_receipt(&receipt_payload)?;
    if apply {
        receipt_payload.verdict = "applied".to_string();
    }
    Ok(())
}

fn load_projection_config(path: &Path) -> Result<ProjectionConfig> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read projection config: {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("failed to parse TOML: {}", path.display()))
}

fn load_state_json(path: &Path) -> Result<Value> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read state receipt: {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse state receipt JSON: {}", path.display()))
}

fn extract_canonical_state(state: &Value) -> Option<String> {
    let candidates = [
        state.get("canonical_state").and_then(Value::as_str),
        state.get("state").and_then(Value::as_str),
        state.get("pr").and_then(|pr| pr.get("canonical_state")).and_then(Value::as_str),
        state.get("pr_state").and_then(Value::as_str),
    ];

    candidates.iter().flatten().next().map(|value| value.to_string())
}

fn extract_labels(state: &Value, key: &str) -> Vec<String> {
    let Some(array) = state.get(key).and_then(Value::as_array) else {
        return Vec::new();
    };

    array.iter().filter_map(Value::as_str).map(ToOwned::to_owned).collect()
}

fn has_valid_merge_ready_receipt(state: &Value) -> bool {
    if state.get("merge_ready_receipt_valid").and_then(Value::as_bool).is_some_and(|v| v) {
        return true;
    }

    if state.get("has_merge_ready_receipt").and_then(Value::as_bool).is_some_and(|v| v) {
        return true;
    }

    state
        .get("merge_readiness_receipt")
        .and_then(|receipt| receipt.get("valid"))
        .and_then(Value::as_bool)
        .is_some_and(|v| v)
}

fn difference(expected: &[String], current: &[String]) -> Vec<String> {
    expected.iter().filter(|item| !current.contains(*item)).cloned().collect()
}

fn intersection(expected_remove: &[String], current: &[String]) -> Vec<String> {
    expected_remove.iter().filter(|item| current.contains(*item)).cloned().collect()
}

fn apply_projection(
    state: &Value,
    receipt: &LabelProjectionReceipt,
    create_missing_labels: bool,
) -> Result<()> {
    let gh_token = std::env::var("GH_TOKEN").context(
        "--apply requested but GH_TOKEN is unavailable; set GH_TOKEN and rerun with --apply",
    )?;
    if gh_token.trim().is_empty() {
        bail!("--apply requested but GH_TOKEN is empty");
    }

    let pr_number = state
        .get("pr_number")
        .and_then(Value::as_u64)
        .ok_or_else(|| eyre!("--apply requires `pr_number` in state receipt"))?;

    for label in &receipt.projected_apply {
        let mut command = Command::new("gh");
        command
            .args(["pr", "edit", &pr_number.to_string(), "--add-label", label])
            .env("GH_TOKEN", &gh_token);

        let status = command.status().context("failed to execute `gh pr edit` for add-label")?;
        if !status.success() {
            if create_missing_labels {
                create_label(label, &gh_token)?;
                let mut retry = Command::new("gh");
                let retry_status = retry
                    .args(["pr", "edit", &pr_number.to_string(), "--add-label", label])
                    .env("GH_TOKEN", &gh_token)
                    .status()
                    .context("failed to retry `gh pr edit` after label creation")?;
                if !retry_status.success() {
                    bail!("failed to apply label `{label}` to PR #{pr_number}");
                }
            } else {
                bail!(
                    "failed to apply label `{label}` to PR #{pr_number}; refusing to create labels without --create-missing-labels"
                );
            }
        }
    }

    for label in &receipt.projected_remove {
        let status = Command::new("gh")
            .args(["pr", "edit", &pr_number.to_string(), "--remove-label", label])
            .env("GH_TOKEN", &gh_token)
            .status()
            .context("failed to execute `gh pr edit` for remove-label")?;
        if !status.success() {
            bail!("failed to remove label `{label}` from PR #{pr_number}");
        }
    }

    Ok(())
}

fn create_label(label: &str, gh_token: &str) -> Result<()> {
    let status = Command::new("gh")
        .args([
            "label",
            "create",
            label,
            "--color",
            "ededed",
            "--description",
            "auto-created by xtask queue project-labels",
        ])
        .env("GH_TOKEN", gh_token)
        .status()
        .context("failed to execute `gh label create`")?;

    if status.success() { Ok(()) } else { bail!("failed to create missing label `{label}`") }
}

fn write_receipt_if_requested(
    path: Option<PathBuf>,
    receipt: &LabelProjectionReceipt,
) -> Result<()> {
    let Some(path) = path else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create receipt dir: {}", parent.display()))?;
    }

    let serialized = serde_json::to_string_pretty(receipt)?;
    fs::write(&path, format!("{serialized}\n"))
        .with_context(|| format!("failed to write receipt: {}", path.display()))
}

fn print_receipt(receipt: &LabelProjectionReceipt) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(receipt)?);
    Ok(())
}
