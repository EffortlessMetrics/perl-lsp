use anyhow::Result;
use assert_cmd::cargo::cargo_bin_cmd;

#[test]
fn check_active_goal_manifest_passes_for_current_manifest() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd.arg("check-active-goal-manifest").output()?;

    assert!(
        output.status.success(),
        "active goal manifest check should pass; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("active goal manifest check passed:"),
        "expected success receipt in stdout; got: {stdout}"
    );
    assert!(stdout.contains("repo=perl-lsp-swarm"), "expected swarm repo in stdout; got: {stdout}");
    assert!(
        stdout.contains("lane=real_perl_editor_trust_v1"),
        "expected active lane in stdout; got: {stdout}"
    );
    assert!(stdout.contains("3 lanes"), "expected lane count in stdout; got: {stdout}");
    assert!(
        stdout.contains("path references"),
        "expected path reference count in stdout; got: {stdout}"
    );

    Ok(())
}
