use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

const DEFAULT_CONFIG_PATH: &str = ".ci/state/label-projection.toml";

#[derive(Debug, Deserialize)]
struct LabelProjectionConfig {
    state: BTreeMap<String, LabelRule>,
}

#[derive(Debug, Deserialize)]
struct LabelRule {
    #[serde(default)]
    apply: Vec<String>,
    #[serde(default)]
    remove: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct QueueState {
    pub canonical_state: String,
    #[serde(default)]
    pub current_labels: Vec<String>,
    #[serde(default)]
    pub pull_request: Option<PullRequestRef>,
    #[serde(default)]
    pub merge_ready_receipt_valid: bool,
}

#[derive(Debug, Deserialize)]
pub struct PullRequestRef {
    pub owner: String,
    pub repo: String,
    pub number: u64,
}

#[derive(Debug, Serialize)]
pub struct LabelProjectionReceipt {
    pub current_labels: Vec<String>,
    pub projected_apply: Vec<String>,
    pub projected_remove: Vec<String>,
    pub skipped: bool,
    pub reason: String,
    pub dry_run: bool,
    pub verdict: String,
}

pub fn run(
    state_path: &Path,
    dry_run_flag: bool,
    apply_flag: bool,
    receipt_path: Option<&Path>,
    create_labels: bool,
) -> Result<()> {
    if dry_run_flag && apply_flag {
        bail!("--dry-run and --apply are mutually exclusive");
    }

    let dry_run = !apply_flag;

    let root = crate::utils::project_root()?;
    let config_path = root.join(DEFAULT_CONFIG_PATH);
    let config_raw = fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read config: {}", config_path.display()))?;
    let config: LabelProjectionConfig = toml::from_str(&config_raw)
        .with_context(|| format!("failed to parse config: {}", config_path.display()))?;

    let state_raw = fs::read_to_string(state_path)
        .with_context(|| format!("failed to read state receipt: {}", state_path.display()))?;
    let state: QueueState = serde_json::from_str(&state_raw)
        .with_context(|| format!("failed to parse state receipt: {}", state_path.display()))?;

    let mut receipt = project_labels(&config, &state, dry_run);

    if !dry_run {
        if std::env::var("GH_TOKEN").is_err() {
            bail!("--apply requires GH_TOKEN in environment");
        }

        if let Some(pr) = &state.pull_request {
            apply_changes(pr, &receipt, create_labels)?;
        } else {
            receipt.skipped = true;
            receipt.reason =
                "state receipt missing pull_request metadata; cannot apply labels".to_string();
            receipt.verdict = "blocked".to_string();
        }
    }

    if let Some(path) = receipt_path {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create receipt dir: {}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(&receipt)?;
        fs::write(path, json)
            .with_context(|| format!("failed to write receipt: {}", path.display()))?;
    } else {
        println!("{}", serde_json::to_string_pretty(&receipt)?);
    }

    if receipt.verdict == "blocked" {
        bail!("label projection blocked: {}", receipt.reason);
    }

    Ok(())
}

fn project_labels(
    config: &LabelProjectionConfig,
    state: &QueueState,
    dry_run: bool,
) -> LabelProjectionReceipt {
    let mut current_labels = state.current_labels.clone();
    current_labels.sort();

    let Some(rule) = config.state.get(&state.canonical_state) else {
        return LabelProjectionReceipt {
            current_labels,
            projected_apply: Vec::new(),
            projected_remove: Vec::new(),
            skipped: true,
            reason: format!("no projection rule for state {}", state.canonical_state),
            dry_run,
            verdict: "noop".to_string(),
        };
    };

    if state.canonical_state == "MERGE_READY" && !state.merge_ready_receipt_valid {
        return LabelProjectionReceipt {
            current_labels,
            projected_apply: Vec::new(),
            projected_remove: Vec::new(),
            skipped: true,
            reason: "merge-ready receipt missing or invalid".to_string(),
            dry_run,
            verdict: "blocked".to_string(),
        };
    }

    let current_set = current_labels.iter().cloned().collect::<BTreeSet<_>>();
    let mut projected_apply = rule
        .apply
        .iter()
        .filter(|label| !current_set.contains(*label))
        .cloned()
        .collect::<Vec<_>>();
    let mut projected_remove = rule
        .remove
        .iter()
        .filter(|label| current_set.contains(*label))
        .cloned()
        .collect::<Vec<_>>();

    projected_apply.sort();
    projected_remove.sort();

    let skipped = projected_apply.is_empty() && projected_remove.is_empty();
    let verdict = if skipped { "noop" } else { "ok" };

    LabelProjectionReceipt {
        current_labels,
        projected_apply,
        projected_remove,
        skipped,
        reason: if skipped {
            "already aligned with projection".to_string()
        } else {
            "computed from canonical state".to_string()
        },
        dry_run,
        verdict: verdict.to_string(),
    }
}

fn apply_changes(
    pr: &PullRequestRef,
    receipt: &LabelProjectionReceipt,
    create_labels: bool,
) -> Result<()> {
    for label in &receipt.projected_apply {
        apply_label(pr, label, create_labels)?;
    }

    for label in &receipt.projected_remove {
        remove_label(pr, label)?;
    }

    Ok(())
}

fn apply_label(pr: &PullRequestRef, label: &str, create_labels: bool) -> Result<()> {
    let endpoint = format!("repos/{}/{}/issues/{}/labels", pr.owner, pr.repo, pr.number);
    run_gh_api(
        "POST",
        &endpoint,
        Some(serde_json::json!({ "labels": [label] })),
        format!("failed to apply label `{label}`"),
    )
    .or_else(|err| {
        if create_labels {
            let create_endpoint = format!("repos/{}/{}/labels", pr.owner, pr.repo);
            run_gh_api(
                "POST",
                &create_endpoint,
                Some(serde_json::json!({ "name": label, "color": "ededed" })),
                format!("failed to create missing label `{label}`"),
            )?;
            run_gh_api(
                "POST",
                &endpoint,
                Some(serde_json::json!({ "labels": [label] })),
                format!("failed to apply label `{label}` after creation"),
            )
        } else {
            Err(err)
        }
    })
}

fn remove_label(pr: &PullRequestRef, label: &str) -> Result<()> {
    let endpoint = format!("repos/{}/{}/issues/{}/labels/{label}", pr.owner, pr.repo, pr.number);
    run_gh_api("DELETE", &endpoint, None, format!("failed to remove label `{label}`"))
}

fn run_gh_api(
    method: &str,
    endpoint: &str,
    body: Option<serde_json::Value>,
    context: String,
) -> Result<()> {
    let mut cmd = Command::new("gh");
    cmd.arg("api")
        .arg("--method")
        .arg(method)
        .arg(endpoint)
        .arg("-H")
        .arg("Accept: application/vnd.github+json");

    if let Some(payload) = body {
        cmd.arg("--input").arg("-");
        let payload = serde_json::to_vec(&payload)?;
        let mut child = cmd
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .with_context(|| format!("{context}: launch gh"))?;

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin.write_all(&payload)?;
        }

        let output = child.wait_with_output()?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("{context}: {}", stderr.trim())
        }
    } else {
        let output = cmd.output().with_context(|| format!("{context}: execute gh"))?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("{context}: {}", stderr.trim())
        }
    }
}
