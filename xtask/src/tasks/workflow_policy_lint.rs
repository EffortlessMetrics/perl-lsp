use color_eyre::eyre::{Context, Result, bail};
use serde::Serialize;
use serde_yaml_ng::{Mapping, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::project_root;

const CONTENTS_WRITE_ALLOWLIST: &[&str] = &["ci.yml"];
const TRUSTED_ACTION_OWNERS: &[&str] = &["actions", "github"];

#[derive(Debug, Clone)]
pub struct WorkflowPolicyLintConfig {
    pub fixture: Option<PathBuf>,
    pub receipt: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
struct WorkflowPolicyReceipt {
    files_scanned: usize,
    violations: Vec<String>,
    warnings: Vec<String>,
}

pub fn run(config: WorkflowPolicyLintConfig) -> Result<()> {
    let workflows = collect_workflow_inputs(&config)?;

    let mut violations = Vec::new();
    let mut warnings = Vec::new();

    for (path, raw) in &workflows {
        let workflow_name = path.file_name().and_then(|name| name.to_str()).unwrap_or("<unknown>");
        let workflow: Value = serde_yaml_ng::from_str(raw)
            .with_context(|| format!("failed to parse workflow YAML: {}", path.display()))?;

        lint_workflow(path, workflow_name, &workflow, &mut violations, &mut warnings);
    }

    if let Some(receipt_path) = config.receipt {
        let receipt = WorkflowPolicyReceipt {
            files_scanned: workflows.len(),
            violations: violations.clone(),
            warnings: warnings.clone(),
        };
        write_receipt(&receipt_path, &receipt)?;
    }

    for warning in &warnings {
        eprintln!("::warning::{warning}");
    }

    if !violations.is_empty() {
        for violation in &violations {
            eprintln!("::error::{violation}");
        }
        bail!("workflow policy lint failed with {} violation(s)", violations.len());
    }

    println!("workflow policy lint passed ({} file(s) scanned)", workflows.len());
    Ok(())
}

fn collect_workflow_inputs(config: &WorkflowPolicyLintConfig) -> Result<Vec<(PathBuf, String)>> {
    if let Some(fixture) = &config.fixture {
        let raw = fs::read_to_string(fixture)
            .with_context(|| format!("failed to read fixture {}", fixture.display()))?;
        return Ok(vec![(fixture.clone(), raw)]);
    }

    let root = project_root()?;
    let workflows_dir = root.join(".github/workflows");
    let mut workflows = Vec::new();

    if !workflows_dir.exists() {
        return Ok(workflows);
    }

    for entry in fs::read_dir(&workflows_dir)
        .with_context(|| format!("failed to read {}", workflows_dir.display()))?
    {
        let path = entry?.path();
        let is_workflow = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext == "yml" || ext == "yaml");
        if !is_workflow {
            continue;
        }

        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read workflow {}", path.display()))?;
        workflows.push((path, raw));
    }

    Ok(workflows)
}

fn lint_workflow(
    path: &Path,
    workflow_name: &str,
    workflow: &Value,
    violations: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    let has_pr = has_event(workflow, "pull_request");
    let has_pr_target = has_event(workflow, "pull_request_target");
    let has_merge_group = has_event(workflow, "merge_group");
    let has_truth_run = triggers_main_push(workflow) || has_merge_group;

    let checkout_pr_head = has_checkout_pr_head(workflow);
    if has_pr_target && checkout_pr_head {
        violations.push(format!("{workflow_name}: pull_request_target must not checkout PR head"));
    }

    if has_pr && has_contents_write(workflow) && !CONTENTS_WRITE_ALLOWLIST.contains(&workflow_name)
    {
        violations.push(format!(
            "{workflow_name}: pull_request workflows must not request contents: write unless allowlisted"
        ));
    }

    if has_write_all_permissions(workflow) {
        violations.push(format!("{workflow_name}: permissions: write-all is forbidden"));
    }

    if (has_pr || has_pr_target) && checkout_pr_head && can_access_secrets(workflow) {
        violations.push(format!("{workflow_name}: untrusted PR code may access secrets"));
    }

    if is_required_style_workflow(path, workflow) {
        if !has_merge_group {
            violations.push(format!(
                "{workflow_name}: required-style workflows must include merge_group"
            ));
        }
        if has_self_path_filter(path, workflow) {
            violations.push(format!(
                "{workflow_name}: required-style workflows must not path-filter themselves"
            ));
        }
    }

    if has_blanket_cancel_in_progress(workflow) && has_truth_run {
        violations.push(format!(
            "{workflow_name}: blanket cancel-in-progress cannot apply to main/merge_group truth runs"
        ));
    }

    for action in find_unpinned_third_party_actions(workflow) {
        warnings.push(format!("{workflow_name}: unpinned third-party action: {action}"));
    }
}

fn write_receipt(path: &Path, receipt: &WorkflowPolicyReceipt) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create receipt directory {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(receipt).context("failed to serialize receipt")?;
    fs::write(path, json).with_context(|| format!("failed to write receipt {}", path.display()))?;
    Ok(())
}

fn has_event(workflow: &Value, event: &str) -> bool {
    let Some(on) = workflow.get("on") else {
        return false;
    };

    match on {
        Value::String(value) => value == event,
        Value::Sequence(items) => items.iter().any(|value| value.as_str() == Some(event)),
        Value::Mapping(map) => map.contains_key(Value::String(event.to_string())),
        _ => false,
    }
}

fn has_contents_write(workflow: &Value) -> bool {
    if has_permissions_write_value(workflow.get("permissions"), "contents") {
        return true;
    }

    workflow.get("jobs").and_then(Value::as_mapping).is_some_and(|jobs| {
        jobs.values().any(|job| {
            has_permissions_write_value(
                job.as_mapping().and_then(|j| j.get("permissions")),
                "contents",
            )
        })
    })
}

fn has_write_all_permissions(workflow: &Value) -> bool {
    has_write_all_value(workflow.get("permissions"))
        || workflow.get("jobs").and_then(Value::as_mapping).is_some_and(|jobs| {
            jobs.values()
                .any(|job| has_write_all_value(job.as_mapping().and_then(|j| j.get("permissions"))))
        })
}

fn has_permissions_write_value(permissions: Option<&Value>, key: &str) -> bool {
    match permissions {
        Some(Value::Mapping(map)) => map
            .get(Value::String(key.to_string()))
            .and_then(Value::as_str)
            .is_some_and(|value| value == "write"),
        _ => false,
    }
}

fn has_write_all_value(permissions: Option<&Value>) -> bool {
    permissions.and_then(Value::as_str).is_some_and(|value| value == "write-all")
}

fn has_checkout_pr_head(workflow: &Value) -> bool {
    workflow
        .get("jobs")
        .and_then(Value::as_mapping)
        .is_some_and(|jobs| jobs.values().any(job_checks_out_pr_head))
}

fn job_checks_out_pr_head(job: &Value) -> bool {
    let Some(job_map) = job.as_mapping() else {
        return false;
    };

    job_map.get("steps").and_then(Value::as_sequence).is_some_and(|steps| {
        steps.iter().any(|step| {
            let Some(step_map) = step.as_mapping() else {
                return false;
            };
            let uses_checkout = step_map
                .get("uses")
                .and_then(Value::as_str)
                .is_some_and(|uses| uses.starts_with("actions/checkout@"));
            if !uses_checkout {
                return false;
            }

            step_map
                .get("with")
                .and_then(Value::as_mapping)
                .and_then(|with| with.get("ref"))
                .is_some_and(|reference| {
                    value_contains(reference, "github.event.pull_request.head")
                        || value_contains(reference, "github.head_ref")
                })
        })
    })
}

fn can_access_secrets(workflow: &Value) -> bool {
    if value_contains(workflow, "secrets.") {
        return true;
    }

    workflow
        .get("jobs")
        .and_then(Value::as_mapping)
        .is_some_and(|jobs| jobs.values().any(|job| value_contains(job, "secrets.")))
}

fn is_required_style_workflow(path: &Path, workflow: &Value) -> bool {
    let name_has_required = workflow
        .get("name")
        .and_then(Value::as_str)
        .is_some_and(|name| name.to_lowercase().contains("required"));
    let file_has_required = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.to_lowercase().contains("required"));

    name_has_required || file_has_required
}

fn has_self_path_filter(path: &Path, workflow: &Value) -> bool {
    let Some(on) = workflow.get("on") else {
        return false;
    };

    let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
    let self_filter = format!(".github/workflows/{file_name}");

    match on {
        Value::Mapping(on_map) => on_map
            .get("pull_request")
            .and_then(Value::as_mapping)
            .and_then(|pr| pr.get("paths"))
            .and_then(Value::as_sequence)
            .is_some_and(|paths| {
                paths.iter().any(|entry| entry.as_str().is_some_and(|path| path == self_filter))
            }),
        _ => false,
    }
}

fn has_blanket_cancel_in_progress(workflow: &Value) -> bool {
    workflow
        .get("concurrency")
        .and_then(|value| match value {
            Value::Mapping(map) => map.get("cancel-in-progress"),
            _ => None,
        })
        .and_then(Value::as_bool)
        .is_some_and(|value| value)
}

fn triggers_main_push(workflow: &Value) -> bool {
    let Some(on) = workflow.get("on") else {
        return false;
    };

    match on {
        Value::Mapping(on_map) => on_map.get("push").is_some_and(push_targets_main_branch),
        _ => false,
    }
}

fn push_targets_main_branch(push: &Value) -> bool {
    match push {
        Value::Null => true,
        Value::Mapping(map) => {
            map.get("branches").and_then(Value::as_sequence).is_none_or(|branches| {
                branches
                    .iter()
                    .filter_map(Value::as_str)
                    .any(|branch| branch == "main" || branch == "master")
            })
        }
        _ => false,
    }
}

fn find_unpinned_third_party_actions(workflow: &Value) -> Vec<String> {
    let mut findings = Vec::new();
    let Some(jobs) = workflow.get("jobs").and_then(Value::as_mapping) else {
        return findings;
    };

    for (job_name, job) in jobs {
        let Some(job_name) = job_name.as_str() else {
            continue;
        };
        let Some(steps) =
            job.as_mapping().and_then(|job| job.get("steps")).and_then(Value::as_sequence)
        else {
            continue;
        };

        for step in steps {
            let Some(step_map) = step.as_mapping() else {
                continue;
            };
            let Some(uses) = step_map.get("uses").and_then(Value::as_str) else {
                continue;
            };
            if !is_third_party_action(uses) || is_sha_pinned(uses) {
                continue;
            }
            findings.push(format!("{job_name}: {uses}"));
        }
    }

    findings
}

fn is_third_party_action(uses: &str) -> bool {
    let Some((owner_repo, _reference)) = uses.split_once('@') else {
        return false;
    };
    let Some((owner, _repo)) = owner_repo.split_once('/') else {
        return false;
    };

    !TRUSTED_ACTION_OWNERS.contains(&owner)
}

fn is_sha_pinned(uses: &str) -> bool {
    let Some((_action, reference)) = uses.split_once('@') else {
        return false;
    };

    reference.len() == 40 && reference.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn value_contains(value: &Value, needle: &str) -> bool {
    match value {
        Value::String(text) => text.contains(needle),
        Value::Sequence(items) => items.iter().any(|item| value_contains(item, needle)),
        Value::Mapping(map) => mapping_contains(map, needle),
        _ => false,
    }
}

fn mapping_contains(map: &Mapping, needle: &str) -> bool {
    map.iter().any(|(key, value)| value_contains(key, needle) || value_contains(value, needle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::Result;

    fn fixture_path(name: &str) -> Result<PathBuf> {
        let root = project_root()?;
        Ok(root.join("xtask/tests/fixtures/workflow-policy").join(name))
    }

    #[test]
    fn fixture_pull_request_target_checkout_head_fails() -> Result<()> {
        let fixture = fixture_path("pull_request_target_checkout_head.yml")?;
        let result = run(WorkflowPolicyLintConfig { fixture: Some(fixture), receipt: None });
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn fixture_pull_request_read_only_passes() -> Result<()> {
        let fixture = fixture_path("pull_request_read_only.yml")?;
        run(WorkflowPolicyLintConfig { fixture: Some(fixture), receipt: None })?;
        Ok(())
    }

    #[test]
    fn fixture_write_all_fails() -> Result<()> {
        let fixture = fixture_path("write_all.yml")?;
        let result = run(WorkflowPolicyLintConfig { fixture: Some(fixture), receipt: None });
        assert!(result.is_err());
        Ok(())
    }
}
