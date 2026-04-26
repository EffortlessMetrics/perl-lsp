use color_eyre::eyre::{Context, Result, bail};
use serde::Deserialize;
use serde::Serialize;
use serde_yaml_ng::Value;
use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::project_root;

const DEFAULT_POLICY_PATH: &str = ".ci/policies/required-checks.toml";
const EXPECTED_CANCEL_IN_PROGRESS: &str = "${{ github.event_name == 'pull_request' }}";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkflowTriggerLintFormat {
    Human,
    Json,
}

#[derive(Debug)]
pub struct WorkflowTriggerLintConfig {
    pub policy: Option<PathBuf>,
    pub receipt: Option<PathBuf>,
    pub fixture: Option<PathBuf>,
    pub format: WorkflowTriggerLintFormat,
}

#[derive(Debug, Deserialize)]
struct RequiredChecksPolicy {
    check: Vec<PolicyCheck>,
}

#[derive(Debug, Deserialize)]
struct PolicyCheck {
    name: String,
    workflow: String,
    required: bool,
}

#[derive(Debug, Serialize)]
struct WorkflowTriggerLintReceipt {
    schema: String,
    ok: bool,
    checked: usize,
    violations: Vec<ViolationReceipt>,
}

#[derive(Debug, Serialize)]
struct ViolationReceipt {
    check_name: String,
    workflow: String,
    violations: Vec<String>,
}

pub fn run(config: WorkflowTriggerLintConfig) -> Result<()> {
    let root = project_root()?;
    let findings = if let Some(fixture) = config.fixture.as_ref() {
        let fixture_path = resolve_path(&root, fixture);
        let violations = lint_required_workflow_file(&fixture_path)?;
        vec![ViolationReceipt {
            check_name: fixture_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("fixture")
                .to_string(),
            workflow: fixture_path.display().to_string(),
            violations,
        }]
    } else {
        let policy_path = resolve_path(
            &root,
            config.policy.as_ref().unwrap_or(&PathBuf::from(DEFAULT_POLICY_PATH)),
        );
        lint_policy(&root, &policy_path)?
    };

    let ok = findings.iter().all(|entry| entry.violations.is_empty());
    let checked = findings.len();
    let violations: Vec<ViolationReceipt> =
        findings.into_iter().filter(|entry| !entry.violations.is_empty()).collect();

    let receipt = WorkflowTriggerLintReceipt {
        schema: "workflow-trigger-lint@1".to_string(),
        ok,
        checked,
        violations,
    };

    if let Some(receipt_path) = config.receipt.as_ref() {
        write_receipt(&root, receipt_path, &receipt)?;
    }

    output_receipt(&receipt, config.format)?;

    if receipt.ok { Ok(()) } else { bail!("workflow-trigger-lint found policy violations") }
}

fn resolve_path(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() { path.to_path_buf() } else { root.join(path) }
}

fn lint_policy(root: &Path, policy_path: &Path) -> Result<Vec<ViolationReceipt>> {
    let policy_raw = fs::read_to_string(policy_path)
        .with_context(|| format!("reading policy {}", policy_path.display()))?;
    let policy: RequiredChecksPolicy = toml::from_str(&policy_raw)
        .with_context(|| format!("parsing policy {}", policy_path.display()))?;

    let mut findings = Vec::new();
    for check in &policy.check {
        if !check.required {
            continue;
        }

        let workflow_path = root.join(&check.workflow);
        let violations = lint_required_workflow_file(&workflow_path)?;
        findings.push(ViolationReceipt {
            check_name: check.name.clone(),
            workflow: check.workflow.clone(),
            violations,
        });
    }

    Ok(findings)
}

fn lint_required_workflow_file(workflow_path: &Path) -> Result<Vec<String>> {
    if !workflow_path.exists() {
        return Ok(vec![format!("workflow file does not exist: {}", workflow_path.display())]);
    }

    let raw = fs::read_to_string(workflow_path)
        .with_context(|| format!("reading workflow {}", workflow_path.display()))?;
    let workflow: Value = serde_yaml_ng::from_str(&raw)
        .with_context(|| format!("parsing workflow YAML {}", workflow_path.display()))?;

    let mut violations = Vec::new();

    let on = workflow.get("on");
    if !has_trigger(on, "pull_request") {
        violations.push("missing on.pull_request trigger".to_string());
    }
    if !has_trigger(on, "merge_group") {
        violations.push("missing on.merge_group trigger".to_string());
    }
    if !has_push_master(on) {
        violations.push("missing on.push trigger for master branch".to_string());
    }
    if has_path_filter(on) {
        violations.push("workflow trigger must not use paths or paths-ignore filters".to_string());
    }
    if !has_event_aware_concurrency(&workflow) {
        violations.push(format!(
            "top-level concurrency.cancel-in-progress must equal {EXPECTED_CANCEL_IN_PROGRESS}"
        ));
    }

    Ok(violations)
}

fn has_trigger(on: Option<&Value>, trigger_name: &str) -> bool {
    match on {
        Some(Value::String(single)) => single == trigger_name,
        Some(Value::Sequence(seq)) => seq.iter().any(|item| item.as_str() == Some(trigger_name)),
        Some(Value::Mapping(map)) => map.contains_key(Value::String(trigger_name.to_string())),
        _ => false,
    }
}

fn has_push_master(on: Option<&Value>) -> bool {
    let Some(on_value) = on else {
        return false;
    };

    match on_value {
        Value::String(single) => single == "push",
        Value::Sequence(seq) => seq.iter().any(|item| item.as_str() == Some("push")),
        Value::Mapping(map) => {
            let Some(push) = map.get(Value::String("push".to_string())) else {
                return false;
            };

            match push {
                Value::Null => true,
                Value::Mapping(push_map) => {
                    if let Some(branches) = push_map.get(Value::String("branches".to_string())) {
                        value_contains_string(branches, "master")
                    } else {
                        false
                    }
                }
                _ => false,
            }
        }
        _ => false,
    }
}

fn value_contains_string(value: &Value, expected: &str) -> bool {
    match value {
        Value::String(single) => single == expected,
        Value::Sequence(seq) => seq.iter().any(|item| item.as_str() == Some(expected)),
        _ => false,
    }
}

fn has_path_filter(on: Option<&Value>) -> bool {
    let Some(Value::Mapping(on_map)) = on else {
        return false;
    };

    if on_map.contains_key(Value::String("paths".to_string()))
        || on_map.contains_key(Value::String("paths-ignore".to_string()))
    {
        return true;
    }

    for event in ["pull_request", "merge_group", "push"] {
        if let Some(Value::Mapping(event_map)) = on_map.get(Value::String(event.to_string()))
            && (event_map.contains_key(Value::String("paths".to_string()))
                || event_map.contains_key(Value::String("paths-ignore".to_string())))
        {
            return true;
        }
    }

    false
}

fn has_event_aware_concurrency(workflow: &Value) -> bool {
    let Some(concurrency) = workflow.get("concurrency") else {
        return false;
    };
    let Some(concurrency_map) = concurrency.as_mapping() else {
        return false;
    };

    concurrency_map
        .get(Value::String("cancel-in-progress".to_string()))
        .and_then(Value::as_str)
        .is_some_and(|value| value == EXPECTED_CANCEL_IN_PROGRESS)
}

fn write_receipt(
    root: &Path,
    relative_path: &Path,
    receipt: &WorkflowTriggerLintReceipt,
) -> Result<()> {
    let receipt_path = resolve_path(root, relative_path);
    if let Some(parent) = receipt_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating receipt directory {}", parent.display()))?;
    }

    let json = serde_json::to_string_pretty(receipt).context("serializing receipt json")?;
    fs::write(&receipt_path, format!("{json}\n"))
        .with_context(|| format!("writing receipt {}", receipt_path.display()))?;

    Ok(())
}

fn output_receipt(
    receipt: &WorkflowTriggerLintReceipt,
    format: WorkflowTriggerLintFormat,
) -> Result<()> {
    match format {
        WorkflowTriggerLintFormat::Human => {
            if receipt.ok {
                println!("workflow-trigger-lint passed (checked {} workflow(s))", receipt.checked);
            } else {
                println!("workflow-trigger-lint failed:");
                for entry in &receipt.violations {
                    println!("- {} ({})", entry.check_name, entry.workflow);
                    for violation in &entry.violations {
                        println!("  - {violation}");
                    }
                }
            }
            Ok(())
        }
        WorkflowTriggerLintFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(receipt).context("serializing json output")?
            );
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_fixture_passes() -> Result<()> {
        let root = project_root()?;
        let fixture = root.join("xtask/tests/fixtures/workflows/valid-required.yml");
        let violations = lint_required_workflow_file(&fixture)?;
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn merge_group_fixture_fails() -> Result<()> {
        let root = project_root()?;
        let fixture = root.join("xtask/tests/fixtures/workflows/missing-merge-group.yml");
        let violations = lint_required_workflow_file(&fixture)?;
        assert!(violations.iter().any(|item| item.contains("merge_group")));
        Ok(())
    }
}
