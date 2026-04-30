//! Enforce trigger/concurrency policy for required CI workflows.

use std::fs;
use std::path::{Path, PathBuf};

use clap::ValueEnum;
use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_yaml_ng::Value;

use crate::utils::project_root;

const REQUIRED_CANCEL_IN_PROGRESS: &str = "${{ github.event_name == 'pull_request' }}";

#[derive(Debug, Clone, Deserialize)]
struct RequiredChecksPolicy {
    check: Vec<PolicyCheck>,
}

#[derive(Debug, Clone, Deserialize)]
struct PolicyCheck {
    name: String,
    workflow: String,
    required: bool,
}

#[derive(Debug, Clone, Serialize)]
struct WorkflowEvaluation {
    name: String,
    workflow: String,
    required: bool,
    ok: bool,
    violations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct WorkflowTriggerLintReceipt {
    schema_version: String,
    policy_path: Option<String>,
    fixture_path: Option<String>,
    overall_ok: bool,
    evaluations: Vec<WorkflowEvaluation>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum WorkflowTriggerLintFormat {
    Text,
    Json,
}

pub fn run(
    policy_path: Option<PathBuf>,
    receipt_path: Option<PathBuf>,
    fixture_path: Option<PathBuf>,
    format: WorkflowTriggerLintFormat,
) -> Result<()> {
    let root = project_root()?;

    let (evaluations, policy_display, fixture_display) = if let Some(fixture) = fixture_path {
        let evaluation = evaluate_fixture(&fixture)?;
        (vec![evaluation], None, Some(fixture.display().to_string()))
    } else {
        let policy = policy_path.unwrap_or_else(|| root.join(".ci/policies/required-checks.toml"));
        let evaluations = evaluate_policy(&root, &policy)?;
        (evaluations, Some(policy.display().to_string()), None)
    };

    let overall_ok = evaluations.iter().all(|entry| entry.ok);
    let receipt = WorkflowTriggerLintReceipt {
        schema_version: "1.0.0".to_string(),
        policy_path: policy_display,
        fixture_path: fixture_display,
        overall_ok,
        evaluations,
    };

    if let Some(path) = receipt_path {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating receipt directory {}", parent.display()))?;
        }
        let payload = serde_json::to_string_pretty(&receipt)?;
        fs::write(&path, payload).with_context(|| format!("writing receipt {}", path.display()))?;
    }

    output(&receipt, format)?;

    if receipt.overall_ok { Ok(()) } else { bail!("workflow-trigger-lint found policy violations") }
}

fn evaluate_policy(root: &Path, policy_path: &Path) -> Result<Vec<WorkflowEvaluation>> {
    let raw = fs::read_to_string(policy_path)
        .with_context(|| format!("reading policy file {}", policy_path.display()))?;
    let policy: RequiredChecksPolicy = toml::from_str(&raw)
        .with_context(|| format!("parsing policy file {}", policy_path.display()))?;

    policy
        .check
        .into_iter()
        .filter(|entry| entry.required)
        .map(|entry| evaluate_required_workflow(root, entry))
        .collect()
}

fn evaluate_fixture(fixture_path: &Path) -> Result<WorkflowEvaluation> {
    let workflow = read_workflow_yaml(fixture_path)?;
    Ok(evaluate_required_entry(
        "fixture",
        fixture_path.display().to_string(),
        true,
        fixture_path.exists(),
        Some(&workflow),
    ))
}

fn evaluate_required_workflow(root: &Path, check: PolicyCheck) -> Result<WorkflowEvaluation> {
    let workflow_path = root.join(&check.workflow);
    let workflow =
        if workflow_path.exists() { Some(read_workflow_yaml(&workflow_path)?) } else { None };

    Ok(evaluate_required_entry(
        &check.name,
        check.workflow,
        check.required,
        workflow_path.exists(),
        workflow.as_ref(),
    ))
}

fn evaluate_required_entry(
    name: &str,
    workflow: String,
    required: bool,
    workflow_exists: bool,
    workflow_yaml: Option<&Value>,
) -> WorkflowEvaluation {
    let mut violations = Vec::new();

    if required {
        if !workflow_exists {
            violations.push("workflow file does not exist".to_string());
        }

        if let Some(yaml) = workflow_yaml {
            if !has_trigger(yaml, "pull_request") {
                violations.push("missing pull_request trigger".to_string());
            }
            if !has_trigger(yaml, "merge_group") {
                violations.push("missing merge_group trigger".to_string());
            }
            if !push_targets_master(yaml) {
                violations.push("push trigger must target master branch".to_string());
            }
            if has_path_filters(yaml) {
                violations.push("path filters are not allowed on required workflows".to_string());
            }
            if !has_event_aware_concurrency(yaml) {
                violations.push(format!(
                    "concurrency.cancel-in-progress must be `{REQUIRED_CANCEL_IN_PROGRESS}`"
                ));
            }
        }
    }

    WorkflowEvaluation {
        name: name.to_string(),
        workflow,
        required,
        ok: violations.is_empty(),
        violations,
    }
}

fn read_workflow_yaml(path: &Path) -> Result<Value> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let parsed = serde_yaml_ng::from_str(&raw)
        .with_context(|| format!("parsing workflow YAML {}", path.display()))?;
    Ok(parsed)
}

fn has_trigger(workflow: &Value, trigger_name: &str) -> bool {
    let Some(on) = get_on(workflow) else {
        return false;
    };

    match on {
        Value::String(value) => value == trigger_name,
        Value::Sequence(values) => {
            values.iter().any(|value| value.as_str().is_some_and(|item| item == trigger_name))
        }
        Value::Mapping(mapping) => {
            mapping.iter().any(|(key, _)| key.as_str().is_some_and(|item| item == trigger_name))
        }
        _ => false,
    }
}

fn push_targets_master(workflow: &Value) -> bool {
    let Some(on) = get_on(workflow) else {
        return false;
    };

    match on {
        Value::Mapping(mapping) => {
            let push = mapping.iter().find_map(|(key, value)| {
                if key.as_str().is_some_and(|name| name == "push") { Some(value) } else { None }
            });

            let Some(push) = push else {
                return false;
            };

            match push {
                Value::Mapping(push_mapping) => push_mapping.iter().any(|(key, value)| {
                    key.as_str().is_some_and(|name| name == "branches")
                        && branch_targets_master(value)
                }),
                _ => false,
            }
        }
        _ => false,
    }
}

fn branch_targets_master(branches: &Value) -> bool {
    match branches {
        Value::String(value) => value == "master" || value == "refs/heads/master",
        Value::Sequence(values) => values.iter().any(|value| {
            value.as_str().is_some_and(|item| item == "master" || item == "refs/heads/master")
        }),
        _ => false,
    }
}

fn has_path_filters(workflow: &Value) -> bool {
    let Some(on) = get_on(workflow) else {
        return false;
    };

    let Value::Mapping(mapping) = on else {
        return false;
    };

    if mapping
        .keys()
        .any(|key| key.as_str().is_some_and(|name| name == "paths" || name == "paths-ignore"))
    {
        return true;
    }

    mapping.iter().any(|(key, value)| {
        let Some(name) = key.as_str() else {
            return false;
        };

        if name != "pull_request" && name != "merge_group" && name != "push" {
            return false;
        }

        match value {
            Value::Mapping(event_map) => event_map.keys().any(|event_key| {
                event_key
                    .as_str()
                    .is_some_and(|event_name| event_name == "paths" || event_name == "paths-ignore")
            }),
            _ => false,
        }
    })
}

fn has_event_aware_concurrency(workflow: &Value) -> bool {
    let Some(concurrency) = get_top_level_field(workflow, "concurrency") else {
        return false;
    };

    let Value::Mapping(map) = concurrency else {
        return false;
    };

    map.iter().any(|(key, value)| {
        key.as_str().is_some_and(|field| field == "cancel-in-progress")
            && value.as_str().is_some_and(|expr| expr.trim() == REQUIRED_CANCEL_IN_PROGRESS)
    })
}

fn output(receipt: &WorkflowTriggerLintReceipt, format: WorkflowTriggerLintFormat) -> Result<()> {
    match format {
        WorkflowTriggerLintFormat::Json => {
            println!("{}", serde_json::to_string_pretty(receipt)?);
        }
        WorkflowTriggerLintFormat::Text => {
            if receipt.overall_ok {
                println!("✓ workflow-trigger-lint passed");
            } else {
                println!("❌ workflow-trigger-lint found violations");
            }

            for evaluation in &receipt.evaluations {
                if evaluation.ok {
                    println!("  - {} ({}) ✓", evaluation.name, evaluation.workflow);
                } else {
                    println!("  - {} ({})", evaluation.name, evaluation.workflow);
                    for violation in &evaluation.violations {
                        println!("      * {}", violation);
                    }
                }
            }
        }
    }
    Ok(())
}

fn get_on(workflow: &Value) -> Option<&Value> {
    get_top_level_field(workflow, "on")
}

fn get_top_level_field<'a>(workflow: &'a Value, field: &str) -> Option<&'a Value> {
    let Value::Mapping(mapping) = workflow else {
        return None;
    };

    mapping.iter().find_map(|(key, value)| {
        if key.as_str().is_some_and(|name| name == field) { Some(value) } else { None }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_fixture(name: &str) -> Result<Value> {
        let root = project_root()?;
        read_workflow_yaml(&root.join("xtask/tests/fixtures/workflows").join(name))
    }

    #[test]
    fn valid_required_fixture_passes() -> Result<()> {
        let fixture = load_fixture("valid-required.yml")?;
        let eval = evaluate_required_entry(
            "fixture",
            "fixture.yml".to_string(),
            true,
            true,
            Some(&fixture),
        );
        assert!(eval.ok);
        Ok(())
    }

    #[test]
    fn missing_merge_group_fixture_fails() -> Result<()> {
        let fixture = load_fixture("missing-merge-group.yml")?;
        let eval = evaluate_required_entry(
            "fixture",
            "fixture.yml".to_string(),
            true,
            true,
            Some(&fixture),
        );
        assert!(!eval.ok);
        assert!(eval.violations.iter().any(|item| item.contains("merge_group")));
        Ok(())
    }
}
