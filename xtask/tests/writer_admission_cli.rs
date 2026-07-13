// Writer-admission CLI integration tests (#3957 W1).
//
// Runs the real `cargo xtask writer-admission --fixture ...` entry point
// against fixtures that trigger each of the three verdicts, and asserts on
// the printed verdict — not just on the library's `run_checks` return
// value — so a regression in argument wiring or output formatting is
// caught, not just a regression in the check logic itself.
use anyhow::Result;
use assert_cmd::cargo::cargo_bin_cmd;
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("writer-admission")
        .join(name)
}

fn run_fixture(name: &str) -> Result<(bool, String)> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .args([
            "writer-admission",
            "--fixture",
            &fixture_path(name).display().to_string(),
            "--json",
        ])
        .output()?;
    let stdout = String::from_utf8(output.stdout)?;
    Ok((output.status.success(), stdout))
}

#[test]
fn healthy_feature_branch_is_pass() -> Result<()> {
    let (ok, stdout) = run_fixture("healthy-feature-branch.json")?;
    assert!(ok, "writer-admission must always exit 0 (advisory-first): {stdout}");
    assert!(stdout.contains("\"verdict\": \"PASS\""), "expected PASS verdict, got: {stdout}");
    Ok(())
}

#[test]
fn detached_main_root_checkout_is_pass_not_misdiagnosed() -> Result<()> {
    let (ok, stdout) = run_fixture("detached-main-root.json")?;
    assert!(ok, "writer-admission must always exit 0: {stdout}");
    assert!(
        stdout.contains("\"verdict\": \"PASS\""),
        "a clean detached root checkout at origin/main must not be misdiagnosed as BLOCK: {stdout}"
    );
    Ok(())
}

#[test]
fn dangling_head_blocks() -> Result<()> {
    let (ok, stdout) = run_fixture("dangling-head.json")?;
    assert!(ok, "advisory-first must still exit 0: {stdout}");
    assert!(stdout.contains("\"verdict\": \"BLOCK\""), "expected BLOCK verdict, got: {stdout}");
    assert!(stdout.contains("symbolic-head"), "expected symbolic-head check to fire: {stdout}");
    Ok(())
}

#[test]
fn shadow_ref_blocks() -> Result<()> {
    let (ok, stdout) = run_fixture("shadow-ref.json")?;
    assert!(ok);
    assert!(stdout.contains("\"verdict\": \"BLOCK\""), "expected BLOCK verdict, got: {stdout}");
    assert!(stdout.contains("shadow-ref"), "expected shadow-ref check to fire: {stdout}");
    assert!(
        stdout.contains("refs/heads/origin/main"),
        "expected the specific shadow ref to be named in the reason: {stdout}"
    );
    Ok(())
}

#[test]
fn root_checkout_on_feature_branch_blocks() -> Result<()> {
    let (ok, stdout) = run_fixture("root-checkout-on-feature-branch.json")?;
    assert!(ok);
    assert!(stdout.contains("\"verdict\": \"BLOCK\""), "expected BLOCK verdict, got: {stdout}");
    assert!(
        stdout.contains("branch-worktree-mapping"),
        "expected branch-worktree-mapping check to fire: {stdout}"
    );
    Ok(())
}

#[test]
fn base_mismatch_blocks() -> Result<()> {
    let (ok, stdout) = run_fixture("base-mismatch.json")?;
    assert!(ok);
    assert!(stdout.contains("\"verdict\": \"BLOCK\""), "expected BLOCK verdict, got: {stdout}");
    assert!(stdout.contains("canonical-base"), "expected canonical-base check to fire: {stdout}");
    Ok(())
}

#[test]
fn low_disk_blocks() -> Result<()> {
    let (ok, stdout) = run_fixture("low-disk.json")?;
    assert!(ok);
    assert!(stdout.contains("\"verdict\": \"BLOCK\""), "expected BLOCK verdict, got: {stdout}");
    assert!(stdout.contains("disk-capacity"), "expected disk-capacity check to fire: {stdout}");
    Ok(())
}

#[test]
fn writer_collision_open_pr_blocks() -> Result<()> {
    let (ok, stdout) = run_fixture("writer-collision-open-pr.json")?;
    assert!(ok);
    assert!(stdout.contains("\"verdict\": \"BLOCK\""), "expected BLOCK verdict, got: {stdout}");
    assert!(
        stdout.contains("writer-collision"),
        "expected writer-collision check to fire: {stdout}"
    );
    Ok(())
}

#[test]
fn gh_unavailable_is_not_proven_never_a_silent_pass() -> Result<()> {
    let (ok, stdout) = run_fixture("gh-unavailable-not-proven.json")?;
    assert!(ok, "advisory-first must still exit 0: {stdout}");
    assert!(
        stdout.contains("\"verdict\": \"NOT_PROVEN\""),
        "gh-unavailable must yield NOT_PROVEN, never a silent PASS: {stdout}"
    );
    Ok(())
}

#[test]
fn human_output_mode_prints_verdict_and_per_check_reasons() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .args([
            "writer-admission",
            "--fixture",
            &fixture_path("shadow-ref.json").display().to_string(),
        ])
        .output()?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("Writer Admission"), "expected human header: {stdout}");
    assert!(stdout.contains("BLOCK"), "expected the BLOCK verdict text: {stdout}");
    assert!(stdout.contains("shadow-ref"), "expected the per-check name: {stdout}");
    Ok(())
}

#[test]
fn writer_admission_never_mutates_the_working_tree() -> Result<()> {
    // Read-only guarantee: running the command against a fixture must not
    // touch git state at all. We assert this indirectly by running twice
    // and confirming identical output (no side effects accumulating).
    let (_, first) = run_fixture("healthy-feature-branch.json")?;
    let (_, second) = run_fixture("healthy-feature-branch.json")?;
    assert_eq!(first, second, "repeated runs against the same fixture must be idempotent");
    Ok(())
}
