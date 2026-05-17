//! Generated public README badge endpoints.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use color_eyre::eyre::{Result, WrapErr, bail};
use serde::{Deserialize, Serialize};

use crate::utils::project_root;

const BADGE_ENDPOINT_DIR: &str = "badges";
const BADGE_ENDPOINT_TARGET_DIR: &str = "target/xtask/badges";
const RIPR_PLUS_FILE: &str = "ripr-plus.json";

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct ShieldsEndpointBadge {
    #[serde(rename = "schemaVersion")]
    schema_version: u8,
    label: String,
    message: String,
    color: String,
}

pub(crate) fn run(check: bool) -> Result<()> {
    let workspace_root = project_root()?;
    let target_dir = workspace_root.join(BADGE_ENDPOINT_TARGET_DIR);
    fs::create_dir_all(&target_dir).wrap_err("creating badge target directory")?;

    ensure_test_efficiency_report(&workspace_root)?;
    let ripr_plus = ripr_plus_badge(&workspace_root)?;
    validate_shields_badge(&ripr_plus, Some("ripr+"))?;
    write_json_pretty(&target_dir.join(RIPR_PLUS_FILE), &ripr_plus)?;

    let committed_dir = workspace_root.join(BADGE_ENDPOINT_DIR);
    if check {
        compare_files(&committed_dir.join(RIPR_PLUS_FILE), &target_dir.join(RIPR_PLUS_FILE))?;
        println!("badges: committed endpoints are current");
        return Ok(());
    }

    fs::create_dir_all(&committed_dir).wrap_err("creating committed badge endpoint directory")?;
    fs::copy(target_dir.join(RIPR_PLUS_FILE), committed_dir.join(RIPR_PLUS_FILE))
        .wrap_err("copying ripr+ badge endpoint into badges/")?;

    println!("badges: refreshed public endpoint JSON under badges/");
    Ok(())
}

fn ensure_test_efficiency_report(workspace_root: &Path) -> Result<()> {
    let report_path = workspace_root.join("target/ripr/reports/test-efficiency.json");
    if report_path.exists() {
        return Ok(());
    }

    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent).wrap_err_with(|| format!("creating {}", parent.display()))?;
    }

    let report = serde_json::json!({
        "schema_version": "0.1",
        "tests": [],
        "metrics": {
            "tests_scanned": 0,
            "reason_counts": {}
        }
    });
    let rendered = serde_json::to_string_pretty(&report)
        .wrap_err("serializing empty test-efficiency report")?;
    fs::write(&report_path, format!("{rendered}\n"))
        .wrap_err_with(|| format!("writing {}", report_path.display()))
}

fn ripr_plus_badge(workspace_root: &Path) -> Result<ShieldsEndpointBadge> {
    let ripr_bin = std::env::var("RIPR_BIN").unwrap_or_else(|_| "ripr".to_string());

    let output = Command::new(&ripr_bin)
        .arg("check")
        .arg("--root")
        .arg(workspace_root)
        .arg("--format")
        .arg("repo-badge-plus-shields")
        .current_dir(workspace_root)
        .output()
        .wrap_err_with(|| format!("running {ripr_bin} for repo-scoped ripr+ badge"))?;

    if !output.status.success() {
        bail!(
            "{ripr_bin} repo-badge-plus-shields failed: {}",
            String::from_utf8_lossy(&output.stderr).trim_end()
        );
    }

    serde_json::from_slice(&output.stdout)
        .wrap_err_with(|| format!("{ripr_bin} emitted invalid Shields endpoint JSON"))
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
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).wrap_err_with(|| format!("creating {}", parent.display()))?;
    }
    let rendered = serde_json::to_string_pretty(badge).wrap_err("serializing badge endpoint")?;
    fs::write(path, format!("{rendered}\n")).wrap_err_with(|| format!("writing {}", path.display()))
}

fn compare_files(committed: &Path, generated: &Path) -> Result<()> {
    let committed_text = fs::read_to_string(committed)
        .wrap_err_with(|| format!("reading committed badge endpoint {}", committed.display()))?;
    let generated_text = fs::read_to_string(generated)
        .wrap_err_with(|| format!("reading generated badge endpoint {}", generated.display()))?;

    if committed_text != generated_text {
        bail!(
            "badge endpoint drift: {} differs from generated {}. Run `cargo xtask badges`.",
            path_for_message(committed),
            path_for_message(generated)
        );
    }

    Ok(())
}

fn path_for_message(path: &Path) -> String {
    match path.strip_prefix(project_root().unwrap_or_else(|_| PathBuf::new())) {
        Ok(relative) => relative.display().to_string(),
        Err(_) => path.display().to_string(),
    }
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
    fn badge_shape_requires_expected_label() -> Result<()> {
        let badge = ShieldsEndpointBadge {
            schema_version: 1,
            label: "ripr+".to_string(),
            message: "0".to_string(),
            color: "brightgreen".to_string(),
        };

        let result = validate_shields_badge(&badge, Some("fixtures"));
        assert!(result.is_err());
        Ok(())
    }
}
