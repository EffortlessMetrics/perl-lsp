use anyhow::Result;
use assert_cmd::cargo::cargo_bin_cmd;

#[test]
fn list_commands_emits_sorted_top_level_names() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd.arg("list-commands").output()?;
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout)?;
    let lines = stdout.lines().collect::<Vec<_>>();

    assert!(lines.contains(&"check-active-goal-manifest"));
    assert!(lines.contains(&"ci"));
    assert!(lines.contains(&"check-active-goal-manifest"));
    assert!(lines.contains(&"check-lsp-318-claims"));
    assert!(lines.contains(&"check-only"));
    assert!(lines.contains(&"check-test-wiring"));
    assert!(lines.contains(&"layer-check"));
    assert!(lines.contains(&"list-commands"));
    assert!(lines.contains(&"srp"));
    assert!(lines.contains(&"srp-microcrates"));
    assert!(lines.contains(&"unwired-scan"));

    let mut sorted = lines.clone();
    sorted.sort_unstable();
    assert_eq!(lines, sorted, "commands should be emitted in sorted order");

    Ok(())
}
