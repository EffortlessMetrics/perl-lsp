//! Enforce trigger/concurrency policy for required CI workflows.

use std::fs;
use std::path::{Path, PathBuf};

use clap::ValueEnum;
use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_yaml_ng::Value;

use crate::utils::project_root;

const REQUIRED_CANCEL_IN_PROGRESS: &str =
    "${{ github.event_name == 'pull_request' && github.event.action == 'synchronize' }}";

#[derive(Debug, Clone, Deserialize)]
struct RequiredChecksPolicy {
    /// Required-shape workflows: the full trigger/concurrency contract below.
    check: Vec<PolicyCheck>,
    /// Branch-protection status-context inventory. These workflows are
    /// deliberately different shapes, so only the default-branch rule applies.
    #[serde(default)]
    checks: Vec<PolicyCheck>,
}

#[derive(Debug, Clone, Deserialize)]
struct PolicyCheck {
    name: String,
    workflow: String,
    /// Absent means advisory. Several `[[checks]]` inventory entries omit the
    /// field entirely, and `cargo xtask merge-ready` reads them the same way:
    /// only an explicit `required = true` is required.
    #[serde(default)]
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

    let shape = policy
        .check
        .into_iter()
        .filter(|entry| entry.required)
        .map(|entry| evaluate_required_workflow(root, entry, RuleSet::RequiredShape));
    let inventory = policy
        .checks
        .into_iter()
        .filter(|entry| entry.required)
        .map(|entry| evaluate_required_workflow(root, entry, RuleSet::DefaultBranchOnly));

    shape.chain(inventory).collect()
}

/// Which rules apply to one policy entry.
///
/// `Perl LSP Rust Small Result` owned a workflow whose `pull_request` filter
/// named `main`, a branch this repository does not have, so it never ran once
/// in the repository's history (#10109). It went uncaught because the lint read
/// only `[[check]]` entries, and because no rule looked at `pull_request`
/// branches at all — `push_targets_master` looked only at `push`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuleSet {
    /// The `ci.yml`-shaped contract: triggers, concurrency, no path filters.
    RequiredShape,
    /// Only: the workflow exists and can fire on pull requests into `master`.
    DefaultBranchOnly,
}

fn evaluate_fixture(fixture_path: &Path) -> Result<WorkflowEvaluation> {
    let workflow = read_workflow_yaml(fixture_path)?;
    Ok(evaluate_required_entry(
        "fixture",
        fixture_path.display().to_string(),
        true,
        fixture_path.exists(),
        Some(&workflow),
        RuleSet::RequiredShape,
    ))
}

fn evaluate_required_workflow(
    root: &Path,
    check: PolicyCheck,
    rules: RuleSet,
) -> Result<WorkflowEvaluation> {
    let workflow_path = root.join(&check.workflow);
    let workflow =
        if workflow_path.exists() { Some(read_workflow_yaml(&workflow_path)?) } else { None };

    Ok(evaluate_required_entry(
        &check.name,
        check.workflow,
        check.required,
        workflow_path.exists(),
        workflow.as_ref(),
        rules,
    ))
}

fn evaluate_required_entry(
    name: &str,
    workflow: String,
    required: bool,
    workflow_exists: bool,
    workflow_yaml: Option<&Value>,
    rules: RuleSet,
) -> WorkflowEvaluation {
    let mut violations = Vec::new();

    if required {
        if !workflow_exists {
            violations.push("workflow file does not exist".to_string());
        }

        if let Some(yaml) = workflow_yaml {
            // Both rule sets: a required context that cannot fire on a pull
            // request into this repository's default branch reports nothing,
            // and a check that never reports proves nothing.
            if !has_trigger(yaml, "pull_request") {
                violations.push("missing pull_request trigger".to_string());
            } else if !pull_request_targets_master(yaml) {
                violations.push("pull_request trigger must target master branch".to_string());
            }

            if rules == RuleSet::RequiredShape {
                if !has_trigger(yaml, "merge_group") {
                    violations.push("missing merge_group trigger".to_string());
                }
                if !push_targets_master(yaml) {
                    violations.push("push trigger must target master branch".to_string());
                }
                if has_path_filters(yaml) {
                    violations
                        .push("path filters are not allowed on required workflows".to_string());
                }
                if !has_event_aware_concurrency(yaml) {
                    violations.push(format!(
                        "concurrency.cancel-in-progress must be `{REQUIRED_CANCEL_IN_PROGRESS}`"
                    ));
                }
                if pull_request_has_label_triggers(yaml) {
                    violations.push(
                        "required CI workflows must not trigger on pull_request labeled/unlabeled"
                            .to_string(),
                    );
                }
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

/// Whether a `pull_request` trigger can fire on pull requests into `master`.
///
/// An absent `branches:` filter matches every base branch, which is why it
/// passes. Only an explicit filter that omits `master` is a violation: that is
/// the shape that silenced `Perl LSP Rust Small Result` for its whole lifetime
/// by naming `main`, which this repository does not have (#10109).
fn pull_request_targets_master(workflow: &Value) -> bool {
    let Some(Value::Mapping(on)) = get_on(workflow) else {
        // List form (`on: [pull_request]`) carries no branch filter.
        return true;
    };

    let Some(Value::Mapping(pull_request)) = on.get(Value::String("pull_request".to_string()))
    else {
        // `pull_request:` with a null or scalar body carries no filter.
        return true;
    };

    match pull_request.get(Value::String("branches".to_string())) {
        Some(branches) => branch_targets_master(branches),
        None => true,
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

fn pull_request_has_label_triggers(workflow: &Value) -> bool {
    get_on(workflow)
        .and_then(Value::as_mapping)
        .and_then(|on| on.get(Value::String("pull_request".to_string())))
        .and_then(Value::as_mapping)
        .and_then(|pr| pr.get(Value::String("types".to_string())))
        .and_then(Value::as_sequence)
        .is_some_and(|types| {
            types
                .iter()
                .filter_map(Value::as_str)
                .any(|event| event == "labeled" || event == "unlabeled")
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
            RuleSet::RequiredShape,
        );
        assert!(eval.ok, "violations were {:?}", eval.violations);
        Ok(())
    }

    #[test]
    fn labeled_pull_request_type_fixture_fails() -> Result<()> {
        let fixture = load_fixture("labeled-required.yml")?;
        let eval = evaluate_required_entry(
            "fixture",
            "fixture.yml".to_string(),
            true,
            true,
            Some(&fixture),
            RuleSet::RequiredShape,
        );
        assert!(!eval.ok);
        assert!(eval.violations.iter().any(|item| item.contains("labeled/unlabeled")));
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
            RuleSet::RequiredShape,
        );
        assert!(!eval.ok);
        assert!(eval.violations.iter().any(|item| item.contains("merge_group")));
        Ok(())
    }

    /// The #10109 shape: every other required rule satisfied, wrong base branch.
    #[test]
    fn pull_request_naming_only_main_fails_under_both_rule_sets() -> Result<()> {
        let fixture = load_fixture("pull-request-wrong-branch.yml")?;
        for rules in [RuleSet::RequiredShape, RuleSet::DefaultBranchOnly] {
            let eval = evaluate_required_entry(
                "fixture",
                "fixture.yml".to_string(),
                true,
                true,
                Some(&fixture),
                rules,
            );
            assert!(!eval.ok, "{rules:?} must reject a pull_request filter naming only main");
            assert!(
                eval.violations
                    .iter()
                    .any(|item| item == "pull_request trigger must target master branch"),
                "{rules:?} violations were {:?}",
                eval.violations
            );
        }
        Ok(())
    }

    /// The inventory rule set carries only the default-branch rule, so a
    /// workflow that is a different shape on purpose must still pass it.
    #[test]
    fn inventory_rule_set_ignores_the_required_shape_contract() -> Result<()> {
        let fixture = load_fixture("missing-merge-group.yml")?;
        let eval = evaluate_required_entry(
            "fixture",
            "fixture.yml".to_string(),
            true,
            true,
            Some(&fixture),
            RuleSet::DefaultBranchOnly,
        );
        assert!(eval.ok, "violations were {:?}", eval.violations);
        Ok(())
    }

    fn parse(yaml: &str) -> Value {
        serde_yaml_ng::from_str(yaml).expect("fixture yaml parses")
    }

    #[test]
    fn pull_request_branch_filter_boundary() {
        // Explicit filters: master present, in either position and either form.
        assert!(pull_request_targets_master(&parse(
            "on:\n  pull_request:\n    branches: [master]"
        )));
        assert!(pull_request_targets_master(&parse(
            "on:\n  pull_request:\n    branches: [master, main]"
        )));
        assert!(pull_request_targets_master(&parse(
            "on:\n  pull_request:\n    branches: [main, master]"
        )));
        assert!(pull_request_targets_master(&parse(
            "on:\n  pull_request:\n    branches:\n      - main\n      - master"
        )));
        assert!(pull_request_targets_master(&parse(
            "on:\n  pull_request:\n    branches: refs/heads/master"
        )));

        // Explicit filters that omit master.
        assert!(!pull_request_targets_master(&parse("on:\n  pull_request:\n    branches: [main]")));
        assert!(!pull_request_targets_master(&parse(
            "on:\n  pull_request:\n    branches: [main, develop]"
        )));
        assert!(!pull_request_targets_master(&parse("on:\n  pull_request:\n    branches: main")));

        // No filter at all matches every base branch, so it passes.
        assert!(pull_request_targets_master(&parse("on: [pull_request]")));
        assert!(pull_request_targets_master(&parse("on:\n  pull_request:\n    types: [opened]")));
        assert!(pull_request_targets_master(&parse("on:\n  pull_request:")));
    }
}
