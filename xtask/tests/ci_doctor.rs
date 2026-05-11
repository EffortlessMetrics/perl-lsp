//! Integration tests for `cargo xtask ci-doctor`.
//!
//! Each test invokes the xtask binary and asserts on exit code and output
//! shape. The tests do not depend on whether specific tools (Perl, perl-lsp)
//! are installed — they verify output structure, not specific check outcomes.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use anyhow::Result;
use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse JSON from stdout bytes.
fn parse_json(bytes: &[u8]) -> Result<Value> {
    let text = std::str::from_utf8(bytes)?;
    Ok(serde_json::from_str(text)?)
}

/// Names of the 8 checks defined in v1.
const CHECK_NAMES: &[&str] = &[
    "rustc-toolchain-match",
    "rustfmt-installed",
    "clippy-installed",
    "git-clean",
    "fmt-drift",
    "perl-interpreter",
    "perl-lsp-binary",
    "platform-env-sanity",
];

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Default mode (no flags) must exit 0 regardless of individual check outcomes.
#[test]
fn default_mode_exits_zero() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.args(["ci-doctor"]).assert().success();
    Ok(())
}

/// Default human output must contain all 8 check section names.
#[test]
fn human_output_shape() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let out = cmd.args(["ci-doctor"]).assert().success().get_output().stdout.clone();
    let text = String::from_utf8(out)?;

    for name in CHECK_NAMES {
        assert!(text.contains(name), "human output missing check name {name:?}; got:\n{text}");
    }
    // Must contain the overall status line.
    assert!(text.contains("ci-doctor"), "human output missing 'ci-doctor' header; got:\n{text}");
    assert!(text.contains("overall:"), "human output missing 'overall:' label; got:\n{text}");
    Ok(())
}

/// `--json` emits a valid schema-v1 receipt with all required fields.
#[test]
fn json_output_schema_v1() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let out = cmd.args(["ci-doctor", "--json"]).assert().success().get_output().stdout.clone();
    let receipt = parse_json(&out)?;

    // Top-level required fields.
    assert_eq!(receipt["schema_version"], 1, "schema_version must be 1");
    assert!(
        receipt["overall"].is_string(),
        "overall must be a string; got: {:?}",
        receipt["overall"]
    );
    assert!(
        ["ok", "warn", "fail"].contains(&receipt["overall"].as_str().unwrap()),
        "overall must be ok|warn|fail; got: {:?}",
        receipt["overall"]
    );

    // Checks array.
    let checks = receipt["checks"].as_array().expect("checks must be an array");
    assert_eq!(
        checks.len(),
        CHECK_NAMES.len(),
        "expected {} checks, got {}",
        CHECK_NAMES.len(),
        checks.len()
    );

    // Verify each check has the required fields and matches the expected name.
    for (i, check) in checks.iter().enumerate() {
        let name = CHECK_NAMES[i];
        assert_eq!(
            check["name"].as_str(),
            Some(name),
            "check[{i}].name mismatch: expected {name:?}, got {:?}",
            check["name"]
        );
        assert!(
            ["ok", "warn", "fail"].contains(&check["status"].as_str().unwrap_or("")),
            "check[{i}].status must be ok|warn|fail; got: {:?}",
            check["status"]
        );
        assert!(check["expected"].is_string(), "check[{i}].expected must be a string");
        assert!(check["actual"].is_string(), "check[{i}].actual must be a string");
        assert!(check["note"].is_string(), "check[{i}].note must be a string");
    }

    Ok(())
}

/// `--strict` exits 1 if any check is warn or fail.
/// We verify this by parsing the JSON first and only asserting failure when
/// the receipt actually contains at least one non-ok check.
#[test]
fn strict_mode_fails_on_warn() -> Result<()> {
    // First get the JSON receipt without strict to know what overall is.
    let mut probe = cargo_bin_cmd!("xtask");
    let probe_out =
        probe.args(["ci-doctor", "--json"]).assert().success().get_output().stdout.clone();
    let receipt = parse_json(&probe_out)?;
    let overall = receipt["overall"].as_str().unwrap_or("ok");

    let mut cmd = cargo_bin_cmd!("xtask");
    let assertion = cmd.args(["ci-doctor", "--strict"]);

    if overall == "ok" {
        // All checks pass — strict mode should also exit 0.
        assertion.assert().success();
    } else {
        // At least one check is warn or fail — strict mode must exit 1.
        assertion.assert().failure().code(1);
    }

    Ok(())
}

/// Default mode (warn-only) exits 0 even when checks are warn/fail.
/// We verify this by checking that the default exit code is always 0.
#[test]
fn default_mode_passes_with_warn() -> Result<()> {
    // Default mode must always exit 0 (it never enforces strict).
    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.args(["ci-doctor"]).assert().success();
    Ok(())
}

/// Perl interpreter absence must produce a `warn` status, not `fail`.
///
/// This test exercises the `perl-interpreter` check by parsing the JSON
/// receipt and verifying that if perl is absent the status is `warn`.
#[test]
fn missing_perl_interpreter_warns_not_fails() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let out = cmd.args(["ci-doctor", "--json"]).assert().success().get_output().stdout.clone();
    let receipt = parse_json(&out)?;

    let checks = receipt["checks"].as_array().expect("checks must be an array");
    let perl_check = checks
        .iter()
        .find(|c| c["name"].as_str() == Some("perl-interpreter"))
        .expect("perl-interpreter check must be present");

    let status = perl_check["status"].as_str().unwrap_or("unknown");
    assert_ne!(
        status, "fail",
        "perl-interpreter status must not be 'fail'; got {status:?} (Perl absence is a warn)"
    );
    // Status must be either "ok" (perl found) or "warn" (not found).
    assert!(
        status == "ok" || status == "warn",
        "perl-interpreter status must be ok or warn; got {status:?}"
    );

    Ok(())
}

/// `--json` and `--strict` flags are composable: JSON is emitted, then exit 1 on warn/fail.
#[test]
fn json_and_strict_are_composable() -> Result<()> {
    // Get JSON receipt to know the expected exit code.
    let mut probe = cargo_bin_cmd!("xtask");
    let probe_out =
        probe.args(["ci-doctor", "--json"]).assert().success().get_output().stdout.clone();
    let receipt = parse_json(&probe_out)?;
    let overall = receipt["overall"].as_str().unwrap_or("ok");

    let mut cmd = cargo_bin_cmd!("xtask");
    let out = cmd
        .args(["ci-doctor", "--json", "--strict"])
        .output()
        .expect("xtask ci-doctor --json --strict must run");

    // JSON output must be valid schema-v1.
    let json = parse_json(&out.stdout)?;
    assert_eq!(json["schema_version"], 1);

    // Exit code must match the overall status.
    let exit_code = out.status.code().unwrap_or(-1);
    if overall == "ok" {
        assert_eq!(exit_code, 0, "strict mode with ok overall must exit 0");
    } else {
        assert_eq!(exit_code, 1, "strict mode with warn/fail overall must exit 1");
    }

    Ok(())
}
