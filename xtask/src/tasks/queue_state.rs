use crate::tasks::queue_snapshot::{PrSnapshot, QueueSnapshot, StatusRollup};
use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const STATE_CONFIG_PATH: &str = ".ci/state/pr-states.toml";

#[derive(Debug, Deserialize)]
struct StateConfig {
    states: BTreeMap<String, StateProjection>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct StateProjection {
    #[serde(default)]
    next_routes: Vec<String>,
    #[serde(default)]
    projected_labels: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct QueueStateReceipt {
    pub receipt_version: String,
    pub mode: String,
    pub evaluated_at: String,
    pub results: Vec<PrStateResult>,
}

#[derive(Debug, Serialize)]
pub struct PrStateResult {
    pub number: u64,
    pub canonical_state: String,
    pub blockers: Vec<String>,
    pub stale_receipts: Vec<String>,
    pub projected_next_routes: Vec<String>,
    pub projected_labels: Vec<String>,
    pub contradictions: Vec<String>,
}

pub fn run(snapshot: PathBuf, dry_run: bool, receipt: PathBuf) -> Result<()> {
    if !dry_run {
        bail!("queue state only supports --dry-run in this phase");
    }

    let snapshot = read_snapshot(&snapshot)?;
    let config = read_state_config(Path::new(STATE_CONFIG_PATH))?;

    let mut results = Vec::with_capacity(snapshot.prs.len());
    for pr in &snapshot.prs {
        results.push(build_state(pr, &config));
    }

    let output = QueueStateReceipt {
        receipt_version: "1.0.0".to_string(),
        mode: "dry-run".to_string(),
        evaluated_at: chrono::Utc::now().to_rfc3339(),
        results,
    };

    if let Some(parent) = receipt.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create receipt dir {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(&output).context("failed to serialize queue state")?;
    fs::write(&receipt, format!("{json}\n"))
        .with_context(|| format!("failed to write queue state receipt to {}", receipt.display()))?;
    println!("Wrote queue state receipt to {}", receipt.display());
    Ok(())
}

fn read_snapshot(path: &Path) -> Result<QueueSnapshot> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed reading queue snapshot {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("failed parsing queue snapshot {}", path.display()))
}

fn read_state_config(path: &Path) -> Result<StateConfig> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed reading state config {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("failed parsing state config {}", path.display()))
}

fn build_state(pr: &PrSnapshot, config: &StateConfig) -> PrStateResult {
    let label_set = pr.labels.iter().map(|s| s.as_str()).collect::<BTreeSet<_>>();
    let mut blockers = Vec::new();
    let mut contradictions = Vec::new();

    let blocker_state = detect_blocker_state(&label_set, &mut blockers);

    let stale_receipts = pr
        .receipts
        .iter()
        .filter_map(|receipt| {
            let Some(head_sha) = &receipt.head_sha else {
                return None;
            };
            if head_sha != &pr.head_sha { Some(receipt.kind.clone()) } else { None }
        })
        .collect::<Vec<_>>();

    if !stale_receipts.is_empty() {
        contradictions.push("one or more receipts are stale against current head_sha".to_string());
    }

    let has_reviewed = label_set.contains("review-reviewed");
    let has_merge_ready_label = label_set.contains("merge-ready");

    let merge_ready_receipt_valid = pr.receipts.iter().any(|receipt| {
        receipt.kind == "merge-readiness"
            && receipt.valid.unwrap_or(false)
            && receipt.base_sha.as_deref() == Some(pr.base_sha.as_str())
    });

    if has_merge_ready_label && !merge_ready_receipt_valid {
        contradictions
            .push("merge-ready label present without valid merge-readiness receipt".to_string());
    }

    let canonical_state = if pr.merged {
        "MERGED".to_string()
    } else if label_set.contains("superseded") {
        "SUPERSEDED".to_string()
    } else if pr.draft {
        "DRAFT".to_string()
    } else if let Some(blocker) = blocker_state {
        if pr.status_rollup == StatusRollup::Green {
            contradictions.push("blocker labels conflict with green CI rollup".to_string());
        }
        blocker
    } else {
        match pr.status_rollup {
            StatusRollup::Red => {
                let classifier = pr.receipts.iter().find(|receipt| receipt.kind == "ci-classifier");
                if let Some(classifier) = classifier {
                    if classifier.valid.unwrap_or(false) {
                        "NEEDS_CI_FIX".to_string()
                    } else {
                        "NEEDS_INFRA_FIX".to_string()
                    }
                } else {
                    "BLOCKED_UNKNOWN".to_string()
                }
            }
            StatusRollup::Pending => {
                if has_reviewed {
                    "REVIEWED_WAITING_CI".to_string()
                } else {
                    "NEW".to_string()
                }
            }
            StatusRollup::Green => {
                if has_merge_ready_label && merge_ready_receipt_valid {
                    "MERGE_READY".to_string()
                } else if has_reviewed {
                    "CI_GREEN".to_string()
                } else {
                    "NEW".to_string()
                }
            }
            StatusRollup::Unknown => "BLOCKED_UNKNOWN".to_string(),
        }
    };

    if !blockers.is_empty() && (canonical_state == "CI_GREEN" || canonical_state == "MERGE_READY") {
        contradictions
            .push("needs-* blockers cannot coexist with CI_GREEN/MERGE_READY".to_string());
    }

    let projection = config.states.get(&canonical_state).cloned().unwrap_or_default();

    PrStateResult {
        number: pr.number,
        canonical_state,
        blockers,
        stale_receipts,
        projected_next_routes: projection.next_routes,
        projected_labels: projection.projected_labels,
        contradictions,
    }
}

fn detect_blocker_state(labels: &BTreeSet<&str>, blockers: &mut Vec<String>) -> Option<String> {
    let ordered = [
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

    let mut matched = None;
    for (label, state) in ordered {
        if labels.contains(label) {
            blockers.push(label.to_string());
            if matched.is_none() {
                matched = Some(state.to_string());
            }
        }
    }

    matched
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(path: &str) -> Result<PrStateResult> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let raw = std::fs::read_to_string(root.join(path))?;
        let pr: PrSnapshot = serde_json::from_str(&raw)?;
        let config = read_state_config(&root.join("../").join(STATE_CONFIG_PATH))?;
        Ok(build_state(&pr, &config))
    }

    #[test]
    fn review_reviewed_and_needs_builder_fix_prefers_blocker_with_contradiction() -> Result<()> {
        let state = fixture("tests/fixtures/queue-state/review-reviewed-needs-builder-fix.json")?;
        assert_eq!(state.canonical_state, "NEEDS_BUILDER_FIX");
        assert!(state.blockers.iter().any(|b| b == "needs-builder-fix"));
        assert!(!state.contradictions.is_empty());
        Ok(())
    }

    #[test]
    fn all_signoffs_ci_green_no_blockers_is_ci_green() -> Result<()> {
        let state = fixture("tests/fixtures/queue-state/all-signoffs-ci-green.json")?;
        assert_eq!(state.canonical_state, "CI_GREEN");
        assert!(state.blockers.is_empty());
        Ok(())
    }

    #[test]
    fn merge_ready_label_with_stale_base_receipt_is_not_merge_ready() -> Result<()> {
        let state = fixture("tests/fixtures/queue-state/merge-ready-stale-base-receipt.json")?;
        assert_ne!(state.canonical_state, "MERGE_READY");
        assert!(state.contradictions.iter().any(|c| c.contains("merge-ready")));
        Ok(())
    }
}
