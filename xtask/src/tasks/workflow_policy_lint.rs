//! Workflow security/policy lint.

use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Context, Result, bail};
use serde::Serialize;
use serde_yaml_ng::Value;

const DEFAULT_RECEIPT_PATH: &str = "target/receipts/workflow-policy.json";
const PULL_REQUEST_WRITE_CONTENTS_ALLOWLIST: &[&str] = &["ci.yml"];
const REQUIRED_STYLE_WORKFLOWS: &[&str] = &["ci.yml", "workflow-policy.yml"];
const REQUIRED_MERGE_GROUP_ALLOWLIST: &[&str] = &["ci.yml"];

#[derive(Debug, Clone, Serialize)]
struct Finding {
    severity: &'static str,
    code: &'static str,
    workflow: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct Receipt {
    schema_version: u32,
    scanned_workflows: usize,
    violations: Vec<Finding>,
    warnings: Vec<Finding>,
}

pub fn run(receipt: Option<PathBuf>, fixture: Option<PathBuf>) -> Result<()> {
    let root = crate::utils::project_root()?;

    let workflows: Vec<PathBuf> = if let Some(fixture_path) = fixture {
        vec![fixture_path]
    } else {
        collect_workflows(&root.join(".github/workflows"))?
    };

    let mut violations = Vec::new();
    let mut warnings = Vec::new();

    for workflow in &workflows {
        let (mut file_violations, mut file_warnings) = lint_workflow(workflow)?;
        violations.append(&mut file_violations);
        warnings.append(&mut file_warnings);
    }

    for finding in &warnings {
        eprintln!("WARN [{}] {}: {}", finding.code, finding.workflow, finding.message);
    }

    if let Some(path) = receipt {
        write_receipt(path, workflows.len(), &violations, &warnings)?;
    }

    if !violations.is_empty() {
        for finding in &violations {
            eprintln!("FAIL [{}] {}: {}", finding.code, finding.workflow, finding.message);
        }
        bail!("workflow policy lint failed with {} violation(s)", violations.len());
    }

    println!(
        "workflow policy lint passed ({} workflows scanned, {} warning(s))",
        workflows.len(),
        warnings.len()
    );
    Ok(())
}

fn collect_workflows(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut workflows = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry.context("reading workflow entry")?.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("yml") {
            workflows.push(path);
        }
    }
    workflows.sort();
    Ok(workflows)
}

fn lint_workflow(path: &Path) -> Result<(Vec<Finding>, Vec<Finding>)> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let workflow: Value = serde_yaml_ng::from_str(&raw)
        .with_context(|| format!("parsing YAML {}", path.display()))?;

    let workflow_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string());

    let triggers = extract_triggers(&workflow);
    let mut violations = Vec::new();
    let mut warnings = Vec::new();

    let checkout_head = has_checkout_pr_head_ref(&workflow);
    let has_write_all = has_write_all_permissions(&workflow);
    let has_secrets = has_non_github_token_secret_reference(&raw);
    let pull_request_has_contents_write = has_pull_request_contents_write(&workflow);

    if has_trigger(&triggers, "pull_request_target") && checkout_head {
        violations.push(fail(
            "WF001",
            &workflow_name,
            "pull_request_target workflow checks out PR head ref",
        ));
    }

    if has_trigger(&triggers, "pull_request")
        && pull_request_has_contents_write
        && !is_allowlisted(&workflow_name, PULL_REQUEST_WRITE_CONTENTS_ALLOWLIST)
    {
        violations.push(fail(
            "WF002",
            &workflow_name,
            "pull_request workflow grants contents: write but is not allowlisted",
        ));
    }

    if has_write_all {
        violations.push(fail(
            "WF003",
            &workflow_name,
            "workflow or job uses permissions: write-all",
        ));
    }

    if checkout_head
        && has_secrets
        && (has_trigger(&triggers, "pull_request") || has_trigger(&triggers, "pull_request_target"))
    {
        violations.push(fail(
            "WF004",
            &workflow_name,
            "untrusted PR code can run with non-GITHUB_TOKEN secrets references",
        ));
    }

    if is_required_style_workflow(&workflow_name)
        && !has_trigger(&triggers, "merge_group")
        && !is_allowlisted(&workflow_name, REQUIRED_MERGE_GROUP_ALLOWLIST)
    {
        violations.push(fail(
            "WF005",
            &workflow_name,
            "required-style workflow must include merge_group trigger",
        ));
    }

    if is_required_style_workflow(&workflow_name) && has_self_path_filter(&workflow, &workflow_name)
    {
        violations.push(fail(
            "WF006",
            &workflow_name,
            "required-style workflow path-filters itself",
        ));
    }

    if is_required_style_workflow(&workflow_name)
        && has_blanket_cancel_in_progress_for_truth_runs(&workflow)
    {
        violations.push(fail(
            "WF007",
            &workflow_name,
            "cancel-in-progress blanket applies to main/master/merge_group truth runs",
        ));
    }

    for action in unpinned_third_party_actions(&workflow) {
        warnings.push(warn(
            "WF008",
            &workflow_name,
            format!("unpinned third-party action detected: {action}"),
        ));
    }

    Ok((violations, warnings))
}

fn extract_triggers(workflow: &Value) -> Vec<String> {
    let mut triggers = Vec::new();
    let on = workflow.get("on");

    if let Some(seq) = on.and_then(Value::as_sequence) {
        for event in seq {
            if let Some(name) = event.as_str() {
                triggers.push(name.to_string());
            }
        }
    } else if let Some(map) = on.and_then(Value::as_mapping) {
        for (key, _) in map {
            if let Some(name) = key.as_str() {
                triggers.push(name.to_string());
            }
        }
    } else if let Some(name) = on.and_then(Value::as_str) {
        triggers.push(name.to_string());
    }

    triggers
}

fn has_checkout_pr_head_ref(workflow: &Value) -> bool {
    let Some(jobs) = workflow.get("jobs").and_then(Value::as_mapping) else {
        return false;
    };

    for (_, job) in jobs {
        let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
            continue;
        };

        for step in steps {
            let uses_checkout = step
                .get("uses")
                .and_then(Value::as_str)
                .map(|uses| uses.starts_with("actions/checkout@"))
                .unwrap_or(false);
            if !uses_checkout {
                continue;
            }

            let ref_text = step
                .get("with")
                .and_then(Value::as_mapping)
                .and_then(|with| with.get(Value::String("ref".to_string())))
                .and_then(Value::as_str)
                .unwrap_or_default();

            if ref_text.contains("github.event.pull_request.head")
                || ref_text.contains("github.head_ref")
            {
                return true;
            }
        }
    }

    false
}

fn has_pull_request_contents_write(workflow: &Value) -> bool {
    let contents = collect_contents_permissions(workflow);
    contents.iter().any(|level| level == "write")
}

fn collect_contents_permissions(workflow: &Value) -> Vec<String> {
    let mut permissions = Vec::new();

    if let Some(top) = workflow.get("permissions") {
        collect_contents_permission_value(top, &mut permissions);
    }

    if let Some(jobs) = workflow.get("jobs").and_then(Value::as_mapping) {
        for (_, job) in jobs {
            if let Some(job_permissions) = job.get("permissions") {
                collect_contents_permission_value(job_permissions, &mut permissions);
            }
        }
    }

    permissions
}

fn collect_contents_permission_value(permissions: &Value, out: &mut Vec<String>) {
    if let Some(level) = permissions.as_str() {
        if level == "write-all" {
            out.push("write".to_string());
        }
        return;
    }

    if let Some(map) = permissions.as_mapping()
        && let Some(contents) =
            map.get(Value::String("contents".to_string())).and_then(Value::as_str)
    {
        out.push(contents.to_string());
    }
}

fn has_write_all_permissions(workflow: &Value) -> bool {
    if workflow
        .get("permissions")
        .and_then(Value::as_str)
        .map(|value| value == "write-all")
        .unwrap_or(false)
    {
        return true;
    }

    let Some(jobs) = workflow.get("jobs").and_then(Value::as_mapping) else {
        return false;
    };

    jobs.values().any(|job| {
        job.get("permissions")
            .and_then(Value::as_str)
            .map(|value| value == "write-all")
            .unwrap_or(false)
    })
}

fn has_non_github_token_secret_reference(raw: &str) -> bool {
    raw.contains("secrets.") && !raw.contains("secrets.GITHUB_TOKEN")
}

fn is_required_style_workflow(workflow_name: &str) -> bool {
    REQUIRED_STYLE_WORKFLOWS.contains(&workflow_name)
}

fn has_self_path_filter(workflow: &Value, workflow_name: &str) -> bool {
    let Some(on) = workflow.get("on").and_then(Value::as_mapping) else {
        return false;
    };

    for event_name in ["pull_request", "push"] {
        let Some(event) = on.get(Value::String(event_name.to_string())).and_then(Value::as_mapping)
        else {
            continue;
        };

        let Some(paths) =
            event.get(Value::String("paths".to_string())).and_then(Value::as_sequence)
        else {
            continue;
        };

        for path in paths {
            let Some(path_str) = path.as_str() else {
                continue;
            };
            if path_str == format!(".github/workflows/{workflow_name}") {
                return true;
            }
        }
    }

    false
}

fn has_blanket_cancel_in_progress_for_truth_runs(workflow: &Value) -> bool {
    let cancel_enabled = workflow
        .get("concurrency")
        .and_then(Value::as_mapping)
        .and_then(|concurrency| concurrency.get(Value::String("cancel-in-progress".to_string())))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if !cancel_enabled {
        return false;
    }

    let triggers = extract_triggers(workflow);
    if has_trigger(&triggers, "merge_group") {
        return true;
    }

    let Some(on) = workflow.get("on").and_then(Value::as_mapping) else {
        return false;
    };

    let Some(push) = on.get(Value::String("push".to_string())).and_then(Value::as_mapping) else {
        return false;
    };

    let Some(branches) =
        push.get(Value::String("branches".to_string())).and_then(Value::as_sequence)
    else {
        return false;
    };

    branches.iter().any(|branch| {
        branch.as_str().map(|name| name == "main" || name == "master").unwrap_or(false)
    })
}

fn unpinned_third_party_actions(workflow: &Value) -> Vec<String> {
    let mut actions = Vec::new();

    let Some(jobs) = workflow.get("jobs").and_then(Value::as_mapping) else {
        return actions;
    };

    for (_, job) in jobs {
        let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
            continue;
        };

        for step in steps {
            let Some(uses) = step.get("uses").and_then(Value::as_str) else {
                continue;
            };
            if uses.starts_with("actions/") {
                continue;
            }
            let Some((_, reference)) = uses.rsplit_once('@') else {
                continue;
            };
            if !is_pinned_sha(reference) {
                actions.push(uses.to_string());
            }
        }
    }

    actions.sort();
    actions.dedup();
    actions
}

fn is_pinned_sha(reference: &str) -> bool {
    reference.len() == 40 && reference.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn is_allowlisted(workflow_name: &str, allowlist: &[&str]) -> bool {
    allowlist.contains(&workflow_name)
}

fn has_trigger(triggers: &[String], trigger: &str) -> bool {
    triggers.iter().any(|candidate| candidate == trigger)
}

fn fail(code: &'static str, workflow: &str, message: &str) -> Finding {
    Finding {
        severity: "error",
        code,
        workflow: workflow.to_string(),
        message: message.to_string(),
    }
}

fn warn(code: &'static str, workflow: &str, message: String) -> Finding {
    Finding { severity: "warning", code, workflow: workflow.to_string(), message }
}

fn write_receipt(
    path: PathBuf,
    scanned_workflows: usize,
    violations: &[Finding],
    warnings: &[Finding],
) -> Result<()> {
    let receipt = Receipt {
        schema_version: 1,
        scanned_workflows,
        violations: violations.to_vec(),
        warnings: warnings.to_vec(),
    };

    let output =
        if path.as_os_str().is_empty() { PathBuf::from(DEFAULT_RECEIPT_PATH) } else { path };

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }

    fs::write(&output, format!("{}\n", serde_json::to_string_pretty(&receipt)?))
        .with_context(|| format!("writing {}", output.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::Result;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/workflow-policy").join(name)
    }

    #[test]
    fn pull_request_target_checkout_head_fails() -> Result<()> {
        let (violations, _) = lint_workflow(&fixture("pull_request_target_checkout_head.yml"))?;
        assert!(violations.iter().any(|finding| finding.code == "WF001"));
        Ok(())
    }

    #[test]
    fn pull_request_read_only_passes() -> Result<()> {
        let (violations, _) = lint_workflow(&fixture("pull_request_read_only.yml"))?;
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn write_all_fails() -> Result<()> {
        let (violations, _) = lint_workflow(&fixture("write_all.yml"))?;
        assert!(violations.iter().any(|finding| finding.code == "WF003"));
        Ok(())
    }
}
