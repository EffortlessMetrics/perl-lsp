use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueSnapshot {
    pub snapshot_id: String,
    pub captured_at: String,
    pub repository: String,
    pub default_branch: String,
    pub master_sha: String,
    #[serde(default)]
    pub ruleset_summary: serde_json::Value,
    pub prs: Vec<PullRequestSnapshot>,
    #[serde(default)]
    pub buckets: DerivedBuckets,
    #[serde(default)]
    pub leases: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestSnapshot {
    pub number: u64,
    pub title: String,
    pub head_sha: String,
    pub base_sha: String,
    pub is_draft: bool,
    pub merge_state_status: Option<String>,
    pub labels: Vec<String>,
    pub status_check_rollup: Vec<StatusCheck>,
    pub updated_at: String,
    pub author: String,
    pub review_decision: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusCheck {
    pub name: String,
    pub state: String,
    #[serde(default)]
    pub head_sha: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
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

pub fn run_snapshot(out: PathBuf, fixture: Option<PathBuf>) -> Result<()> {
    let snapshot = if let Some(fixture_path) = fixture {
        let fixture_text = fs::read_to_string(&fixture_path)
            .with_context(|| format!("failed to read fixture {}", fixture_path.display()))?;
        serde_json::from_str::<QueueSnapshot>(&fixture_text)
            .with_context(|| format!("failed to parse fixture {}", fixture_path.display()))?
    } else {
        snapshot_from_gh_cli()?
    };

    let mut with_buckets = snapshot;
    with_buckets.buckets = derive_buckets(&with_buckets.prs);

    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create output directory {}", parent.display()))?;
    }

    let payload = serde_json::to_string_pretty(&with_buckets)?;
    fs::write(&out, payload).with_context(|| format!("failed to write {}", out.display()))?;
    println!("wrote queue snapshot to {}", out.display());
    Ok(())
}

fn snapshot_from_gh_cli() -> Result<QueueSnapshot> {
    let root = project_root()?;

    // Fetch repository name (nameWithOwner).
    let repo_output = Command::new("gh")
        .current_dir(&root)
        .args(["repo", "view", "--json", "nameWithOwner", "--jq", ".nameWithOwner"])
        .output()
        .context("failed to execute gh repo view")?;
    let repository = if repo_output.status.success() {
        String::from_utf8_lossy(&repo_output.stdout).trim().to_string()
    } else {
        "unknown".to_string()
    };

    // Fetch current master SHA via git.
    let sha_output = Command::new("git")
        .current_dir(&root)
        .args(["rev-parse", "origin/master"])
        .output()
        .context("failed to execute git rev-parse")?;
    let master_sha = if sha_output.status.success() {
        String::from_utf8_lossy(&sha_output.stdout).trim().to_string()
    } else {
        "unknown".to_string()
    };

    let output = Command::new("gh")
        .current_dir(&root)
        .args([
            "pr",
            "list",
            "--state",
            "open",
            "--limit",
            "200",
            "--json",
            "number,title,isDraft,headRefOid,baseRefOid,mergeStateStatus,labels,statusCheckRollup,updatedAt,author,reviewDecision",
        ])
        .output()
        .context("failed to execute gh pr list")?;

    if !output.status.success() {
        bail!("gh pr list failed");
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    let prs_json: Vec<serde_json::Value> = serde_json::from_str(&raw)?;
    let prs = prs_json
        .into_iter()
        .map(|pr| PullRequestSnapshot {
            number: pr.get("number").and_then(serde_json::Value::as_u64).unwrap_or_default(),
            title: pr
                .get("title")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            head_sha: pr
                .get("headRefOid")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            base_sha: pr
                .get("baseRefOid")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            is_draft: pr.get("isDraft").and_then(serde_json::Value::as_bool).unwrap_or(false),
            merge_state_status: pr
                .get("mergeStateStatus")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string),
            labels: pr
                .get("labels")
                .and_then(serde_json::Value::as_array)
                .map(|labels| {
                    labels
                        .iter()
                        .filter_map(|label| {
                            label
                                .get("name")
                                .and_then(serde_json::Value::as_str)
                                .map(ToString::to_string)
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
            status_check_rollup: pr
                .get("statusCheckRollup")
                .and_then(serde_json::Value::as_array)
                .map(|checks| {
                    checks
                        .iter()
                        .map(|check| StatusCheck {
                            name: check
                                .get("name")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("unknown")
                                .to_string(),
                            state: check
                                .get("conclusion")
                                .or_else(|| check.get("state"))
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("UNKNOWN")
                                .to_string(),
                            head_sha: check
                                .get("commit")
                                .and_then(|commit| commit.get("oid"))
                                .and_then(serde_json::Value::as_str)
                                .map(ToString::to_string),
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
            updated_at: pr
                .get("updatedAt")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            author: pr
                .get("author")
                .and_then(|v| v.get("login"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            review_decision: pr
                .get("reviewDecision")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string),
        })
        .collect::<Vec<_>>();

    let now = chrono::Utc::now();
    let snapshot_id = format!("gh-snapshot-{}", now.to_rfc3339());
    Ok(QueueSnapshot {
        snapshot_id,
        captured_at: now.to_rfc3339(),
        repository,
        default_branch: "master".to_string(),
        master_sha,
        ruleset_summary: serde_json::json!({"source":"gh-cli"}),
        buckets: derive_buckets(&prs),
        prs,
        leases: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pr(number: u64, labels: Vec<&str>, checks: Vec<(&str, &str)>) -> PullRequestSnapshot {
        PullRequestSnapshot {
            number,
            title: format!("PR {number}"),
            head_sha: "abc".to_string(),
            base_sha: "def".to_string(),
            is_draft: false,
            merge_state_status: Some("CLEAN".to_string()),
            labels: labels.into_iter().map(ToString::to_string).collect(),
            status_check_rollup: checks
                .into_iter()
                .map(|(name, state)| StatusCheck {
                    name: name.to_string(),
                    state: state.to_string(),
                    head_sha: None,
                })
                .collect(),
            updated_at: "2026-04-26T00:00:00Z".to_string(),
            author: "bot".to_string(),
            review_decision: None,
        }
    }

    #[test]
    fn cancelled_check_routes_to_needs_ci_fix() {
        let prs = vec![make_pr(1, vec![], vec![("ci", "CANCELLED")])];
        let buckets = derive_buckets(&prs);
        assert!(buckets.needs_ci_fix.contains(&1), "CANCELLED must route to needs_ci_fix");
        assert!(!buckets.ci_green.contains(&1));
    }

    #[test]
    fn timed_out_check_routes_to_needs_ci_fix() {
        let prs = vec![make_pr(2, vec![], vec![("ci", "TIMED_OUT")])];
        let buckets = derive_buckets(&prs);
        assert!(buckets.needs_ci_fix.contains(&2), "TIMED_OUT must route to needs_ci_fix");
    }

    #[test]
    fn action_required_routes_to_needs_ci_fix() {
        let prs = vec![make_pr(3, vec![], vec![("ci", "ACTION_REQUIRED")])];
        let buckets = derive_buckets(&prs);
        assert!(buckets.needs_ci_fix.contains(&3), "ACTION_REQUIRED must route to needs_ci_fix");
    }

    #[test]
    fn success_routes_to_ci_green() {
        let prs = vec![make_pr(4, vec![], vec![("ci", "success")])];
        let buckets = derive_buckets(&prs);
        assert!(buckets.ci_green.contains(&4));
        assert!(!buckets.needs_ci_fix.contains(&4));
    }

    #[test]
    fn failure_routes_to_needs_ci_fix() {
        let prs = vec![make_pr(5, vec![], vec![("ci", "failure")])];
        let buckets = derive_buckets(&prs);
        assert!(buckets.needs_ci_fix.contains(&5));
    }

    #[test]
    fn docs_only_skip_is_expected_skip() {
        let required = HashSet::from(["ci / merge gate (pull_request)".to_string()]);
        let status = normalize_check_status(
            &StatusCheck { name: "ux-gate".to_string(), state: "SKIPPED".to_string(), head_sha: None },
            "abc",
            &required,
        );
        assert_eq!(status, NormalizedStatus::ExpectedSkip);
    }

    #[test]
    fn required_skip_is_unexpected_skip() {
        let required = HashSet::from(["ci / merge gate (pull_request)".to_string()]);
        let status = normalize_check_status(
            &StatusCheck {
                name: "ci / merge gate (pull_request)".to_string(),
                state: "SKIPPED".to_string(),
                head_sha: None,
            },
            "abc",
            &required,
        );
        assert_eq!(status, NormalizedStatus::UnexpectedSkip);
    }

    #[test]
    fn stale_status_when_check_sha_differs_from_pr_head() {
        let required = HashSet::new();
        let status = normalize_check_status(
            &StatusCheck {
                name: "ci / merge gate (pull_request)".to_string(),
                state: "SUCCESS".to_string(),
                head_sha: Some("oldsha".to_string()),
            },
            "newsha",
            &required,
        );
        assert_eq!(status, NormalizedStatus::Stale);
    }

    #[test]
    fn failed_and_pending_statuses_normalize() {
        let required = HashSet::new();
        let failed = normalize_check_status(
            &StatusCheck { name: "ci".to_string(), state: "FAILURE".to_string(), head_sha: None },
            "abc",
            &required,
        );
        let pending = normalize_check_status(
            &StatusCheck {
                name: "ci".to_string(),
                state: "IN_PROGRESS".to_string(),
                head_sha: None,
            },
            "abc",
            &required,
        );
        assert_eq!(failed, NormalizedStatus::Failed);
        assert_eq!(pending, NormalizedStatus::Pending);
    }
}

pub fn derive_buckets(prs: &[PullRequestSnapshot]) -> DerivedBuckets {
    let required_checks = load_required_checks().unwrap_or_default();
    let mut buckets = DerivedBuckets::default();
    for pr in prs {
        let normalized = normalize_rollup_statuses(pr, &required_checks);
        let has_failing = normalized.iter().any(|status| *status == NormalizedStatus::Failed);
        let has_pending = normalized.iter().any(|status| *status == NormalizedStatus::Pending);
        let has_stale = normalized.iter().any(|status| *status == NormalizedStatus::Stale);
        let has_unexpected_skip =
            normalized.iter().any(|status| *status == NormalizedStatus::UnexpectedSkip);
        let has_expected_skip =
            normalized.iter().any(|status| *status == NormalizedStatus::ExpectedSkip);
        let has_passed = normalized.iter().any(|status| *status == NormalizedStatus::Passed);
        let all_green = !normalized.is_empty()
            && !has_failing
            && !has_pending
            && !has_stale
            && !has_unexpected_skip
            && (has_passed || has_expected_skip);
        let labels = &pr.labels;

        if pr.is_draft {
            buckets.draft.push(pr.number);
        }
        if labels.iter().any(|label| label == "merge-ready") {
            buckets.merge_ready.push(pr.number);
        }
        if labels.iter().any(|label| label == "needs-builder-fix") {
            buckets.needs_builder_fix.push(pr.number);
        }
        if labels.iter().any(|label| label == "needs-diff-fix") {
            buckets.needs_diff_fix.push(pr.number);
        }
        if labels.iter().any(|label| label == "diff-audited") && all_green {
            buckets.diff_audited_waiting_ci.push(pr.number);
        }

        if has_failing || has_unexpected_skip || has_stale {
            buckets.needs_ci_fix.push(pr.number);
        } else if all_green {
            buckets.ci_green.push(pr.number);
        } else if has_pending {
            buckets.blocked_unknown.push(pr.number);
        } else if pr.merge_state_status.as_deref() == Some("DIRTY")
            || pr.merge_state_status.as_deref() == Some("UNKNOWN")
        {
            buckets.stale_or_dirty.push(pr.number);
        } else {
            buckets.blocked_unknown.push(pr.number);
        }
    }
    buckets
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizedStatus {
    Passed,
    Failed,
    Pending,
    ExpectedSkip,
    UnexpectedSkip,
    Stale,
}

fn normalize_rollup_statuses(
    pr: &PullRequestSnapshot,
    required_checks: &HashSet<String>,
) -> Vec<NormalizedStatus> {
    let mut normalized = Vec::with_capacity(pr.status_check_rollup.len());
    for check in &pr.status_check_rollup {
        normalized.push(normalize_check_status(check, &pr.head_sha, required_checks));
    }
    normalized
}

fn normalize_check_status(
    check: &StatusCheck,
    pr_head_sha: &str,
    required_checks: &HashSet<String>,
) -> NormalizedStatus {
    if check.head_sha.as_deref().is_some_and(|head| head != pr_head_sha) {
        return NormalizedStatus::Stale;
    }

    let state = check.state.to_ascii_uppercase();
    match state.as_str() {
        "SUCCESS" | "NEUTRAL" => NormalizedStatus::Passed,
        "FAILURE" | "CANCELLED" | "TIMED_OUT" | "ACTION_REQUIRED" | "ERROR" => {
            NormalizedStatus::Failed
        }
        "SKIPPED" => {
            if required_checks.contains(&check.name) {
                NormalizedStatus::UnexpectedSkip
            } else {
                NormalizedStatus::ExpectedSkip
            }
        }
        "IN_PROGRESS" | "QUEUED" | "WAITING" | "PENDING" => NormalizedStatus::Pending,
        _ => NormalizedStatus::Pending,
    }
}

fn load_required_checks() -> Result<HashSet<String>> {
    let root = project_root()?;
    let policy_path = root.join(".ci/policies/required-checks.toml");
    let raw = fs::read_to_string(&policy_path)
        .with_context(|| format!("failed to read {}", policy_path.display()))?;
    let value: toml::Value = toml::from_str(&raw)
        .with_context(|| format!("failed to parse {}", policy_path.display()))?;

    let mut checks = HashSet::new();
    if let Some(array) = value.get("checks").and_then(toml::Value::as_array) {
        for item in array {
            if let Some(name) = item.get("name").and_then(toml::Value::as_str) {
                checks.insert(name.to_string());
            }
        }
    }
    Ok(checks)
}
