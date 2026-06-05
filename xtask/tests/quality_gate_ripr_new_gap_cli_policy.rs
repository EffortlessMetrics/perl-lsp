//! Contract tests for RIPR new-gap quality-gate CLI behavior.

use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::Command as StdCommand,
};

use assert_cmd::Command;
use serde_json::{Value, json};
use tempfile::tempdir;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

#[test]
fn quality_gate_cli_blocks_new_ripr_gaps_with_actionable_receipt() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let ripr = dir.path().join("ripr-plus.json");
    let ripr_pr = dir.path().join("repo-exposure.json");
    let review = dir.path().join("comments.json");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");
    let head = current_head(&root)?;

    write_ripr_plus_receipt(&ripr, &head)?;
    write_ripr_pr_receipt(&ripr_pr, &head, 2)?;
    write_review_guidance_receipt(&review, &head)?;

    let output =
        new_ripr_quality_gate_command(&root, &ripr, &ripr_pr, &review, &receipt, &summary)?
            .output()?;
    assert!(
        !output.status.success(),
        "new RIPR gap enforcement must fail when diff-scoped severe_gaps is nonzero"
    );

    let payload: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    assert_eq!(payload.pointer("/kind").and_then(Value::as_str), Some("quality_gate"));
    assert_eq!(payload.pointer("/mode").and_then(Value::as_str), Some("enforce-new-ripr"));
    assert_eq!(payload.pointer("/decision").and_then(Value::as_str), Some("fail"));
    assert_eq!(payload.pointer("/ripr_plus/status").and_then(Value::as_str), Some("present"));
    assert_eq!(payload.pointer("/ripr_pr/status").and_then(Value::as_str), Some("present"));
    assert_eq!(payload.pointer("/ripr_pr/new_unresolved").and_then(Value::as_u64), Some(2));
    assert_eq!(payload.pointer("/review_guidance/status").and_then(Value::as_str), Some("present"));

    let action = next_action(&payload, "new_ripr_gap")?;
    assert_eq!(action.pointer("/top_gaps/0/gap_id").and_then(Value::as_str), Some("RIPR-SPEC-CLI"));
    assert_eq!(
        action.pointer("/top_gaps/0/path").and_then(Value::as_str),
        Some("crates/perl-parser/src/lib.rs")
    );
    assert_eq!(action.pointer("/top_gaps/0/line").and_then(Value::as_u64), Some(42));
    assert_eq!(action.pointer("/top_gaps/0/seam").and_then(Value::as_str), Some("exact_seam_line"));
    assert_eq!(
        action.pointer("/top_gaps/0/reason").and_then(Value::as_str),
        Some("changed parser branch has only weak proof")
    );
    assert_eq!(
        action.pointer("/top_gaps/0/suggested_test").and_then(Value::as_str),
        Some("prove parser branch recovery")
    );
    assert!(
        action.pointer("/top_gaps/1").is_none(),
        "non-actionable guidance rows must not become repair packets: {action}"
    );
    assert_blocking_actions_have_repair_contract(&payload)?;

    let markdown = fs::read_to_string(&summary)?;
    assert!(markdown.contains("new_ripr_gap"), "{markdown}");
    assert!(markdown.contains("RIPR-SPEC-CLI"), "{markdown}");
    assert!(markdown.contains("exact_seam_line"), "{markdown}");
    assert!(markdown.contains("changed parser branch has only weak proof"), "{markdown}");
    assert!(
        !markdown.contains("RIPR-SPEC-NON-ACTIONABLE"),
        "markdown must not turn incomplete review guidance into repair work: {markdown}"
    );

    Ok(())
}

#[test]
fn quality_gate_cli_renders_summary_only_review_guidance_as_repair_packet() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let ripr = dir.path().join("ripr-plus.json");
    let ripr_pr = dir.path().join("repo-exposure.json");
    let review = dir.path().join("comments.json");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");
    let head = current_head(&root)?;

    write_ripr_plus_receipt(&ripr, &head)?;
    write_ripr_pr_receipt(&ripr_pr, &head, 1)?;
    write_summary_only_review_guidance_receipt(&review, &head)?;

    let output =
        new_ripr_quality_gate_command(&root, &ripr, &ripr_pr, &review, &receipt, &summary)?
            .output()?;
    assert!(
        !output.status.success(),
        "new RIPR gap enforcement must fail with summary-only guidance"
    );

    let payload: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    assert_eq!(payload.pointer("/review_guidance/status").and_then(Value::as_str), Some("present"));
    let action = next_action(&payload, "new_ripr_gap")?;
    assert_eq!(action.pointer("/top_gaps/0/source").and_then(Value::as_str), Some("summary_only"));
    assert_eq!(
        action.pointer("/top_gaps/0/gap_id").and_then(Value::as_str),
        Some("RIPR-SPEC-SUMMARY")
    );
    assert_eq!(
        action.pointer("/top_gaps/0/path").and_then(Value::as_str),
        Some("xtask/src/tasks/quality_gate.rs")
    );
    assert_eq!(action.pointer("/top_gaps/0/line").and_then(Value::as_u64), Some(137));
    assert_eq!(
        action.pointer("/top_gaps/0/seam").and_then(Value::as_str),
        Some("summary renderer")
    );
    assert_eq!(
        action.pointer("/top_gaps/0/reason").and_then(Value::as_str),
        Some("summary-only guidance still needs a repair packet")
    );
    assert_eq!(
        action.pointer("/top_gaps/0/suggested_test").and_then(Value::as_str),
        Some("cover summary-only review guidance")
    );
    assert_blocking_actions_have_repair_contract(&payload)?;

    let markdown = fs::read_to_string(&summary)?;
    for required in [
        "RIPR-SPEC-SUMMARY",
        "xtask/src/tasks/quality_gate.rs:137",
        "summary renderer",
        "summary-only guidance still needs a repair packet",
        "cover summary-only review guidance",
    ] {
        assert!(
            markdown.contains(required),
            "summary-only guidance missing `{required}`: {markdown}"
        );
    }

    Ok(())
}

#[test]
fn quality_gate_cli_passes_when_new_ripr_receipts_are_current_and_zero() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let ripr = dir.path().join("ripr-plus.json");
    let ripr_pr = dir.path().join("repo-exposure.json");
    let review = dir.path().join("comments.json");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");
    let head = current_head(&root)?;

    write_ripr_plus_receipt(&ripr, &head)?;
    write_ripr_pr_receipt(&ripr_pr, &head, 0)?;
    write_empty_review_guidance_receipt(&review, &head)?;

    new_ripr_quality_gate_command(&root, &ripr, &ripr_pr, &review, &receipt, &summary)?
        .assert()
        .success();

    let payload: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    assert_eq!(payload.pointer("/decision").and_then(Value::as_str), Some("pass"));
    assert_eq!(payload.pointer("/ripr_pr/new_unresolved").and_then(Value::as_u64), Some(0));
    assert_eq!(payload.get("next_actions").and_then(Value::as_array).map(Vec::len), Some(0));

    new_ripr_quality_gate_command(&root, &ripr, &ripr_pr, &review, &receipt, &summary)?
        .arg("--check")
        .assert()
        .success();

    Ok(())
}

#[test]
fn quality_gate_cli_blocks_missing_review_guidance_when_new_gaps_exist() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let ripr = dir.path().join("ripr-plus.json");
    let ripr_pr = dir.path().join("repo-exposure.json");
    let review = dir.path().join("missing-comments.json");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");
    let head = current_head(&root)?;

    write_ripr_plus_receipt(&ripr, &head)?;
    write_ripr_pr_receipt(&ripr_pr, &head, 1)?;

    let output =
        new_ripr_quality_gate_command(&root, &ripr, &ripr_pr, &review, &receipt, &summary)?
            .output()?;
    assert!(!output.status.success(), "new RIPR gaps must fail when review guidance is missing");
    assert_failure_stderr_points_to_receipt_and_summary(
        &String::from_utf8(output.stderr)?,
        &receipt,
        &summary,
    )?;

    let payload: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    assert_eq!(payload.pointer("/ripr_pr/new_unresolved").and_then(Value::as_u64), Some(1));
    assert_eq!(payload.pointer("/review_guidance/status").and_then(Value::as_str), Some("missing"));
    let new_gap = next_action(&payload, "new_ripr_gap")?;
    assert_eq!(new_gap.get("new_unresolved").and_then(Value::as_u64), Some(1));
    assert!(
        new_gap.get("top_gaps").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "missing review guidance must not invent gap repair rows: {new_gap}"
    );
    let review_action = next_action(&payload, "ripr_review_receipt_not_current")?;
    assert_eq!(review_action.get("reason").and_then(Value::as_str), Some("missing"));
    assert!(!payload.get("next_actions").and_then(Value::as_array).is_some_and(|actions| {
        actions.iter().any(|action| {
            action.get("kind").and_then(Value::as_str)
                == Some("ripr_review_guidance_not_actionable")
        })
    }));
    assert_blocking_actions_have_repair_contract(&payload)?;

    let markdown = fs::read_to_string(&summary)?;
    for required in [
        "### new_ripr_gap",
        "### ripr_review_receipt_not_current",
        "- repair: `Regenerate and check the RIPR review-guidance receipt for this HEAD",
        "rtk cargo xtask ripr-review-comments --base origin/HEAD --head HEAD --check",
    ] {
        assert!(
            markdown.contains(required),
            "missing review guidance summary lacks `{required}`: {markdown}"
        );
    }

    Ok(())
}

#[test]
fn quality_gate_cli_new_ripr_exception_policy_action_uses_new_ripr_mode_commands() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let ripr = dir.path().join("ripr-plus.json");
    let ripr_pr = dir.path().join("repo-exposure.json");
    let review = dir.path().join("comments.json");
    let missing_policy = dir.path().join("missing-quality-gate-exceptions.toml");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");
    let head = current_head(&root)?;

    write_ripr_plus_receipt(&ripr, &head)?;
    write_ripr_pr_receipt(&ripr_pr, &head, 0)?;
    write_empty_review_guidance_receipt(&review, &head)?;

    let output =
        new_ripr_quality_gate_command(&root, &ripr, &ripr_pr, &review, &receipt, &summary)?
            .arg("--exception-policy")
            .arg(&missing_policy)
            .output()?;
    assert!(!output.status.success(), "missing exception policy must fail new-RIPR mode");

    let payload: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    let action = next_action(&payload, "quality_exception_policy_not_current")?;
    assert_eq!(action.get("reason").and_then(Value::as_str), Some("missing"));
    assert_blocking_actions_have_repair_contract(&payload)?;
    assert_action_commands_use_quality_gate_mode(action, "enforce-new-ripr")?;
    for field in ["verify", "receipt"] {
        let command = action.get(field).and_then(Value::as_str).ok_or("missing command")?;
        assert!(
            command.contains("--ripr-receipt")
                && command.contains("--ripr-pr-receipt")
                && command.contains("--review-receipt"),
            "new-RIPR exception policy {field} command must include RIPR proof inputs: {command}"
        );
    }

    Ok(())
}

#[test]
fn quality_gate_cli_new_ripr_invalid_exception_action_uses_new_ripr_mode_commands() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let ripr = dir.path().join("ripr-plus.json");
    let ripr_pr = dir.path().join("repo-exposure.json");
    let review = dir.path().join("comments.json");
    let policy = dir.path().join("quality-gate-exceptions.toml");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");
    let head = current_head(&root)?;

    write_ripr_plus_receipt(&ripr, &head)?;
    write_ripr_pr_receipt(&ripr_pr, &head, 0)?;
    write_empty_review_guidance_receipt(&review, &head)?;
    write_invalid_exception_policy(&policy)?;

    let output =
        new_ripr_quality_gate_command(&root, &ripr, &ripr_pr, &review, &receipt, &summary)?
            .arg("--exception-policy")
            .arg(&policy)
            .output()?;
    assert!(!output.status.success(), "invalid exception policy must fail new-RIPR mode");

    let payload: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    let action = next_action(&payload, "quality_exception_invalid")?;
    assert_eq!(action.get("id").and_then(Value::as_str), Some("ripr-total-burndown"));
    assert_blocking_actions_have_repair_contract(&payload)?;
    assert_action_commands_use_quality_gate_mode(action, "enforce-new-ripr")?;
    for field in ["verify", "receipt"] {
        let command = action.get(field).and_then(Value::as_str).ok_or("missing command")?;
        assert!(
            command.contains("--ripr-receipt")
                && command.contains("--ripr-pr-receipt")
                && command.contains("--review-receipt"),
            "new-RIPR invalid exception {field} command must include RIPR proof inputs: {command}"
        );
    }

    Ok(())
}

#[test]
fn quality_gate_cli_check_blocks_stale_new_ripr_gate_json_receipt() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let ripr = dir.path().join("ripr-plus.json");
    let ripr_pr = dir.path().join("repo-exposure.json");
    let review = dir.path().join("comments.json");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");
    let head = current_head(&root)?;

    write_ripr_plus_receipt(&ripr, &head)?;
    write_ripr_pr_receipt(&ripr_pr, &head, 0)?;
    write_empty_review_guidance_receipt(&review, &head)?;
    new_ripr_quality_gate_command(&root, &ripr, &ripr_pr, &review, &receipt, &summary)?
        .assert()
        .success();

    fs::write(
        &receipt,
        serde_json::to_string_pretty(&json!({
            "schema_version": 1,
            "kind": "quality_gate",
            "mode": "enforce-new-ripr",
            "decision": "pass",
            "head": "stale-quality-gate-head"
        }))?,
    )?;

    let output =
        new_ripr_quality_gate_command(&root, &ripr, &ripr_pr, &review, &receipt, &summary)?
            .arg("--check")
            .output()?;
    assert!(!output.status.success(), "quality-gate --check must fail when JSON receipt is stale");

    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("quality gate JSON receipt is stale")
            && stderr.contains(&receipt.to_string_lossy().to_string()),
        "stale JSON receipt failure must name the stale proof file: {stderr}"
    );

    Ok(())
}

#[test]
fn quality_gate_cli_check_blocks_stale_new_ripr_gate_markdown_summary() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let ripr = dir.path().join("ripr-plus.json");
    let ripr_pr = dir.path().join("repo-exposure.json");
    let review = dir.path().join("comments.json");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");
    let head = current_head(&root)?;

    write_ripr_plus_receipt(&ripr, &head)?;
    write_ripr_pr_receipt(&ripr_pr, &head, 0)?;
    write_empty_review_guidance_receipt(&review, &head)?;
    new_ripr_quality_gate_command(&root, &ripr, &ripr_pr, &review, &receipt, &summary)?
        .assert()
        .success();

    fs::write(&summary, "# Quality Gate\n\nstale summary\n")?;

    let output =
        new_ripr_quality_gate_command(&root, &ripr, &ripr_pr, &review, &receipt, &summary)?
            .arg("--check")
            .output()?;
    assert!(
        !output.status.success(),
        "quality-gate --check must fail when Markdown summary is stale"
    );

    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("quality gate Markdown summary is stale")
            && stderr.contains(&summary.to_string_lossy().to_string()),
        "stale Markdown summary failure must name the stale proof file: {stderr}"
    );

    Ok(())
}

#[test]
fn quality_gate_cli_blocks_new_ripr_when_required_receipts_are_missing() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let ripr = dir.path().join("missing-ripr-plus.json");
    let ripr_pr = dir.path().join("missing-repo-exposure.json");
    let review = dir.path().join("missing-comments.json");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");
    let head = current_head(&root)?;

    let output =
        new_ripr_quality_gate_command(&root, &ripr, &ripr_pr, &review, &receipt, &summary)?
            .output()?;
    assert!(!output.status.success(), "missing required RIPR receipts must fail");
    assert_failure_stderr_points_to_receipt_and_summary(
        &String::from_utf8(output.stderr)?,
        &receipt,
        &summary,
    )?;

    let payload: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    assert_eq!(payload.pointer("/ripr_plus/status").and_then(Value::as_str), Some("missing"));
    assert_eq!(payload.pointer("/ripr_pr/status").and_then(Value::as_str), Some("missing"));
    assert_eq!(payload.pointer("/review_guidance/status").and_then(Value::as_str), Some("missing"));
    assert_blocking_actions_have_repair_contract(&payload)?;

    let ripr_action = next_action(&payload, "ripr_receipt_not_current")?;
    assert_eq!(ripr_action.get("expected_head").and_then(Value::as_str), Some(head.as_str()));
    assert!(
        ripr_action
            .get("repair")
            .and_then(Value::as_str)
            .is_some_and(|repair| repair.contains("does not require total RIPR+ zero yet")),
        "repo-wide receipt failure must describe transitional semantics: {ripr_action}"
    );

    let pr_action = next_action(&payload, "ripr_pr_receipt_not_current")?;
    assert_eq!(pr_action.get("expected_head_sha").and_then(Value::as_str), Some(head.as_str()));
    assert!(
        pr_action.get("verify").and_then(Value::as_str).is_some_and(|verify| verify
            .starts_with("rtk cargo xtask ripr-pr --base origin/HEAD --head HEAD")
            && verify.contains("--check")),
        "diff receipt failure must carry focused verify command: {pr_action}"
    );

    let review_action = next_action(&payload, "ripr_review_receipt_not_current")?;
    assert_eq!(review_action.get("expected_head_sha").and_then(Value::as_str), Some(head.as_str()));
    assert!(
        review_action.get("repair").and_then(Value::as_str).is_some_and(|repair| {
            repair.contains("exact file, line, seam, and suggested proof")
        }),
        "review guidance failure must explain why the receipt is required: {review_action}"
    );

    Ok(())
}

#[test]
fn quality_gate_cli_blocks_new_ripr_when_required_receipts_are_invalid() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let ripr = dir.path().join("ripr-plus.json");
    let ripr_pr = dir.path().join("repo-exposure.json");
    let review = dir.path().join("comments.json");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");

    fs::write(&ripr, "{not-json")?;
    fs::write(&ripr_pr, "{not-json")?;
    fs::write(&review, "{not-json")?;

    let output =
        new_ripr_quality_gate_command(&root, &ripr, &ripr_pr, &review, &receipt, &summary)?
            .output()?;
    assert!(!output.status.success(), "invalid required RIPR receipts must fail");
    assert_failure_stderr_points_to_receipt_and_summary(
        &String::from_utf8(output.stderr)?,
        &receipt,
        &summary,
    )?;

    let payload: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    assert_eq!(payload.pointer("/ripr_plus/status").and_then(Value::as_str), Some("invalid"));
    assert_eq!(payload.pointer("/ripr_pr/status").and_then(Value::as_str), Some("invalid"));
    assert_eq!(payload.pointer("/review_guidance/status").and_then(Value::as_str), Some("invalid"));

    for kind in [
        "ripr_receipt_not_current",
        "ripr_pr_receipt_not_current",
        "ripr_review_receipt_not_current",
    ] {
        let action = next_action(&payload, kind)?;
        assert_eq!(action.get("reason").and_then(Value::as_str), Some("invalid"));
    }
    assert_blocking_actions_have_repair_contract(&payload)?;

    Ok(())
}

#[test]
fn quality_gate_cli_blocks_new_ripr_when_receipts_are_stale() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let ripr = dir.path().join("ripr-plus.json");
    let ripr_pr = dir.path().join("repo-exposure.json");
    let review = dir.path().join("comments.json");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");

    write_ripr_plus_receipt(&ripr, "quality-gate-cli-stale-head")?;
    write_ripr_pr_receipt(&ripr_pr, "quality-gate-cli-stale-pr-head", 0)?;
    write_empty_review_guidance_receipt(&review, "quality-gate-cli-stale-review-head")?;

    let output =
        new_ripr_quality_gate_command(&root, &ripr, &ripr_pr, &review, &receipt, &summary)?
            .output()?;
    assert!(!output.status.success(), "stale required RIPR receipts must fail");

    let payload: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    assert_eq!(payload.pointer("/ripr_plus/status").and_then(Value::as_str), Some("stale"));
    assert_eq!(payload.pointer("/ripr_pr/status").and_then(Value::as_str), Some("stale"));
    assert_eq!(payload.pointer("/review_guidance/status").and_then(Value::as_str), Some("stale"));

    let ripr_action = next_action(&payload, "ripr_receipt_not_current")?;
    assert_eq!(
        ripr_action.get("receipt_head").and_then(Value::as_str),
        Some("quality-gate-cli-stale-head")
    );
    let pr_action = next_action(&payload, "ripr_pr_receipt_not_current")?;
    assert_eq!(
        pr_action.get("receipt_head_sha").and_then(Value::as_str),
        Some("quality-gate-cli-stale-pr-head")
    );
    let review_action = next_action(&payload, "ripr_review_receipt_not_current")?;
    assert_eq!(
        review_action.get("receipt_head_sha").and_then(Value::as_str),
        Some("quality-gate-cli-stale-review-head")
    );

    Ok(())
}

#[test]
fn quality_gate_cli_passes_when_review_guidance_generation_failed_without_new_gaps() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let ripr = dir.path().join("ripr-plus.json");
    let ripr_pr = dir.path().join("repo-exposure.json");
    let review = dir.path().join("comments.json");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");
    let head = current_head(&root)?;

    write_ripr_plus_receipt(&ripr, &head)?;
    write_ripr_pr_receipt(&ripr_pr, &head, 0)?;
    write_error_review_guidance_receipt(&review, &head)?;

    new_ripr_quality_gate_command(&root, &ripr, &ripr_pr, &review, &receipt, &summary)?
        .assert()
        .success();

    let payload: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    assert_eq!(payload.pointer("/review_guidance/status").and_then(Value::as_str), Some("error"));
    assert_eq!(payload.pointer("/decision").and_then(Value::as_str), Some("pass"));
    assert_eq!(payload.get("next_actions").and_then(Value::as_array).map(Vec::len), Some(0));

    Ok(())
}

#[test]
fn quality_gate_cli_blocks_new_ripr_when_review_guidance_generation_failed_with_new_gaps()
-> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let ripr = dir.path().join("ripr-plus.json");
    let ripr_pr = dir.path().join("repo-exposure.json");
    let review = dir.path().join("comments.json");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");
    let head = current_head(&root)?;

    write_ripr_plus_receipt(&ripr, &head)?;
    write_ripr_pr_receipt(&ripr_pr, &head, 1)?;
    write_error_review_guidance_receipt(&review, &head)?;

    let output =
        new_ripr_quality_gate_command(&root, &ripr, &ripr_pr, &review, &receipt, &summary)?
            .output()?;
    assert!(
        !output.status.success(),
        "new RIPR gaps must fail when review guidance producer returned an error receipt"
    );

    let payload: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    assert_eq!(payload.pointer("/review_guidance/status").and_then(Value::as_str), Some("error"));
    assert_eq!(payload.pointer("/ripr_pr/new_unresolved").and_then(Value::as_u64), Some(1));
    next_action(&payload, "new_ripr_gap")?;
    let action = next_action(&payload, "ripr_review_receipt_not_current")?;
    assert_eq!(action.get("reason").and_then(Value::as_str), Some("error"));
    assert!(
        action.get("repair").and_then(Value::as_str).is_some_and(|repair| {
            repair.contains("exact file, line, seam, and suggested proof")
        }),
        "failed review guidance must point agents back to receipt regeneration: {action}"
    );
    assert_blocking_actions_have_repair_contract(&payload)?;

    Ok(())
}

#[test]
fn quality_gate_cli_blocks_new_ripr_when_review_guidance_is_not_actionable() -> TestResult {
    let root = repo_root()?;
    let dir = tempdir()?;
    let ripr = dir.path().join("ripr-plus.json");
    let ripr_pr = dir.path().join("repo-exposure.json");
    let review = dir.path().join("comments.json");
    let receipt = dir.path().join("quality-gate.json");
    let summary = dir.path().join("quality-gate.md");
    let head = current_head(&root)?;

    write_ripr_plus_receipt(&ripr, &head)?;
    write_ripr_pr_receipt(&ripr_pr, &head, 1)?;
    write_non_actionable_review_guidance_receipt(&review, &head)?;

    let output =
        new_ripr_quality_gate_command(&root, &ripr, &ripr_pr, &review, &receipt, &summary)?
            .output()?;
    assert!(
        !output.status.success(),
        "new RIPR gaps must fail when review guidance has no actionable repair packet"
    );

    let payload: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    assert_eq!(
        payload.pointer("/review_guidance/status").and_then(Value::as_str),
        Some("incomplete")
    );
    let action = next_action(&payload, "ripr_review_receipt_not_current")?;
    assert_eq!(action.get("reason").and_then(Value::as_str), Some("incomplete"));

    let markdown = fs::read_to_string(&summary)?;
    assert!(
        !markdown.contains("RIPR-SPEC-NON-ACTIONABLE"),
        "incomplete guidance must not be rendered as a repair packet: {markdown}"
    );

    Ok(())
}

fn new_ripr_quality_gate_command(
    root: &Path,
    ripr: &Path,
    ripr_pr: &Path,
    review: &Path,
    receipt: &Path,
    summary: &Path,
) -> TestResult<Command> {
    let mut command = Command::cargo_bin("xtask")?;
    command.current_dir(root).args(["quality-gate", "--mode", "enforce-new-ripr"]);
    command.arg("--ripr-receipt").arg(ripr);
    command.arg("--ripr-pr-receipt").arg(ripr_pr);
    command.arg("--review-receipt").arg(review);
    command.arg("--receipt").arg(receipt);
    command.arg("--summary").arg(summary);
    Ok(command)
}

fn next_action<'a>(receipt: &'a Value, kind: &str) -> TestResult<&'a Value> {
    receipt
        .get("next_actions")
        .and_then(Value::as_array)
        .and_then(|actions| {
            actions.iter().find(|action| action.get("kind").and_then(Value::as_str) == Some(kind))
        })
        .ok_or_else(|| format!("missing next action `{kind}`").into())
}

fn assert_blocking_actions_have_repair_contract(receipt: &Value) -> TestResult {
    let actions =
        receipt.get("next_actions").and_then(Value::as_array).ok_or("missing next_actions")?;
    let blocking = actions
        .iter()
        .filter(|action| action.get("blocking").and_then(Value::as_bool) == Some(true))
        .collect::<Vec<_>>();
    if blocking.is_empty() {
        return Err("receipt must contain at least one blocking action".into());
    }
    for action in blocking {
        let kind = action.get("kind").and_then(Value::as_str).unwrap_or("unknown");
        let path = action
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("blocking action {kind} missing path"))?;
        if path.trim().is_empty() {
            return Err(format!("blocking action {kind} has empty path").into());
        }
        for field in ["repair", "verify", "receipt"] {
            let value = action
                .get(field)
                .and_then(Value::as_str)
                .ok_or_else(|| format!("blocking action {kind} missing {field}"))?;
            if value.trim().is_empty() {
                return Err(format!("blocking action {kind} has empty {field}").into());
            }
            if matches!(field, "verify" | "receipt") && !value.starts_with("rtk ") {
                return Err(format!("blocking action {kind} {field} must use rtk: {value}").into());
            }
        }
    }
    Ok(())
}

fn assert_failure_stderr_points_to_receipt_and_summary(
    stderr: &str,
    receipt: &Path,
    summary: &Path,
) -> TestResult {
    let receipt = receipt.to_string_lossy();
    let summary = summary.to_string_lossy();
    for required in
        ["quality gate failed", "see receipt", receipt.as_ref(), "summary", summary.as_ref()]
    {
        assert!(
            stderr.contains(required),
            "quality-gate failure stderr missing `{required}`: {stderr}"
        );
    }
    Ok(())
}

fn assert_action_commands_use_quality_gate_mode(action: &Value, mode: &str) -> TestResult {
    for field in ["verify", "receipt"] {
        let command = action
            .get(field)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("action missing {field}: {action}"))?;
        assert!(
            command.contains(&format!("quality-gate --mode {mode} ")),
            "action {field} must use active quality-gate mode `{mode}`: {command}"
        );
        for other_mode in ["enforce-patch-coverage", "enforce"] {
            assert!(
                !command.contains(&format!("--mode {other_mode} ")),
                "action {field} must not use unrelated mode `{other_mode}`: {command}"
            );
        }
    }
    Ok(())
}

fn repo_root() -> TestResult<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "xtask manifest must be nested under repo root".into())
}

fn current_head(root: &Path) -> TestResult<String> {
    let output = StdCommand::new("git").args(["rev-parse", "HEAD"]).current_dir(root).output()?;
    if !output.status.success() {
        return Err(format!("git rev-parse HEAD failed with status {}", output.status).into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn write_ripr_plus_receipt(path: &Path, head: &str) -> TestResult {
    write_json(
        path,
        json!({
            "schema_version": 1,
            "kind": "ripr_plus_baseline",
            "head": head,
            "unresolved": 0,
            "top_files": []
        }),
    )
}

fn write_invalid_exception_policy(path: &Path) -> TestResult {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        path,
        r##"schema_version = 1
policy = "quality-gate-exceptions"
owner = "EffortlessMetrics"
status = "active"
updated = "2026-05-28"
due_review = "fail"

[requirements]
required_active = ["ripr-total-burndown"]

[[exception]]
id = "ripr-total-burndown"
kind = "permanent_bypass"
owner = "proof-lane"
reason = "transition burn-down remains active"
final_target = "repo-wide ripr+ unresolved total = 0"
evidence = "target/receipts/quality/ripr-plus.json"
removal_criteria = "remove when RIPR+ total is zero"
created = "2026-05-28"
review_after = "2099-01-01"
expires = "2099-12-31"
"##,
    )?;
    Ok(())
}

fn write_ripr_pr_receipt(path: &Path, head: &str, severe_gaps: u64) -> TestResult {
    write_json(
        path,
        json!({
            "schema_version": "0.1",
            "tool": "ripr",
            "kind": "pr_evidence",
            "scope": "diff",
            "base": "quality-gate-cli-test-base",
            "base_sha": "quality-gate-cli-test-base-sha",
            "head": "HEAD",
            "head_sha": head,
            "summary": {
                "changed_files": 1,
                "weakly_exposed": severe_gaps,
                "reachable_unrevealed": 0,
                "no_static_path": 0,
                "severe_gaps": severe_gaps
            }
        }),
    )
}

fn write_review_guidance_receipt(path: &Path, head: &str) -> TestResult {
    write_json(
        path,
        json!({
            "schema_version": "0.1",
            "tool": "ripr",
            "status": "advisory",
            "base": "quality-gate-cli-test-base",
            "base_sha": "quality-gate-cli-test-base-sha",
            "head": "HEAD",
            "head_sha": head,
            "summary": {
                "comments": 2,
                "summary_only": 0,
                "suppressed": 0
            },
            "comments": [
                actionable_review_guidance_item(),
                {
                    "canonical_gap_id": "RIPR-SPEC-NON-ACTIONABLE",
                    "kind": "focused_test",
                    "severity": "severe",
                    "reason": "missing placement line should not become a repair packet",
                    "placement": {
                        "path": "crates/perl-parser/src/lib.rs",
                        "mode": "exact_seam_line"
                    },
                    "suggested_test": {
                        "intent": "this row is intentionally incomplete"
                    }
                }
            ],
            "summary_only": [],
            "suppressed": [],
            "warnings": []
        }),
    )
}

fn write_summary_only_review_guidance_receipt(path: &Path, head: &str) -> TestResult {
    write_json(
        path,
        json!({
            "schema_version": "0.1",
            "tool": "ripr",
            "status": "advisory",
            "base": "quality-gate-cli-test-base",
            "base_sha": "quality-gate-cli-test-base-sha",
            "head": "HEAD",
            "head_sha": head,
            "summary": {
                "comments": 0,
                "summary_only": 1,
                "suppressed": 0
            },
            "comments": [],
            "summary_only": [
                {
                    "gap_id": "RIPR-SPEC-SUMMARY",
                    "file": "xtask/src/tasks/quality_gate.rs",
                    "line": 137,
                    "owner": "summary renderer",
                    "why": "summary-only guidance still needs a repair packet",
                    "repair": "cover summary-only review guidance"
                }
            ],
            "suppressed": [],
            "warnings": []
        }),
    )
}

fn write_non_actionable_review_guidance_receipt(path: &Path, head: &str) -> TestResult {
    write_json(
        path,
        json!({
            "schema_version": "0.1",
            "tool": "ripr",
            "status": "advisory",
            "base": "quality-gate-cli-test-base",
            "base_sha": "quality-gate-cli-test-base-sha",
            "head": "HEAD",
            "head_sha": head,
            "summary": {
                "comments": 1,
                "summary_only": 0,
                "suppressed": 0
            },
            "comments": [
                {
                    "canonical_gap_id": "RIPR-SPEC-NON-ACTIONABLE",
                    "kind": "focused_test",
                    "severity": "severe",
                    "reason": "missing seam details should not become a repair packet",
                    "placement": {
                        "path": "crates/perl-parser/src/lib.rs",
                        "line": 42
                    },
                    "suggested_test": {
                        "intent": "this row is intentionally incomplete"
                    }
                }
            ],
            "summary_only": [],
            "suppressed": [],
            "warnings": []
        }),
    )
}

fn write_error_review_guidance_receipt(path: &Path, head: &str) -> TestResult {
    write_json(
        path,
        json!({
            "schema_version": "0.1",
            "tool": "ripr",
            "status": "error",
            "base": "quality-gate-cli-test-base",
            "base_sha": "quality-gate-cli-test-base-sha",
            "head": "HEAD",
            "head_sha": head,
            "summary": {
                "comments": 0,
                "summary_only": 0,
                "suppressed": 0
            },
            "comments": [],
            "summary_only": [],
            "suppressed": [],
            "warnings": [
                {
                    "kind": "tool_error",
                    "message": "ripr review-comments failed",
                    "path": null
                }
            ]
        }),
    )
}

fn write_empty_review_guidance_receipt(path: &Path, head: &str) -> TestResult {
    write_json(
        path,
        json!({
            "schema_version": "0.1",
            "tool": "ripr",
            "status": "advisory",
            "base": "quality-gate-cli-test-base",
            "base_sha": "quality-gate-cli-test-base-sha",
            "head": "HEAD",
            "head_sha": head,
            "summary": {
                "comments": 0,
                "summary_only": 0,
                "suppressed": 0
            },
            "comments": [],
            "summary_only": [],
            "suppressed": [],
            "warnings": []
        }),
    )
}

fn actionable_review_guidance_item() -> Value {
    json!({
        "canonical_gap_id": "RIPR-SPEC-CLI",
        "kind": "focused_test",
        "severity": "severe",
        "reason": "changed parser branch has only weak proof",
        "placement": {
            "path": "crates/perl-parser/src/lib.rs",
            "line": 42,
            "mode": "exact_seam_line"
        },
        "suggested_test": {
            "intent": "prove parser branch recovery"
        }
    })
}

fn write_json(path: &Path, value: Value) -> TestResult {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(&value)?)?;
    Ok(())
}
