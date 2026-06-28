//! Generated public badge endpoint tasks.
//!
//! Public README badges are repository-scoped Shields endpoint JSON. Detailed
//! evidence stays in `target/` or CI artifacts.

use std::fs;
use std::path::Path;
use std::process::Command;

use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::utils::project_root;

const BADGE_ENDPOINT_DIR: &str = "badges";
const BADGE_ENDPOINT_TARGET_DIR: &str = "target/xtask/badges";

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
struct ShieldsEndpointBadge {
    #[serde(rename = "schemaVersion")]
    schema_version: u8,
    label: String,
    message: String,
    color: String,
}

pub fn run(check: bool) -> Result<()> {
    let workspace_root = project_root()?;
    badges_at_root(&workspace_root, check)
}

fn badges_at_root(workspace_root: &Path, check: bool) -> Result<()> {
    let target_dir = workspace_root.join(BADGE_ENDPOINT_TARGET_DIR);
    fs::create_dir_all(&target_dir)
        .with_context(|| format!("creating {}", target_dir.display()))?;

    let ripr_plus = ripr_plus_badge(workspace_root)?;
    validate_shields_badge(&ripr_plus, Some("ripr+"))?;
    let target_ripr = target_dir.join("ripr-plus.json");
    write_json_pretty(&target_ripr, &ripr_plus)?;

    let committed_dir = workspace_root.join(BADGE_ENDPOINT_DIR);
    let committed_ripr = committed_dir.join("ripr-plus.json");

    if check {
        compare_files(&committed_ripr, &target_ripr)?;
        println!("badges: committed endpoints are current");
        return Ok(());
    }

    fs::create_dir_all(&committed_dir)
        .with_context(|| format!("creating {}", committed_dir.display()))?;
    fs::copy(&target_ripr, &committed_ripr).with_context(|| {
        format!(
            "copying generated badge endpoint from {} to {}",
            target_ripr.display(),
            committed_ripr.display()
        )
    })?;

    println!("badges: refreshed public endpoint JSON under badges/");
    Ok(())
}

fn ripr_plus_badge(workspace_root: &Path) -> Result<ShieldsEndpointBadge> {
    let ripr_bin = std::env::var("RIPR_BIN").unwrap_or_else(|_| "ripr".to_string());

    let output = Command::new(&ripr_bin)
        .arg("check")
        .arg("--root")
        .arg(workspace_root)
        .arg("--format")
        .arg("repo-badge-json")
        .current_dir(workspace_root)
        .output()
        .with_context(|| format!("running {ripr_bin} for repo-scoped ripr+ badge"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(color_eyre::eyre::eyre!("ripr check failed for ripr+ badge: {stderr}"));
    }

    let badge_json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .with_context(|| format!("{ripr_bin} emitted invalid repo-badge-json"))?;

    let counts = badge_json.get("counts");
    let count = |key: &str| counts.and_then(|v| v.get(key)).and_then(|v| v.as_u64()).unwrap_or(0);
    let unresolved =
        count("unsuppressed_exposure_gaps") + count("unsuppressed_test_efficiency_findings");

    Ok(ShieldsEndpointBadge {
        schema_version: 1,
        label: "ripr+".to_string(),
        message: unresolved.to_string(),
        color: if unresolved == 0 { "brightgreen".to_string() } else { "yellow".to_string() },
    })
}

fn validate_shields_badge(
    badge: &ShieldsEndpointBadge,
    expected_label: Option<&str>,
) -> Result<()> {
    if badge.schema_version != 1 {
        bail!("badge `{}` has unsupported schemaVersion", badge.label);
    }

    if let Some(expected_label) = expected_label {
        if badge.label != expected_label {
            bail!("badge label drifted: got `{}`, expected `{expected_label}`", badge.label);
        }
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
    let json = serde_json::to_string_pretty(badge)?;
    fs::write(path, format!("{json}\n")).with_context(|| format!("writing {}", path.display()))
}

fn compare_files(committed_path: &Path, generated_path: &Path) -> Result<()> {
    let committed = fs::read(committed_path).with_context(|| {
        format!("reading committed badge endpoint {}", committed_path.display())
    })?;
    let generated = fs::read(generated_path).with_context(|| {
        format!("reading generated badge endpoint {}", generated_path.display())
    })?;

    if committed != generated {
        bail!(
            "badge endpoint drift detected for {}; run `cargo xtask badges`",
            committed_path.display()
        );
    }

    Ok(())
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
    fn empty_message_is_rejected() -> Result<()> {
        let badge = ShieldsEndpointBadge {
            schema_version: 1,
            label: "ripr+".to_string(),
            message: "".to_string(),
            color: "brightgreen".to_string(),
        };

        assert!(validate_shields_badge(&badge, Some("ripr+")).is_err());
        Ok(())
    }

    #[test]
    fn label_drift_is_rejected() -> Result<()> {
        let badge = ShieldsEndpointBadge {
            schema_version: 1,
            label: "coverage".to_string(),
            message: "0".to_string(),
            color: "brightgreen".to_string(),
        };

        assert!(validate_shields_badge(&badge, Some("ripr+")).is_err());
        Ok(())
    }

    #[test]
    fn ripr_plus_badge_message_is_sum_of_gap_counts() -> Result<()> {
        // Simulate repo-badge-json output from ripr
        let badge_json = serde_json::json!({
            "counts": {
                "unsuppressed_exposure_gaps": 3,
                "unsuppressed_test_efficiency_findings": 2,
                "suppressed_exposure_gaps": 1,
                "suppressed_test_efficiency_findings": 0
            }
        });
        let counts = badge_json.get("counts");
        let count =
            |key: &str| counts.and_then(|v| v.get(key)).and_then(|v| v.as_u64()).unwrap_or(0);
        let unresolved =
            count("unsuppressed_exposure_gaps") + count("unsuppressed_test_efficiency_findings");
        assert_eq!(unresolved, 5);

        let badge = ShieldsEndpointBadge {
            schema_version: 1,
            label: "ripr+".to_string(),
            message: unresolved.to_string(),
            color: if unresolved == 0 { "brightgreen".to_string() } else { "yellow".to_string() },
        };
        assert_eq!(badge.message, "5");
        assert_eq!(badge.color, "yellow");
        validate_shields_badge(&badge, Some("ripr+"))
    }

    #[test]
    fn ripr_plus_badge_zero_unresolved_is_brightgreen() -> Result<()> {
        let badge = ShieldsEndpointBadge {
            schema_version: 1,
            label: "ripr+".to_string(),
            message: "0".to_string(),
            color: "brightgreen".to_string(),
        };
        validate_shields_badge(&badge, Some("ripr+"))
    }

    #[test]
    fn ripr_plus_badge_missing_counts_defaults_to_zero() -> Result<()> {
        // repo-badge-json with no counts field: should produce message "0", not panic
        let badge_json = serde_json::json!({});
        let counts = badge_json.get("counts");
        let count =
            |key: &str| counts.and_then(|v| v.get(key)).and_then(|v| v.as_u64()).unwrap_or(0);
        let unresolved =
            count("unsuppressed_exposure_gaps") + count("unsuppressed_test_efficiency_findings");
        assert_eq!(unresolved, 0);
        Ok(())
    }
}
