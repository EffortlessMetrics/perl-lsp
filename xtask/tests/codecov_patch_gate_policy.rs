//! Contract tests for Codecov patch-coverage policy.

use std::fs;
use std::path::PathBuf;

use serde_yaml_ng::Value;

fn project_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("CARGO_MANIFEST_DIR has no parent")?
        .to_path_buf())
}

#[test]
fn codecov_patch_status_is_advisory_95_with_no_threshold() -> Result<(), Box<dyn std::error::Error>>
{
    let root = project_root()?;
    let raw_config = fs::read_to_string(root.join("codecov.yml"))?;
    let config: Value = serde_yaml_ng::from_str(&raw_config)?;

    assert_eq!(
        yaml_path(&config, &["codecov", "require_ci_to_pass"]),
        Some("false"),
        "Codecov statuses must not wait for unrelated CI/test gates to pass"
    );
    assert_eq!(
        yaml_path(&config, &["codecov", "notify", "wait_for_ci"]),
        Some("false"),
        "Codecov notifications must not be delayed on unrelated CI/test gates"
    );
    assert_eq!(
        yaml_path(&config, &["coverage", "status", "patch", "default", "target"]),
        Some("95%"),
        "Codecov patch status must require 95% coverage"
    );
    assert_eq!(
        yaml_path(&config, &["coverage", "status", "patch", "default", "threshold"]),
        Some("0%"),
        "Codecov patch status must have no threshold allowance"
    );
    assert_eq!(
        yaml_path(&config, &["coverage", "status", "patch", "default", "if_ci_failed"]),
        Some("ignore"),
        "Codecov patch status must evaluate coverage independently from unrelated CI/test failures"
    );
    assert_ne!(
        yaml_path(&config, &["coverage", "status", "patch", "default", "informational"],),
        Some("false"),
        "Codecov patch status must not be configured as blocking"
    );
    assert_eq!(
        yaml_path(&config, &["coverage", "status", "patch", "default", "informational"],),
        Some("true"),
        "Codecov patch status must be advisory; RIPR+ and focused tests are required"
    );
    assert_eq!(
        yaml_path(&config, &["coverage", "status", "project", "default", "target"],),
        Some("95%"),
        "Codecov project status should advertise the final 95% target"
    );
    assert_eq!(
        yaml_path(&config, &["coverage", "status", "project", "default", "if_ci_failed"]),
        Some("ignore"),
        "Codecov project status must not inherit unrelated CI/test failures"
    );
    assert_eq!(
        yaml_path(&config, &["coverage", "status", "project", "default", "informational"],),
        Some("true"),
        "project coverage remains informational while burn-down is open"
    );
    assert!(
        !raw_config.contains("- \"xtask/**\""),
        "proof-rail xtask code must not be ignored by Codecov"
    );
    let ignored_paths = config
        .get("ignore")
        .and_then(Value::as_sequence)
        .ok_or("codecov.yml is missing ignore paths")?;
    for required_ignore in
        [".github/**", ".ci/**", "codecov.yml", "docs/**", "justfile", "xtask/tests/**"]
    {
        assert!(
            ignored_paths
                .iter()
                .any(|path| matches!(path, Value::String(path) if path == required_ignore)),
            "Codecov patch status must ignore non-LCOV proof-lane path `{required_ignore}`"
        );
    }
    assert_eq!(
        yaml_path(&config, &["flags", "xtask", "paths", "0"]),
        Some("xtask/src/"),
        "Codecov must expose an xtask flag so proof-rail coverage is inspectable"
    );

    Ok(())
}

#[test]
fn codecov_flags_do_not_carry_status_targets() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root()?;
    let config: Value = serde_yaml_ng::from_str(&fs::read_to_string(root.join("codecov.yml"))?)?;
    let flags =
        config.get("flags").and_then(Value::as_mapping).ok_or("codecov.yml is missing flags")?;

    for (name, config) in flags {
        assert!(
            config.get("target").is_none(),
            "Codecov flag {name:?} must not use unsupported target fields"
        );
    }

    Ok(())
}

#[test]
fn coverage_docs_describe_advisory_patch_policy_without_pr_wiring()
-> Result<(), Box<dyn std::error::Error>> {
    let root = project_root()?;
    let coverage_doc = fs::read_to_string(root.join("docs/how-to/COVERAGE.md"))?;
    let rollout_doc = fs::read_to_string(root.join("docs/ci/codecov-rollout.md"))?;
    let coverage_readme = fs::read_to_string(root.join(".ci/README-coverage.md"))?;

    let ci_section = section_block(&coverage_doc, "## CI Integration")
        .ok_or("coverage how-to is missing CI Integration section")?;
    assert!(
        ci_section.contains("Patch coverage is an advisory coverage signal")
            && ci_section.contains("not a normal PR merge gate")
            && ci_section.contains("Coverage does not run on PRs or merge queues")
            && ci_section.contains("Project coverage remains")
            && ci_section.contains("informational during burn-down")
            && ci_section.contains("Scheduled and manually dispatched coverage runs"),
        "coverage how-to must describe the advisory Codecov rollout posture"
    );
    assert!(
        !ci_section.contains("front-door PR coverage gate")
            && !ci_section.contains("quality-gate --mode enforce-new-ripr")
            && !ci_section.contains("quality-gate --mode enforce "),
        "coverage docs must not preserve old blocking, RIPR, or final quality-gate CI language"
    );

    let current_policy = section_block(&rollout_doc, "## Advisory Codecov posture")
        .ok_or("rollout doc is missing advisory Codecov posture")?;
    assert!(
        current_policy.contains("patch `95%` / `0%` is the advisory coverage target")
            && current_policy.contains("project `95%` remains informational")
            && current_policy.contains("no longer runs on PRs or merge groups")
            && current_policy.contains("nightly/manual events")
            && current_policy.contains("quality-gate --mode enforce-patch-coverage"),
        "rollout doc must describe the active advisory Codecov posture"
    );
    assert!(
        rollout_doc.contains("Historical Codecov ladder")
            && rollout_doc.contains("advisory manual/nightly coverage"),
        "rollout doc must mark the older Codecov ladder as historical"
    );
    assert!(
        coverage_readme.contains("rtk just coverage-summary")
            && coverage_readme.contains("rtk just coverage-branch-gate")
            && coverage_readme.contains("rtk just coverage-baseline-refresh"),
        "coverage README must show rtk-prefixed local coverage policy commands"
    );
    assert!(
        coverage_doc.contains("Codecov / Patch 95")
            && coverage_doc.contains("fail_ci_if_error: false")
            && coverage_doc.contains("local quality-gate receipt"),
        "coverage how-to must describe the local receipt as the advisory patch proof"
    );
    for stale_phrase in [
        "fail_ci_if_error: true",
        "upload failures block PRs",
        "missing token, upload error, or Codecov processing failure prevents the PR from merging",
    ] {
        assert!(
            !coverage_doc.contains(stale_phrase),
            "coverage how-to must not preserve stale blocking Codecov upload guidance: {stale_phrase}"
        );
    }

    Ok(())
}

fn yaml_path<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for key in path {
        current = match current {
            Value::Mapping(mapping) => mapping.get(Value::String((*key).to_string()))?,
            Value::Sequence(items) => items.get(key.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    match current {
        Value::String(value) => Some(value.as_str()),
        Value::Bool(true) => Some("true"),
        Value::Bool(false) => Some("false"),
        _ => None,
    }
}

fn section_block<'a>(content: &'a str, heading: &str) -> Option<&'a str> {
    let start = content.find(heading)?;
    let rest = &content[start..];
    let next = rest
        .lines()
        .skip(1)
        .scan(heading.len() + 1, |offset, line| {
            let current = *offset;
            *offset += line.len() + 1;
            Some((current, line))
        })
        .find_map(
            |(offset, line)| {
                if line.starts_with("## ") && line != heading { Some(offset) } else { None }
            },
        )
        .unwrap_or(rest.len());
    Some(&rest[..next])
}
