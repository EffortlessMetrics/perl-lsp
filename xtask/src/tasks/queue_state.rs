use chrono::Utc;
use color_eyre::eyre::{Context, Result, eyre};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::tasks::queue_snapshot::{PullRequestSnapshot, QueueSnapshot, ReceiptRef};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CanonicalState {
    Draft,
    New,
    NeedsStandardsReview,
    NeedsDeepReview,
    NeedsDiffAudit,
    NeedsMaintainerReview,
    NeedsBuilderFix,
    NeedsDiffFix,
    NeedsCiFix,
    NeedsCascadeUpdate,
    NeedsInfraFix,
    ReviewedWaitingCi,
    CiGreen,
    MergeReady,
    Queued,
    Merged,
    Superseded,
    BlockedUnknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrStateResult {
    pub number: u64,
    pub canonical_state: CanonicalState,
    pub blockers: Vec<String>,
    pub stale_receipts: Vec<String>,
    pub projected_next_routes: Vec<String>,
    pub projected_labels: Vec<String>,
    pub contradictions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStateReceipt {
    pub schema_version: u32,
    pub dry_run: bool,
    pub generated_at: String,
    pub results: Vec<PrStateResult>,
}

pub fn run(snapshot: PathBuf, dry_run: bool, receipt: PathBuf) -> Result<()> {
    let raw = fs::read_to_string(&snapshot)
        .with_context(|| format!("failed to read snapshot {}", snapshot.display()))?;
    let snapshot: QueueSnapshot =
        serde_json::from_str(&raw).with_context(|| "failed to parse snapshot json")?;

    let results = snapshot.prs.iter().map(derive_state).collect::<Result<Vec<_>>>()?;
    let receipt_doc = QueueStateReceipt {
        schema_version: 1,
        dry_run,
        generated_at: Utc::now().to_rfc3339(),
        results,
    };

    if let Some(parent) = receipt.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create output directory: {}", parent.display()))?;
    }

    let json = serde_json::to_string_pretty(&receipt_doc).context("failed to serialize receipt")?;
    fs::write(&receipt, json)
        .with_context(|| format!("failed to write receipt {}", receipt.display()))?;
    println!("wrote queue state receipt to {}", receipt.display());
    Ok(())
}

fn derive_state(pr: &PullRequestSnapshot) -> Result<PrStateResult> {
    if pr.head_sha.trim().is_empty() || pr.base_sha.trim().is_empty() {
        return Err(eyre!("pr {} missing required head_sha/base_sha", pr.number));
    }

    let mut blockers = Vec::new();
    let mut stale_receipts = Vec::new();
    let mut contradictions = Vec::new();

    let has_label = |label: &str| pr.labels.iter().any(|l| l == label);

    for blocker in [
        "needs-standards-review",
        "needs-deep-review",
        "needs-diff-audit",
        "needs-maintainer-review",
        "needs-builder-fix",
        "needs-diff-fix",
        "needs-ci-fix",
        "needs-cascade-update",
        "needs-infra-fix",
    ] {
        if has_label(blocker) {
            blockers.push(blocker.to_string());
        }
    }

    if receipt_stale(pr.receipts.as_slice(), "review", pr.head_sha.as_str()) {
        stale_receipts.push("review".to_string());
    }

    if receipt_stale_by_base(pr.receipts.as_slice(), "merge-readiness", pr.base_sha.as_str()) {
        stale_receipts.push("merge-readiness".to_string());
    }

    let has_reviewed = has_label("review-reviewed");
    let ci_green = pr.status_rollup == "green";
    let ci_red = pr.status_rollup == "red";
    let merge_ready_label = has_label("merge-ready");
    let merge_ready_receipt_valid = has_valid_receipt(
        pr.receipts.as_slice(),
        "merge-readiness",
        pr.head_sha.as_str(),
        pr.base_sha.as_str(),
    );

    let canonical_state = if pr.draft {
        CanonicalState::Draft
    } else if has_label("queued") {
        CanonicalState::Queued
    } else if has_label("merged") {
        CanonicalState::Merged
    } else if has_label("superseded") {
        CanonicalState::Superseded
    } else if has_label("needs-builder-fix") {
        CanonicalState::NeedsBuilderFix
    } else if has_label("needs-diff-fix") {
        CanonicalState::NeedsDiffFix
    } else if has_label("needs-ci-fix") {
        CanonicalState::NeedsCiFix
    } else if has_label("needs-infra-fix") {
        CanonicalState::NeedsInfraFix
    } else if has_label("needs-cascade-update") {
        CanonicalState::NeedsCascadeUpdate
    } else if has_label("needs-maintainer-review") {
        CanonicalState::NeedsMaintainerReview
    } else if has_label("needs-diff-audit") {
        CanonicalState::NeedsDiffAudit
    } else if has_label("needs-deep-review") {
        CanonicalState::NeedsDeepReview
    } else if has_label("needs-standards-review") {
        CanonicalState::NeedsStandardsReview
    } else if ci_red && !has_label("needs-ci-fix") {
        CanonicalState::BlockedUnknown
    } else if merge_ready_label && merge_ready_receipt_valid && ci_green && blockers.is_empty() {
        CanonicalState::MergeReady
    } else if ci_green && blockers.is_empty() {
        CanonicalState::CiGreen
    } else if has_reviewed {
        CanonicalState::ReviewedWaitingCi
    } else {
        CanonicalState::New
    };

    if !blockers.is_empty()
        && (canonical_state == CanonicalState::CiGreen
            || canonical_state == CanonicalState::MergeReady)
    {
        contradictions.push(
            "needs-* blockers present while canonical state indicates green/merge-ready"
                .to_string(),
        );
    }

    if has_reviewed && has_label("needs-builder-fix") {
        contradictions.push("review-reviewed contradicts needs-builder-fix".to_string());
    }

    if merge_ready_label && !merge_ready_receipt_valid {
        contradictions
            .push("merge-ready label present without valid merge-readiness receipt".to_string());
    }

    if ci_red && !has_label("needs-ci-fix") {
        contradictions.push("ci red without classifier; ownership unknown".to_string());
    }

    let projected_next_routes = next_routes(&canonical_state);
    let projected_labels = projected_labels_for_state(&canonical_state);

    Ok(PrStateResult {
        number: pr.number,
        canonical_state,
        blockers,
        stale_receipts,
        projected_next_routes,
        projected_labels,
        contradictions,
    })
}

fn next_routes(state: &CanonicalState) -> Vec<String> {
    match state {
        CanonicalState::Draft => vec!["author:ready-for-review".to_string()],
        CanonicalState::New => vec!["review:standards".to_string()],
        CanonicalState::NeedsStandardsReview => vec!["review:standards".to_string()],
        CanonicalState::NeedsDeepReview => vec!["review:deep".to_string()],
        CanonicalState::NeedsDiffAudit => vec!["review:diff-audit".to_string()],
        CanonicalState::NeedsMaintainerReview => vec!["review:maintainer".to_string()],
        CanonicalState::NeedsBuilderFix
        | CanonicalState::NeedsDiffFix
        | CanonicalState::NeedsCiFix
        | CanonicalState::NeedsCascadeUpdate
        | CanonicalState::NeedsInfraFix => vec!["author:fix".to_string()],
        CanonicalState::ReviewedWaitingCi => vec!["ci:run".to_string()],
        CanonicalState::CiGreen => vec!["review:merge-readiness".to_string()],
        CanonicalState::MergeReady => vec!["queue:enqueue".to_string()],
        CanonicalState::Queued => vec!["queue:merge".to_string()],
        CanonicalState::Merged | CanonicalState::Superseded => vec![],
        CanonicalState::BlockedUnknown => vec!["triage:classifier".to_string()],
    }
}

fn projected_labels_for_state(state: &CanonicalState) -> Vec<String> {
    match state {
        CanonicalState::Draft => vec!["state:draft".to_string()],
        CanonicalState::New => vec!["state:new".to_string()],
        CanonicalState::NeedsStandardsReview => vec!["needs-standards-review".to_string()],
        CanonicalState::NeedsDeepReview => vec!["needs-deep-review".to_string()],
        CanonicalState::NeedsDiffAudit => vec!["needs-diff-audit".to_string()],
        CanonicalState::NeedsMaintainerReview => vec!["needs-maintainer-review".to_string()],
        CanonicalState::NeedsBuilderFix => vec!["needs-builder-fix".to_string()],
        CanonicalState::NeedsDiffFix => vec!["needs-diff-fix".to_string()],
        CanonicalState::NeedsCiFix => vec!["needs-ci-fix".to_string()],
        CanonicalState::NeedsCascadeUpdate => vec!["needs-cascade-update".to_string()],
        CanonicalState::NeedsInfraFix => vec!["needs-infra-fix".to_string()],
        CanonicalState::ReviewedWaitingCi => vec!["review-reviewed".to_string()],
        CanonicalState::CiGreen => vec!["ci-green".to_string()],
        CanonicalState::MergeReady => vec!["merge-ready".to_string()],
        CanonicalState::Queued => vec!["queued".to_string()],
        CanonicalState::Merged => vec!["merged".to_string()],
        CanonicalState::Superseded => vec!["superseded".to_string()],
        CanonicalState::BlockedUnknown => vec!["blocked-unknown".to_string()],
    }
}

fn has_valid_receipt(receipts: &[ReceiptRef], kind: &str, head_sha: &str, base_sha: &str) -> bool {
    receipts.iter().any(|receipt| {
        receipt.kind == kind
            && receipt.valid.unwrap_or(false)
            && receipt.head_sha.as_deref() == Some(head_sha)
            && receipt.base_sha.as_deref() == Some(base_sha)
    })
}

fn receipt_stale(receipts: &[ReceiptRef], kind: &str, head_sha: &str) -> bool {
    receipts
        .iter()
        .find(|receipt| receipt.kind == kind)
        .and_then(|receipt| receipt.head_sha.as_deref())
        .map(|receipt_head| receipt_head != head_sha)
        .unwrap_or(false)
}

fn receipt_stale_by_base(receipts: &[ReceiptRef], kind: &str, base_sha: &str) -> bool {
    receipts
        .iter()
        .find(|receipt| receipt.kind == kind)
        .and_then(|receipt| receipt.base_sha.as_deref())
        .map(|receipt_base| receipt_base != base_sha)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn load_snapshot(path: &Path) -> Result<QueueSnapshot> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read fixture snapshot {}", path.display()))?;
        serde_json::from_str::<QueueSnapshot>(&raw)
            .with_context(|| format!("failed to parse fixture snapshot {}", path.display()))
    }

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("queue-state")
            .join(name)
    }

    #[test]
    fn reviewed_plus_builder_fix_prioritizes_builder_fix() -> Result<()> {
        let snapshot = load_snapshot(&fixture_path("review-reviewed-needs-builder-fix.json"))?;
        let first = snapshot.prs.first().ok_or_else(|| eyre!("fixture missing PR entry"))?;
        let state = derive_state(first)?;
        assert_eq!(state.canonical_state, CanonicalState::NeedsBuilderFix);
        assert!(state.contradictions.iter().any(|entry| entry.contains("contradicts")));
        Ok(())
    }

    #[test]
    fn signoffs_plus_green_no_blockers_is_ci_green() -> Result<()> {
        let snapshot = load_snapshot(&fixture_path("all-signoffs-ci-green.json"))?;
        let first = snapshot.prs.first().ok_or_else(|| eyre!("fixture missing PR entry"))?;
        let state = derive_state(first)?;
        assert_eq!(state.canonical_state, CanonicalState::CiGreen);
        Ok(())
    }

    #[test]
    fn merge_ready_with_stale_base_receipt_is_not_merge_ready() -> Result<()> {
        let snapshot = load_snapshot(&fixture_path("merge-ready-stale-base-receipt.json"))?;
        let first = snapshot.prs.first().ok_or_else(|| eyre!("fixture missing PR entry"))?;
        let state = derive_state(first)?;
        assert_ne!(state.canonical_state, CanonicalState::MergeReady);
        assert!(state.stale_receipts.iter().any(|entry| entry == "merge-readiness"));
        Ok(())
    }
}
