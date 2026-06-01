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
        ["self-hosted", "linux", "x64", "em-ci", "trusted-pr", "workflow-nano"],
        "Droid PR Review must stay on the trusted self-hosted workflow-nano runner"
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
