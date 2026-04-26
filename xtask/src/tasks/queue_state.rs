use crate::tasks::queue_snapshot::{PullRequestFacts, QueueSnapshot};
use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const CANONICAL_STATES: &[&str] = &[
    "DRAFT",
    "NEW",
    "NEEDS_STANDARDS_REVIEW",
    "NEEDS_DEEP_REVIEW",
    "NEEDS_DIFF_AUDIT",
    "NEEDS_MAINTAINER_REVIEW",
    "NEEDS_BUILDER_FIX",
    "NEEDS_DIFF_FIX",
    "NEEDS_CI_FIX",
    "NEEDS_CASCADE_UPDATE",
    "NEEDS_INFRA_FIX",
    "REVIEWED_WAITING_CI",
    "CI_GREEN",
    "MERGE_READY",
    "QUEUED",
    "MERGED",
    "SUPERSEDED",
    "BLOCKED_UNKNOWN",
];

#[derive(Debug, Clone)]
pub struct QueueStateConfig {
    pub snapshot: PathBuf,
    pub dry_run: bool,
    pub receipt: PathBuf,
    pub receipts_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueueReceipt {
    pub pr_number: u64,
    pub receipt_type: String,
    pub head_sha: Option<String>,
    pub base_sha: Option<String>,
    #[serde(default = "default_true")]
    pub valid: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueueStateReceipt {
    schema_version: u32,
    dry_run: bool,
    states: Vec<PrStateResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PrStateResult {
    pr_number: u64,
    canonical_state: String,
    blockers: Vec<String>,
    stale_receipts: Vec<String>,
    projected_next_routes: Vec<String>,
    projected_labels: Vec<String>,
    contradictions: Vec<String>,
}

pub fn run(config: QueueStateConfig) -> Result<()> {
    let snapshot_json = fs::read_to_string(&config.snapshot)
        .with_context(|| format!("failed to read {}", config.snapshot.display()))?;
    let snapshot: QueueSnapshot =
        serde_json::from_str(&snapshot_json).context("failed to parse snapshot json")?;

    let receipts = load_receipts(config.receipts_dir.as_deref())?;

    let states = snapshot.pull_requests.iter().map(|pr| derive_state(pr, &receipts)).collect();

    let payload = QueueStateReceipt { schema_version: 1, dry_run: config.dry_run, states };

    if let Some(parent) = config.receipt.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let json = serde_json::to_string_pretty(&payload)?;
    fs::write(&config.receipt, json)
        .with_context(|| format!("failed to write {}", config.receipt.display()))?;

    Ok(())
}

fn load_receipts(receipts_dir: Option<&Path>) -> Result<Vec<QueueReceipt>> {
    let path = receipts_dir.unwrap_or_else(|| Path::new("target/receipts"));
    if !path.exists() {
        return Ok(Vec::new());
    }

    let mut receipts = Vec::new();
    for entry in fs::read_dir(path).with_context(|| format!("failed to read {}", path.display()))? {
        let entry = entry?;
        let entry_path = entry.path();
        let extension = entry_path.extension().and_then(|ext| ext.to_str());
        if extension != Some("json") {
            continue;
        }

        let raw = fs::read_to_string(&entry_path)
            .with_context(|| format!("failed to read receipt {}", entry_path.display()))?;
        if let Ok(receipt) = serde_json::from_str::<QueueReceipt>(&raw) {
            receipts.push(receipt);
        }
    }

    Ok(receipts)
}

fn derive_state(pr: &PullRequestFacts, receipts: &[QueueReceipt]) -> PrStateResult {
    let label_set: BTreeSet<&str> = pr.labels.iter().map(String::as_str).collect();

    let pr_receipts: Vec<&QueueReceipt> =
        receipts.iter().filter(|receipt| receipt.pr_number == pr.number).collect();
    let (stale_receipts, has_valid_merge_ready) = evaluate_receipts(pr, &pr_receipts);

    let mut contradictions = Vec::new();
    let blockers = gather_blockers(&label_set, &pr.status_rollup);

    let canonical_state = if pr.draft {
        "DRAFT"
    } else if pr.merged {
        "MERGED"
    } else if label_set.contains("superseded") {
        "SUPERSEDED"
    } else if label_set.contains("queued") {
        "QUEUED"
    } else if let Some(blocker_state) = blocking_state(&label_set) {
        blocker_state
    } else if label_set.contains("needs-standards-review") {
        "NEEDS_STANDARDS_REVIEW"
    } else if label_set.contains("needs-deep-review") {
        "NEEDS_DEEP_REVIEW"
    } else if label_set.contains("needs-diff-audit") {
        "NEEDS_DIFF_AUDIT"
    } else if label_set.contains("needs-maintainer-review") {
        "NEEDS_MAINTAINER_REVIEW"
    } else if pr.status_rollup == "red" {
        "BLOCKED_UNKNOWN"
    } else if pr.status_rollup == "green" {
        if label_set.contains("merge-ready") && has_valid_merge_ready {
            "MERGE_READY"
        } else {
            "CI_GREEN"
        }
    } else if label_set.contains("review-reviewed") {
        "REVIEWED_WAITING_CI"
    } else {
        "NEW"
    }
    .to_string();
    debug_assert!(CANONICAL_STATES.contains(&canonical_state.as_str()));

    let has_needs_blocker = blocking_state(&label_set).is_some();
    if has_needs_blocker && (canonical_state == "CI_GREEN" || canonical_state == "MERGE_READY") {
        contradictions.push("needs_blocker_conflicts_with_green_state".to_string());
    }
    if has_needs_blocker
        && (label_set.contains("review-reviewed")
            || label_set.contains("ci-green")
            || label_set.contains("merge-ready"))
    {
        contradictions.push("conflicting_positive_signals_with_needs_blocker".to_string());
    }
    if label_set.contains("merge-ready") && !has_valid_merge_ready {
        contradictions.push("merge_ready_label_without_valid_receipt".to_string());
    }

    PrStateResult {
        pr_number: pr.number,
        canonical_state: canonical_state.clone(),
        blockers,
        stale_receipts,
        projected_next_routes: next_routes(&canonical_state),
        projected_labels: projected_labels(&canonical_state),
        contradictions,
    }
}

fn evaluate_receipts(pr: &PullRequestFacts, receipts: &[&QueueReceipt]) -> (Vec<String>, bool) {
    let mut stale_receipts = Vec::new();
    let mut merge_ready_valid = false;

    for receipt in receipts {
        if receipt.receipt_type == "review"
            && receipt.head_sha.as_deref().is_some_and(|head_sha| head_sha != pr.head_sha)
        {
            stale_receipts.push("review_head_sha_mismatch".to_string());
        }

        if receipt.receipt_type == "merge-readiness" {
            let head_ok =
                receipt.head_sha.as_deref().is_some_and(|head_sha| head_sha == pr.head_sha);
            let base_ok =
                receipt.base_sha.as_deref().is_some_and(|base_sha| base_sha == pr.base_sha);
            if head_ok && base_ok && receipt.valid {
                merge_ready_valid = true;
            } else {
                stale_receipts.push("merge_readiness_stale_or_invalid".to_string());
            }
        }
    }

    (stale_receipts, merge_ready_valid)
}

fn blocking_state(labels: &BTreeSet<&str>) -> Option<&'static str> {
    if labels.contains("needs-builder-fix") {
        Some("NEEDS_BUILDER_FIX")
    } else if labels.contains("needs-diff-fix") {
        Some("NEEDS_DIFF_FIX")
    } else if labels.contains("needs-ci-fix") {
        Some("NEEDS_CI_FIX")
    } else if labels.contains("needs-cascade-update") {
        Some("NEEDS_CASCADE_UPDATE")
    } else if labels.contains("needs-infra-fix") {
        Some("NEEDS_INFRA_FIX")
    } else {
        None
    }
}

fn gather_blockers(labels: &BTreeSet<&str>, status_rollup: &str) -> Vec<String> {
    let mut blockers = Vec::new();

    for blocked in [
        "needs-builder-fix",
        "needs-diff-fix",
        "needs-ci-fix",
        "needs-cascade-update",
        "needs-infra-fix",
    ] {
        if labels.contains(blocked) {
            blockers.push(blocked.to_string());
        }
    }

    if status_rollup == "red" && !labels.contains("needs-ci-fix") {
        blockers.push("ci_red_unclassified".to_string());
    }

    blockers
}

fn next_routes(state: &str) -> Vec<String> {
    let route = match state {
        "DRAFT" => "await-ready-for-review",
        "NEW" => "standards-review",
        "NEEDS_STANDARDS_REVIEW" => "standards-review",
        "NEEDS_DEEP_REVIEW" => "deep-review",
        "NEEDS_DIFF_AUDIT" => "diff-audit",
        "NEEDS_MAINTAINER_REVIEW" => "maintainer-review",
        "NEEDS_BUILDER_FIX"
        | "NEEDS_DIFF_FIX"
        | "NEEDS_CI_FIX"
        | "NEEDS_CASCADE_UPDATE"
        | "NEEDS_INFRA_FIX" => "author-fix",
        "REVIEWED_WAITING_CI" => "await-ci",
        "CI_GREEN" => "merge-readiness",
        "MERGE_READY" => "merge-queue",
        "QUEUED" => "queue-wait",
        "MERGED" => "closed",
        "SUPERSEDED" => "close-superseded",
        "BLOCKED_UNKNOWN" => "triage",
        _ => "triage",
    };
    vec![route.to_string()]
}

fn projected_labels(state: &str) -> Vec<String> {
    match state {
        "DRAFT" => vec!["state:draft".to_string()],
        "NEW" => vec!["state:new".to_string()],
        "CI_GREEN" => vec!["state:ci-green".to_string()],
        "MERGE_READY" => vec!["state:merge-ready".to_string()],
        "QUEUED" => vec!["state:queued".to_string()],
        other => vec![format!("state:{}", other.to_lowercase())],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::Result;

    #[derive(Debug, Deserialize)]
    struct Fixture {
        pr: PullRequestFacts,
        receipts: Vec<QueueReceipt>,
        expected_state: String,
        expected_contradiction: Option<String>,
    }

    #[test]
    fn canonical_state_list_is_complete() {
        assert_eq!(CANONICAL_STATES.len(), 18);
    }

    #[test]
    fn fixture_review_plus_builder_fix_maps_to_builder_fix_with_contradiction() -> Result<()> {
        let fixture = load_fixture("review-reviewed-needs-builder-fix.json")?;
        let actual = derive_state(&fixture.pr, &fixture.receipts);
        assert_eq!(actual.canonical_state, fixture.expected_state);
        let expected = fixture.expected_contradiction.unwrap_or_default();
        assert!(actual.contradictions.iter().any(|item| item == &expected));
        Ok(())
    }

    #[test]
    fn fixture_all_signoffs_green_maps_to_ci_green() -> Result<()> {
        let fixture = load_fixture("all-signoffs-ci-green.json")?;
        let actual = derive_state(&fixture.pr, &fixture.receipts);
        assert_eq!(actual.canonical_state, fixture.expected_state);
        Ok(())
    }

    #[test]
    fn fixture_merge_ready_label_with_stale_base_not_merge_ready() -> Result<()> {
        let fixture = load_fixture("merge-ready-stale-base-receipt.json")?;
        let actual = derive_state(&fixture.pr, &fixture.receipts);
        assert_eq!(actual.canonical_state, fixture.expected_state);
        assert!(
            actual.stale_receipts.iter().any(|value| value == "merge_readiness_stale_or_invalid")
        );
        Ok(())
    }

    fn load_fixture(name: &str) -> Result<Fixture> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("queue-state")
            .join(name);
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read fixture {}", path.display()))?;
        let fixture = serde_json::from_str::<Fixture>(&raw)
            .with_context(|| format!("failed to parse fixture {}", path.display()))?;
        Ok(fixture)
    }
}
