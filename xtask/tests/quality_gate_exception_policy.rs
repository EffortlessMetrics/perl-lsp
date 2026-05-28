//! Contract tests for temporary quality-gate exception policy.

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
fn quality_gate_cli_reports_active_temporary_exceptions_as_final_blockers() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let coverage = dir.path().join("coverage-baseline.json");
    let policy = dir.path().join("quality-gate-exceptions.toml");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");

    write_coverage_receipt(&coverage, &current_head(&root)?, 97.1)?;
    write_exception_policy(&policy, "fail", &["ripr-total-burndown", "project-coverage-burndown"])?;

    patch_quality_gate_command(&root, &coverage, &policy, &receipt, &summary)?.assert().success();

    let payload: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    assert_eq!(payload.pointer("/decision").and_then(Value::as_str), Some("pass"));
    assert_eq!(
        payload.pointer("/temporary_exceptions/status").and_then(Value::as_str),
        Some("present")
    );
    assert_eq!(
        payload.pointer("/temporary_exceptions/active_count").and_then(Value::as_u64),
        Some(2)
    );
    assert_eq!(
        payload.pointer("/temporary_exceptions/final_enforcement_blocked").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/temporary_exceptions/active/0/blocks_final_enforcement")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(payload.get("next_actions").and_then(Value::as_array).map(Vec::len), Some(0));

    let markdown = fs::read_to_string(&summary)?;
    assert!(markdown.contains("active temporary exceptions: `2`"), "{markdown}");
    assert!(markdown.contains("final enforcement blocked: `true`"), "{markdown}");

    Ok(())
}

#[test]
fn quality_gate_cli_blocks_missing_exception_policy() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let coverage = dir.path().join("coverage-baseline.json");
    let policy = dir.path().join("missing-quality-gate-exceptions.toml");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");

    write_coverage_receipt(&coverage, &current_head(&root)?, 97.1)?;

    let output =
        patch_quality_gate_command(&root, &coverage, &policy, &receipt, &summary)?.output()?;
    assert!(!output.status.success(), "missing exception ledger must fail");

    let payload: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    assert_eq!(
        payload.pointer("/temporary_exceptions/status").and_then(Value::as_str),
        Some("missing")
    );
    let action = next_action(&payload, "quality_exception_policy_not_current")?;
    assert_eq!(action.get("reason").and_then(Value::as_str), Some("missing"));
    assert_repair_contract(action)?;

    Ok(())
}

#[test]
fn quality_gate_cli_blocks_expired_quality_exception() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let coverage = dir.path().join("coverage-baseline.json");
    let policy = dir.path().join("quality-gate-exceptions.toml");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");

    write_coverage_receipt(&coverage, &current_head(&root)?, 97.1)?;
    write_policy_text(&policy, &policy_with_one_exception("fail", "2099-01-01", "2000-01-01"))?;

    let output =
        patch_quality_gate_command(&root, &coverage, &policy, &receipt, &summary)?.output()?;
    assert!(!output.status.success(), "expired exception must fail");

    let payload: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    let action = next_action(&payload, "quality_exception_expired")?;
    assert_eq!(action.get("id").and_then(Value::as_str), Some("ripr-total-burndown"));
    assert_repair_contract(action)?;

    Ok(())
}

#[test]
fn quality_gate_cli_blocks_due_review_when_policy_requires_failure() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let coverage = dir.path().join("coverage-baseline.json");
    let policy = dir.path().join("quality-gate-exceptions.toml");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");

    write_coverage_receipt(&coverage, &current_head(&root)?, 97.1)?;
    write_policy_text(&policy, &policy_with_one_exception("fail", "2000-01-01", "2099-01-01"))?;

    let output =
        patch_quality_gate_command(&root, &coverage, &policy, &receipt, &summary)?.output()?;
    assert!(!output.status.success(), "due review must fail when due_review = fail");

    let payload: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    let action = next_action(&payload, "quality_exception_review_due")?;
    assert_eq!(action.get("blocking").and_then(Value::as_bool), Some(true));
    assert_eq!(action.get("id").and_then(Value::as_str), Some("ripr-total-burndown"));
    assert_repair_contract(action)?;

    Ok(())
}

#[test]
fn quality_gate_cli_warns_due_review_when_policy_allows_warning() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let coverage = dir.path().join("coverage-baseline.json");
    let policy = dir.path().join("quality-gate-exceptions.toml");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");

    write_coverage_receipt(&coverage, &current_head(&root)?, 97.1)?;
    write_policy_text(&policy, &policy_with_one_exception("warn", "2000-01-01", "2099-01-01"))?;

    patch_quality_gate_command(&root, &coverage, &policy, &receipt, &summary)?.assert().success();

    let payload: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    assert_eq!(payload.pointer("/decision").and_then(Value::as_str), Some("pass"));
    let action = next_action(&payload, "quality_exception_review_due")?;
    assert_eq!(action.get("blocking").and_then(Value::as_bool), Some(false));

    Ok(())
}

#[test]
fn quality_gate_cli_blocks_missing_required_exception() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let coverage = dir.path().join("coverage-baseline.json");
    let policy = dir.path().join("quality-gate-exceptions.toml");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");

    write_coverage_receipt(&coverage, &current_head(&root)?, 97.1)?;
    write_policy_text(
        &policy,
        &format!(
            "{}\n[requirements]\nrequired_active = [\"project-coverage-burndown\"]\n{}",
            policy_header("fail"),
            exception_entry("ripr-total-burndown", "2099-01-01", "2099-12-31")
        ),
    )?;

    let output =
        patch_quality_gate_command(&root, &coverage, &policy, &receipt, &summary)?.output()?;
    assert!(!output.status.success(), "missing required exception must fail");

    let payload: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    let action = next_action(&payload, "quality_exception_required_missing")?;
    assert_eq!(
        action.pointer("/missing/0").and_then(Value::as_str),
        Some("project-coverage-burndown")
    );
    assert_repair_contract(action)?;

    Ok(())
}

#[test]
fn coverage_and_ripr_status_doc_links_exception_policy() -> TestResult {
    let root = repo_root()?;
    let doc =
        fs::read_to_string(root.join("docs/project/status/coverage_and_ripr_enforcement.md"))?;
    let index = fs::read_to_string(root.join("docs/project/status/index.md"))?;

    for required in [
        "policy/quality-gate-exceptions.toml",
        "Expired exceptions fail the quality gate",
        "final-enforcement blockers",
        "Temporary exceptions are not success criteria",
    ] {
        assert!(doc.contains(required), "status doc missing `{required}`");
    }
    assert!(
        index.contains("coverage_and_ripr_enforcement.md"),
        "status index must link the proof-lane policy status doc"
    );

    Ok(())
}

fn patch_quality_gate_command(
    root: &Path,
    coverage: &Path,
    policy: &Path,
    receipt: &Path,
    summary: &Path,
) -> TestResult<Command> {
    let mut command = Command::cargo_bin("xtask")?;
    command.current_dir(root).args(["quality-gate", "--mode", "enforce-patch-coverage"]);
    command.arg("--coverage-receipt").arg(coverage);
    command.arg("--exception-policy").arg(policy);
    command.args(["--codecov", "codecov.yml"]);
    command.arg("--receipt").arg(receipt);
    command.arg("--summary").arg(summary);
    Ok(command)
}

fn write_coverage_receipt(path: &Path, head: &str, patch: f64) -> TestResult {
    write_json(
        path,
        json!({
            "schema_version": 1,
            "kind": "coverage_baseline",
            "head": head,
            "lcov": "target/lcov.info",
            "coverage": {
                "patch": patch
            },
            "files_below_target": []
        }),
    )
}

fn write_exception_policy(path: &Path, due_review: &str, required_active: &[&str]) -> TestResult {
    let required =
        required_active.iter().map(|id| format!("\"{id}\"")).collect::<Vec<_>>().join(", ");
    let text = format!(
        "{}\n[requirements]\nrequired_active = [{required}]\n{}\n{}",
        policy_header(due_review),
        exception_entry("ripr-total-burndown", "2099-01-01", "2099-12-31"),
        exception_entry("project-coverage-burndown", "2099-01-01", "2099-12-31")
    );
    write_policy_text(path, &text)
}

fn write_policy_text(path: &Path, text: &str) -> TestResult {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)?;
    Ok(())
}

fn policy_with_one_exception(due_review: &str, review_after: &str, expires: &str) -> String {
    format!(
        "{}\n[requirements]\nrequired_active = [\"ripr-total-burndown\"]\n{}",
        policy_header(due_review),
        exception_entry("ripr-total-burndown", review_after, expires)
    )
}

fn policy_header(due_review: &str) -> String {
    format!(
        r#"schema_version = 1
policy = "quality-gate-exceptions"
owner = "EffortlessMetrics"
status = "active"
updated = "2026-05-28"
due_review = "{due_review}"
"#
    )
}

fn exception_entry(id: &str, review_after: &str, expires: &str) -> String {
    format!(
        r#"
[[exception]]
id = "{id}"
owner = "proof-lane"
reason = "transition burn-down remains active"
final_target = "final proof target is met"
evidence = "target/receipts/quality/quality-gate.json"
removal_criteria = "remove this exception when final enforcement is blocking"
created = "2026-05-28"
review_after = "{review_after}"
expires = "{expires}"
"#
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

fn write_json(path: &Path, value: Value) -> TestResult {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(&value)?)?;
    Ok(())
}
