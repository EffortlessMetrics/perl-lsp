use assert_cmd::Command;
use color_eyre::Result;

#[test]
fn semantic_scorecard_is_deterministic_and_loads_fixtures() -> Result<()> {
    let mut cmd = Command::cargo_bin("xtask")?;
    let output = cmd.arg("semantic-scorecard").assert().success().get_output().stdout.clone();
    let as_text = String::from_utf8(output)?;

    assert!(as_text.contains("\"fixture_count\": 12"));
    assert!(as_text.contains("\"metric_count\": 11"));
    assert!(as_text.contains("\"definition_hit_at_1\""));
    assert!(as_text.contains("\"baseline_pending\""));

    let mut second = Command::cargo_bin("xtask")?;
    let second_output = second.arg("semantic-scorecard").assert().success().get_output().stdout.clone();
    let second_text = String::from_utf8(second_output)?;
    assert_eq!(as_text, second_text, "scorecard output must be deterministic");

    Ok(())
}
