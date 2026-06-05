//! Contract tests for final quality-gate CLI behavior.

use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::Command as StdCommand,
};

use assert_cmd::Command;
use serde_json::{Value, json};
use tempfile::tempdir;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

#[test]
fn quality_gate_final_enforce_passes_with_complete_current_proof() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let paths = FixturePaths::new(dir.path());
    let head = current_head(&root)?;

    write_coverage_receipt(&paths.coverage, &head, 97.2, 95.4, "workspace")?;
    write_final_codecov(&paths.codecov)?;
    write_ripr_plus_receipt(&paths.ripr, &head, 0)?;
    write_ripr_pr_receipt(&paths.ripr_pr, &head, 0)?;
    write_empty_review_guidance_receipt(&paths.review, &head)?;
    write_empty_exception_policy(&paths.exceptions)?;

    final_quality_gate_command(&root, &paths)?.assert().success();

    let payload: Value = serde_json::from_str(&fs::read_to_string(&paths.receipt)?)?;
    assert_eq!(payload.pointer("/mode").and_then(Value::as_str), Some("enforce"));
    assert_eq!(payload.pointer("/decision").and_then(Value::as_str), Some("pass"));
    assert_eq!(payload.pointer("/coverage/project").and_then(Value::as_f64), Some(95.4));
    assert_eq!(payload.pointer("/coverage/scope").and_then(Value::as_str), Some("workspace"));
    assert_eq!(payload.pointer("/ripr_plus/unresolved").and_then(Value::as_u64), Some(0));
    assert_eq!(payload.pointer("/ripr_pr/new_unresolved").and_then(Value::as_u64), Some(0));
    assert_eq!(
        payload.pointer("/temporary_exceptions/active_count").and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(payload.get("next_actions").and_then(Value::as_array).map(Vec::len), Some(0));

    final_quality_gate_command(&root, &paths)?.arg("--check").assert().success();

    Ok(())
}

#[test]
fn quality_gate_final_check_blocks_stale_json_receipt() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let paths = FixturePaths::new(dir.path());
    let head = current_head(&root)?;

    write_coverage_receipt(&paths.coverage, &head, 97.2, 95.4, "workspace")?;
    write_final_codecov(&paths.codecov)?;
    write_ripr_plus_receipt(&paths.ripr, &head, 0)?;
    write_ripr_pr_receipt(&paths.ripr_pr, &head, 0)?;
    write_empty_review_guidance_receipt(&paths.review, &head)?;
    write_empty_exception_policy(&paths.exceptions)?;
    final_quality_gate_command(&root, &paths)?.assert().success();

    fs::write(
        &paths.receipt,
        serde_json::to_string_pretty(&json!({
            "schema_version": 1,
            "kind": "quality_gate",
            "mode": "enforce",
            "decision": "pass",
            "head": "stale-quality-gate-head"
        }))?,
    )?;

    let output = final_quality_gate_command(&root, &paths)?.arg("--check").output()?;
    assert!(!output.status.success(), "quality-gate --check must fail on stale JSON proof");

    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("quality gate JSON receipt is stale")
            && stderr.contains(&paths.receipt.to_string_lossy().to_string()),
        "stale JSON receipt failure must name the stale proof file: {stderr}"
    );

    Ok(())
}

#[test]
fn quality_gate_final_check_blocks_stale_markdown_summary() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let paths = FixturePaths::new(dir.path());
    let head = current_head(&root)?;

    write_coverage_receipt(&paths.coverage, &head, 97.2, 95.4, "workspace")?;
    write_final_codecov(&paths.codecov)?;
    write_ripr_plus_receipt(&paths.ripr, &head, 0)?;
    write_ripr_pr_receipt(&paths.ripr_pr, &head, 0)?;
    write_empty_review_guidance_receipt(&paths.review, &head)?;
    write_empty_exception_policy(&paths.exceptions)?;
    final_quality_gate_command(&root, &paths)?.assert().success();

    fs::write(&paths.summary, "# Quality Gate\n\nstale summary\n")?;

    let output = final_quality_gate_command(&root, &paths)?.arg("--check").output()?;
    assert!(!output.status.success(), "quality-gate --check must fail on stale Markdown proof");

    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("quality gate Markdown summary is stale")
            && stderr.contains(&paths.summary.to_string_lossy().to_string()),
        "stale Markdown summary failure must name the stale proof file: {stderr}"
    );

    Ok(())
}

#[test]
fn quality_gate_final_enforce_blocks_total_ripr_project_coverage_and_active_exceptions()
-> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let paths = FixturePaths::new(dir.path());
    let head = current_head(&root)?;

    write_coverage_receipt(&paths.coverage, &head, 97.2, 94.9, "workspace")?;
    write_final_codecov(&paths.codecov)?;
    write_ripr_plus_receipt(&paths.ripr, &head, 2)?;
    write_ripr_pr_receipt(&paths.ripr_pr, &head, 0)?;
    write_empty_review_guidance_receipt(&paths.review, &head)?;
    write_active_exception_policy(&paths.exceptions)?;

    let output = final_quality_gate_command(&root, &paths)?.output()?;
    assert!(!output.status.success(), "final enforcement must fail while final debt remains");

    let payload: Value = serde_json::from_str(&fs::read_to_string(&paths.receipt)?)?;
    let ripr = next_action(&payload, "ripr_total_unresolved")?;
    assert_eq!(ripr.get("unresolved").and_then(Value::as_u64), Some(2));
    assert_repair_contract(ripr)?;
    let project = next_action(&payload, "project_coverage_below_target")?;
    assert_eq!(project.get("current").and_then(Value::as_f64), Some(94.9));
    assert_repair_contract(project)?;
    let exception = next_action(&payload, "quality_exception_active_final_blocker")?;
    assert_eq!(
        exception.pointer("/active/0/id").and_then(Value::as_str),
        Some("ripr-total-burndown")
    );
    assert_repair_contract(exception)?;

    Ok(())
}

#[test]
fn quality_gate_final_enforce_blocks_missing_receipts() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let paths = FixturePaths::new(dir.path());

    write_final_codecov(&paths.codecov)?;
    write_empty_exception_policy(&paths.exceptions)?;

    let output = final_quality_gate_command(&root, &paths)?.output()?;
    assert!(!output.status.success(), "final enforcement must fail without proof receipts");

    let payload: Value = serde_json::from_str(&fs::read_to_string(&paths.receipt)?)?;
    for kind in [
        "coverage_receipt_not_current",
        "ripr_receipt_not_current",
        "ripr_pr_receipt_not_current",
        "ripr_review_receipt_not_current",
    ] {
        assert_repair_contract(next_action(&payload, kind)?)?;
    }

    Ok(())
}

#[test]
fn quality_gate_final_exception_policy_action_uses_final_mode_commands() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let paths = FixturePaths::new(dir.path());
    let head = current_head(&root)?;

    write_coverage_receipt(&paths.coverage, &head, 97.2, 95.4, "workspace")?;
    write_final_codecov(&paths.codecov)?;
    write_ripr_plus_receipt(&paths.ripr, &head, 0)?;
    write_ripr_pr_receipt(&paths.ripr_pr, &head, 0)?;
    write_empty_review_guidance_receipt(&paths.review, &head)?;

    let output = final_quality_gate_command(&root, &paths)?.output()?;
    assert!(!output.status.success(), "final enforcement must fail without exception policy");

    let payload: Value = serde_json::from_str(&fs::read_to_string(&paths.receipt)?)?;
    let action = next_action(&payload, "quality_exception_policy_not_current")?;
    assert_eq!(action.get("reason").and_then(Value::as_str), Some("missing"));
    assert_repair_contract(action)?;
    assert_action_commands_use_quality_gate_mode(action, "enforce")?;
    for field in ["verify", "receipt"] {
        let command = action.get(field).and_then(Value::as_str).ok_or("missing command")?;
        assert!(
            command.contains("--ripr-receipt") && command.contains("--ripr-pr-receipt"),
            "final exception policy {field} command must include full final proof inputs: {command}"
        );
    }

    Ok(())
}

#[test]
fn quality_gate_final_invalid_exception_action_uses_final_mode_commands() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let paths = FixturePaths::new(dir.path());
    let head = current_head(&root)?;

    write_coverage_receipt(&paths.coverage, &head, 97.2, 95.4, "workspace")?;
    write_final_codecov(&paths.codecov)?;
    write_ripr_plus_receipt(&paths.ripr, &head, 0)?;
    write_ripr_pr_receipt(&paths.ripr_pr, &head, 0)?;
    write_empty_review_guidance_receipt(&paths.review, &head)?;
    write_invalid_exception_policy(&paths.exceptions)?;

    let output = final_quality_gate_command(&root, &paths)?.output()?;
    assert!(!output.status.success(), "final enforcement must fail on invalid exception policy");

    let payload: Value = serde_json::from_str(&fs::read_to_string(&paths.receipt)?)?;
    let action = next_action(&payload, "quality_exception_invalid")?;
    assert_eq!(action.get("id").and_then(Value::as_str), Some("ripr-total-burndown"));
    assert_repair_contract(action)?;
    assert_action_commands_use_quality_gate_mode(action, "enforce")?;
    for field in ["verify", "receipt"] {
        let command = action.get(field).and_then(Value::as_str).ok_or("missing command")?;
        assert!(
            command.contains("--coverage-receipt")
                && command.contains("--ripr-receipt")
                && command.contains("--ripr-pr-receipt")
                && command.contains("--review-receipt"),
            "final invalid exception {field} command must include full final proof inputs: {command}"
        );
    }

    Ok(())
}

#[test]
fn quality_gate_final_invalid_exception_policy_toml_uses_final_mode_commands() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let paths = FixturePaths::new(dir.path());

    write_complete_final_proof_inputs(&root, &paths)?;
    write_invalid_toml_exception_policy(&paths.exceptions)?;

    let output = final_quality_gate_command(&root, &paths)?.output()?;
    assert!(
        !output.status.success(),
        "final enforcement must fail when exception policy TOML is invalid"
    );

    let payload: Value = serde_json::from_str(&fs::read_to_string(&paths.receipt)?)?;
    let action = next_action(&payload, "quality_exception_policy_not_current")?;
    assert_eq!(action.get("reason").and_then(Value::as_str), Some("invalid_toml"));
    assert_repair_contract(action)?;
    assert_action_commands_use_quality_gate_mode(action, "enforce")?;

    Ok(())
}

#[test]
fn quality_gate_final_invalid_exception_policy_header_uses_final_mode_commands() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let paths = FixturePaths::new(dir.path());

    write_complete_final_proof_inputs(&root, &paths)?;
    write_invalid_header_exception_policy(&paths.exceptions)?;

    let output = final_quality_gate_command(&root, &paths)?.output()?;
    assert!(
        !output.status.success(),
        "final enforcement must fail when exception policy header is invalid"
    );

    let payload: Value = serde_json::from_str(&fs::read_to_string(&paths.receipt)?)?;
    let action = next_action(&payload, "quality_exception_policy_not_current")?;
    assert_eq!(action.get("reason").and_then(Value::as_str), Some("invalid_header"));
    assert_repair_contract(action)?;
    assert_action_commands_use_quality_gate_mode(action, "enforce")?;

    Ok(())
}

#[test]
fn quality_gate_final_invalid_exception_policy_metadata_uses_final_mode_commands() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let paths = FixturePaths::new(dir.path());

    write_complete_final_proof_inputs(&root, &paths)?;
    write_invalid_metadata_exception_policy(&paths.exceptions)?;

    let output = final_quality_gate_command(&root, &paths)?.output()?;
    assert!(
        !output.status.success(),
        "final enforcement must fail when exception policy metadata is invalid"
    );

    let payload: Value = serde_json::from_str(&fs::read_to_string(&paths.receipt)?)?;
    let action = next_action(&payload, "quality_exception_policy_not_current")?;
    assert_eq!(action.get("reason").and_then(Value::as_str), Some("invalid_metadata"));
    assert_repair_contract(action)?;
    assert_action_commands_use_quality_gate_mode(action, "enforce")?;

    Ok(())
}

#[test]
fn quality_gate_final_invalid_exception_dates_use_final_mode_commands() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let paths = FixturePaths::new(dir.path());

    write_complete_final_proof_inputs(&root, &paths)?;
    write_invalid_date_exception_policy(&paths.exceptions)?;

    let output = final_quality_gate_command(&root, &paths)?.output()?;
    assert!(
        !output.status.success(),
        "final enforcement must fail when exception policy dates are invalid"
    );

    let payload: Value = serde_json::from_str(&fs::read_to_string(&paths.receipt)?)?;
    let action = next_action(&payload, "quality_exception_invalid")?;
    assert_eq!(action.get("id").and_then(Value::as_str), Some("ripr-total-burndown"));
    let reason = action.get("reason").and_then(Value::as_str).unwrap_or_default();
    assert!(reason.contains("created, review_after, and expires must use YYYY-MM-DD"), "{reason}");
    assert_repair_contract(action)?;
    assert_action_commands_use_quality_gate_mode(action, "enforce")?;

    Ok(())
}

#[test]
fn quality_gate_final_enforce_blocks_invalid_receipts() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let paths = FixturePaths::new(dir.path());

    write_final_codecov(&paths.codecov)?;
    write_empty_exception_policy(&paths.exceptions)?;
    fs::write(&paths.coverage, "{not-json")?;
    fs::write(&paths.ripr, "{not-json")?;
    fs::write(&paths.ripr_pr, "{not-json")?;
    fs::write(&paths.review, "{not-json")?;

    let output = final_quality_gate_command(&root, &paths)?.output()?;
    assert!(!output.status.success(), "final enforcement must fail on invalid proof receipts");

    let payload: Value = serde_json::from_str(&fs::read_to_string(&paths.receipt)?)?;
    assert_eq!(payload.pointer("/coverage/status").and_then(Value::as_str), Some("invalid"));
    assert_eq!(payload.pointer("/ripr_plus/status").and_then(Value::as_str), Some("invalid"));
    assert_eq!(payload.pointer("/ripr_pr/status").and_then(Value::as_str), Some("invalid"));
    assert_eq!(payload.pointer("/review_guidance/status").and_then(Value::as_str), Some("invalid"));

    for kind in [
        "coverage_receipt_not_current",
        "ripr_receipt_not_current",
        "ripr_pr_receipt_not_current",
        "ripr_review_receipt_not_current",
    ] {
        let action = next_action(&payload, kind)?;
        assert_eq!(action.get("reason").and_then(Value::as_str), Some("invalid"));
        assert_repair_contract(action)?;
    }

    Ok(())
}

#[test]
fn quality_gate_final_enforce_blocks_advisory_project_policy_and_partial_scope() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let paths = FixturePaths::new(dir.path());
    let head = current_head(&root)?;

    write_coverage_receipt(&paths.coverage, &head, 97.2, 96.0, "xtask")?;
    write_advisory_project_codecov(&paths.codecov)?;
    write_ripr_plus_receipt(&paths.ripr, &head, 0)?;
    write_ripr_pr_receipt(&paths.ripr_pr, &head, 0)?;
    write_empty_review_guidance_receipt(&paths.review, &head)?;
    write_empty_exception_policy(&paths.exceptions)?;

    let output = final_quality_gate_command(&root, &paths)?.output()?;
    assert!(!output.status.success(), "final enforcement must require blocking project policy");

    let payload: Value = serde_json::from_str(&fs::read_to_string(&paths.receipt)?)?;
    assert_repair_contract(next_action(&payload, "codecov_project_policy_not_blocking")?)?;
    let scope = next_action(&payload, "coverage_scope_not_workspace")?;
    assert!(
        scope.get("reason").and_then(Value::as_str).is_some_and(|reason| reason.contains("xtask")),
        "scope failure must name the non-workspace scope: {scope}"
    );
    assert_repair_contract(scope)?;

    Ok(())
}

#[test]
fn quality_gate_final_enforce_blocks_unknown_final_metrics() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let paths = FixturePaths::new(dir.path());
    let head = current_head(&root)?;

    write_coverage_receipt_without_final_metrics(&paths.coverage, &head)?;
    write_final_codecov(&paths.codecov)?;
    write_ripr_plus_receipt_without_unresolved(&paths.ripr, &head)?;
    write_ripr_pr_receipt_without_severe_gaps(&paths.ripr_pr, &head)?;
    write_empty_review_guidance_receipt(&paths.review, &head)?;
    write_empty_exception_policy(&paths.exceptions)?;

    let output = final_quality_gate_command(&root, &paths)?.output()?;
    assert!(!output.status.success(), "final enforcement must fail on unknown final metrics");

    let payload: Value = serde_json::from_str(&fs::read_to_string(&paths.receipt)?)?;
    for kind in [
        "patch_coverage_unknown",
        "project_coverage_unknown",
        "ripr_total_unknown",
        "new_ripr_gap_unknown",
    ] {
        assert_repair_contract(next_action(&payload, kind)?)?;
    }

    Ok(())
}

struct FixturePaths {
    coverage: PathBuf,
    codecov: PathBuf,
    ripr: PathBuf,
    ripr_pr: PathBuf,
    review: PathBuf,
    exceptions: PathBuf,
    receipt: PathBuf,
    summary: PathBuf,
}

impl FixturePaths {
    fn new(root: &Path) -> Self {
        Self {
            coverage: root.join("coverage-baseline.json"),
            codecov: root.join("codecov.yml"),
            ripr: root.join("ripr-plus.json"),
            ripr_pr: root.join("repo-exposure.json"),
            review: root.join("comments.json"),
            exceptions: root.join("quality-gate-exceptions.toml"),
            receipt: root.join("quality-gate.json"),
            summary: root.join("quality-gate.md"),
        }
    }
}

fn final_quality_gate_command(root: &Path, paths: &FixturePaths) -> TestResult<Command> {
    let mut command = Command::cargo_bin("xtask")?;
    command.current_dir(root).args(["quality-gate", "--mode", "enforce"]);
    command.arg("--coverage-receipt").arg(&paths.coverage);
    command.arg("--codecov").arg(&paths.codecov);
    command.arg("--ripr-receipt").arg(&paths.ripr);
    command.arg("--ripr-pr-receipt").arg(&paths.ripr_pr);
    command.arg("--review-receipt").arg(&paths.review);
    command.arg("--exception-policy").arg(&paths.exceptions);
    command.arg("--receipt").arg(&paths.receipt);
    command.arg("--summary").arg(&paths.summary);
    Ok(command)
}

fn write_complete_final_proof_inputs(root: &Path, paths: &FixturePaths) -> TestResult {
    let head = current_head(root)?;
    write_coverage_receipt(&paths.coverage, &head, 97.2, 95.4, "workspace")?;
    write_final_codecov(&paths.codecov)?;
    write_ripr_plus_receipt(&paths.ripr, &head, 0)?;
    write_ripr_pr_receipt(&paths.ripr_pr, &head, 0)?;
    write_empty_review_guidance_receipt(&paths.review, &head)?;
    Ok(())
}

fn write_coverage_receipt(
    path: &Path,
    head: &str,
    patch: f64,
    project: f64,
    scope: &str,
) -> TestResult {
    write_json(
        path,
        json!({
            "schema_version": 1,
            "kind": "coverage_baseline",
            "head": head,
            "scope": scope,
            "lcov": "target/lcov.info",
            "coverage": {
                "patch": patch,
                "project": project
            },
            "files_below_target": [
                {
                    "path": "xtask/src/tasks/quality_gate.rs",
                    "line_coverage": 72.0,
                    "sample_uncovered_lines": [41, 42, 43]
                }
            ]
        }),
    )
}

fn write_coverage_receipt_without_final_metrics(path: &Path, head: &str) -> TestResult {
    write_json(
        path,
        json!({
            "schema_version": 1,
            "kind": "coverage_baseline",
            "head": head,
            "scope": "workspace",
            "lcov": "target/lcov.info",
            "coverage": {},
            "files_below_target": []
        }),
    )
}

fn write_final_codecov(path: &Path) -> TestResult {
    write_text(
        path,
        r#"coverage:
  status:
    project:
      default:
        target: 95%
        threshold: 0.25%
    patch:
      default:
        target: 95%
        threshold: 0%
"#,
    )
}

fn write_advisory_project_codecov(path: &Path) -> TestResult {
    write_text(
        path,
        r#"coverage:
  status:
    project:
      default:
        target: 95%
        threshold: 2%
        informational: true
    patch:
      default:
        target: 95%
        threshold: 0%
"#,
    )
}

fn write_ripr_plus_receipt(path: &Path, head: &str, unresolved: u64) -> TestResult {
    write_json(
        path,
        json!({
            "schema_version": 1,
            "kind": "ripr_plus_baseline",
            "head": head,
            "unresolved": unresolved
        }),
    )
}

fn write_ripr_plus_receipt_without_unresolved(path: &Path, head: &str) -> TestResult {
    write_json(
        path,
        json!({
            "schema_version": 1,
            "kind": "ripr_plus_baseline",
            "head": head
        }),
    )
}

fn write_ripr_pr_receipt(path: &Path, head: &str, severe_gaps: u64) -> TestResult {
    write_json(
        path,
        json!({
            "schema_version": "0.1",
            "tool": "ripr",
            "kind": "pr_evidence",
            "scope": "diff",
            "base": "quality-gate-cli-test-base",
            "base_sha": "quality-gate-cli-test-base-sha",
            "head": "HEAD",
            "head_sha": head,
            "summary": {
                "severe_gaps": severe_gaps
            }
        }),
    )
}

fn write_ripr_pr_receipt_without_severe_gaps(path: &Path, head: &str) -> TestResult {
    write_json(
        path,
        json!({
            "schema_version": "0.1",
            "tool": "ripr",
            "kind": "pr_evidence",
            "scope": "diff",
            "base": "quality-gate-cli-test-base",
            "base_sha": "quality-gate-cli-test-base-sha",
            "head": "HEAD",
            "head_sha": head,
            "summary": {}
        }),
    )
}

fn write_empty_review_guidance_receipt(path: &Path, head: &str) -> TestResult {
    write_json(
        path,
        json!({
            "schema_version": "0.1",
            "tool": "ripr",
            "status": "advisory",
            "base": "quality-gate-cli-test-base",
            "base_sha": "quality-gate-cli-test-base-sha",
            "head": "HEAD",
            "head_sha": head,
            "summary": {
                "comments": 0,
                "summary_only": 0,
                "suppressed": 0
            },
            "comments": [],
            "summary_only": [],
            "suppressed": []
        }),
    )
}

fn write_empty_exception_policy(path: &Path) -> TestResult {
    write_text(
        path,
        r##"schema_version = 1
policy = "quality-gate-exceptions"
owner = "EffortlessMetrics"
status = "active"
updated = "2026-05-28"
due_review = "fail"

[requirements]
required_active = []
"##,
    )
}

fn write_active_exception_policy(path: &Path) -> TestResult {
    write_text(
        path,
        r##"schema_version = 1
policy = "quality-gate-exceptions"
owner = "EffortlessMetrics"
status = "active"
updated = "2026-05-28"
due_review = "fail"

[requirements]
required_active = ["ripr-total-burndown"]

[[exception]]
id = "ripr-total-burndown"
kind = "temporary_burndown"
scope = "ripr_plus_total"
owner = "proof-lane"
issue = "#8197"
reason = "transition burn-down remains active"
final_target = "repo-wide ripr+ unresolved total = 0"
evidence = "target/receipts/quality/ripr-plus.json"
removal_criteria = "remove when RIPR+ total is zero"
created = "2026-05-28"
review_after = "2099-01-01"
expires = "2099-12-31"
"##,
    )
}

fn write_invalid_exception_policy(path: &Path) -> TestResult {
    write_text(
        path,
        r##"schema_version = 1
policy = "quality-gate-exceptions"
owner = "EffortlessMetrics"
status = "active"
updated = "2026-05-28"
due_review = "fail"

[requirements]
required_active = ["ripr-total-burndown"]

[[exception]]
id = "ripr-total-burndown"
kind = "permanent_bypass"
owner = "proof-lane"
reason = "transition burn-down remains active"
final_target = "repo-wide ripr+ unresolved total = 0"
evidence = "target/receipts/quality/ripr-plus.json"
removal_criteria = "remove when RIPR+ total is zero"
created = "2026-05-28"
review_after = "2099-01-01"
expires = "2099-12-31"
"##,
    )
}

fn write_invalid_toml_exception_policy(path: &Path) -> TestResult {
    write_text(
        path,
        r##"schema_version =
policy = "quality-gate-exceptions"
"##,
    )
}

fn write_invalid_header_exception_policy(path: &Path) -> TestResult {
    write_text(
        path,
        r##"schema_version = 2
policy = "quality-gate-exceptions"
owner = "EffortlessMetrics"
status = "active"
updated = "2026-05-28"
due_review = "fail"

[requirements]
required_active = []
"##,
    )
}

fn write_invalid_metadata_exception_policy(path: &Path) -> TestResult {
    write_text(
        path,
        r##"schema_version = 1
policy = "quality-gate-exceptions"
owner = ""
status = "active"
updated = "2026-05-28"
due_review = "fail"

[requirements]
required_active = []
"##,
    )
}

fn write_invalid_date_exception_policy(path: &Path) -> TestResult {
    write_text(
        path,
        r##"schema_version = 1
policy = "quality-gate-exceptions"
owner = "EffortlessMetrics"
status = "active"
updated = "2026-05-28"
due_review = "fail"

[requirements]
required_active = []

[[exception]]
id = "ripr-total-burndown"
kind = "temporary_burndown"
scope = "ripr_plus_total"
owner = "proof-lane"
issue = "#8197"
reason = "transition burn-down remains active"
final_target = "repo-wide ripr+ unresolved total = 0"
evidence = "target/receipts/quality/ripr-plus.json"
removal_criteria = "remove when RIPR+ total is zero"
created = "2026/05/28"
review_after = "not-a-date"
expires = "also-not-a-date"
"##,
    )
}

fn next_action<'a>(receipt: &'a Value, kind: &str) -> TestResult<&'a Value> {
    receipt
        .get("next_actions")
        .and_then(Value::as_array)
        .and_then(|actions| {
            actions.iter().find(|action| action.get("kind").and_then(Value::as_str) == Some(kind))
        })
        .ok_or_else(|| format!("missing next action `{kind}`").into())
}

fn assert_repair_contract(action: &Value) -> TestResult {
    for field in ["repair", "verify", "receipt"] {
        let value = action
            .get(field)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("action missing {field}: {action}"))?;
        assert!(!value.trim().is_empty(), "action {field} must be non-empty: {action}");
        if matches!(field, "verify" | "receipt") {
            assert!(value.starts_with("rtk "), "action {field} must use rtk: {value}");
        }
    }
    Ok(())
}

fn assert_action_commands_use_quality_gate_mode(action: &Value, mode: &str) -> TestResult {
    for field in ["verify", "receipt"] {
        let command = action
            .get(field)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("action missing {field}: {action}"))?;
        assert!(
            command.contains(&format!("quality-gate --mode {mode} ")),
            "action {field} must use active quality-gate mode `{mode}`: {command}"
        );
        for other_mode in ["enforce-patch-coverage", "enforce-new-ripr"] {
            assert!(
                !command.contains(&format!("--mode {other_mode}")),
                "action {field} must not use unrelated mode `{other_mode}`: {command}"
            );
        }
    }
    Ok(())
}

fn repo_root() -> TestResult<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "xtask manifest must be nested under repo root".into())
}

fn current_head(root: &Path) -> TestResult<String> {
    let output = StdCommand::new("git").args(["rev-parse", "HEAD"]).current_dir(root).output()?;
    if !output.status.success() {
        return Err(format!("git rev-parse HEAD failed with status {}", output.status).into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn write_text(path: &Path, value: &str) -> TestResult {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, value)?;
    Ok(())
}

fn write_json(path: &Path, value: Value) -> TestResult {
    write_text(path, &serde_json::to_string_pretty(&value)?)
}
