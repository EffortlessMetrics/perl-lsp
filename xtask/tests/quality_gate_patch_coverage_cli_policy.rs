//! Contract tests for patch-coverage quality-gate CLI behavior.

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
fn coverage_how_to_documents_patch_gate_cli_guidance() -> TestResult {
    let root = repo_root()?;
    let coverage_doc = fs::read_to_string(root.join("docs/how-to/COVERAGE.md"))?;

    assert!(
        coverage_doc.contains(
            "rtk cargo xtask coverage-baseline --lcov target/lcov.info --receipt target/receipts/quality/coverage-baseline.json --codecov codecov.yml --patch-coverage <patch-percent>"
        ),
        "coverage how-to must show the rtk-prefixed coverage receipt command"
    );
    assert!(
        coverage_doc.contains(
            "rtk cargo xtask quality-gate --mode enforce-patch-coverage --coverage-receipt target/receipts/quality/coverage-baseline.json --codecov codecov.yml"
        ),
        "coverage how-to must show the rtk-prefixed patch quality-gate command"
    );
    assert!(
        coverage_doc.contains("patch_coverage_unknown")
            && coverage_doc.contains("sample uncovered lines")
            && coverage_doc.contains("behavior-oriented tests"),
        "coverage how-to must describe actionable patch coverage repair guidance"
    );
    assert!(
        coverage_doc.contains("coverage.patch")
            && coverage_doc.contains("coverage.project")
            && coverage_doc.contains("project coverage")
            && coverage_doc.contains("burn-down number"),
        "coverage how-to must explain that the receipt carries project coverage visibility"
    );

    Ok(())
}

#[test]
fn coverage_baseline_writes_and_checks_receipt_with_actionable_file_samples() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let lcov = dir.path().join("lcov.info");
    let coverage = dir.path().join("coverage-baseline.json");

    write_lcov(&lcov)?;

    coverage_baseline_command(&root, &lcov, &coverage, Some(97.1))?.assert().success();

    let payload: Value = serde_json::from_str(&fs::read_to_string(&coverage)?)?;
    assert_eq!(payload.get("kind").and_then(Value::as_str), Some("coverage_baseline"));
    assert_eq!(payload.get("head").and_then(Value::as_str), Some(current_head(&root)?.as_str()));
    assert_eq!(payload.pointer("/coverage/patch").and_then(Value::as_f64), Some(97.1));
    assert_eq!(payload.pointer("/coverage/project").and_then(Value::as_f64), Some(60.0));
    assert_eq!(payload.pointer("/measured/line_found").and_then(Value::as_u64), Some(5));
    assert_eq!(payload.pointer("/measured/line_hit").and_then(Value::as_u64), Some(3));
    assert_eq!(
        payload.pointer("/files_below_target/0/path").and_then(Value::as_str),
        Some("crates/perl-parser/src/lib.rs")
    );
    assert_eq!(
        payload.pointer("/files_below_target/0/sample_uncovered_lines/0").and_then(Value::as_u64),
        Some(12)
    );
    assert!(
        payload
            .get("patch_files_below_target")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty),
        "manual patch coverage receipts should not invent changed-file guidance: {payload}"
    );
    assert!(
        !payload
            .pointer("/files_below_target/0/sample_uncovered_lines")
            .and_then(Value::as_array)
            .is_some_and(|lines| lines.iter().any(|line| line.as_u64() == Some(0))),
        "coverage baseline must reject LCOV DA entries whose line number is 0"
    );

    coverage_baseline_command(&root, &lcov, &coverage, Some(97.1))?
        .arg("--check")
        .assert()
        .success();

    Ok(())
}

#[test]
fn quality_gate_cli_writes_and_checks_patch_gate_receipts() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let lcov = dir.path().join("lcov.info");
    let coverage = dir.path().join("coverage-baseline.json");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");

    write_lcov(&lcov)?;
    coverage_baseline_command(&root, &lcov, &coverage, Some(97.1))?.assert().success();

    patch_quality_gate_command(&root, &coverage, &receipt, &summary, None)?.assert().success();

    let payload: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    assert_eq!(payload.get("kind").and_then(Value::as_str), Some("quality_gate"));
    assert_eq!(payload.get("mode").and_then(Value::as_str), Some("enforce-patch-coverage"));
    assert_eq!(payload.get("decision").and_then(Value::as_str), Some("pass"));
    assert_eq!(payload.pointer("/coverage/status").and_then(Value::as_str), Some("present"));
    assert_eq!(payload.pointer("/coverage/patch").and_then(Value::as_f64), Some(97.1));
    assert_eq!(payload.pointer("/coverage/project").and_then(Value::as_f64), Some(60.0));
    assert_eq!(payload.pointer("/coverage/scope").and_then(Value::as_str), Some("unspecified"));
    assert_eq!(
        payload.pointer("/coverage/codecov_config_status").and_then(Value::as_str),
        Some("present")
    );

    let markdown = fs::read_to_string(&summary)?;
    assert!(markdown.contains("## Quality Gates"), "{markdown}");
    assert!(markdown.contains("patch coverage: `97.10%` / `95.00%`"), "{markdown}");
    assert!(markdown.contains("project coverage: `60.00%` / `95.00%`"), "{markdown}");
    assert!(markdown.contains("coverage scope: `unspecified`"), "{markdown}");

    patch_quality_gate_command(&root, &coverage, &receipt, &summary, None)?
        .arg("--check")
        .assert()
        .success();

    Ok(())
}

#[test]
fn quality_gate_cli_check_blocks_stale_patch_gate_json_receipt() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let lcov = dir.path().join("lcov.info");
    let coverage = dir.path().join("coverage-baseline.json");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");

    write_lcov(&lcov)?;
    coverage_baseline_command(&root, &lcov, &coverage, Some(97.1))?.assert().success();
    patch_quality_gate_command(&root, &coverage, &receipt, &summary, None)?.assert().success();

    fs::write(
        &receipt,
        serde_json::to_string_pretty(&json!({
            "schema_version": 1,
            "kind": "quality_gate",
            "mode": "enforce-patch-coverage",
            "decision": "pass",
            "head": "stale-quality-gate-head"
        }))?,
    )?;

    let output = patch_quality_gate_command(&root, &coverage, &receipt, &summary, None)?
        .arg("--check")
        .output()?;
    assert!(!output.status.success(), "quality-gate --check must fail when JSON receipt is stale");

    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("quality gate JSON receipt is stale")
            && stderr.contains(&receipt.to_string_lossy().to_string()),
        "stale JSON receipt failure must name the stale proof file: {stderr}"
    );

    Ok(())
}

#[test]
fn quality_gate_cli_check_blocks_stale_patch_gate_markdown_summary() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let lcov = dir.path().join("lcov.info");
    let coverage = dir.path().join("coverage-baseline.json");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");

    write_lcov(&lcov)?;
    coverage_baseline_command(&root, &lcov, &coverage, Some(97.1))?.assert().success();
    patch_quality_gate_command(&root, &coverage, &receipt, &summary, None)?.assert().success();

    fs::write(&summary, "# Quality Gate\n\nstale summary\n")?;

    let output = patch_quality_gate_command(&root, &coverage, &receipt, &summary, None)?
        .arg("--check")
        .output()?;
    assert!(
        !output.status.success(),
        "quality-gate --check must fail when Markdown summary is stale"
    );

    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("quality gate Markdown summary is stale")
            && stderr.contains(&summary.to_string_lossy().to_string()),
        "stale Markdown summary failure must name the stale proof file: {stderr}"
    );

    Ok(())
}

#[test]
fn quality_gate_cli_blocks_patch_gate_when_coverage_receipt_is_missing() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let coverage = dir.path().join("missing-coverage-baseline.json");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");

    let output =
        patch_quality_gate_command(&root, &coverage, &receipt, &summary, None)?.output()?;
    assert!(!output.status.success(), "missing coverage receipt must fail the gate");
    assert_failure_stderr_points_to_receipt_and_summary(
        &String::from_utf8(output.stderr)?,
        &receipt,
        &summary,
    );

    let payload: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    assert_eq!(payload.get("decision").and_then(Value::as_str), Some("fail"));
    assert_eq!(payload.pointer("/coverage/status").and_then(Value::as_str), Some("missing"));

    let action = next_action(&payload, "coverage_receipt_not_current")?;
    assert_eq!(action.get("reason").and_then(Value::as_str), Some("missing"));
    assert_repair_contract(action)?;
    assert!(
        action.get("verify").and_then(Value::as_str).is_some_and(|verify| verify
            .contains("coverage-baseline")
            && verify.contains("--check")),
        "missing receipt action must carry focused verify command: {action}"
    );

    Ok(())
}

#[test]
fn quality_gate_cli_blocks_patch_gate_when_coverage_receipt_is_stale() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let coverage = dir.path().join("coverage-baseline.json");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");

    write_coverage_receipt(&coverage, "quality-gate-cli-stale-head", Some(97.1), json!([]))?;

    let output =
        patch_quality_gate_command(&root, &coverage, &receipt, &summary, None)?.output()?;
    assert!(!output.status.success(), "stale coverage receipt must fail the gate");
    assert_failure_stderr_points_to_receipt_and_summary(
        &String::from_utf8(output.stderr)?,
        &receipt,
        &summary,
    );

    let payload: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    assert_eq!(payload.pointer("/coverage/status").and_then(Value::as_str), Some("stale"));

    let action = next_action(&payload, "coverage_receipt_not_current")?;
    assert_eq!(action.get("reason").and_then(Value::as_str), Some("stale"));
    assert_eq!(
        action.get("receipt_head").and_then(Value::as_str),
        Some("quality-gate-cli-stale-head")
    );
    assert_eq!(
        action.get("expected_head").and_then(Value::as_str),
        Some(current_head(&root)?.as_str())
    );
    assert_repair_contract(action)?;

    Ok(())
}

#[test]
fn quality_gate_cli_blocks_patch_gate_when_patch_coverage_is_unknown() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let coverage = dir.path().join("coverage-baseline.json");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");

    write_coverage_receipt(&coverage, &current_head(&root)?, None, json!([]))?;

    let output =
        patch_quality_gate_command(&root, &coverage, &receipt, &summary, None)?.output()?;
    assert!(
        !output.status.success(),
        "patch coverage must fail when the receipt is current but no patch percent is available"
    );

    let payload: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    assert_eq!(payload.pointer("/coverage/status").and_then(Value::as_str), Some("present"));
    assert!(payload.pointer("/coverage/patch").is_some_and(Value::is_null));

    let action = next_action(&payload, "patch_coverage_unknown")?;
    assert_repair_contract(action)?;
    assert!(
        action
            .get("repair")
            .and_then(Value::as_str)
            .is_some_and(|repair| repair.contains("patch coverage percentage")),
        "unknown patch coverage failure must tell the agent what evidence is missing: {action}"
    );

    Ok(())
}

#[test]
fn quality_gate_cli_blocks_patch_coverage_below_target_with_file_guidance() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let coverage = dir.path().join("coverage-baseline.json");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");

    write_coverage_receipt_with_patch_files(
        &coverage,
        &current_head(&root)?,
        Some(94.9),
        coverage_gap_files(),
        patch_coverage_gap_files(),
    )?;

    let output =
        patch_quality_gate_command(&root, &coverage, &receipt, &summary, None)?.output()?;
    assert!(!output.status.success(), "patch coverage below 95% must fail the gate");
    assert_failure_stderr_points_to_receipt_and_summary(
        &String::from_utf8(output.stderr)?,
        &receipt,
        &summary,
    );

    let payload: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    assert_eq!(payload.pointer("/coverage/patch").and_then(Value::as_f64), Some(94.9));

    let action = next_action(&payload, "patch_coverage_below_target")?;
    assert_eq!(action.get("current").and_then(Value::as_f64), Some(94.9));
    assert_eq!(action.get("target").and_then(Value::as_f64), Some(95.0));
    assert_eq!(action.get("source").and_then(Value::as_str), Some("coverage_receipt"));
    assert_eq!(
        action.pointer("/top_files/0/path").and_then(Value::as_str),
        Some("xtask/src/tasks/ripr_evidence.rs")
    );
    assert_eq!(
        action.pointer("/top_files/0/sample_uncovered_lines/0").and_then(Value::as_u64),
        Some(212)
    );
    assert!(
        action
            .get("suggested_test")
            .and_then(Value::as_str)
            .is_some_and(|suggested| suggested.contains("error paths")
                && suggested.contains("output contracts")),
        "patch coverage failure must suggest behavior-oriented tests: {action}"
    );
    assert_repair_contract(action)?;

    let markdown = fs::read_to_string(&summary)?;
    assert!(markdown.contains("patch_coverage_below_target"), "{markdown}");
    assert!(markdown.contains("xtask/src/tasks/ripr_evidence.rs"), "{markdown}");
    assert!(markdown.contains("sample uncovered lines: 212, 213, 214"), "{markdown}");
    assert!(!markdown.contains("crates/perl-ast-v2/src/lib.rs"), "{markdown}");

    Ok(())
}

fn coverage_baseline_command(
    root: &Path,
    lcov: &Path,
    receipt: &Path,
    patch: Option<f64>,
) -> TestResult<Command> {
    let mut command = Command::cargo_bin("xtask")?;
    command.current_dir(root).args(["coverage-baseline", "--lcov"]);
    command.arg(lcov);
    command.arg("--receipt").arg(receipt);
    command.args(["--codecov", "codecov.yml"]);
    if let Some(patch) = patch {
        command.arg("--patch-coverage").arg(format!("{patch:.2}"));
    }
    Ok(command)
}

fn patch_quality_gate_command(
    root: &Path,
    coverage: &Path,
    receipt: &Path,
    summary: &Path,
    patch: Option<f64>,
) -> TestResult<Command> {
    let mut command = Command::cargo_bin("xtask")?;
    command.current_dir(root).args(["quality-gate", "--mode", "enforce-patch-coverage"]);
    command.arg("--coverage-receipt").arg(coverage);
    command.args(["--codecov", "codecov.yml"]);
    command.arg("--receipt").arg(receipt);
    command.arg("--summary").arg(summary);
    if let Some(patch) = patch {
        command.arg("--patch-coverage").arg(format!("{patch:.2}"));
    }
    Ok(command)
}

fn write_lcov(path: &Path) -> TestResult {
    fs::write(
        path,
        "\
TN:
SF:crates/perl-parser/src/lib.rs
DA:0,0
DA:12,0
DA:13,0
DA:17,1
end_of_record
SF:xtask/src/tasks/quality_gate.rs
DA:21,1
DA:22,1
end_of_record
",
    )?;
    Ok(())
}

fn write_coverage_receipt(
    path: &Path,
    head: &str,
    patch: Option<f64>,
    files_below_target: Value,
) -> TestResult {
    write_coverage_receipt_with_patch_files(path, head, patch, files_below_target, json!([]))
}

fn write_coverage_receipt_with_patch_files(
    path: &Path,
    head: &str,
    patch: Option<f64>,
    files_below_target: Value,
    patch_files_below_target: Value,
) -> TestResult {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut coverage = serde_json::Map::new();
    if let Some(patch) = patch {
        coverage.insert("patch".to_string(), json!(patch));
    }
    fs::write(
        path,
        serde_json::to_string_pretty(&json!({
            "schema_version": 1,
            "kind": "coverage_baseline",
            "head": head,
            "lcov": "target/lcov.info",
            "coverage": coverage,
            "codecov_status": {
                "patch": {
                    "default": {
                        "target": "95%",
                        "threshold": "0%",
                        "if_ci_failed": "error"
                    }
                }
            },
            "measured": {
                "line_hit": 96,
                "line_found": 100,
                "line_coverage": 96.0
            },
            "patch_files_below_target": patch_files_below_target,
            "files_below_target": files_below_target
        }))?,
    )?;
    Ok(())
}

fn coverage_gap_files() -> Value {
    json!([
        {
            "path": "crates/perl-ast-v2/src/lib.rs",
            "line_hit": 4,
            "line_found": 10,
            "line_coverage": 40.0,
            "sample_uncovered_lines": [12, 13, 17]
        }
    ])
}

fn patch_coverage_gap_files() -> Value {
    json!([
        {
            "path": "xtask/src/tasks/ripr_evidence.rs",
            "line_hit": 55,
            "line_found": 81,
            "line_coverage": 67.9,
            "sample_uncovered_lines": [212, 213, 214]
        }
    ])
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

fn assert_failure_stderr_points_to_receipt_and_summary(
    stderr: &str,
    receipt: &Path,
    summary: &Path,
) {
    let receipt = receipt.to_string_lossy();
    let summary = summary.to_string_lossy();
    for required in
        ["quality gate failed", "see receipt", receipt.as_ref(), "summary", summary.as_ref()]
    {
        assert!(
            stderr.contains(required),
            "quality-gate failure stderr missing `{required}`: {stderr}"
        );
    }
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
