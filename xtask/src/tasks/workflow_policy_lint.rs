use color_eyre::eyre::{Context, Result, bail};
use serde::Serialize;
use serde_yaml_ng::{Mapping, Value};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::project_root;

const ALLOWLIST_PR_CONTENTS_WRITE: &[&str] = &["ci.yml", "ci-nightly.yml"];
const POLICY_WARN_UNPINNED_ACTIONS: bool = true;
const ALLOWLIST_BLANKET_CANCEL_IN_PROGRESS: &[&str] = &["docs-deploy.yml", "post-merge-status.yml"];

/// Workflow files that intentionally have no `policy/ci-lane-whitelist.toml`
/// entry. Add an entry here only when there's a documented reason — e.g. a
/// release/publish workflow that's release-time-only and not part of
/// per-PR economics.
const ALLOWLIST_WORKFLOW_LANE_MISSING: &[&str] = &[
    // Release / publish workflows: out of scope for the per-PR economics map.
    "brew-bump.yml",
    "chocolatey-bump.yml",
    "docker-publish.yml",
    "docs-deploy.yml",
    "post-merge-corpus-ratchet.yml",
    "post-merge-status.yml",
    "post-publish-smoke.yml",
    "publish-crates.yml",
    "publish-extension.yml",
    "publish-dry-run.yml",
    "release-orchestration.yml",
    "release.yml",
    "scoop-bump.yml",
    "tokmd.yml",
    "version-bump.yml",
    "vscode-published-extension-smoke.yml",
    "winget-bump.yml",
    // Schedule/utility workflows tracked separately from the lane economics.
    "ci-gate-self-tests.yml",
    "merge-ready-reconciler.yml",
    "triage-issues.yml",
    "workflow-trigger-lint.yml",
];

#[derive(Debug, Clone)]
pub struct WorkflowPolicyLintConfig {
    pub receipt: Option<PathBuf>,
    pub fixture: Option<PathBuf>,
    /// Run the per-workflow lane-whitelist check against
    /// `policy/ci-lane-whitelist.toml`. Advisory (warning-level) until the
    /// whitelist has stabilized.
    pub check_lane_whitelist: bool,
}

#[derive(Debug, Clone, Serialize)]
struct LintIssue {
    level: &'static str,
    code: &'static str,
    workflow: String,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
struct WorkflowPolicyReceipt {
    schema_version: &'static str,
    receipt_kind: &'static str,
    passed: bool,
    error_count: usize,
    warning_count: usize,
    issues: Vec<LintIssue>,
}

pub fn run(config: WorkflowPolicyLintConfig) -> Result<()> {
    let root = project_root()?;
    let mut issues = Vec::new();

    if let Some(fixture) = config.fixture {
        lint_workflow_file(&fixture, true, &mut issues)?;
    } else {
        let workflows_dir = root.join(".github").join("workflows");
        if workflows_dir.exists() {
            for entry in fs::read_dir(&workflows_dir)
                .with_context(|| format!("reading {}", workflows_dir.display()))?
            {
                let path = entry.context("reading workflow entry")?.path();
                let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
                    continue;
                };
                if ext != "yml" && ext != "yaml" {
                    continue;
                }
                lint_workflow_file(&path, false, &mut issues)?;
            }
        }

        if config.check_lane_whitelist {
            check_lane_whitelist(&root, &mut issues)?;
        }
    }

    issues.sort_by(|left, right| {
        (&left.level, &left.workflow, &left.code, &left.message).cmp(&(
            &right.level,
            &right.workflow,
            &right.code,
            &right.message,
        ))
    });

    let error_count = issues.iter().filter(|issue| issue.level == "error").count();
    let warning_count = issues.iter().filter(|issue| issue.level == "warning").count();
    let passed = error_count == 0;

    for issue in &issues {
        let prefix = if issue.level == "error" { "error" } else { "warning" };
        eprintln!("::{prefix}::{} [{}] {}", issue.workflow, issue.code, issue.message);
    }

    if let Some(receipt_path) = config.receipt {
        let receipt = WorkflowPolicyReceipt {
            schema_version: "1.0.0",
            receipt_kind: "workflow_policy_lint",
            passed,
            error_count,
            warning_count,
            issues,
        };
        if let Some(parent) = receipt_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating receipt directory {}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(&receipt).context("serializing receipt")?;
        fs::write(&receipt_path, format!("{json}\n"))
            .with_context(|| format!("writing receipt {}", receipt_path.display()))?;
        println!("Workflow policy lint receipt written: {}", receipt_path.display());
    }

    if !passed {
        bail!(
            "workflow policy lint failed with {} error(s) and {} warning(s)",
            error_count,
            warning_count
        );
    }

    println!(
        "Workflow policy lint passed ({} error(s), {} warning(s))",
        error_count, warning_count
    );
    Ok(())
}

fn lint_workflow_file(path: &Path, is_fixture: bool, issues: &mut Vec<LintIssue>) -> Result<()> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("reading workflow file {}", path.display()))?;
    let workflow: Value = serde_yaml_ng::from_str(&raw)
        .with_context(|| format!("parsing workflow YAML {}", path.display()))?;

    let workflow_name = if is_fixture {
        path.display().to_string()
    } else {
        path.file_name().and_then(|name| name.to_str()).unwrap_or("<unknown>").to_string()
    };

    let triggers = triggers(&workflow);

    if is_pull_request_target(&triggers) && checks_out_pr_head(&workflow) {
        issues.push(LintIssue {
            level: "error",
            code: "PR_TARGET_CHECKOUT_HEAD",
            workflow: workflow_name.clone(),
            message:
                "pull_request_target workflow checks out pull_request.head commit/ref (unsafe)"
                    .to_string(),
        });
    }

    if has_write_all_permissions(&workflow) {
        issues.push(LintIssue {
            level: "error",
            code: "WRITE_ALL_PERMISSIONS",
            workflow: workflow_name.clone(),
            message: "workflow declares permissions: write-all".to_string(),
        });
    }

    if is_pull_request(&triggers)
        && has_contents_write_permission(&workflow)
        && !is_contents_write_allowlisted(&workflow_name)
    {
        issues.push(LintIssue {
            level: "error",
            code: "PR_CONTENTS_WRITE",
            workflow: workflow_name.clone(),
            message: "pull_request workflow requests contents: write and is not in the allowlist"
                .to_string(),
        });
    }

    if is_untrusted_pr_secret_exposure(&triggers, &workflow) {
        issues.push(LintIssue {
            level: "error",
            code: "UNTRUSTED_PR_SECRETS",
            workflow: workflow_name.clone(),
            message: "untrusted PR code path appears to consume secrets.*".to_string(),
        });
    }

    if is_required_style(&workflow) {
        if !triggers.iter().any(|trigger| trigger == "merge_group") {
            issues.push(LintIssue {
                level: "error",
                code: "REQUIRED_STYLE_MISSING_MERGE_GROUP",
                workflow: workflow_name.clone(),
                message: "required-style workflow must include merge_group trigger".to_string(),
            });
        }

        if pull_request_has_paths_filter(&workflow) {
            issues.push(LintIssue {
                level: "error",
                code: "REQUIRED_STYLE_SELF_FILTERED",
                workflow: workflow_name.clone(),
                message: "required-style workflow must not path-filter itself".to_string(),
            });
        }
    }

    if blanket_cancel_in_progress(&workflow)
        && !ALLOWLIST_BLANKET_CANCEL_IN_PROGRESS.iter().any(|value| *value == workflow_name)
    {
        issues.push(LintIssue {
            level: "error",
            code: "BLANKET_CANCEL_IN_PROGRESS",
            workflow: workflow_name.clone(),
            message:
                "concurrency.cancel-in-progress must be false (or expression-gated) for master/merge_group truth runs"
                    .to_string(),
        });
    }

    if pull_request_has_label_triggers(&workflow)
        && cancel_in_progress_cancels_all_pr_events(&workflow)
    {
        issues.push(LintIssue {
            level: "error",
            code: "LABEL_EVENT_CANCELS_PR_RUN",
            workflow: workflow_name.clone(),
            message: "pull_request labeled/unlabeled workflows must not cancel in-progress PR runs; use github.event.action == 'synchronize' or remove label triggers".to_string(),
        });
    }

    if POLICY_WARN_UNPINNED_ACTIONS {
        for action in collect_unpinned_actions(&workflow) {
            issues.push(LintIssue {
                level: "warning",
                code: "UNPINNED_ACTION",
                workflow: workflow_name.clone(),
                message: format!("third-party action is not pinned to a commit SHA: {action}"),
            });
        }
    }

    Ok(())
}

fn is_contents_write_allowlisted(workflow_name: &str) -> bool {
    ALLOWLIST_PR_CONTENTS_WRITE.contains(&workflow_name)
}

fn triggers(workflow: &Value) -> Vec<String> {
    let Some(on) = workflow.get("on") else {
        return Vec::new();
    };
    match on {
        Value::String(single) => vec![single.clone()],
        Value::Sequence(values) => {
            values.iter().filter_map(Value::as_str).map(ToOwned::to_owned).collect()
        }
        Value::Mapping(values) => {
            values.keys().filter_map(Value::as_str).map(ToOwned::to_owned).collect()
        }
        _ => Vec::new(),
    }
}

fn is_pull_request(triggers: &[String]) -> bool {
    triggers.iter().any(|trigger| trigger == "pull_request")
}

fn is_pull_request_target(triggers: &[String]) -> bool {
    triggers.iter().any(|trigger| trigger == "pull_request_target")
}

fn is_required_style(workflow: &Value) -> bool {
    workflow
        .get("x-workflow-policy")
        .and_then(Value::as_mapping)
        .and_then(|mapping| mapping.get(Value::String("required-style".to_string())))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn pull_request_has_paths_filter(workflow: &Value) -> bool {
    workflow
        .get("on")
        .and_then(Value::as_mapping)
        .and_then(|mapping| mapping.get(Value::String("pull_request".to_string())))
        .and_then(Value::as_mapping)
        .is_some_and(|mapping| {
            mapping.contains_key(Value::String("paths".to_string()))
                || mapping.contains_key(Value::String("paths-ignore".to_string()))
        })
}

fn has_write_all_permissions(workflow: &Value) -> bool {
    workflow.get("permissions").and_then(Value::as_str).is_some_and(|value| value == "write-all")
}

fn has_contents_write_permission(workflow: &Value) -> bool {
    if workflow
        .get("permissions")
        .and_then(Value::as_mapping)
        .and_then(|mapping| mapping.get(Value::String("contents".to_string())))
        .and_then(Value::as_str)
        .is_some_and(|value| value == "write")
    {
        return true;
    }

    workflow.get("jobs").and_then(Value::as_mapping).is_some_and(|jobs| {
        jobs.values().any(|job| {
            job.as_mapping()
                .and_then(|mapping| mapping.get(Value::String("permissions".to_string())))
                .and_then(Value::as_mapping)
                .and_then(|mapping| mapping.get(Value::String("contents".to_string())))
                .and_then(Value::as_str)
                .is_some_and(|value| value == "write")
        })
    })
}

fn checks_out_pr_head(workflow: &Value) -> bool {
    workflow
        .get("jobs")
        .and_then(Value::as_mapping)
        .is_some_and(|jobs| jobs.values().any(job_checks_out_pr_head))
}

fn job_checks_out_pr_head(job: &Value) -> bool {
    let Some(steps) = job
        .as_mapping()
        .and_then(|mapping| mapping.get(Value::String("steps".to_string())))
        .and_then(Value::as_sequence)
    else {
        return false;
    };

    steps.iter().any(|step| {
        let Some(mapping) = step.as_mapping() else {
            return false;
        };
        let Some(uses) = mapping.get(Value::String("uses".to_string())).and_then(Value::as_str)
        else {
            return false;
        };
        if !uses.starts_with("actions/checkout") {
            return false;
        }
        let Some(with) = mapping.get(Value::String("with".to_string())).and_then(Value::as_mapping)
        else {
            return false;
        };

        with.values().filter_map(Value::as_str).any(|value| {
            value.contains("github.event.pull_request.head.sha")
                || value.contains("github.event.pull_request.head.ref")
        })
    })
}

fn is_untrusted_pr_secret_exposure(triggers: &[String], workflow: &Value) -> bool {
    let Some(jobs) = workflow.get("jobs").and_then(Value::as_mapping) else {
        return false;
    };

    // We only block proven dangerous shapes:
    // pull_request_target + checkout of PR head + secrets usage in the same job.
    if is_pull_request_target(triggers) {
        return jobs.values().any(|job| {
            let Some(job_map) = job.as_mapping() else {
                return false;
            };
            job_runs_untrusted_code(job_map) && map_contains_secrets_in_mapping(job_map)
        });
    }

    false
}

fn job_runs_untrusted_code(job_map: &Mapping) -> bool {
    if job_map.contains_key(Value::String("run".to_string())) {
        return true;
    }

    let Some(steps) = job_map.get(Value::String("steps".to_string())).and_then(Value::as_sequence)
    else {
        return false;
    };

    steps.iter().any(|step| {
        let Some(step_map) = step.as_mapping() else {
            return false;
        };
        step_map.contains_key(Value::String("run".to_string())) || step_uses_checkout(step_map)
    })
}

fn step_uses_checkout(step_map: &Mapping) -> bool {
    step_map
        .get(Value::String("uses".to_string()))
        .and_then(Value::as_str)
        .is_some_and(|uses| uses.starts_with("actions/checkout"))
}

fn map_contains_secrets_in_mapping(map: &Mapping) -> bool {
    map.iter().any(|(key, nested)| map_contains_secrets(key) || map_contains_secrets(nested))
}

fn map_contains_secrets(value: &Value) -> bool {
    match value {
        Value::String(text) => text.contains("secrets."),
        Value::Sequence(values) => values.iter().any(map_contains_secrets),
        Value::Mapping(map) => map
            .iter()
            .any(|(key, nested)| map_contains_secrets(key) || map_contains_secrets(nested)),
        _ => false,
    }
}

fn blanket_cancel_in_progress(workflow: &Value) -> bool {
    let trigger_names = triggers(workflow);
    let has_truth_runs = trigger_names.iter().any(|trigger| trigger == "merge_group")
        || workflow
            .get("on")
            .and_then(Value::as_mapping)
            .and_then(|mapping| mapping.get(Value::String("push".to_string())))
            .and_then(Value::as_mapping)
            .and_then(|push| push.get(Value::String("branches".to_string())))
            .and_then(Value::as_sequence)
            .is_some_and(|branches| {
                branches
                    .iter()
                    .filter_map(Value::as_str)
                    .any(|branch| branch == "main" || branch == "master")
            });

    if !has_truth_runs {
        return false;
    }

    let Some(concurrency) = workflow.get("concurrency") else {
        return false;
    };

    if let Some(boolean) = concurrency.as_bool() {
        return boolean;
    }

    let Some(map) = concurrency.as_mapping() else {
        return false;
    };

    map.get(Value::String("cancel-in-progress".to_string()))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn pull_request_has_label_triggers(workflow: &Value) -> bool {
    workflow
        .get("on")
        .and_then(Value::as_mapping)
        .and_then(|mapping| mapping.get(Value::String("pull_request".to_string())))
        .and_then(Value::as_mapping)
        .and_then(|pull_request| pull_request.get(Value::String("types".to_string())))
        .and_then(Value::as_sequence)
        .is_some_and(|types| {
            types
                .iter()
                .filter_map(Value::as_str)
                .any(|event| event == "labeled" || event == "unlabeled")
        })
}

fn cancel_in_progress_cancels_all_pr_events(workflow: &Value) -> bool {
    let Some(concurrency) = workflow.get("concurrency") else {
        return false;
    };

    if let Some(enabled) = concurrency.as_bool() {
        return enabled;
    }

    let Some(map) = concurrency.as_mapping() else {
        return false;
    };

    let Some(cancel) = map.get(Value::String("cancel-in-progress".to_string())) else {
        return false;
    };

    if let Some(enabled) = cancel.as_bool() {
        return enabled;
    }

    cancel.as_str().is_some_and(|expr| expr.trim() == "${{ github.event_name == 'pull_request' }}")
}

fn collect_unpinned_actions(workflow: &Value) -> Vec<String> {
    let Some(jobs) = workflow.get("jobs").and_then(Value::as_mapping) else {
        return Vec::new();
    };

    let mut actions = Vec::new();
    for job in jobs.values() {
        let Some(steps) = job
            .as_mapping()
            .and_then(|mapping| mapping.get(Value::String("steps".to_string())))
            .and_then(Value::as_sequence)
        else {
            continue;
        };

        for step in steps {
            let Some(uses) = step
                .as_mapping()
                .and_then(|mapping| mapping.get(Value::String("uses".to_string())))
                .and_then(Value::as_str)
            else {
                continue;
            };
            if uses.starts_with("./") || uses.starts_with("docker://") {
                continue;
            }
            if uses.starts_with("actions/") || uses.starts_with("github/") {
                continue;
            }
            if !is_sha_pinned(uses) {
                actions.push(uses.to_string());
            }
        }
    }
    actions.sort();
    actions.dedup();
    actions
}

fn is_sha_pinned(uses: &str) -> bool {
    let Some((_, reference)) = uses.rsplit_once('@') else {
        return false;
    };
    reference.len() == 40 && reference.chars().all(|ch| ch.is_ascii_hexdigit())
}

/// Validate that every workflow under `.github/workflows/` is referenced by at
/// least one `[[lane]]` entry in `policy/ci-lane-whitelist.toml`, OR is in the
/// `ALLOWLIST_WORKFLOW_LANE_MISSING` allowlist (release/utility workflows).
///
/// Issues are emitted at warning level — advisory until the whitelist has
/// stabilized. PR 11 introduces this as advisory; promotion to error level
/// happens only after a calibration window.
fn check_lane_whitelist(root: &Path, issues: &mut Vec<LintIssue>) -> Result<()> {
    let whitelist_path = root.join("policy").join("ci-lane-whitelist.toml");
    if !whitelist_path.exists() {
        // Whitelist not present in this repo; silently skip rather than failing.
        return Ok(());
    }

    let whitelist_text = fs::read_to_string(&whitelist_path)
        .with_context(|| format!("reading {}", whitelist_path.display()))?;
    let whitelist: toml::Value = toml::from_str(&whitelist_text)
        .with_context(|| format!("parsing {}", whitelist_path.display()))?;

    // Collect workflow paths referenced by whitelist lanes.
    let mut whitelisted_workflows: HashSet<String> = HashSet::new();
    if let Some(lanes) = whitelist.get("lane").and_then(|v| v.as_array()) {
        for lane in lanes {
            if let Some(workflow) = lane.get("workflow").and_then(|v| v.as_str()) {
                whitelisted_workflows.insert(workflow.to_string());
            }
        }
    }

    let workflows_dir = root.join(".github").join("workflows");
    if !workflows_dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(&workflows_dir)
        .with_context(|| format!("reading {}", workflows_dir.display()))?
    {
        let path = entry.context("reading workflow entry")?.path();
        let Some(ext) = path.extension().and_then(|v| v.to_str()) else {
            continue;
        };
        if ext != "yml" && ext != "yaml" {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if ALLOWLIST_WORKFLOW_LANE_MISSING.contains(&file_name) {
            continue;
        }
        let workflow_ref = format!(".github/workflows/{file_name}");
        if !whitelisted_workflows.contains(&workflow_ref) {
            issues.push(LintIssue {
                level: "warning",
                code: "LANE_WHITELIST_MISSING",
                workflow: file_name.to_string(),
                message: format!(
                    "workflow has no `[[lane]]` entry in policy/ci-lane-whitelist.toml \
                     (and is not in ALLOWLIST_WORKFLOW_LANE_MISSING). Add an entry or \
                     allowlist with reason."
                ),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_path(name: &str) -> Result<PathBuf> {
        let root = project_root()?;
        Ok(root.join("xtask/tests/fixtures/workflow-policy").join(name))
    }

    #[test]
    fn fixture_pr_target_checkout_head_fails() -> Result<()> {
        let path = fixture_path("pull_request_target_checkout_head.yml")?;
        let mut issues = Vec::new();
        lint_workflow_file(&path, true, &mut issues)?;
        assert!(issues.iter().any(|issue| issue.code == "PR_TARGET_CHECKOUT_HEAD"));
        Ok(())
    }

    #[test]
    fn fixture_pull_request_read_only_passes() -> Result<()> {
        let path = fixture_path("pull_request_read_only.yml")?;
        let mut issues = Vec::new();
        lint_workflow_file(&path, true, &mut issues)?;
        assert!(issues.iter().all(|issue| issue.level != "error"));
        Ok(())
    }

    #[test]
    fn fixture_label_event_cancel_expression_fails() -> Result<()> {
        let path = fixture_path("label_event_cancel_expression.yml")?;
        let mut issues = Vec::new();
        lint_workflow_file(&path, true, &mut issues)?;
        assert!(issues.iter().any(|issue| issue.code == "LABEL_EVENT_CANCELS_PR_RUN"));
        Ok(())
    }

    #[test]
    fn fixture_label_event_synchronize_cancel_passes() -> Result<()> {
        let path = fixture_path("label_event_synchronize_cancel.yml")?;
        let mut issues = Vec::new();
        lint_workflow_file(&path, true, &mut issues)?;
        assert!(issues.iter().all(|issue| issue.code != "LABEL_EVENT_CANCELS_PR_RUN"));
        Ok(())
    }

    #[test]
    fn fixture_write_all_fails() -> Result<()> {
        let path = fixture_path("write_all_permissions.yml")?;
        let mut issues = Vec::new();
        lint_workflow_file(&path, true, &mut issues)?;
        assert!(issues.iter().any(|issue| issue.code == "WRITE_ALL_PERMISSIONS"));
        Ok(())
    }
}
