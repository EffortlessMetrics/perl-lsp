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
fn healthy_feature_branch_guidance_reports_existing_worktree_for_reuse() -> Result<()> {
    // #3957 W2: the fixture already has exactly one worktree entry mapped
    // to the target branch — the real CLI entry point (not just the
    // `compute_guidance` unit) must surface it as a REUSE candidate.
    let (ok, stdout) = run_fixture("healthy-feature-branch.json")?;
    assert!(ok, "writer-admission must always exit 0 (advisory-first): {stdout}");
    assert!(
        stdout.contains("\"existing_worktree_path\": \"/repo/.claude/worktrees/agent-1\""),
        "expected guidance.existing_worktree_path to name the REUSE candidate: {stdout}"
    );
    assert!(
        stdout.contains("\"remote_branch_sha\": null"),
        "this fixture has no remote_branch.sha set — RESUME guidance must stay null: {stdout}"
    );
    Ok(())
}

#[test]
fn resume_existing_remote_branch_guidance_reports_remote_sha_not_a_reuse_path() -> Result<()> {
    // #3957 W2: no local worktree maps to the branch, but it already
    // exists on the remote — a RESUME candidate, not REUSE.
    let (ok, stdout) = run_fixture("resume-existing-remote-branch.json")?;
    assert!(ok, "writer-admission must always exit 0 (advisory-first): {stdout}");
    assert!(
        stdout.contains("\"remote_branch_sha\": \"9c1c9c1c9c1c9c1c9c1c9c1c9c1c9c1c9c1c9c1c\""),
        "expected guidance.remote_branch_sha to name the RESUME candidate: {stdout}"
    );
    assert!(
        stdout.contains("\"existing_worktree_path\": null"),
        "no worktree maps to the branch in this fixture — REUSE guidance must stay null: {stdout}"
    );
    // This fixture is otherwise clean (no shadow ref, no dirty state, no PR,
    // healthy disk) — the RESUME signal must not itself flip the verdict to
    // BLOCK; `guidance` is informational only.
    assert!(stdout.contains("\"verdict\": \"PASS\""), "expected PASS verdict, got: {stdout}");
    Ok(())
}

#[test]
fn human_output_mode_prints_resume_guidance_line() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .args([
            "writer-admission",
            "--fixture",
            &fixture_path("resume-existing-remote-branch.json").display().to_string(),
        ])
        .output()?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("[GUIDANCE]") && stdout.contains("RESUME"),
        "expected a human-readable RESUME guidance line: {stdout}"
    );
    Ok(())
}

#[test]
fn root_checkout_on_feature_branch_never_offers_root_as_reuse() -> Result<()> {
    // Regression for a P1 caught by independent execution review of #3957
    // W2 (confirmed by both the reviewer and an automated code-review bot):
    // the root checkout is on the target feature branch with no dedicated
    // worktree for it — `worktree_mapping.entries` has exactly one match,
    // and it's the root's own entry. The real CLI must never surface that
    // as `guidance.existing_worktree_path`, which would let `/start-work`'s
    // Step 6c REUSE outcome hand an operator straight back into the
    // production root checkout — `branch-worktree-mapping` already BLOCKs
    // this exact condition; `guidance` must not independently contradict it.
    let (ok, stdout) = run_fixture("root-checkout-reuse-suppressed.json")?;
    assert!(ok, "writer-admission must always exit 0 (advisory-first): {stdout}");
    assert!(
        stdout.contains("\"existing_worktree_path\": null"),
        "REUSE must never be offered for the root checkout: {stdout}"
    );
    assert!(
        stdout.contains("\"verdict\": \"BLOCK\""),
        "branch-worktree-mapping must still BLOCK this root-on-feature-branch condition: {stdout}"
    );
    assert!(
        stdout.contains("branch-worktree-mapping"),
        "expected the branch-worktree-mapping check to fire: {stdout}"
    );
    Ok(())
}

#[test]
fn remote_branch_lookup_failure_is_surfaced_not_silently_dropped() -> Result<()> {
    // A genuine `refs/remotes/origin/<branch>` lookup failure (not a
    // legitimate "branch doesn't exist yet" absence) must be visible in the
    // JSON output via `remote_branch_lookup_error`, not silently collapsed
    // into the same `remote_branch_sha: null` a brand-new branch would
    // produce — otherwise a consumer can't tell "safe to ADMIT" apart from
    // "the instrument itself failed".
    let (ok, stdout) = run_fixture("remote-branch-lookup-failure.json")?;
    assert!(ok, "writer-admission must always exit 0 (advisory-first): {stdout}");
    assert!(
        stdout.contains("\"remote_branch_sha\": null"),
        "no SHA was resolved on a lookup failure: {stdout}"
    );
    assert!(
        stdout.contains("\"remote_branch_lookup_error\": \"git rev-parse --verify failed: fatal: not a git repository\""),
        "expected the lookup failure to be surfaced in guidance, not silently dropped: {stdout}"
    );
    Ok(())
}

#[test]
fn human_output_mode_prints_remote_branch_lookup_failure_guidance_line() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .args([
            "writer-admission",
            "--fixture",
            &fixture_path("remote-branch-lookup-failure.json").display().to_string(),
        ])
        .output()?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("[GUIDANCE]") && stdout.contains("NOT_PROVEN"),
        "expected a human-readable NOT_PROVEN guidance line for the lookup failure: {stdout}"
    );
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
