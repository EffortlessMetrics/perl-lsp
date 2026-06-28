use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
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

    // Fetch current main SHA via git.
    let sha_output = Command::new("git")
        .current_dir(&root)
        .args(["rev-parse", "origin/main"])
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
}

pub fn derive_buckets(prs: &[PullRequestSnapshot]) -> DerivedBuckets {
    let mut buckets = DerivedBuckets::default();
    for pr in prs {
        let has_failing = pr.status_check_rollup.iter().any(|check| {
            let s = check.state.to_ascii_uppercase();
            s == "FAILURE" || s == "CANCELLED" || s == "TIMED_OUT" || s == "ACTION_REQUIRED"
        });
        let all_green = !pr.status_check_rollup.is_empty()
            && pr.status_check_rollup.iter().all(|check| {
                check.state.eq_ignore_ascii_case("success")
                    || check.state.eq_ignore_ascii_case("neutral")
                    || check.state.eq_ignore_ascii_case("skipped")
            });
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

        if has_failing {
            buckets.needs_ci_fix.push(pr.number);
        } else if all_green {
            buckets.ci_green.push(pr.number);
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
