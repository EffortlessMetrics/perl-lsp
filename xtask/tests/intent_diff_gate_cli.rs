use assert_cmd::Command;
use color_eyre::eyre::Result;

#[test]
fn fixture_6780_like_docs_only_claim_fails() -> Result<()> {
    let output = Command::cargo_bin("xtask")?
        .args([
            "intent-diff-gate",
            "--fixture",
            "tests/fixtures/intent-diff/6780-docs-only-code-fix-claim.json",
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "expected failure; stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn fixture_partial_refs_passes() -> Result<()> {
    let output = Command::cargo_bin("xtask")?
        .args([
            "intent-diff-gate",
            "--fixture",
            "tests/fixtures/intent-diff/partial-refs-pass.json",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "expected success; stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn fixture_valid_closeout_target_path_passes() -> Result<()> {
    let output = Command::cargo_bin("xtask")?
        .args([
            "intent-diff-gate",
            "--fixture",
            "tests/fixtures/intent-diff/valid-closeout-target-path.json",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "expected success; stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
