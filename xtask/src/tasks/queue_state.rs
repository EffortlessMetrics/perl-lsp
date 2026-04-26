use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::tasks::queue_snapshot::{PullRequestSnapshot, QueueSnapshot};

const BLOCKER_LABELS: &[(&str, &str)] = &[
    ("needs-standards-review", "NEEDS_STANDARDS_REVIEW"),
    ("needs-deep-review", "NEEDS_DEEP_REVIEW"),
    ("needs-diff-audit", "NEEDS_DIFF_AUDIT"),
    ("needs-maintainer-review", "NEEDS_MAINTAINER_REVIEW"),
    ("needs-builder-fix", "NEEDS_BUILDER_FIX"),
    ("needs-diff-fix", "NEEDS_DIFF_FIX"),
    ("needs-ci-fix", "NEEDS_CI_FIX"),
    ("needs-cascade-update", "NEEDS_CASCADE_UPDATE"),
    ("needs-infra-fix", "NEEDS_INFRA_FIX"),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStateReceipt {
    pub generated_at: String,
    pub dry_run: bool,
    pub states: Vec<PrQueueState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrQueueState {
    pub number: u64,
    pub canonical_state: String,
    pub blockers: Vec<String>,
    pub stale_receipts: Vec<String>,
    pub projected_next_routes: Vec<String>,
    pub projected_labels: Vec<String>,
    pub contradictions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct InputReceipt {
    kind: String,
    #[serde(default)]
    pr_number: Option<u64>,
    #[serde(default)]
    head_sha: Option<String>,
    #[serde(default)]
    base_sha: Option<String>,
    #[serde(default)]
    valid: Option<bool>,
}

pub fn run(snapshot: PathBuf, dry_run: bool, receipt: Option<PathBuf>) -> Result<()> {
    let snapshot: QueueSnapshot = serde_json::from_slice(
        &fs::read(&snapshot).with_context(|| format!("failed reading {}", snapshot.display()))?,
    )
    .context("failed to parse queue snapshot JSON")?;

    let receipt_index = load_receipts_index(Path::new("target/receipts"))?;
    let states: Vec<PrQueueState> = snapshot
        .prs
        .iter()
        .map(|pr| derive_state(pr, receipt_index.get(&pr.number).cloned().unwrap_or_default()))
        .collect();

    let payload =
        QueueStateReceipt { generated_at: chrono::Utc::now().to_rfc3339(), dry_run, states };
    if let Some(receipt_path) = receipt {
        if let Some(parent) = receipt_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed creating {}", parent.display()))?;
        }
        fs::write(&receipt_path, serde_json::to_vec_pretty(&payload)?)
            .with_context(|| format!("failed writing {}", receipt_path.display()))?;
        println!("wrote queue state receipt to {}", receipt_path.display());
    } else {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    }

    Ok(())
}

fn derive_state(pr: &PullRequestSnapshot, receipts: Vec<InputReceipt>) -> PrQueueState {
    let label_set: std::collections::BTreeSet<String> =
        pr.labels.iter().map(|l| l.to_ascii_lowercase()).collect();

    let mut blockers = Vec::new();
    for (label, state) in BLOCKER_LABELS {
        if label_set.contains(*label) {
            blockers.push((*state).to_string());
        }
    }

    let mut stale_receipts = Vec::new();
    let mut has_valid_merge_ready_receipt = false;
    let mut has_reviewed_receipt = false;

    for receipt in receipts {
        if let Some(head_sha) = receipt.head_sha.as_ref() && head_sha != &pr.head_sha {
            stale_receipts.push(receipt.kind.clone());
        }
        if receipt.kind == "merge-readiness"
            && receipt.valid.unwrap_or(false)
            && let Some(base_sha) = receipt.base_sha.as_ref()
            && base_sha == &pr.base_sha
        {
            has_valid_merge_ready_receipt = true;
        }
        if receipt.kind == "review" && receipt.valid.unwrap_or(true) {
            has_reviewed_receipt = true;
        }
    }

    let mut contradictions = Vec::new();
    let ci_green = matches_ci_green(pr.status_rollup.as_deref());
    let ci_red = matches_ci_red(pr.status_rollup.as_deref());
    let reviewed = label_set.contains("review-reviewed") || has_reviewed_receipt;
    let merge_ready_labeled = label_set.contains("merge-ready");

    let canonical_state = if pr.merged_at.is_some() {
        "MERGED".to_string()
    } else if label_set.contains("superseded") {
        "SUPERSEDED".to_string()
    } else if pr.draft {
        "DRAFT".to_string()
    } else if !blockers.is_empty() {
        if reviewed {
            contradictions
                .push("review-reviewed conflicts with needs-* blocker labels".to_string());
        }
        blockers[0].clone()
    } else if ci_red {
        if label_set.contains("needs-ci-fix") {
            "NEEDS_CI_FIX".to_string()
        } else {
            "BLOCKED_UNKNOWN".to_string()
        }
    } else if merge_ready_labeled && ci_green && has_valid_merge_ready_receipt {
        "MERGE_READY".to_string()
    } else if merge_ready_labeled && !has_valid_merge_ready_receipt {
        contradictions
            .push("merge-ready label present without valid merge-readiness receipt".to_string());
        if reviewed { "REVIEWED_WAITING_CI".to_string() } else { "NEW".to_string() }
    } else if label_set.contains("queued") {
        "QUEUED".to_string()
    } else if ci_green {
        "CI_GREEN".to_string()
    } else if reviewed {
        "REVIEWED_WAITING_CI".to_string()
    } else {
        "NEW".to_string()
    };

    if !blockers.is_empty() && (canonical_state == "CI_GREEN" || canonical_state == "MERGE_READY") {
        contradictions.push("needs-* blockers prevent CI_GREEN/MERGE_READY".to_string());
    }

    let projected_next_routes = match canonical_state.as_str() {
        "DRAFT" => vec!["author:ready_for_review".to_string()],
        "NEW" => vec!["review:standards".to_string()],
        "REVIEWED_WAITING_CI" => vec!["ci:run".to_string()],
        "CI_GREEN" => vec!["review:merge-readiness".to_string()],
        "MERGE_READY" => vec!["queue:enqueue".to_string()],
        "QUEUED" => vec!["queue:merge".to_string()],
        "BLOCKED_UNKNOWN" => vec!["triage:classify-failure".to_string()],
        _ if canonical_state.starts_with("NEEDS_") => vec!["owner:resolve-blocker".to_string()],
        _ => Vec::new(),
    };

    let projected_labels = vec![canonical_state.to_ascii_lowercase()];

    PrQueueState {
        number: pr.number,
        canonical_state,
        blockers,
        stale_receipts,
        projected_next_routes,
        projected_labels,
        contradictions,
    }
}

fn matches_ci_green(state: Option<&str>) -> bool {
    matches!(state.map(|s| s.to_ascii_uppercase()), Some(s) if s == "GREEN" || s == "SUCCESS")
}

fn matches_ci_red(state: Option<&str>) -> bool {
    matches!(state.map(|s| s.to_ascii_uppercase()), Some(s) if s == "RED" || s == "FAILURE" || s == "ERROR")
}

fn load_receipts_index(base: &Path) -> Result<BTreeMap<u64, Vec<InputReceipt>>> {
    let mut by_pr = BTreeMap::<u64, Vec<InputReceipt>>::new();
    if !base.exists() {
        return Ok(by_pr);
    }

    for entry in fs::read_dir(base).with_context(|| format!("failed to read {}", base.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        let receipt: InputReceipt = match serde_json::from_slice(&bytes) {
            Ok(receipt) => receipt,
            Err(_) => continue,
        };
        if let Some(number) = receipt.pr_number {
            by_pr.entry(number).or_default().push(receipt);
        }
    }

    Ok(by_pr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reviewed_and_needs_builder_fix_reports_blocker_with_contradiction() -> Result<()> {
        let (pr, receipts) = load_fixture("review-reviewed-needs-builder-fix")?;
        let state = derive_state(&pr, receipts);
        assert_eq!(state.canonical_state, "NEEDS_BUILDER_FIX");
        assert!(state.contradictions.iter().any(|c| c.contains("conflicts")));
        Ok(())
    }

    #[test]
    fn all_signoffs_and_ci_green_results_ci_green() -> Result<()> {
        let (pr, receipts) = load_fixture("all-signoffs-ci-green")?;
        let state = derive_state(&pr, receipts);
        assert_eq!(state.canonical_state, "CI_GREEN");
        Ok(())
    }

    #[test]
    fn stale_merge_ready_receipt_is_not_merge_ready() -> Result<()> {
        let (pr, receipts) = load_fixture("merge-ready-stale-base-receipt")?;
        let state = derive_state(&pr, receipts);
        assert_ne!(state.canonical_state, "MERGE_READY");
        assert!(state.contradictions.iter().any(|c| c.contains("merge-ready")));
        Ok(())
    }

    fn load_fixture(name: &str) -> Result<(PullRequestSnapshot, Vec<InputReceipt>)> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("queue-state")
            .join(format!("{name}.json"));
        let value: serde_json::Value = serde_json::from_slice(&fs::read(&path)?)?;
        let pr: PullRequestSnapshot = serde_json::from_value(value["pr"].clone())?;
        let receipts: Vec<InputReceipt> = serde_json::from_value(value["receipts"].clone())?;
        Ok((pr, receipts))
    }
}
