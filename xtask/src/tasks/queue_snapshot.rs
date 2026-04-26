use chrono::{DateTime, Utc};
use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueSnapshot {
    pub snapshot_id: String,
    pub captured_at: DateTime<Utc>,
    pub repository: String,
    pub default_branch: String,
    pub master_sha: String,
    pub ruleset_summary: String,
    pub prs: Vec<PrSnapshot>,
    pub buckets: DerivedBuckets,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrSnapshot {
    pub number: u64,
    pub title: String,
    pub head_sha: String,
    pub base_sha: String,
    pub is_draft: bool,
    pub merge_state_status: String,
    pub labels: Vec<String>,
    pub status_check_rollup: Vec<CheckRollup>,
    pub updated_at: DateTime<Utc>,
    pub author: String,
    #[serde(default)]
    pub review_decision: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckRollup {
    pub name: String,
    pub conclusion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DerivedBuckets {
    pub merge_ready: Vec<u64>,
    pub ci_green: Vec<u64>,
    pub needs_ci_fix: Vec<u64>,
    pub needs_builder_fix: Vec<u64>,
    pub needs_diff_fix: Vec<u64>,
    pub diff_audited_waiting_ci: Vec<u64>,
    pub stale_or_dirty: Vec<u64>,
    pub draft: Vec<u64>,
    pub blocked_unknown: Vec<u64>,
}

pub fn snapshot(fixture: Option<&Path>, out: &Path, now: DateTime<Utc>) -> Result<()> {
    let mut snapshot: QueueSnapshot = if let Some(fixture_path) = fixture {
        read_json(fixture_path)?
    } else {
        QueueSnapshot {
            snapshot_id: format!("gh-snapshot-{}", now.to_rfc3339()),
            captured_at: now,
            repository: "EffortlessMetrics/perl-lsp".to_string(),
            default_branch: "master".to_string(),
            master_sha: "unknown".to_string(),
            ruleset_summary: "convention-only".to_string(),
            prs: Vec::new(),
            buckets: DerivedBuckets::default(),
        }
    };

    snapshot.buckets = derive_buckets(&snapshot.prs);
    write_json(out, &snapshot)
}

pub fn derive_buckets(prs: &[PrSnapshot]) -> DerivedBuckets {
    let mut buckets = DerivedBuckets::default();

    for pr in prs {
        let has_failure = pr.status_check_rollup.iter().any(|check| check.conclusion == "FAILURE");
        let all_success = !pr.status_check_rollup.is_empty()
            && pr.status_check_rollup.iter().all(|check| check.conclusion == "SUCCESS");

        if pr.is_draft {
            buckets.draft.push(pr.number);
            continue;
        }

        if all_success {
            buckets.ci_green.push(pr.number);
        } else if has_failure {
            buckets.needs_ci_fix.push(pr.number);
        }

        if pr.labels.iter().any(|label| label == "needs-builder-fix") {
            buckets.needs_builder_fix.push(pr.number);
        }
        if pr.labels.iter().any(|label| label == "needs-diff-fix") {
            buckets.needs_diff_fix.push(pr.number);
        }

        if pr.merge_state_status == "DIRTY" || pr.merge_state_status == "UNKNOWN" {
            buckets.stale_or_dirty.push(pr.number);
        }

        if pr.labels.iter().any(|label| label == "diff-audited") && all_success {
            buckets.merge_ready.push(pr.number);
        } else if pr.labels.iter().any(|label| label == "diff-audited") {
            buckets.diff_audited_waiting_ci.push(pr.number);
        }

        if !all_success
            && !has_failure
            && !pr.is_draft
            && pr.merge_state_status != "DIRTY"
            && pr.merge_state_status != "UNKNOWN"
        {
            buckets.blocked_unknown.push(pr.number);
        }
    }

    buckets
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read JSON file: {}", path.display()))?;
    let normalized = text.replace("\r\n", "\n");
    serde_json::from_str(&normalized)
        .with_context(|| format!("failed to parse JSON file: {}", path.display()))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create parent dir for {}", path.display()))?;
    }
    let payload = serde_json::to_string_pretty(value)?;
    fs::write(path, payload)
        .with_context(|| format!("failed to write JSON file: {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::Result;

    #[test]
    fn queue_snapshot_fixture_derives_buckets() -> Result<()> {
        let input = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/queue-snapshot/open-prs.fixture.json");
        let snapshot: QueueSnapshot = read_json(&input)?;
        let buckets = derive_buckets(&snapshot.prs);
        assert_eq!(buckets.merge_ready, vec![6854]);
        assert_eq!(buckets.needs_ci_fix, vec![6855]);
        Ok(())
    }
}
