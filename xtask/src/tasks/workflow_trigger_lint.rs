use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_yaml_ng::{Mapping, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::project_root;

const REQUIRED_CANCEL_IN_PROGRESS: &str = "${{ github.event_name == 'pull_request' }}";

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum WorkflowTriggerLintFormat {
    Human,
    Json,
}

#[derive(Debug, Deserialize)]
struct RequiredChecksPolicy {
    #[serde(rename = "check")]
    checks: Vec<CheckEntry>,
}

#[derive(Debug, Deserialize)]
struct CheckEntry {
    name: String,
    workflow: String,
    required: bool,
}

#[derive(Debug, Serialize)]
pub struct WorkflowTriggerLintReceipt {
    schema_version: u32,
    policy: String,
    checked_workflows: Vec<String>,
    violations: Vec<WorkflowViolation>,
}

#[derive(Debug, Serialize)]
pub struct WorkflowViolation {
    workflow: String,
    check_name: Option<String>,
    rule: String,
    message: String,
}

pub fn run(
    policy_path: Option<PathBuf>,
    receipt_path: Option<PathBuf>,
    fixture: Option<PathBuf>,
    format: WorkflowTriggerLintFormat,
) -> Result<()> {
    let root = project_root()?;

    let (checked_workflows, violations, policy_label) = if let Some(fixture_path) = fixture {
        let workflow_path = if fixture_path.is_absolute() {
            fixture_path
        } else {
            root.join(fixture_path)
        };
        let violation_list = lint_workflow_file(&workflow_path, None)?;
        (vec![display_from_root(&root, &workflow_path)], violation_list, "fixture".to_string())
    } else {
        let policy_path = policy_path.unwrap_or_else(|| PathBuf::from(".ci/policies/required-checks.toml"));
        let policy_full_path = if policy_path.is_absolute() {
            policy_path
        } else {
            root.join(policy_path)
        };
        let policy = load_policy(&policy_full_path)?;
        let mut checked = Vec::new();
        let mut all_violations = Vec::new();

        for check in policy.checks.into_iter().filter(|entry| entry.required) {
            let workflow_path = root.join(&check.workflow);
            checked.push(check.workflow.clone());
            let mut violations = lint_workflow_file(&workflow_path, Some(&check.name))?;
            all_violations.append(&mut violations);
        }

        (
            checked,
            all_violations,
            display_from_root(&root, &policy_full_path),
        )
    };

    let receipt = WorkflowTriggerLintReceipt {
        schema_version: 1,
        policy: policy_label,
        checked_workflows,
        violations,
    };

    if let Some(receipt_path) = receipt_path {
        let path = if receipt_path.is_absolute() {
            receipt_path
        } else {
            root.join(receipt_path)
        };
        write_receipt(&path, &receipt)?;
    }

    emit_output(&receipt, format)?;

    if receipt.violations.is_empty() {
        return Ok(());
    }

    bail!("workflow trigger lint detected {} violation(s)", receipt.violations.len())
}

fn load_policy(path: &Path) -> Result<RequiredChecksPolicy> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("reading required checks policy at {}", path.display()))?;
    toml::from_str(&raw)
        .with_context(|| format!("parsing required checks policy at {}", path.display()))
}

fn lint_workflow_file(path: &Path, check_name: Option<&str>) -> Result<Vec<WorkflowViolation>> {
    if !path.exists() {
        return Ok(vec![WorkflowViolation {
            workflow: path.display().to_string(),
            check_name: check_name.map(std::string::ToString::to_string),
            rule: "workflow-file-exists".to_string(),
            message: "workflow file does not exist".to_string(),
        }]);
    }

    let raw = fs::read_to_string(path)
        .with_context(|| format!("reading workflow file at {}", path.display()))?;
    let workflow: Value = serde_yaml_ng::from_str(&raw)
        .with_context(|| format!("parsing workflow YAML at {}", path.display()))?;

    let workflow_label = path.display().to_string();
    let mut violations = Vec::new();

    let on = find_on_mapping(&workflow);
    match on {
        Some(mapping) => {
            if !mapping.contains_key(Value::String("pull_request".to_string())) {
                violations.push(violation(
                    &workflow_label,
                    check_name,
                    "has-pull-request-trigger",
                    "required workflows must include pull_request trigger",
                ));
            }
            if !mapping.contains_key(Value::String("merge_group".to_string())) {
                violations.push(violation(
                    &workflow_label,
                    check_name,
                    "has-merge-group-trigger",
                    "required workflows must include merge_group trigger",
                ));
            }

            match mapping.get(Value::String("push".to_string())) {
                Some(push_value) => {
                    if !push_has_master_branch(push_value) {
                        violations.push(violation(
                            &workflow_label,
                            check_name,
                            "has-push-master-trigger",
                            "required workflows must include push trigger for master",
                        ));
                    }
                }
                None => violations.push(violation(
                    &workflow_label,
                    check_name,
                    "has-push-master-trigger",
                    "required workflows must include push trigger for master",
                )),
            }

            if has_paths_filters(mapping) {
                violations.push(violation(
                    &workflow_label,
                    check_name,
                    "no-path-filters",
                    "required workflows must not define top-level pull_request paths or paths-ignore filters",
                ));
            }
        }
        None => {
            violations.push(violation(
                &workflow_label,
                check_name,
                "has-pull-request-trigger",
                "required workflows must include pull_request trigger",
            ));
            violations.push(violation(
                &workflow_label,
                check_name,
                "has-merge-group-trigger",
                "required workflows must include merge_group trigger",
            ));
            violations.push(violation(
                &workflow_label,
                check_name,
                "has-push-master-trigger",
                "required workflows must include push trigger for master",
            ));
        }
    }

    if !has_required_concurrency(&workflow) {
        violations.push(violation(
            &workflow_label,
            check_name,
            "event-aware-concurrency",
            "required workflows must set concurrency.cancel-in-progress to ${{ github.event_name == 'pull_request' }}",
        ));
    }

    Ok(violations)
}

fn violation(workflow: &str, check_name: Option<&str>, rule: &str, message: &str) -> WorkflowViolation {
    WorkflowViolation {
        workflow: workflow.to_string(),
        check_name: check_name.map(std::string::ToString::to_string),
        rule: rule.to_string(),
        message: message.to_string(),
    }
}

fn find_on_mapping(workflow: &Value) -> Option<&Mapping> {
    workflow
        .as_mapping()
        .and_then(|mapping| {
            mapping
                .get(Value::String("on".to_string()))
                .or_else(|| mapping.get(Value::Bool(true)))
        })
        .and_then(Value::as_mapping)
}

fn push_has_master_branch(push_value: &Value) -> bool {
    match push_value {
        Value::Mapping(mapping) => mapping
            .get(Value::String("branches".to_string()))
            .is_some_and(branches_include_master),
        Value::String(branch) => branch == "master",
        Value::Sequence(seq) => seq.iter().any(|entry| entry.as_str() == Some("master")),
        _ => false,
    }
}

fn branches_include_master(value: &Value) -> bool {
    match value {
        Value::String(branch) => branch == "master",
        Value::Sequence(branches) => branches.iter().any(|entry| entry.as_str() == Some("master")),
        _ => false,
    }
}

fn has_paths_filters(on_mapping: &Mapping) -> bool {
    on_mapping
        .get(Value::String("pull_request".to_string()))
        .and_then(Value::as_mapping)
        .is_some_and(|pull_request| {
            pull_request.contains_key(Value::String("paths".to_string()))
                || pull_request.contains_key(Value::String("paths-ignore".to_string()))
        })
}

fn has_required_concurrency(workflow: &Value) -> bool {
    workflow
        .as_mapping()
        .and_then(|mapping| mapping.get(Value::String("concurrency".to_string())))
        .and_then(Value::as_mapping)
        .and_then(|concurrency| concurrency.get(Value::String("cancel-in-progress".to_string())))
        .and_then(Value::as_str)
        == Some(REQUIRED_CANCEL_IN_PROGRESS)
}

fn write_receipt(path: &Path, receipt: &WorkflowTriggerLintReceipt) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating receipt directory {}", parent.display()))?;
    }

    let json = serde_json::to_string_pretty(receipt).context("serializing lint receipt JSON")?;
    fs::write(path, json).with_context(|| format!("writing receipt to {}", path.display()))
}

fn emit_output(receipt: &WorkflowTriggerLintReceipt, format: WorkflowTriggerLintFormat) -> Result<()> {
    match format {
        WorkflowTriggerLintFormat::Human => {
            if receipt.violations.is_empty() {
                println!(
                    "workflow-trigger-lint passed for {} workflow(s)",
                    receipt.checked_workflows.len()
                );
                return Ok(());
            }

            println!(
                "workflow-trigger-lint found {} violation(s) across {} workflow(s):",
                receipt.violations.len(),
                receipt.checked_workflows.len()
            );
            for violation in &receipt.violations {
                println!(
                    "- {} [{}] {}",
                    violation.workflow, violation.rule, violation.message
                );
            }
        }
        WorkflowTriggerLintFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(receipt)
                    .context("serializing workflow trigger lint output")?
            );
        }
    }

    Ok(())
}

fn display_from_root(root: &Path, absolute: &Path) -> String {
    absolute
        .strip_prefix(root)
        .map(|relative| relative.display().to_string())
        .unwrap_or_else(|_| absolute.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::eyre;

    fn fixture_path(path: &str) -> PathBuf {
        project_root()
            .map(|root| root.join(path))
            .unwrap_or_else(|_| PathBuf::from(path))
    }

    #[test]
    fn valid_required_fixture_passes() -> Result<()> {
        let path = fixture_path("xtask/tests/fixtures/workflows/valid-required.yml");
        let violations = lint_workflow_file(&path, Some("CI Gate"))?;
        if !violations.is_empty() {
            return Err(eyre!("expected no violations, found: {}", violations.len()));
        }
        Ok(())
    }

    #[test]
    fn missing_merge_group_fixture_fails() -> Result<()> {
        let path = fixture_path("xtask/tests/fixtures/workflows/missing-merge-group.yml");
        let violations = lint_workflow_file(&path, Some("CI Gate"))?;
        if !violations.iter().any(|v| v.rule == "has-merge-group-trigger") {
            return Err(eyre!("expected has-merge-group-trigger violation"));
        }
        Ok(())
    }

    #[test]
    fn path_filtered_fixture_fails() -> Result<()> {
        let path = fixture_path("xtask/tests/fixtures/workflows/path-filtered.yml");
        let violations = lint_workflow_file(&path, Some("CI Gate"))?;
        if !violations.iter().any(|v| v.rule == "no-path-filters") {
            return Err(eyre!("expected no-path-filters violation"));
        }
        Ok(())
    }

    #[test]
    fn bad_concurrency_fixture_fails() -> Result<()> {
        let path = fixture_path("xtask/tests/fixtures/workflows/bad-concurrency.yml");
        let violations = lint_workflow_file(&path, Some("CI Gate"))?;
        if !violations.iter().any(|v| v.rule == "event-aware-concurrency") {
            return Err(eyre!("expected event-aware-concurrency violation"));
        }
        Ok(())
    }
}
