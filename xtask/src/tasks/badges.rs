//! Generated public Shields endpoint badges.
//!
//! README badges are repo-scoped public trust markers. Diff-scoped evidence
//! stays in PR artifacts under `target/` and must not be copied into these
//! committed endpoint files.

use std::fs;
use std::path::Path;
use std::process::Command;

use color_eyre::eyre::{Context, Result, bail};
use serde_json::json;

const BADGE_ENDPOINT_DIR: &str = "badges";
const BADGE_ENDPOINT_TARGET_DIR: &str = "target/xtask/badges";
const RIPR_PLUS_ENDPOINT: &str = "ripr-plus.json";

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct ShieldsEndpointBadge {
    #[serde(rename = "schemaVersion")]
    pub schema_version: u8,
    pub label: String,
    pub message: String,
    pub color: String,
}

pub fn run(check: bool) -> Result<()> {
    let workspace_root = crate::utils::project_root()?;
    let target_dir = workspace_root.join(BADGE_ENDPOINT_TARGET_DIR);
    fs::create_dir_all(&target_dir).wrap_err_with(|| {
        format!("failed to create badge target directory `{}`", target_dir.display())
    })?;

    let ripr_plus = ripr_plus_badge(&workspace_root)?;
    validate_shields_badge(&ripr_plus, Some("ripr+"))?;
    write_json_pretty(&target_dir.join(RIPR_PLUS_ENDPOINT), &ripr_plus)?;

    if check {
        let committed_path = workspace_root.join(BADGE_ENDPOINT_DIR).join(RIPR_PLUS_ENDPOINT);
        let generated_path = target_dir.join(RIPR_PLUS_ENDPOINT);
        compare_files(&committed_path, &generated_path)?;
        println!("badges: committed endpoints are current");
        return Ok(());
    }

    let committed_dir = workspace_root.join(BADGE_ENDPOINT_DIR);
    fs::create_dir_all(&committed_dir).wrap_err_with(|| {
        format!("failed to create committed badge directory `{}`", committed_dir.display())
    })?;
    fs::copy(target_dir.join(RIPR_PLUS_ENDPOINT), committed_dir.join(RIPR_PLUS_ENDPOINT))
        .wrap_err("failed to refresh committed ripr+ badge endpoint")?;

    println!("badges: refreshed public endpoint JSON under badges/");
    Ok(())
}

fn ripr_plus_badge(workspace_root: &Path) -> Result<ShieldsEndpointBadge> {
    ensure_test_efficiency_report(workspace_root)?;

    let ripr_bin = std::env::var("RIPR_BIN").unwrap_or_else(|_| "ripr".to_string());

    let output = Command::new(&ripr_bin)
        .arg("check")
        .arg("--root")
        .arg(workspace_root)
        .arg("--format")
        .arg("repo-badge-plus-shields")
        .current_dir(workspace_root)
        .output()
        .wrap_err_with(|| format!("failed to run `{ripr_bin}` for ripr+ badge endpoint"))?;

    if !output.status.success() {
        bail!(
            "{ripr_bin} repo-badge-plus-shields failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    serde_json::from_slice(&output.stdout)
        .wrap_err_with(|| format!("{ripr_bin} emitted invalid Shields endpoint JSON"))
}

fn ensure_test_efficiency_report(workspace_root: &Path) -> Result<()> {
    let report_path = workspace_root.join("target/ripr/reports/test-efficiency.json");
    if report_path.exists() {
        return Ok(());
    }

    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent).wrap_err_with(|| {
            format!("failed to create ripr report directory `{}`", parent.display())
        })?;
    }

    let report = json!({
        "schema_version": "0.1",
        "tests": [],
        "metrics": {
            "tests_scanned": 0,
            "reason_counts": {
                "no_assertion_detected": 0,
                "smoke_oracle_only": 0,
                "relational_oracle": 0,
                "broad_oracle": 0,
                "assertion_may_not_match_detected_owner": 0,
                "opaque_helper_or_fixture_boundary": 0,
                "no_activation_literal_detected": 0,
                "expected_value_computed_from_detected_owner_path": 0,
                "duplicate_activation_and_oracle_shape": 0
            }
        }
    });
    let bytes = serde_json::to_vec_pretty(&report)
        .wrap_err("failed to serialize neutral ripr test-efficiency report")?;
    fs::write(&report_path, [&bytes[..], b"\n"].concat()).wrap_err_with(|| {
        format!("failed to write ripr test-efficiency report `{}`", report_path.display())
    })
}

pub fn validate_shields_badge(
    badge: &ShieldsEndpointBadge,
    expected_label: Option<&str>,
) -> Result<()> {
    if badge.schema_version != 1 {
        bail!("badge `{}` has unsupported schemaVersion", badge.label);
    }

    if let Some(expected_label) = expected_label
        && badge.label != expected_label
    {
        bail!("badge label drifted: got `{}`, expected `{expected_label}`", badge.label);
    }

    if badge.message.trim().is_empty() {
        bail!("badge `{}` has empty message", badge.label);
    }

    if badge.color.trim().is_empty() {
        bail!("badge `{}` has empty color", badge.label);
    }

    Ok(())
}

fn write_json_pretty(path: &Path, badge: &ShieldsEndpointBadge) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(badge).wrap_err("failed to serialize Shields badge")?;
    fs::write(path, [&bytes[..], b"\n"].concat())
        .wrap_err_with(|| format!("failed to write badge endpoint `{}`", path.display()))
}

fn compare_files(committed_path: &Path, generated_path: &Path) -> Result<()> {
    let committed = read_file(committed_path)?;
    let generated = read_file(generated_path)?;

    if committed != generated {
        bail!(
            "badge endpoint drifted: `{}` differs from generated `{}`; run `cargo xtask badges`",
            committed_path.display(),
            generated_path.display()
        );
    }

    Ok(())
}

fn read_file(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).wrap_err_with(|| format!("failed to read `{}`", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ripr_plus_badge_shape_is_stable() -> Result<()> {
        let badge = ShieldsEndpointBadge {
            schema_version: 1,
            label: "ripr+".to_string(),
            message: "0".to_string(),
            color: "brightgreen".to_string(),
        };

        validate_shields_badge(&badge, Some("ripr+"))
    }

    #[test]
    fn badge_rejects_empty_message() {
        let badge = ShieldsEndpointBadge {
            schema_version: 1,
            label: "ripr+".to_string(),
            message: " ".to_string(),
            color: "brightgreen".to_string(),
        };

        assert!(validate_shields_badge(&badge, Some("ripr+")).is_err());
    }
}
