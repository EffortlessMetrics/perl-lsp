//! CI workflow spend audit task.
//!
//! Mirrors `scripts/ci-audit-workflows.py` by checking workflow files for
//! pull-request-triggered jobs that do not have explicit gating (`if:`).

use color_eyre::eyre::{Context, Result, eyre};
use serde_yaml_ng::Value;
use std::fs;

use crate::utils::project_root;

const ALLOWED_WORKFLOWS: &[&str] = &[
    "ci.yml",
    "check-ignored.yml",
    "ci-security.yml",
    // Path-filtered to UX-relevant crates only; runs on all PRs touching LSP/DAP/extension
    "ux-regression-gate.yml",
    // publish-dry-run.yml is a PR gate that runs unconditionally on Cargo.toml
    // path changes. The `paths:` filter in the workflow trigger is the cost gate
    // (only triggers when Cargo.toml or publish scripts change). Adding an `if:`
    // on top of the paths filter would be redundant and defeat the purpose.
    "publish-dry-run.yml",
    // ci-gate-self-tests.yml is path-filtered to gate scripts only.
    // Runs only when the gate scripts or self-test scripts change.
    "ci-gate-self-tests.yml",
];

const ALLOWED_UNGATED_JOBS: &[&str] = &["tautology-check", "test-metrics", "fmt", "clippy"];

pub fn run() -> Result<()> {
    let root = project_root()?;
    let workflows_dir = root.join(".github/workflows");

    if !workflows_dir.exists() {
        println!("✓ No .github/workflows directory found");
        return Ok(());
    }

    let mut violations: Vec<String> = Vec::new();

    for entry in fs::read_dir(&workflows_dir)
        .with_context(|| format!("reading {}", workflows_dir.display()))?
    {
        let path = entry.context("reading workflow entry")?.path();
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_none_or(|ext| ext != "yml" && ext != "yaml")
        {
            continue;
        }

        let workflow_name = path.file_name().and_then(|p| p.to_str()).unwrap_or("<unknown>");
        if ALLOWED_WORKFLOWS.contains(&workflow_name) {
            continue;
        }

        let raw = fs::read_to_string(&path)
            .with_context(|| format!("reading workflow file {}", path.display()))?;
        let workflow: Value = match serde_yaml_ng::from_str(&raw) {
            Ok(value) => value,
            Err(err) => {
                violations.push(format!("{workflow_name}: YAML parse error: {err}"));
                continue;
            }
        };

        if !has_pr_trigger(&workflow) {
            continue;
        }

        if let Some(jobs) = workflow.get("jobs").and_then(Value::as_mapping) {
            for (job_name, job_cfg) in jobs {
                let Some(name) = job_name.as_str() else {
                    continue;
                };

                if ALLOWED_UNGATED_JOBS.contains(&name) {
                    continue;
                }

                let Some(job_cfg) = job_cfg.as_mapping() else {
                    continue;
                };

                if !job_cfg.contains_key(Value::String("if".to_string())) {
                    violations.push(format!(
                        "{workflow_name}:{name} - runs on PRs without if: condition"
                    ));
                }
            }
        }
    }

    if violations.is_empty() {
        println!("✓ CI workflow spend audit passed");
        return Ok(());
    }

    println!("❌ CI Spend Audit Failed");
    println!();
    println!("The following jobs run on every PR without gating:");
    for violation in violations {
        println!("  - {}", violation);
    }
    println!();
    println!("Fix by adding one of:");
    println!("  1. if: contains(github.event.pull_request.labels.*.name, 'ci:<label>')");
    println!("  2. Add job to ALLOWED_UNGATED_JOBS in `cargo xtask ci-audit-workflows`");
    println!("  3. Add workflow to ALLOWED_WORKFLOWS (if entire workflow is cheap)");
    Err(eyre!("CI workflow spend audit found ungated jobs"))
}

fn has_pr_trigger(workflow: &Value) -> bool {
    let on = workflow.get("on");
    let Some(on) = on else {
        return false;
    };

    match on {
        Value::Sequence(values) => {
            values.iter().any(|value| value.as_str() == Some("pull_request"))
        }
        Value::Mapping(values) => values.contains_key(Value::String("pull_request".to_string())),
        Value::String(value) => value == "pull_request",
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pr_trigger_matches_map_form() {
        let workflow = serde_yaml_ng::from_str(
            r#"on:
  pull_request:
    types: [opened, synchronize]
jobs:
  check:
    if: github.event_name == 'pull_request'
"#,
        )
        .expect("valid yaml");
        assert!(has_pr_trigger(&workflow));
    }

    #[test]
    fn pr_trigger_matches_list_form() {
        let workflow = serde_yaml_ng::from_str(r#"on: [push, pull_request]"#).expect("valid yaml");
        assert!(has_pr_trigger(&workflow));
    }

    #[test]
    fn pr_trigger_rejects_non_pr_trigger() {
        let workflow =
            serde_yaml_ng::from_str(r#"on: [push, workflow_dispatch]"#).expect("valid yaml");
        assert!(!has_pr_trigger(&workflow));
    }
}
