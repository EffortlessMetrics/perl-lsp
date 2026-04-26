use assert_cmd::Command;
use color_eyre::eyre::Result;
use std::path::PathBuf;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn fixture_that_removes_parser_lane_fails() -> Result<()> {
    let fixture =
        manifest_dir().join("tests/fixtures/scope-meta-gate/remove-parser-lane-fail.json");
    let receipt = manifest_dir().join("target/receipts/scope-meta-gate-fixture-fail.json");

    let output = Command::cargo_bin("xtask")?
        .current_dir(manifest_dir())
        .args([
            "scope-meta-gate",
            "--fixture",
            &fixture.to_string_lossy(),
            "--receipt",
            &receipt.to_string_lossy(),
        ])
        .output()?;

    assert!(!output.status.success(), "fixture removing parser lane must fail");
    Ok(())
}

#[test]
fn fixture_that_expands_docs_scope_warns_and_passes() -> Result<()> {
    let fixture = manifest_dir().join("tests/fixtures/scope-meta-gate/docs-expands-warn.json");
    let receipt = manifest_dir().join("target/receipts/scope-meta-gate-fixture-warn.json");

    let output = Command::cargo_bin("xtask")?
        .current_dir(manifest_dir())
        .args([
            "scope-meta-gate",
            "--fixture",
            &fixture.to_string_lossy(),
            "--receipt",
            &receipt.to_string_lossy(),
        ])
        .output()?;

    assert!(output.status.success(), "scope expansion fixture should not fail");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("warn") || stdout.contains("pass"),
        "expected warn/pass verdict in output, got: {stdout}"
    );
    Ok(())
}

#[test]
fn receipt_validates_against_schema_when_python_jsonschema_is_available() -> Result<()> {
    let fixture = manifest_dir().join("tests/fixtures/scope-meta-gate/docs-expands-warn.json");
    let receipt = manifest_dir().join("target/receipts/scope-meta-gate-fixture-schema.json");

    let output = Command::cargo_bin("xtask")?
        .current_dir(manifest_dir())
        .args([
            "scope-meta-gate",
            "--fixture",
            &fixture.to_string_lossy(),
            "--receipt",
            &receipt.to_string_lossy(),
        ])
        .output()?;
    assert!(output.status.success(), "receipt generation should succeed");

    let py = std::process::Command::new("python3")
        .args([
            "-c",
            "import importlib.util,sys;sys.exit(0 if importlib.util.find_spec('jsonschema') else 1)",
        ])
        .status()?;

    if !py.success() {
        return Ok(());
    }

    let schema = manifest_dir().join("../.ci/receipts/schemas/scope-meta-gate.schema.json");
    let validate = std::process::Command::new("python3")
        .args([
            "-c",
            &format!(
                "import json, jsonschema; s=json.load(open(r'{}')); d=json.load(open(r'{}')); jsonschema.validate(d, s)",
                schema.display(),
                receipt.display()
            ),
        ])
        .status()?;
    assert!(validate.success(), "jsonschema validation should pass when jsonschema is installed");
    Ok(())
}
