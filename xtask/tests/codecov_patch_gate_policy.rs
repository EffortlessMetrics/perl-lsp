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
fn codecov_patch_status_requires_95_with_no_threshold() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root()?;
    let raw_config = fs::read_to_string(root.join("codecov.yml"))?;
    let config: Value = serde_yaml_ng::from_str(&raw_config)?;

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
    assert_ne!(
        yaml_path(&config, &["coverage", "status", "patch", "default", "informational"],),
        Some("true"),
        "Codecov patch status must be blocking, not informational"
    );
    assert_eq!(
        yaml_path(&config, &["coverage", "status", "project", "default", "target"],),
        Some("95%"),
        "Codecov project status should advertise the final 95% target"
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
fn coverage_docs_describe_patch_front_door_without_ci_wiring()
-> Result<(), Box<dyn std::error::Error>> {
    let root = project_root()?;
    let coverage_doc = fs::read_to_string(root.join("docs/how-to/COVERAGE.md"))?;
    let rollout_doc = fs::read_to_string(root.join("docs/ci/codecov-rollout.md"))?;
    let coverage_readme = fs::read_to_string(root.join(".ci/README-coverage.md"))?;

    assert!(
        coverage_doc.contains("| **Patch** | 95% | Blocking PR gate with `0%` threshold |")
            && coverage_doc.contains(
                "| **Project** | 95% | Informational during burn-down; final target is blocking |",
            ),
        "coverage how-to must describe the Codecov patch/project policy targets"
    );

    let ci_section = section_block(&coverage_doc, "## CI Integration")
        .ok_or("coverage how-to is missing CI Integration section")?;
    assert!(
        ci_section.contains("Patch coverage is the front-door PR coverage gate")
            && ci_section.contains("Project coverage remains informational during burn-down")
            && ci_section.contains("Workflow wiring remains a separate follow-up slice"),
        "coverage how-to must describe the transitional Codecov rollout posture"
    );
    assert!(
        !ci_section.contains("ci:coverage")
            && !ci_section.contains("quality-gate --mode enforce-new-ripr")
            && !ci_section.contains("quality-gate --mode enforce "),
        "coverage docs must not preserve label-gated, RIPR, or final quality-gate CI language"
    );

    let current_policy = section_block(&rollout_doc, "## Proof-lane Codecov posture")
        .ok_or("rollout doc is missing proof-lane Codecov posture")?;
    assert!(
        current_policy.contains("patch `95%` / `0%`")
            && current_policy.contains("project `95%` remains informational")
            && current_policy.contains("does not implement workflow enforcement")
            && current_policy.contains("`quality-gate` CLI"),
        "rollout doc must describe the active PR2 policy/docs boundary"
    );
    assert!(
        rollout_doc.contains("Historical Codecov ladder")
            && rollout_doc.contains("superseded by the proof-enforcement lane"),
        "rollout doc must mark the older Codecov ladder as historical"
    );
    assert!(
        coverage_readme.contains("rtk just coverage-summary")
            && coverage_readme.contains("rtk just coverage-branch-gate")
            && coverage_readme.contains("rtk just coverage-baseline-refresh"),
        "coverage README must show rtk-prefixed local coverage policy commands"
    );

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
