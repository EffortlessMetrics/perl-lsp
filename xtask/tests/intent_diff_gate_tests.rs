use assert_cmd::Command;
use color_eyre::eyre::Result;
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/intent-diff").join(name)
}

fn run_fixture(name: &str) -> Result<std::process::Output> {
    let fixture = fixture_path(name);
    let fixture_arg = fixture.to_string_lossy().into_owned();
    Ok(Command::cargo_bin("xtask")?
        .args(["intent-diff-gate", "--fixture", fixture_arg.as_str()])
        .output()?)
}

#[test]
fn fixture_doc_only_code_fix_claim_fails() -> Result<()> {
    let output = run_fixture("doc_only_code_fix_claim.json")?;

    assert!(
        !output.status.success(),
        "expected failure, got stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

#[test]
fn fixture_partial_pr_with_refs_passes() -> Result<()> {
    let output = run_fixture("partial_refs_pass.json")?;

    assert!(
        output.status.success(),
        "expected success, got stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

#[test]
fn fixture_valid_closeout_target_path_touched_passes() -> Result<()> {
    let output = run_fixture("valid_closeout_target_path_touched.json")?;

    assert!(
        output.status.success(),
        "expected success, got stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}
