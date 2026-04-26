use color_eyre::eyre::{Context, Result, bail};
use regex::Regex;
use serde::Serialize;
use serde_yaml_ng::Value;
use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::project_root;

const CONTENTS_WRITE_ALLOWLIST: &[&str] = &["ci.yml", "release-orchestration.yml"];
const WARN_ON_UNPINNED_THIRD_PARTY_ACTIONS: bool = true;

#[derive(Debug)]
pub struct WorkflowPolicyLintConfig {
    pub receipt: Option<PathBuf>,
    pub fixture: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize)]
struct Diagnostic {
    severity: Severity,
    rule: &'static str,
    workflow: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct Receipt {
    schema_version: &'static str,
    lint: &'static str,
    passed: bool,
    errors: usize,
    warnings: usize,
    diagnostics: Vec<Diagnostic>,
}

pub fn run(config: WorkflowPolicyLintConfig) -> Result<()> {
    let root = project_root()?;
    let mut diagnostics = Vec::new();

    if let Some(fixture) = config.fixture {
        lint_one_workflow(&root, &fixture, &mut diagnostics)?;
    } else {
        let workflows_dir = root.join(".github/workflows");
        for entry in fs::read_dir(&workflows_dir)
            .with_context(|| format!("reading {}", workflows_dir.display()))?
        {
            let path =
                entry.with_context(|| format!("reading {}", workflows_dir.display()))?.path();
            if path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_none_or(|ext| ext != "yml" && ext != "yaml")
            {
                continue;
            }
            lint_one_workflow(&root, &path, &mut diagnostics)?;
        }
    }

    let errors = diagnostics.iter().filter(|d| matches!(d.severity, Severity::Error)).count();
    let warnings = diagnostics.iter().filter(|d| matches!(d.severity, Severity::Warning)).count();

    for d in &diagnostics {
        let level = match d.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        println!("::{level} file={}::{} [{}]", d.workflow, d.message, d.rule);
    }

    if let Some(receipt_path) = config.receipt {
        let receipt = Receipt {
            schema_version: "1.0.0",
            lint: "workflow-policy",
            passed: errors == 0,
            errors,
            warnings,
            diagnostics,
        };
        let parent = receipt_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| root.join("target/receipts"));
        fs::create_dir_all(&parent)
            .with_context(|| format!("creating receipt directory {}", parent.display()))?;
        fs::write(&receipt_path, serde_json::to_string_pretty(&receipt)?)
            .with_context(|| format!("writing receipt {}", receipt_path.display()))?;
        println!("workflow-policy-lint receipt: {}", receipt_path.display());
    }

    if errors > 0 {
        bail!("workflow-policy-lint found {errors} error(s) and {warnings} warning(s)");
    }

    println!("workflow-policy-lint passed with {warnings} warning(s)");
    Ok(())
}

fn lint_one_workflow(
    root: &Path,
    workflow_path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<()> {
    let raw = fs::read_to_string(workflow_path)
        .with_context(|| format!("reading workflow file {}", workflow_path.display()))?;

    let value: Value = serde_yaml_ng::from_str(&raw)
        .with_context(|| format!("parsing workflow YAML {}", workflow_path.display()))?;

    let workflow = workflow_path.strip_prefix(root).unwrap_or(workflow_path).display().to_string();
    let filename =
        workflow_path.file_name().and_then(|name| name.to_str()).unwrap_or("<unknown>").to_string();

    let has_pr = has_trigger(&value, "pull_request");
    let has_pr_target = has_trigger(&value, "pull_request_target");
    let has_merge_group = has_trigger(&value, "merge_group");
    let required_style = has_pr && has_push_main_or_master(&value);

    if has_pr_target && checks_out_pr_head(&value) {
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            rule: "pull_request_target_checkout_head",
            workflow: workflow.clone(),
            message: "pull_request_target workflow checks out PR head ref/sha".to_string(),
        });
    }

    if has_pr
        && has_contents_write_permission(&value)
        && !CONTENTS_WRITE_ALLOWLIST.contains(&filename.as_str())
    {
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            rule: "pull_request_contents_write",
            workflow: workflow.clone(),
            message: "pull_request workflow declares contents: write without allowlist entry"
                .to_string(),
        });
    }

    if has_write_all_permission(&value) {
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            rule: "permissions_write_all",
            workflow: workflow.clone(),
            message: "workflow uses permissions: write-all".to_string(),
        });
    }

    if has_pr_target && checks_out_pr_head(&value) && raw.contains("secrets.") {
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            rule: "untrusted_pr_code_with_secrets",
            workflow: workflow.clone(),
            message: "untrusted PR code appears to access secrets in pull_request_target workflow"
                .to_string(),
        });
    }

    if required_style && !has_merge_group {
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            rule: "required_workflow_missing_merge_group",
            workflow: workflow.clone(),
            message: "required-style workflow is missing merge_group trigger".to_string(),
        });
    }

    if required_style && workflow_filters_itself(&value, &workflow) {
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            rule: "required_workflow_self_path_filter",
            workflow: workflow.clone(),
            message: "required-style workflow path filters itself".to_string(),
        });
    }

    if required_style && blanket_cancel_in_progress(&value) {
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            rule: "blanket_cancel_in_progress_on_truth_runs",
            workflow: workflow.clone(),
            message: "required-style workflow sets blanket cancel-in-progress for master/merge_group truth runs"
                .to_string(),
        });
    }

    if WARN_ON_UNPINNED_THIRD_PARTY_ACTIONS {
        for action in unpinned_third_party_actions(&value)? {
            diagnostics.push(Diagnostic {
                severity: Severity::Warning,
                rule: "unpinned_third_party_action",
                workflow: workflow.clone(),
                message: format!("third-party action is not commit-pinned: {action}"),
            });
        }
    }

    Ok(())
}

fn has_trigger(workflow: &Value, event: &str) -> bool {
    let Some(on) = workflow_on_value(workflow) else {
        return false;
    };

    match on {
        Value::String(value) => value == event,
        Value::Sequence(sequence) => sequence.iter().any(|v| v.as_str() == Some(event)),
        Value::Mapping(mapping) => mapping.contains_key(Value::String(event.to_string())),
        _ => false,
    }
}

fn has_push_main_or_master(workflow: &Value) -> bool {
    let Some(on) = workflow_on_value(workflow) else {
        return false;
    };
    let Value::Mapping(mapping) = on else {
        return false;
    };

    let Some(push) = mapping.get(Value::String("push".to_string())) else {
        return false;
    };

    if let Some(branches) = push
        .as_mapping()
        .and_then(|m| m.get(Value::String("branches".to_string())))
        .and_then(Value::as_sequence)
    {
        return branches.iter().filter_map(Value::as_str).any(|b| b == "main" || b == "master");
    }

    true
}

fn workflow_filters_itself(workflow: &Value, workflow_path: &str) -> bool {
    let Some(on) = workflow_on_value(workflow) else {
        return false;
    };
    let Value::Mapping(mapping) = on else {
        return false;
    };

    ["pull_request", "push"].iter().any(|event| {
        mapping.get(Value::String((*event).to_string())).and_then(Value::as_mapping).is_some_and(
            |event_cfg| {
                ["paths", "paths-ignore"].iter().any(|key| {
                    event_cfg
                        .get(Value::String((*key).to_string()))
                        .and_then(Value::as_sequence)
                        .is_some_and(|paths| {
                            paths.iter().filter_map(Value::as_str).any(|path| path == workflow_path)
                        })
                })
            },
        )
    })
}

fn blanket_cancel_in_progress(workflow: &Value) -> bool {
    workflow
        .get("concurrency")
        .and_then(Value::as_mapping)
        .and_then(|concurrency| concurrency.get(Value::String("cancel-in-progress".to_string())))
        .is_some_and(|value| match value {
            Value::Bool(flag) => *flag,
            Value::String(flag) => flag.trim() == "true",
            _ => false,
        })
}

fn has_contents_write_permission(workflow: &Value) -> bool {
    permission_value_is_write(workflow.get("permissions"), "contents")
        || workflow.get("jobs").and_then(Value::as_mapping).is_some_and(|jobs| {
            jobs.values().any(|job| {
                permission_value_is_write(
                    job.as_mapping().and_then(|j| j.get(Value::String("permissions".to_string()))),
                    "contents",
                )
            })
        })
}

fn has_write_all_permission(workflow: &Value) -> bool {
    is_write_all(workflow.get("permissions"))
        || workflow.get("jobs").and_then(Value::as_mapping).is_some_and(|jobs| {
            jobs.values().any(|job| {
                is_write_all(
                    job.as_mapping().and_then(|j| j.get(Value::String("permissions".to_string()))),
                )
            })
        })
}

fn permission_value_is_write(permissions: Option<&Value>, scope: &str) -> bool {
    let Some(permissions) = permissions else {
        return false;
    };

    if let Value::String(s) = permissions {
        return s == "write-all";
    }

    permissions
        .as_mapping()
        .and_then(|m| m.get(Value::String(scope.to_string())))
        .and_then(Value::as_str)
        .is_some_and(|value| value == "write")
}

fn is_write_all(permissions: Option<&Value>) -> bool {
    permissions.and_then(Value::as_str).is_some_and(|p| p == "write-all")
}

fn checks_out_pr_head(workflow: &Value) -> bool {
    workflow.get("jobs").and_then(Value::as_mapping).is_some_and(|jobs| {
        jobs.values().any(|job| {
            job.as_mapping()
                .and_then(|job_map| job_map.get(Value::String("steps".to_string())))
                .and_then(Value::as_sequence)
                .is_some_and(|steps| {
                    steps.iter().any(|step| {
                        let Some(step_map) = step.as_mapping() else {
                            return false;
                        };
                        let uses_checkout = step_map
                            .get(Value::String("uses".to_string()))
                            .and_then(Value::as_str)
                            .is_some_and(|uses| uses.starts_with("actions/checkout@"));

                        if !uses_checkout {
                            return false;
                        }

                        step_map
                            .get(Value::String("with".to_string()))
                            .and_then(Value::as_mapping)
                            .and_then(|with| with.get(Value::String("ref".to_string())))
                            .and_then(Value::as_str)
                            .is_some_and(|r| r.contains("github.event.pull_request.head"))
                    })
                })
        })
    })
}

fn unpinned_third_party_actions(workflow: &Value) -> Result<Vec<String>> {
    let pinned_sha = Regex::new(r"^[a-f0-9]{40}$").context("compile pin regex")?;
    let mut warnings = Vec::new();

    if let Some(jobs) = workflow.get("jobs").and_then(Value::as_mapping) {
        for job in jobs.values() {
            let Some(steps) = job
                .as_mapping()
                .and_then(|j| j.get(Value::String("steps".to_string())))
                .and_then(Value::as_sequence)
            else {
                continue;
            };

            for step in steps {
                let Some(uses) = step
                    .as_mapping()
                    .and_then(|m| m.get(Value::String("uses".to_string())))
                    .and_then(Value::as_str)
                else {
                    continue;
                };

                if uses.starts_with("./")
                    || uses.starts_with("docker://")
                    || uses.starts_with("actions/")
                {
                    continue;
                }

                let Some((_, reference)) = uses.split_once('@') else {
                    warnings.push(uses.to_string());
                    continue;
                };

                if !pinned_sha.is_match(reference) {
                    warnings.push(uses.to_string());
                }
            }
        }
    }

    Ok(warnings)
}

fn workflow_on_value(workflow: &Value) -> Option<&Value> {
    workflow.get("on").or_else(|| {
        workflow.as_mapping().and_then(|mapping| {
            mapping.get(Value::Bool(true)).or_else(|| mapping.get(Value::String("on".to_string())))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_pr_target_checkout_head_fails() -> Result<()> {
        let root = project_root()?;
        let fixture =
            root.join("xtask/tests/fixtures/workflow-policy/pull_request_target_checkout_head.yml");
        let mut diagnostics = Vec::new();
        lint_one_workflow(&root, &fixture, &mut diagnostics)?;
        assert!(diagnostics.iter().any(|d| d.rule == "pull_request_target_checkout_head"));
        Ok(())
    }

    #[test]
    fn fixture_pull_request_read_only_passes() -> Result<()> {
        let root = project_root()?;
        let fixture = root.join("xtask/tests/fixtures/workflow-policy/pull_request_read_only.yml");
        let mut diagnostics = Vec::new();
        lint_one_workflow(&root, &fixture, &mut diagnostics)?;
        assert!(diagnostics.iter().all(|d| !matches!(d.severity, Severity::Error)));
        Ok(())
    }

    #[test]
    fn fixture_write_all_fails() -> Result<()> {
        let root = project_root()?;
        let fixture = root.join("xtask/tests/fixtures/workflow-policy/write_all.yml");
        let mut diagnostics = Vec::new();
        lint_one_workflow(&root, &fixture, &mut diagnostics)?;
        assert!(diagnostics.iter().any(|d| d.rule == "permissions_write_all"));
        Ok(())
    }
}
