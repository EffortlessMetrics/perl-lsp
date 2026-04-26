use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail, eyre};
use serde::{Deserialize, Serialize};
use serde_yaml_ng::{Mapping, Value};
use std::fs;
use std::path::{Path, PathBuf};

const REQUIRED_CANCEL_IN_PROGRESS: &str = "${{ github.event_name == 'pull_request' }}";

#[derive(Debug, Clone)]
pub struct WorkflowTriggerLintConfig {
    pub policy: PathBuf,
    pub fixture: Option<PathBuf>,
    pub receipt: Option<PathBuf>,
    pub format: WorkflowTriggerLintFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowTriggerLintFormat {
    Text,
    Json,
}

#[derive(Debug, Deserialize)]
struct RequiredChecksPolicy {
    check: Vec<RequiredCheck>,
}

#[derive(Debug, Deserialize)]
struct RequiredCheck {
    name: String,
    workflow: String,
    required: bool,
}

#[derive(Debug, Serialize)]
struct LintReceipt {
    passed: bool,
    mode: String,
    checks: Vec<WorkflowLintResult>,
}

#[derive(Debug, Serialize)]
struct WorkflowLintResult {
    policy_name: String,
    workflow: String,
    required: bool,
    passed: bool,
    violations: Vec<String>,
}

pub fn run(config: WorkflowTriggerLintConfig) -> Result<()> {
    let root = project_root()?;

    let results = if let Some(fixture) = &config.fixture {
        let fixture_path = resolve_input_path(&root, fixture);
        vec![lint_single_file(
            "fixture",
            fixture_path.strip_prefix(&root).unwrap_or(&fixture_path),
            true,
            &fixture_path,
        )?]
    } else {
        let policy_path = resolve_input_path(&root, &config.policy);
        let policy_raw = fs::read_to_string(&policy_path)
            .with_context(|| format!("reading policy {}", policy_path.display()))?;
        let policy: RequiredChecksPolicy =
            toml::from_str(&policy_raw).with_context(|| "parsing policy TOML")?;

        let mut results = Vec::new();
        for check in policy.check.into_iter().filter(|check| check.required) {
            let workflow_path = root.join(&check.workflow);
            if !workflow_path.exists() {
                results.push(WorkflowLintResult {
                    policy_name: check.name,
                    workflow: check.workflow,
                    required: true,
                    passed: false,
                    violations: vec!["workflow file does not exist".to_string()],
                });
                continue;
            }

            results.push(lint_single_file(
                &check.name,
                Path::new(&check.workflow),
                true,
                &workflow_path,
            )?);
        }

        results
    };

    let passed = results.iter().all(|result| result.passed);
    let mode = if config.fixture.is_some() {
        "fixture".to_string()
    } else {
        "policy".to_string()
    };
    let receipt = LintReceipt { passed, mode, checks: results };

    if let Some(path) = &config.receipt {
        let receipt_path = resolve_input_path(&root, path);
        if let Some(parent) = receipt_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        fs::write(&receipt_path, serde_json::to_string_pretty(&receipt)?)
            .with_context(|| format!("writing receipt {}", receipt_path.display()))?;
    }

    match config.format {
        WorkflowTriggerLintFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&receipt)?);
        }
        WorkflowTriggerLintFormat::Text => {
            if receipt.passed {
                println!("✓ workflow-trigger-lint passed");
            } else {
                println!("❌ workflow-trigger-lint failed");
                for check in &receipt.checks {
                    if !check.passed {
                        println!("- {} ({})", check.policy_name, check.workflow);
                        for violation in &check.violations {
                            println!("  - {}", violation);
                        }
                    }
                }
            }
        }
    }

    if !receipt.passed {
        return Err(eyre!("workflow-trigger-lint failed"));
    }

    Ok(())
}

fn resolve_input_path(root: &Path, input: &Path) -> PathBuf {
    if input.is_absolute() { input.to_path_buf() } else { root.join(input) }
}

fn lint_single_file(
    policy_name: &str,
    workflow: &Path,
    required: bool,
    workflow_path: &Path,
) -> Result<WorkflowLintResult> {
    let raw = fs::read_to_string(workflow_path)
        .with_context(|| format!("reading workflow file {}", workflow_path.display()))?;
    let parsed: Value = serde_yaml_ng::from_str(&raw)
        .with_context(|| format!("parsing workflow file {}", workflow_path.display()))?;

    let mut violations = Vec::new();

    if !has_event_trigger(&parsed, "pull_request") {
        violations.push("missing pull_request trigger".to_string());
    }
    if !has_event_trigger(&parsed, "merge_group") {
        violations.push("missing merge_group trigger".to_string());
    }
    if !has_push_master_trigger(&parsed) {
        violations.push("push trigger must include only/at least master branch".to_string());
    }
    if has_paths_filter(&parsed) {
        violations.push("workflow trigger cannot use paths/paths-ignore filters".to_string());
    }
    if !has_event_aware_concurrency(&parsed) {
        violations.push(format!(
            "concurrency.cancel-in-progress must be {}",
            REQUIRED_CANCEL_IN_PROGRESS
        ));
    }

    let passed = violations.is_empty();
    Ok(WorkflowLintResult {
        policy_name: policy_name.to_string(),
        workflow: workflow.display().to_string(),
        required,
        passed,
        violations,
    })
}

fn on_mapping(workflow: &Value) -> Option<&Mapping> {
    let root = workflow.as_mapping()?;
    root.get(Value::String("on".to_string()))
        .or_else(|| root.get(Value::Bool(true)))
        .and_then(Value::as_mapping)
}

fn has_event_trigger(workflow: &Value, event: &str) -> bool {
    let Some(root) = workflow.as_mapping() else {
        return false;
    };

    if let Some(on) = root
        .get(Value::String("on".to_string()))
        .or_else(|| root.get(Value::Bool(true)))
    {
        match on {
            Value::String(value) => value == event,
            Value::Sequence(values) => values.iter().any(|value| value.as_str() == Some(event)),
            Value::Mapping(values) => values.contains_key(Value::String(event.to_string())),
            _ => false,
        }
    } else {
        false
    }
}

fn has_push_master_trigger(workflow: &Value) -> bool {
    let Some(on) = on_mapping(workflow) else {
        return false;
    };

    let Some(push) = on.get(Value::String("push".to_string())) else {
        return false;
    };

    match push {
        Value::Mapping(mapping) => {
            let Some(branches) = mapping.get(Value::String("branches".to_string())) else {
                return false;
            };
            branches_includes_master(branches)
        }
        _ => false,
    }
}

fn branches_includes_master(branches: &Value) -> bool {
    match branches {
        Value::String(value) => value == "master",
        Value::Sequence(values) => values.iter().any(|value| value.as_str() == Some("master")),
        _ => false,
    }
}

fn has_paths_filter(workflow: &Value) -> bool {
    let Some(on) = on_mapping(workflow) else {
        return false;
    };

    if on.contains_key(Value::String("paths".to_string()))
        || on.contains_key(Value::String("paths-ignore".to_string()))
    {
        return true;
    }

    for value in on.values() {
        let Some(mapping) = value.as_mapping() else {
            continue;
        };

        if mapping.contains_key(Value::String("paths".to_string()))
            || mapping.contains_key(Value::String("paths-ignore".to_string()))
        {
            return true;
        }
    }

    false
}

fn has_event_aware_concurrency(workflow: &Value) -> bool {
    let Some(root) = workflow.as_mapping() else {
        return false;
    };

    let Some(concurrency) = root.get(Value::String("concurrency".to_string())) else {
        return false;
    };

    let Some(mapping) = concurrency.as_mapping() else {
        return false;
    };

    let Some(cancel) = mapping.get(Value::String("cancel-in-progress".to_string())) else {
        return false;
    };

    cancel
        .as_str()
        .is_some_and(|value| value.trim() == REQUIRED_CANCEL_IN_PROGRESS)
}

pub fn parse_format(format: &str) -> Result<WorkflowTriggerLintFormat> {
    match format {
        "text" => Ok(WorkflowTriggerLintFormat::Text),
        "json" => Ok(WorkflowTriggerLintFormat::Json),
        other => bail!("unsupported workflow-trigger-lint format: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn detects_missing_merge_group() -> Result<()> {
        let yaml = r#"on:
  pull_request:
  push:
    branches: [master]
concurrency:
  group: x
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}
"#;
        let value: Value = serde_yaml_ng::from_str(yaml)?;
        assert!(!has_event_trigger(&value, "merge_group"));
        Ok(())
    }

    #[test]
    fn detects_paths_filters() -> Result<()> {
        let yaml = r#"on:
  pull_request:
    paths:
      - "crates/**"
  merge_group:
  push:
    branches: [master]
concurrency:
  group: x
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}
"#;
        let value: Value = serde_yaml_ng::from_str(yaml)?;
        assert!(has_paths_filter(&value));
        Ok(())
    }

    #[test]
    fn json_format_parses() -> Result<()> {
        assert_eq!(parse_format("json")?, WorkflowTriggerLintFormat::Json);
        Ok(())
    }

    #[test]
    fn receipt_shape_serializes() -> Result<()> {
        let receipt = LintReceipt {
            passed: true,
            mode: "fixture".to_string(),
            checks: vec![WorkflowLintResult {
                policy_name: "fixture".to_string(),
                workflow: "x.yml".to_string(),
                required: true,
                passed: true,
                violations: Vec::new(),
            }],
        };
        let json = serde_json::to_value(receipt)?;
        assert_eq!(json["passed"], json!(true));
        Ok(())
    }
}
