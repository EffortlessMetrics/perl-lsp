//! Policy guards for the external Droid PR review workflow.

use std::{collections::BTreeMap, fs, path::PathBuf};

use anyhow::{Result, anyhow};
use serde_yaml_ng::Value;

#[test]
fn droid_review_runs_on_trusted_self_hosted_factory_runner() -> Result<()> {
    let (_content, workflow) = droid_review_workflow()?;
    let job = mapping_value(mapping_value(&workflow, "jobs")?, "droid-review")?;
    let runs_on = sequence_strings(mapping_value(job, "runs-on")?)?;

    assert_eq!(
        runs_on,
        ["self-hosted", "linux", "x64", "em-ci", "trusted-pr", "review-nano", "droid-review"],
        "Droid PR Review must stay on the trusted self-hosted Droid review runner"
    );

    Ok(())
}

#[test]
fn droid_review_keeps_secret_bearing_pr_guard() -> Result<()> {
    let (content, workflow) = droid_review_workflow()?;
    let job = mapping_value(mapping_value(&workflow, "jobs")?, "droid-review")?;
    let condition = scalar_string(mapping_value(job, "if")?)?;

    assert!(
        condition.contains("github.event.pull_request.draft == false")
            && condition
                .contains("github.event.pull_request.head.repo.full_name == github.repository"),
        "Droid PR Review must only expose secrets to non-draft same-repo PRs"
    );
    assert!(
        content.contains("types: [opened, synchronize, reopened, ready_for_review]"),
        "Droid PR Review must keep the narrow pull_request event set"
    );

    Ok(())
}

#[test]
fn droid_review_uses_temp_factory_home_for_m3_custom_model() -> Result<()> {
    let (content, _workflow) = droid_review_workflow()?;

    for required in [
        "droid_home=\"${RUNNER_TEMP}/droid-home\"",
        "mkdir -p \"${droid_home}/.factory\"",
        "\"displayName\": \"MiniMax-M3\"",
        "\"model\": \"MiniMax-M3\"",
        "\"baseUrl\": \"https://api.minimax.io/anthropic\"",
        "review_model: custom:MiniMax-M3-0",
        "HOME: ${{ runner.temp }}/droid-home",
    ] {
        assert!(content.contains(required), "Droid PR Review missing `{required}`");
    }
    assert!(
        !content.contains("MiniMax-M2.7"),
        "Droid PR Review must not regress to the legacy MiniMax M2.7 model"
    );

    Ok(())
}

#[test]
fn droid_review_uses_pinned_safe_action_without_debug_artifacts() -> Result<()> {
    let (content, _workflow) = droid_review_workflow()?;
    let action_line = content
        .lines()
        .find(|line| line.trim_start().starts_with("uses: EffortlessMetrics/droid-action-safe@"))
        .ok_or_else(|| anyhow!("Droid PR Review safe action step missing"))?;
    let pinned_ref = action_line
        .split('@')
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .ok_or_else(|| anyhow!("Droid action pin missing"))?;

    assert_eq!(pinned_ref.len(), 40, "Droid action must be pinned by full commit SHA");
    assert!(
        pinned_ref.chars().all(|ch| ch.is_ascii_hexdigit()),
        "Droid action pin must be a commit SHA, got `{pinned_ref}`"
    );
    assert!(
        !content.contains("Factory-AI/droid-action@main"),
        "Droid PR Review must not use mutable Factory action refs"
    );
    assert!(
        content.contains("upload_debug_artifacts: false"),
        "Droid PR Review must keep raw debug artifact upload disabled"
    );

    Ok(())
}

#[test]
fn droid_review_clears_legacy_anthropic_env_vars() -> Result<()> {
    let (content, _workflow) = droid_review_workflow()?;

    assert!(
        content.contains("ANTHROPIC_AUTH_TOKEN: \"\"")
            && content.contains("ANTHROPIC_BASE_URL: \"\""),
        "Droid PR Review must clear legacy Anthropic env vars so Factory uses the custom model"
    );

    Ok(())
}

#[test]
fn droid_review_permissions_stay_minimal() -> Result<()> {
    let (_content, workflow) = droid_review_workflow()?;
    let job = mapping_value(mapping_value(&workflow, "jobs")?, "droid-review")?;
    let permissions = string_map(mapping_value(job, "permissions")?)?;
    let expected = BTreeMap::from([
        ("actions".to_string(), "read".to_string()),
        ("contents".to_string(), "read".to_string()),
        ("id-token".to_string(), "write".to_string()),
        ("issues".to_string(), "write".to_string()),
        ("pull-requests".to_string(), "write".to_string()),
    ]);

    assert_eq!(permissions, expected, "Droid PR Review permissions changed unexpectedly");

    Ok(())
}

#[test]
fn droid_review_emits_live_run_receipt() -> Result<()> {
    let (_content, workflow) = droid_review_workflow()?;
    let job = mapping_value(mapping_value(&workflow, "jobs")?, "droid-review")?;
    let review_step = named_step(job, "Run Droid Auto Review")?;
    let receipt_step = named_step(job, "Write Droid live-run receipt")?;
    let upload_step = named_step(job, "Upload Droid live-run receipt")?;

    assert_eq!(
        scalar_string(mapping_value(review_step, "id")?)?,
        "droid_review",
        "Droid action step must expose an id so the receipt can record action outcome"
    );

    assert_eq!(
        scalar_string(mapping_value(receipt_step, "if")?)?,
        "always()",
        "Droid live-run receipt must be written even when the advisory review action fails"
    );
    let receipt_env = string_map(mapping_value(receipt_step, "env")?)?;
    assert_eq!(
        receipt_env.get("DROID_REVIEW_OUTCOME").map(String::as_str),
        Some("${{ steps.droid_review.outcome }}"),
        "Droid live-run receipt must derive review_posted from the Droid action outcome"
    );
    let receipt_run = scalar_string(mapping_value(receipt_step, "run")?)?;
    for required in [
        "receipt_path=\"${receipt_dir}/droid-live-run.json\"",
        "\"check\": \"droid-live-run\"",
        "\"schema_version\": \"1\"",
        "\"event\": \"pull_request\"",
        "\"verdict\": \"${verdict}\"",
        "\"workflow\": \"Droid PR Review\"",
        "\"model\": \"custom:MiniMax-M3-0\"",
        "\"runner_labels\": [\"self-hosted\", \"linux\", \"x64\", \"em-ci\", \"trusted-pr\", \"review-nano\", \"droid-review\"]",
        "\"action\": \"EffortlessMetrics/droid-action-safe\"",
        "\"review_posted\": ${review_posted}",
        "\"debug_artifacts_uploaded\": false",
        "\"anthropic_env_cleared\": true",
    ] {
        assert!(receipt_run.contains(required), "Droid live-run receipt missing `{required}`");
    }

    assert_eq!(
        scalar_string(mapping_value(upload_step, "if")?)?,
        "always()",
        "Droid live-run receipt upload must run for advisory failures"
    );
    assert_eq!(
        scalar_string(mapping_value(upload_step, "uses")?)?,
        "./.github/actions/upload-receipt",
        "Droid live-run receipt must use the repo-local receipt upload action"
    );
    let upload_with = string_map(mapping_value(upload_step, "with")?)?;
    assert_eq!(
        upload_with.get("receipt-path").map(String::as_str),
        Some("${{ runner.temp }}/droid-receipts/droid-live-run.json"),
        "Droid live-run receipt upload path changed unexpectedly"
    );
    assert_eq!(
        upload_with.get("artifact-name").map(String::as_str),
        Some("droid-live-run-${{ github.run_id }}-${{ github.run_attempt }}"),
        "Droid live-run receipt artifact name should identify the workflow run"
    );
    assert_eq!(
        upload_with.get("generate-summary").map(String::as_str),
        Some("false"),
        "Droid live-run receipt must not use gate-receipt summary rendering"
    );

    Ok(())
}

#[test]
fn droid_live_run_receipt_schema_is_registered() -> Result<()> {
    let root = repo_root()?;
    let registry = fs::read_to_string(root.join(".ci/receipts/registry.toml"))?;

    for required in [
        "check = \"droid-live-run\"",
        "schema = \".ci/receipts/schemas/droid-live-run.schema.json\"",
        "producer = \"Droid PR Review\"",
        "required_fields = [\"check\", \"schema_version\", \"event\", \"verdict\", \"workflow\", \"model\", \"runner_labels\", \"action\", \"review_posted\", \"debug_artifacts_uploaded\", \"anthropic_env_cleared\"]",
    ] {
        assert!(
            registry.contains(required),
            "Droid live-run receipt registry missing `{required}`"
        );
    }

    let schema_path = root.join(".ci/receipts/schemas/droid-live-run.schema.json");
    let schema_text = fs::read_to_string(&schema_path)?;
    let schema: serde_json::Value = serde_json::from_str(&schema_text)?;

    assert_eq!(schema["properties"]["check"]["const"], "droid-live-run");
    assert_eq!(schema["properties"]["event"]["const"], "pull_request");
    assert_eq!(schema["properties"]["workflow"]["const"], "Droid PR Review");
    assert_eq!(schema["properties"]["model"]["const"], "custom:MiniMax-M3-0");
    assert_eq!(schema["properties"]["action"]["const"], "EffortlessMetrics/droid-action-safe");
    assert_eq!(schema["properties"]["debug_artifacts_uploaded"]["const"], false);
    assert_eq!(schema["properties"]["anthropic_env_cleared"]["const"], true);

    Ok(())
}

fn droid_review_workflow() -> Result<(String, Value)> {
    let path = repo_root()?.join(".github/workflows/droid-review.yml");
    let content = fs::read_to_string(&path)?;
    let workflow = serde_yaml_ng::from_str(&content)?;
    Ok((content, workflow))
}

fn repo_root() -> Result<PathBuf> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("xtask must live under the repository root"))
}

fn mapping_value<'a>(value: &'a Value, key: &str) -> Result<&'a Value> {
    value
        .as_mapping()
        .ok_or_else(|| anyhow!("expected YAML mapping while looking for `{key}`"))?
        .get(Value::String(key.to_string()))
        .ok_or_else(|| anyhow!("missing YAML key `{key}`"))
}

fn sequence_strings(value: &Value) -> Result<Vec<&str>> {
    value
        .as_sequence()
        .ok_or_else(|| anyhow!("expected YAML sequence"))?
        .iter()
        .map(scalar_string)
        .collect()
}

fn sequence_values(value: &Value) -> Result<&[Value]> {
    value.as_sequence().map(Vec::as_slice).ok_or_else(|| anyhow!("expected YAML sequence"))
}

fn named_step<'a>(job: &'a Value, name: &str) -> Result<&'a Value> {
    let steps = sequence_values(mapping_value(job, "steps")?)?;

    for step in steps {
        if mapping_value(step, "name").ok().and_then(|value| scalar_string(value).ok())
            == Some(name)
        {
            return Ok(step);
        }
    }

    Err(anyhow!("missing workflow step `{name}`"))
}

fn scalar_string(value: &Value) -> Result<&str> {
    value.as_str().ok_or_else(|| anyhow!("expected YAML string scalar"))
}

fn string_map(value: &Value) -> Result<BTreeMap<String, String>> {
    value
        .as_mapping()
        .ok_or_else(|| anyhow!("expected YAML mapping"))?
        .iter()
        .map(|(key, value)| {
            let key = scalar_string(key)?.to_string();
            let value = scalar_string(value)?.to_string();
            Ok((key, value))
        })
        .collect()
}
