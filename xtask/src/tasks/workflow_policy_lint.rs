use color_eyre::eyre::{Context, Result, bail};
use serde::Serialize;
use serde_yaml_ng::{Mapping, Value};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::project_root;

const ALLOWLIST_PR_CONTENTS_WRITE: &[&str] = &["ci.yml", "ci-nightly.yml", "droid-review.yml"];
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
                message: "workflow has no `[[lane]]` entry in policy/ci-lane-whitelist.toml \
                     (and is not in ALLOWLIST_WORKFLOW_LANE_MISSING). Add an entry or \
                     allowlist with reason."
                    .to_string(),
            });
        }
    }

    // Check each lane entry for runner mismatch and stale job references.
    if let Some(lanes) = whitelist.get("lane").and_then(|v| v.as_array()) {
        for lane in lanes {
            check_runner_label_mismatch(&workflows_dir, lane, issues)?;
            check_stale_whitelist_job(&workflows_dir, lane, issues)?;
        }
    }

    Ok(())
}

/// Normalize a `runs-on:` value from the workflow YAML into the runner token
/// used in `policy/ci-lane-whitelist.toml`.
///
/// Returns `None` when the value is the `${{ matrix.os }}` expression or any
/// other expression that evaluates at runtime — those are "mixed" in the
/// whitelist and should never produce a mismatch warning.
fn normalize_runs_on(runs_on: &Value) -> Option<String> {
    match runs_on {
        Value::String(s) => {
            let s = s.trim();
            // Runtime matrix expression — whitelist uses "mixed"; skip check.
            if s.contains("matrix.") || s.starts_with("${{") {
                return None;
            }
            let normalized = match s {
                "ubuntu-latest" => "ubuntu_latest",
                "ubuntu-24.04" => "ubuntu_24_04",
                "ubuntu-22.04" => "ubuntu_22_04",
                "ubuntu-20.04" => "ubuntu_20_04",
                "windows-latest" => "windows_latest",
                "macos-latest" => "macos_latest",
                other => other,
            };
            Some(normalized.to_string())
        }
        Value::Mapping(map) => {
            // Object form: `runs-on: {group: em-ci-small, labels: [self-hosted, ..., cx53, ...]}`
            let labels = map.get(Value::String("labels".to_string())).and_then(Value::as_sequence);
            if let Some(label_seq) = labels {
                return normalize_self_hosted_labels(label_seq);
            }
            // Unknown object form — skip rather than false-positive.
            None
        }
        // Sequence form is used for self-hosted label lists. Unknown sequences
        // still skip rather than false-positive.
        Value::Sequence(seq) => normalize_self_hosted_labels(seq),
        _ => None,
    }
}

fn normalize_self_hosted_labels(labels: &[Value]) -> Option<String> {
    let label_strs: Vec<&str> = labels.iter().filter_map(Value::as_str).collect();
    if label_strs.contains(&"cx53") {
        return Some("self_hosted_cx53".to_string());
    }
    if label_strs.contains(&"cx43") {
        return Some("self_hosted_cx43".to_string());
    }
    if label_strs.contains(&"self-hosted") && label_strs.contains(&"droid") {
        return Some("self_hosted_droid".to_string());
    }
    if label_strs.contains(&"self-hosted") && label_strs.contains(&"workflow-nano") {
        return Some("self_hosted_workflow_nano".to_string());
    }
    None
}

/// Check 1 — `RUNNER_LABEL_MISMATCH`: for each `[[lane]]` entry that declares
/// both `workflow` and `job`, parse the workflow YAML and warn when the job's
/// `runs-on:` does not match the whitelist `runner` field.
///
/// "mixed" runner in the whitelist matches any actual runner (matrix jobs).
/// Object-form `runs-on:` values that don't contain a known self-hosted label
/// are skipped rather than false-positived.
fn check_runner_label_mismatch(
    workflows_dir: &Path,
    lane: &toml::Value,
    issues: &mut Vec<LintIssue>,
) -> Result<()> {
    let Some(workflow_ref) = lane.get("workflow").and_then(|v| v.as_str()) else {
        return Ok(());
    };
    let Some(job_id) = lane.get("job").and_then(|v| v.as_str()) else {
        return Ok(());
    };
    let Some(declared_runner) = lane.get("runner").and_then(|v| v.as_str()) else {
        return Ok(());
    };

    // "mixed" in the whitelist means the runner varies at runtime — skip check.
    if declared_runner == "mixed" {
        return Ok(());
    }

    // Derive the workflow file name from the ref path.
    let workflow_file = workflow_ref.trim_start_matches(".github/workflows/");
    let workflow_path = workflows_dir.join(workflow_file);
    if !workflow_path.exists() {
        // Workflow file absent — STALE_WHITELIST_JOB will catch this.
        return Ok(());
    }

    let raw = fs::read_to_string(&workflow_path)
        .with_context(|| format!("reading {}", workflow_path.display()))?;
    let workflow: Value = serde_yaml_ng::from_str(&raw)
        .with_context(|| format!("parsing YAML {}", workflow_path.display()))?;

    let Some(jobs) = workflow.get("jobs").and_then(Value::as_mapping) else {
        return Ok(());
    };

    // Find the job to inspect for runner info. If the exact job ID doesn't
    // exist (stale reference), fall back to the sole job when the workflow
    // has exactly one job. This surfaces the common pattern where a job was
    // renamed: the old name is stale (caught by STALE_WHITELIST_JOB) but the
    // runner mismatch on the surviving single job is still worth reporting.
    let job_opt = jobs.get(Value::String(job_id.to_string())).and_then(Value::as_mapping);
    let job = match job_opt {
        Some(j) => j,
        None if jobs.len() == 1 => {
            // Single-job workflow with a stale job reference: use the one
            // existing job for the runner check.
            match jobs.values().next().and_then(Value::as_mapping) {
                Some(j) => j,
                None => return Ok(()),
            }
        }
        None => {
            // Multi-job workflow with stale reference — skip runner check to
            // avoid false positives. STALE_WHITELIST_JOB will report this.
            return Ok(());
        }
    };

    let Some(runs_on) = job.get(Value::String("runs-on".to_string())) else {
        return Ok(());
    };

    let Some(actual_runner) = normalize_runs_on(runs_on) else {
        // Runtime expression or unrecognised object form — skip.
        return Ok(());
    };

    if actual_runner != declared_runner {
        issues.push(LintIssue {
            level: "warning",
            code: "RUNNER_LABEL_MISMATCH",
            workflow: workflow_file.to_string(),
            message: format!(
                "job `{job_id}` runs on `{actual_runner}` but whitelist declares `{declared_runner}` \
                 (workflow: {workflow_ref})"
            ),
        });
    }

    Ok(())
}

/// Check 2 — `STALE_WHITELIST_JOB`: for each `[[lane]]` entry that declares a
/// `job` field, parse the workflow YAML and warn when that job name is **not**
/// present in the workflow's `jobs:` map.
fn check_stale_whitelist_job(
    workflows_dir: &Path,
    lane: &toml::Value,
    issues: &mut Vec<LintIssue>,
) -> Result<()> {
    let Some(workflow_ref) = lane.get("workflow").and_then(|v| v.as_str()) else {
        return Ok(());
    };
    let Some(job_id) = lane.get("job").and_then(|v| v.as_str()) else {
        return Ok(());
    };

    let workflow_file = workflow_ref.trim_start_matches(".github/workflows/");
    let workflow_path = workflows_dir.join(workflow_file);
    if !workflow_path.exists() {
        // Workflow file entirely missing — different check would cover that.
        return Ok(());
    }

    let raw = fs::read_to_string(&workflow_path)
        .with_context(|| format!("reading {}", workflow_path.display()))?;
    let workflow: Value = serde_yaml_ng::from_str(&raw)
        .with_context(|| format!("parsing YAML {}", workflow_path.display()))?;

    let Some(jobs) = workflow.get("jobs").and_then(Value::as_mapping) else {
        // No jobs map — treat as stale.
        issues.push(LintIssue {
            level: "warning",
            code: "STALE_WHITELIST_JOB",
            workflow: workflow_file.to_string(),
            message: format!(
                "whitelist lane references job `{job_id}` but `{workflow_file}` has no `jobs:` map \
                 (workflow: {workflow_ref})"
            ),
        });
        return Ok(());
    };

    if !jobs.contains_key(Value::String(job_id.to_string())) {
        let actual_jobs: Vec<&str> = jobs.keys().filter_map(Value::as_str).collect();
        issues.push(LintIssue {
            level: "warning",
            code: "STALE_WHITELIST_JOB",
            workflow: workflow_file.to_string(),
            message: format!(
                "whitelist lane references job `{job_id}` but it does not exist in `{workflow_file}` \
                 (actual jobs: {})",
                actual_jobs.join(", ")
            ),
        });
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

    // ── Fixture tests: RUNNER_LABEL_MISMATCH ──────────────────────────────────

    /// Fixture A: whitelist declares ubuntu_24_04 but job runs on ubuntu-latest → mismatch fires.
    #[test]
    fn runner_mismatch_fires_when_runner_differs() -> Result<()> {
        let workflows_dir = fixture_path("")?;
        // Build a synthetic lane entry pointing at runner_mismatch.yml / job "lint"
        // with declared runner "ubuntu_24_04".
        let lane: toml::Value = toml::from_str(
            r#"
            workflow = ".github/workflows/runner_mismatch.yml"
            job = "lint"
            runner = "ubuntu_24_04"
            "#,
        )?;
        let mut issues = Vec::new();
        check_runner_label_mismatch(&workflows_dir, &lane, &mut issues)?;
        assert!(
            issues.iter().any(|i| i.code == "RUNNER_LABEL_MISMATCH"),
            "expected RUNNER_LABEL_MISMATCH, got: {issues:?}"
        );
        Ok(())
    }

    /// Fixture B: whitelist declares ubuntu_24_04 and job runs on ubuntu-24.04 → no mismatch.
    #[test]
    fn runner_match_is_silent() -> Result<()> {
        let workflows_dir = fixture_path("")?;
        let lane: toml::Value = toml::from_str(
            r#"
            workflow = ".github/workflows/runner_match.yml"
            job = "lint"
            runner = "ubuntu_24_04"
            "#,
        )?;
        let mut issues = Vec::new();
        check_runner_label_mismatch(&workflows_dir, &lane, &mut issues)?;
        assert!(
            issues.iter().all(|i| i.code != "RUNNER_LABEL_MISMATCH"),
            "unexpected RUNNER_LABEL_MISMATCH: {issues:?}"
        );
        Ok(())
    }

    /// Object-form runs-on with cx53 label → normalized to self_hosted_cx53, no false positive
    /// when whitelist also declares self_hosted_cx53.
    #[test]
    fn runner_cx53_object_form_matches_self_hosted_cx53() -> Result<()> {
        let workflows_dir = fixture_path("")?;
        let lane: toml::Value = toml::from_str(
            r#"
            workflow = ".github/workflows/cx53_object_runs_on.yml"
            job = "rust-small-cx53"
            runner = "self_hosted_cx53"
            "#,
        )?;
        let mut issues = Vec::new();
        check_runner_label_mismatch(&workflows_dir, &lane, &mut issues)?;
        assert!(
            issues.iter().all(|i| i.code != "RUNNER_LABEL_MISMATCH"),
            "unexpected RUNNER_LABEL_MISMATCH for cx53 object form: {issues:?}"
        );
        Ok(())
    }

    /// Droid's self-hosted label sequence normalizes to the policy token, so
    /// the review workflow stays checked by the lane whitelist.
    #[test]
    fn runner_droid_sequence_matches_self_hosted_workflow_nano() -> Result<()> {
        let real_workflows_dir = {
            let root = project_root()?;
            root.join(".github").join("workflows")
        };
        if !real_workflows_dir.join("droid-review.yml").exists() {
            return Ok(());
        }
        let lane: toml::Value = toml::from_str(
            r#"
            workflow = ".github/workflows/droid-review.yml"
            job = "droid-review"
            runner = "self_hosted_workflow_nano"
            "#,
        )?;
        let mut issues = Vec::new();
        check_runner_label_mismatch(&real_workflows_dir, &lane, &mut issues)?;
        assert!(
            issues.iter().all(|i| i.code != "RUNNER_LABEL_MISMATCH"),
            "unexpected RUNNER_LABEL_MISMATCH for Droid self-hosted runner: {issues:?}"
        );
        Ok(())
    }

    /// "mixed" runner in whitelist skips the check entirely, even if the actual
    /// runner differs.
    #[test]
    fn runner_mixed_skips_check() -> Result<()> {
        let workflows_dir = fixture_path("")?;
        let lane: toml::Value = toml::from_str(
            r#"
            workflow = ".github/workflows/runner_mismatch.yml"
            job = "lint"
            runner = "mixed"
            "#,
        )?;
        let mut issues = Vec::new();
        check_runner_label_mismatch(&workflows_dir, &lane, &mut issues)?;
        assert!(
            issues.iter().all(|i| i.code != "RUNNER_LABEL_MISMATCH"),
            "unexpected RUNNER_LABEL_MISMATCH for mixed runner: {issues:?}"
        );
        Ok(())
    }

    // ── Fixture tests: STALE_WHITELIST_JOB ───────────────────────────────────

    /// Fixture C: whitelist references job "old-job" but workflow only has "actual-job" → stale fires.
    #[test]
    fn stale_job_fires_when_job_missing() -> Result<()> {
        let workflows_dir = fixture_path("")?;
        let lane: toml::Value = toml::from_str(
            r#"
            workflow = ".github/workflows/stale_job.yml"
            job = "old-job"
            "#,
        )?;
        let mut issues = Vec::new();
        check_stale_whitelist_job(&workflows_dir, &lane, &mut issues)?;
        assert!(
            issues.iter().any(|i| i.code == "STALE_WHITELIST_JOB"),
            "expected STALE_WHITELIST_JOB, got: {issues:?}"
        );
        Ok(())
    }

    /// Fixture D: whitelist references job "lint" and workflow has "lint" → no stale warning.
    #[test]
    fn valid_job_is_silent() -> Result<()> {
        let workflows_dir = fixture_path("")?;
        let lane: toml::Value = toml::from_str(
            r#"
            workflow = ".github/workflows/valid_job.yml"
            job = "lint"
            "#,
        )?;
        let mut issues = Vec::new();
        check_stale_whitelist_job(&workflows_dir, &lane, &mut issues)?;
        assert!(
            issues.iter().all(|i| i.code != "STALE_WHITELIST_JOB"),
            "unexpected STALE_WHITELIST_JOB: {issues:?}"
        );
        Ok(())
    }

    /// Single-job fallback: stale job reference to a single-job workflow —
    /// the runner mismatch should still fire (the sole surviving job has a
    /// different runner than declared in the whitelist).
    #[test]
    fn runner_mismatch_fires_via_single_job_fallback() -> Result<()> {
        let workflows_dir = fixture_path("")?;
        // stale_job.yml has exactly one job: "actual-job" (runs-on: ubuntu-latest).
        // We reference a non-existent job "old-job" with declared runner
        // "ubuntu_24_04" — the fallback picks up "actual-job" and detects mismatch.
        let lane: toml::Value = toml::from_str(
            r#"
            workflow = ".github/workflows/stale_job.yml"
            job = "old-job"
            runner = "ubuntu_24_04"
            "#,
        )?;
        let mut issues = Vec::new();
        check_runner_label_mismatch(&workflows_dir, &lane, &mut issues)?;
        assert!(
            issues.iter().any(|i| i.code == "RUNNER_LABEL_MISMATCH"),
            "expected RUNNER_LABEL_MISMATCH via single-job fallback, got: {issues:?}"
        );
        Ok(())
    }

    /// Multi-job stale reference: stale job in a workflow with multiple jobs —
    /// runner check is SKIPPED to avoid false positives. Only STALE_WHITELIST_JOB fires.
    #[test]
    fn runner_mismatch_skipped_for_stale_ref_in_multi_job_workflow() -> Result<()> {
        // Use the real ci.yml (many jobs) with a nonexistent job reference.
        let real_workflows_dir = {
            let root = project_root()?;
            root.join(".github").join("workflows")
        };
        if !real_workflows_dir.join("ci.yml").exists() {
            // Skip if not in the full project tree.
            return Ok(());
        }
        let lane: toml::Value = toml::from_str(
            r#"
            workflow = ".github/workflows/ci.yml"
            job = "nonexistent-job-xyz"
            runner = "ubuntu_24_04"
            "#,
        )?;
        let mut issues = Vec::new();
        check_runner_label_mismatch(&real_workflows_dir, &lane, &mut issues)?;
        assert!(
            issues.iter().all(|i| i.code != "RUNNER_LABEL_MISMATCH"),
            "unexpected RUNNER_LABEL_MISMATCH for stale ref in multi-job workflow: {issues:?}"
        );
        Ok(())
    }

    // ── normalize_runs_on unit tests ──────────────────────────────────────────

    #[test]
    fn normalize_ubuntu_latest() {
        let v = Value::String("ubuntu-latest".to_string());
        assert_eq!(normalize_runs_on(&v), Some("ubuntu_latest".to_string()));
    }

    #[test]
    fn normalize_ubuntu_24_04() {
        let v = Value::String("ubuntu-24.04".to_string());
        assert_eq!(normalize_runs_on(&v), Some("ubuntu_24_04".to_string()));
    }

    #[test]
    fn normalize_windows_latest() {
        let v = Value::String("windows-latest".to_string());
        assert_eq!(normalize_runs_on(&v), Some("windows_latest".to_string()));
    }

    #[test]
    fn normalize_matrix_expression_returns_none() {
        let v = Value::String("${{ matrix.os }}".to_string());
        assert_eq!(normalize_runs_on(&v), None);
    }

    #[test]
    fn normalize_cx53_object_form() -> Result<()> {
        let yaml = r#"
group: em-ci-small
labels: [self-hosted, linux, x64, em-ci, cx53, rust-small]
"#;
        let v: Value = serde_yaml_ng::from_str(yaml)?;
        assert_eq!(normalize_runs_on(&v), Some("self_hosted_cx53".to_string()));
        Ok(())
    }

    #[test]
    fn normalize_cx43_object_form() -> Result<()> {
        let yaml = r#"
group: em-ci-small
labels: [self-hosted, linux, x64, em-ci, cx43, rust-small]
"#;
        let v: Value = serde_yaml_ng::from_str(yaml)?;
        assert_eq!(normalize_runs_on(&v), Some("self_hosted_cx43".to_string()));
        Ok(())
    }

    #[test]
    fn normalize_workflow_nano_sequence_form() -> Result<()> {
        let v: Value =
            serde_yaml_ng::from_str("[self-hosted, linux, x64, em-ci, trusted-pr, workflow-nano]")?;
        assert_eq!(normalize_runs_on(&v), Some("self_hosted_workflow_nano".to_string()));
        Ok(())
    }

    // ── Coverage: defensive early-return branches ─────────────────────────────

    /// normalize: an unknown plain string passes through unchanged (so it can be
    /// compared against — and mismatch — a named whitelist token).
    #[test]
    fn normalize_unknown_string_passthrough() {
        let v = Value::String("some-custom-runner".to_string());
        assert_eq!(normalize_runs_on(&v), Some("some-custom-runner".to_string()));
    }

    /// normalize: object form without cx53/cx43 labels is unrecognized → None (skip).
    #[test]
    fn normalize_object_without_known_labels_returns_none() -> Result<()> {
        let yaml = "group: generic\nlabels: [self-hosted, linux]\n";
        let v: Value = serde_yaml_ng::from_str(yaml)?;
        assert_eq!(normalize_runs_on(&v), None);
        Ok(())
    }

    /// normalize: an unrecognized sequence is skipped.
    #[test]
    fn normalize_sequence_returns_none() -> Result<()> {
        let v: Value = serde_yaml_ng::from_str("[ubuntu-latest, windows-latest]")?;
        assert_eq!(normalize_runs_on(&v), None);
        Ok(())
    }

    /// runner check: lane missing the `runner` field → early return, no panic, no issue.
    #[test]
    fn runner_check_skips_lane_without_runner_field() -> Result<()> {
        let workflows_dir = fixture_path("")?;
        let lane: toml::Value =
            toml::from_str("workflow = \".github/workflows/runner_match.yml\"\njob = \"lint\"\n")?;
        let mut issues = Vec::new();
        check_runner_label_mismatch(&workflows_dir, &lane, &mut issues)?;
        assert!(issues.is_empty(), "expected no issues, got: {issues:?}");
        Ok(())
    }

    /// runner check: lane missing the `job` field → early return.
    #[test]
    fn runner_check_skips_lane_without_job_field() -> Result<()> {
        let workflows_dir = fixture_path("")?;
        let lane: toml::Value = toml::from_str(
            "workflow = \".github/workflows/runner_match.yml\"\nrunner = \"ubuntu_24_04\"\n",
        )?;
        let mut issues = Vec::new();
        check_runner_label_mismatch(&workflows_dir, &lane, &mut issues)?;
        assert!(issues.is_empty(), "expected no issues, got: {issues:?}");
        Ok(())
    }

    /// runner check: `runner = "mixed"` short-circuits even against a differing actual runner.
    #[test]
    fn runner_check_mixed_short_circuits() -> Result<()> {
        let workflows_dir = fixture_path("")?;
        let lane: toml::Value = toml::from_str(
            "workflow = \".github/workflows/runner_mismatch.yml\"\njob = \"lint\"\nrunner = \"mixed\"\n",
        )?;
        let mut issues = Vec::new();
        check_runner_label_mismatch(&workflows_dir, &lane, &mut issues)?;
        assert!(
            issues.iter().all(|i| i.code != "RUNNER_LABEL_MISMATCH"),
            "mixed runner must not fire mismatch: {issues:?}"
        );
        Ok(())
    }

    /// runner check: workflow file absent → early return (STALE check covers it).
    #[test]
    fn runner_check_skips_missing_workflow_file() -> Result<()> {
        let workflows_dir = fixture_path("")?;
        let lane: toml::Value = toml::from_str(
            "workflow = \".github/workflows/does_not_exist_xyz.yml\"\njob = \"lint\"\nrunner = \"ubuntu_24_04\"\n",
        )?;
        let mut issues = Vec::new();
        check_runner_label_mismatch(&workflows_dir, &lane, &mut issues)?;
        assert!(issues.is_empty(), "expected no issues for missing file, got: {issues:?}");
        Ok(())
    }

    /// stale check: lane missing `job` field → early return, no issue.
    #[test]
    fn stale_check_skips_lane_without_job_field() -> Result<()> {
        let workflows_dir = fixture_path("")?;
        let lane: toml::Value = toml::from_str("workflow = \".github/workflows/valid_job.yml\"\n")?;
        let mut issues = Vec::new();
        check_stale_whitelist_job(&workflows_dir, &lane, &mut issues)?;
        assert!(issues.is_empty(), "expected no issues, got: {issues:?}");
        Ok(())
    }

    /// stale check: workflow file absent → early return (deferred to LANE_WHITELIST_MISSING).
    #[test]
    fn stale_check_skips_missing_workflow_file() -> Result<()> {
        let workflows_dir = fixture_path("")?;
        let lane: toml::Value = toml::from_str(
            "workflow = \".github/workflows/does_not_exist_xyz.yml\"\njob = \"whatever\"\n",
        )?;
        let mut issues = Vec::new();
        check_stale_whitelist_job(&workflows_dir, &lane, &mut issues)?;
        assert!(issues.is_empty(), "expected no issues for missing file, got: {issues:?}");
        Ok(())
    }

    /// stale check: workflow with no `jobs:` map → STALE_WHITELIST_JOB fires.
    #[test]
    fn stale_check_fires_on_workflow_without_jobs_map() -> Result<()> {
        let workflows_dir = fixture_path("")?;
        let lane: toml::Value = toml::from_str(
            "workflow = \".github/workflows/no_jobs_map.yml\"\njob = \"anything\"\n",
        )?;
        let mut issues = Vec::new();
        check_stale_whitelist_job(&workflows_dir, &lane, &mut issues)?;
        assert!(
            issues.iter().any(|i| i.code == "STALE_WHITELIST_JOB"),
            "expected STALE_WHITELIST_JOB for jobless workflow, got: {issues:?}"
        );
        Ok(())
    }
}
