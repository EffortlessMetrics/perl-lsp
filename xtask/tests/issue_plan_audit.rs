use anyhow::Result;
use assert_cmd::cargo::cargo_bin_cmd;
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("issue-plan")
        .join(name)
}

#[test]
fn clean_fixture_reports_no_findings() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .args([
            "issue-plan",
            "audit",
            "--fixture",
            &fixture_path("clean.json").display().to_string(),
            "--dry-run",
            "--format",
            "json",
        ])
        .output()?;

    assert!(output.status.success(), "audit should always exit 0");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("\"findings_count\": 0"), "expected no findings, got: {stdout}");
    Ok(())
}

#[test]
fn drift_fixture_reports_findings_but_still_exits_zero() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .args([
            "issue-plan",
            "audit",
            "--fixture",
            &fixture_path("drift.json").display().to_string(),
            "--dry-run",
            "--format",
            "json",
        ])
        .output()?;

    // Report-only: findings are surfaced, but the command never fails.
    assert!(output.status.success(), "report-only audit must exit 0 even with findings");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("builder-ready-on-closed"), "missing closed-drift finding: {stdout}");
    assert!(
        stdout.contains("routing-label-contradiction"),
        "missing routing-contradiction finding: {stdout}"
    );
    assert!(stdout.contains("placeholder-issue-ref"), "missing #0000 finding: {stdout}");
    Ok(())
}
