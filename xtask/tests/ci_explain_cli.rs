use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use tempfile::TempDir;

fn sample_receipt_passing() -> serde_json::Value {
    serde_json::json!({
        "schema_version": "gates.v1",
        "metadata": {},
        "gates": [
            {
                "gate_name": "fmt",
                "tier": "A",
                "status": "pass",
                "required": true,
                "duration_ms": 100,
                "command": "cargo xtask fmt --check"
            }
        ],
        "summary": {
            "total_gates": 1,
            "passed": 1,
            "failed": 0,
            "skipped": 0,
            "total_duration_ms": 100,
            "overall_status": "pass"
        }
    })
}

fn sample_receipt_failing() -> serde_json::Value {
    serde_json::json!({
        "schema_version": "gates.v1",
        "metadata": {},
        "gates": [
            {
                "gate_name": "test",
                "tier": "A",
                "status": "fail",
                "required": true,
                "duration_ms": 500,
                "command": "cargo test -p perl-parser --lib",
                "first_failure": {
                    "site": "crates/perl-parser/src/lib.rs:42",
                    "test": "tests::my_test",
                    "message": "assertion failed",
                    "exit_code": 101
                },
                "output_summary": "FAILED tests::my_test"
            }
        ],
        "summary": {
            "total_gates": 1,
            "passed": 0,
            "failed": 1,
            "skipped": 0,
            "total_duration_ms": 500,
            "overall_status": "fail"
        }
    })
}

#[test]
fn ci_explain_all_passing_prints_all_gates_passing() {
    let temp = TempDir::new().expect("create temp dir");
    let receipt_path = temp.path().join("receipt.json");
    fs::write(&receipt_path, serde_json::to_string_pretty(&sample_receipt_passing()).unwrap())
        .expect("write sample receipt");

    let output = cargo_bin_cmd!("xtask")
        .args(["ci", "explain", "--receipt", receipt_path.to_str().unwrap()])
        .output()
        .expect("run xtask ci explain");

    assert!(output.status.success(), "exit code: {:?}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("All gates passing"), "expected 'All gates passing' in: {stdout}");
}

#[test]
fn ci_explain_failing_gate_prints_blocking_check() {
    let temp = TempDir::new().expect("create temp dir");
    let receipt_path = temp.path().join("receipt.json");
    fs::write(&receipt_path, serde_json::to_string_pretty(&sample_receipt_failing()).unwrap())
        .expect("write sample receipt");

    let output = cargo_bin_cmd!("xtask")
        .args(["ci", "explain", "--receipt", receipt_path.to_str().unwrap()])
        .output()
        .expect("run xtask ci explain");

    assert!(output.status.success(), "exit code: {:?}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("blocking_check:"), "expected 'blocking_check:' in: {stdout}");
    assert!(stdout.contains("test"), "expected gate name 'test' in: {stdout}");
    assert!(
        stdout.contains("code_regression"),
        "expected failure_class 'code_regression' in: {stdout}"
    );
    assert!(
        stdout.contains("crates/perl-parser/src/lib.rs:42"),
        "expected source_file_line in: {stdout}"
    );
    assert!(stdout.contains("reproduce:"), "expected 'reproduce:' in: {stdout}");
}

#[test]
fn ci_explain_missing_receipt_prints_inconclusive() {
    let temp = TempDir::new().expect("create temp dir");
    let receipt_path = temp.path().join("nonexistent-receipt.json");

    let output = cargo_bin_cmd!("xtask")
        .args(["ci", "explain", "--receipt", receipt_path.to_str().unwrap()])
        .output()
        .expect("run xtask ci explain");

    assert!(output.status.success(), "exit code: {:?}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("inconclusive"), "expected 'inconclusive' in: {stdout}");
    assert!(stdout.contains("cargo xtask gates"), "expected hint 'cargo xtask gates' in: {stdout}");
}

#[test]
fn ci_explain_malformed_receipt_prints_distinct_inconclusive() {
    let temp = TempDir::new().expect("create temp dir");
    let receipt_path = temp.path().join("bad.json");
    fs::write(&receipt_path, b"not valid json { }").expect("write bad receipt");

    let output = cargo_bin_cmd!("xtask")
        .args(["ci", "explain", "--receipt", receipt_path.to_str().unwrap()])
        .output()
        .expect("run xtask ci explain");

    assert!(output.status.success(), "exit code: {:?}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("inconclusive"), "expected 'inconclusive' in: {stdout}");
    assert!(stdout.contains("malformed"), "expected 'malformed' in: {stdout}");
    assert!(
        !stdout.contains("cargo xtask gates"),
        "must not emit absent-file hint for malformed receipt"
    );
}

#[test]
fn ci_explain_unsupported_schema_prints_distinct_inconclusive() {
    let temp = TempDir::new().expect("create temp dir");
    let receipt_path = temp.path().join("receipt.json");
    fs::write(&receipt_path, br#"{"schema_version":"gates.v99","gates":[]}"#)
        .expect("write receipt with unsupported schema");

    let output = cargo_bin_cmd!("xtask")
        .args(["ci", "explain", "--receipt", receipt_path.to_str().unwrap()])
        .output()
        .expect("run xtask ci explain");

    assert!(output.status.success(), "exit code: {:?}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("inconclusive"), "expected 'inconclusive' in: {stdout}");
    assert!(stdout.contains("gates.v99"), "expected version in: {stdout}");
    assert!(stdout.contains("upgrade xtask"), "expected upgrade hint in: {stdout}");
}
