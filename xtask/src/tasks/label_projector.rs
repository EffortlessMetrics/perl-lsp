use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_CONFIG_PATH: &str = ".ci/state/label-projection.toml";

#[derive(Debug, Clone)]
pub struct LabelProjectorConfig {
    pub state_path: PathBuf,
    pub dry_run_flag: bool,
    pub apply: bool,
    pub receipt_path: Option<PathBuf>,
    pub projection_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct ProjectionFile {
    state: BTreeMap<String, ProjectionRule>,
}

#[derive(Debug, Deserialize)]
struct ProjectionRule {
    #[serde(default)]
    apply: Vec<String>,
    #[serde(default)]
    remove: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct LabelProjectionReceipt {
    pub current_labels: Vec<String>,
    pub projected_apply: Vec<String>,
    pub projected_remove: Vec<String>,
    pub skipped: bool,
    pub reason: Option<String>,
    pub dry_run: bool,
    pub verdict: String,
}

pub fn run(config: LabelProjectorConfig) -> Result<()> {
    let effective_dry_run = !config.apply || config.dry_run_flag;
    let state_value = read_json(&config.state_path)?;
    let canonical_state = state_value
        .get("canonical_state")
        .or_else(|| state_value.get("state"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            color_eyre::eyre::eyre!(
                "state JSON must contain `canonical_state` (or legacy `state`) string"
            )
        })?;

    let projection_path =
        config.projection_path.clone().unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_PATH));
    let projection = read_projection_file(&projection_path)?;
    let rule = projection.state.get(&canonical_state).ok_or_else(|| {
        color_eyre::eyre::eyre!("no label projection rule for state `{canonical_state}`")
    })?;

    let mut receipt = build_receipt(&state_value, &canonical_state, rule, !effective_dry_run)?;

    if !effective_dry_run {
        ensure_apply_ready(&state_value)?;
        apply_projection(&state_value, &receipt)?;
        receipt.dry_run = false;
        receipt.verdict = "applied".to_string();
    }

    if let Some(path) = config.receipt_path {
        write_json(&path, &receipt)?;
        println!("wrote label projection receipt: {}", path.display());
    } else {
        println!("{}", serde_json::to_string_pretty(&receipt)?);
    }

    Ok(())
}

fn build_receipt(
    state_value: &Value,
    canonical_state: &str,
    rule: &ProjectionRule,
    apply_mode: bool,
) -> Result<LabelProjectionReceipt> {
    let current_labels = read_labels(state_value)?;
    let current_set: std::collections::BTreeSet<&str> =
        current_labels.iter().map(String::as_str).collect();

    let mut projected_apply = rule
        .apply
        .iter()
        .filter(|label| !current_set.contains(label.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    projected_apply.sort_unstable();

    let mut projected_remove = rule
        .remove
        .iter()
        .filter(|label| current_set.contains(label.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    projected_remove.sort_unstable();

    let merge_ready_ok = merge_ready_receipt_valid(state_value);
    if canonical_state == "MERGE_READY" && !merge_ready_ok {
        projected_apply.retain(|label| label != "merge-ready");
        return Ok(LabelProjectionReceipt {
            current_labels,
            projected_apply,
            projected_remove,
            skipped: true,
            reason: Some(
                "missing valid merge-ready receipt; refusing to project merge-ready".to_string(),
            ),
            dry_run: !apply_mode,
            verdict: "refused".to_string(),
        });
    }

    Ok(LabelProjectionReceipt {
        current_labels,
        projected_apply,
        projected_remove,
        skipped: false,
        reason: None,
        dry_run: !apply_mode,
        verdict: if apply_mode { "pending-apply" } else { "dry-run" }.to_string(),
    })
}

fn merge_ready_receipt_valid(state_value: &Value) -> bool {
    state_value
        .get("merge_ready_receipt")
        .and_then(|value| value.get("valid"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn ensure_apply_ready(state_value: &Value) -> Result<()> {
    let token = std::env::var("GH_TOKEN").unwrap_or_default();
    if token.trim().is_empty() {
        bail!("--apply requires GH_TOKEN in the environment");
    }

    if state_value.get("pr").and_then(|value| value.get("number")).and_then(Value::as_u64).is_none()
    {
        bail!("--apply requires state JSON with `pr.number`");
    }

    if state_value.get("pr").and_then(|value| value.get("repo")).and_then(Value::as_str).is_none() {
        bail!("--apply requires state JSON with `pr.repo` (owner/name)");
    }

    Ok(())
}

fn apply_projection(state_value: &Value, receipt: &LabelProjectionReceipt) -> Result<()> {
    if receipt.skipped {
        return Ok(());
    }

    let pr_number = state_value
        .get("pr")
        .and_then(|value| value.get("number"))
        .and_then(Value::as_u64)
        .ok_or_else(|| color_eyre::eyre::eyre!("missing pr.number"))?;
    let repo = state_value
        .get("pr")
        .and_then(|value| value.get("repo"))
        .and_then(Value::as_str)
        .ok_or_else(|| color_eyre::eyre::eyre!("missing pr.repo"))?;

    for label in &receipt.projected_apply {
        let mut command = Command::new("gh");
        command
            .arg("api")
            .arg(format!("repos/{repo}/issues/{pr_number}/labels"))
            .arg("--method")
            .arg("POST")
            .arg("-f")
            .arg(format!("labels[]={label}"));
        let status = command.status().with_context(|| "failed running gh api")?;
        if !status.success() {
            bail!("failed applying label `{label}` to PR #{pr_number}");
        }
    }

    for label in &receipt.projected_remove {
        let mut command = Command::new("gh");
        command
            .arg("api")
            .arg(format!("repos/{repo}/issues/{pr_number}/labels/{label}"))
            .arg("--method")
            .arg("DELETE");
        let status = command.status().with_context(|| "failed running gh api")?;
        if !status.success() {
            bail!("failed removing label `{label}` from PR #{pr_number}");
        }
    }

    Ok(())
}

fn read_labels(state_value: &Value) -> Result<Vec<String>> {
    let labels = state_value
        .get("current_labels")
        .or_else(|| state_value.get("labels"))
        .and_then(Value::as_array)
        .ok_or_else(|| color_eyre::eyre::eyre!("state JSON must contain `current_labels` array"))?;

    labels
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| color_eyre::eyre::eyre!("labels must be strings"))
        })
        .collect()
}

fn read_projection_file(path: &Path) -> Result<ProjectionFile> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("reading label projection config {}", path.display()))?;
    toml::from_str(&content)
        .with_context(|| format!("parsing label projection config {}", path.display()))
}

fn read_json(path: &Path) -> Result<Value> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("reading state JSON {}", path.display()))?;
    serde_json::from_str(&content).with_context(|| format!("parsing state JSON {}", path.display()))
}

fn write_json(path: &Path, receipt: &LabelProjectionReceipt) -> Result<()> {
    let Some(parent) = path.parent() else {
        bail!("receipt path must have a parent directory");
    };
    fs::create_dir_all(parent)
        .with_context(|| format!("creating receipt directory {}", parent.display()))?;
    let content = serde_json::to_string_pretty(receipt)?;
    fs::write(path, format!("{content}\n"))
        .with_context(|| format!("writing receipt {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn dry_run_needs_builder_fix_projects_expected_labels() -> Result<()> {
        let fixture = PathBuf::from("tests/fixtures/label-projector/needs-builder-fix.json");
        let state_value = read_json(&fixture)?;
        let projection = read_projection_file(Path::new("../.ci/state/label-projection.toml"))?;
        let rule = projection
            .state
            .get("NEEDS_BUILDER_FIX")
            .ok_or_else(|| color_eyre::eyre::eyre!("missing NEEDS_BUILDER_FIX rule"))?;

        let receipt = build_receipt(&state_value, "NEEDS_BUILDER_FIX", rule, false)?;

        assert_eq!(receipt.projected_apply, vec!["needs-builder-fix".to_string()]);
        assert_eq!(
            receipt.projected_remove,
            vec!["ci-green".to_string(), "merge-ready".to_string(), "review-reviewed".to_string()]
        );
        assert!(receipt.reason.is_none());
        assert_eq!(receipt.verdict, "dry-run");
        Ok(())
    }

    #[test]
    fn merge_ready_is_refused_without_merge_receipt() -> Result<()> {
        let fixture =
            PathBuf::from("tests/fixtures/label-projector/merge-ready-missing-receipt.json");
        let state_value = read_json(&fixture)?;
        let projection = read_projection_file(Path::new("../.ci/state/label-projection.toml"))?;
        let rule = projection
            .state
            .get("MERGE_READY")
            .ok_or_else(|| color_eyre::eyre::eyre!("missing MERGE_READY rule"))?;

        let receipt = build_receipt(&state_value, "MERGE_READY", rule, false)?;

        assert!(receipt.skipped);
        assert_eq!(receipt.verdict, "refused");
        assert!(receipt.reason.unwrap_or_default().contains("missing valid merge-ready receipt"));
        assert!(!receipt.projected_apply.iter().any(|entry| entry == "merge-ready"));
        Ok(())
    }

    #[test]
    fn merge_ready_projects_when_receipt_valid() -> Result<()> {
        let state_value = json!({
            "canonical_state": "MERGE_READY",
            "current_labels": ["ci-green", "needs-ci-fix"],
            "merge_ready_receipt": { "valid": true }
        });
        let projection = read_projection_file(Path::new("../.ci/state/label-projection.toml"))?;
        let rule = projection
            .state
            .get("MERGE_READY")
            .ok_or_else(|| color_eyre::eyre::eyre!("missing MERGE_READY rule"))?;

        let receipt = build_receipt(&state_value, "MERGE_READY", rule, false)?;

        assert!(!receipt.skipped);
        assert!(receipt.projected_apply.iter().any(|entry| entry == "merge-ready"));
        Ok(())
    }
}
