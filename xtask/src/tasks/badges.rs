//! Public Shields badge endpoint generation.

use color_eyre::eyre::{Result, WrapErr, eyre};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const BADGE_ENDPOINT_DIR: &str = "badges";
const BADGE_ENDPOINT_TARGET_DIR: &str = "target/xtask/badges";
const RIPR_PLUS_ENDPOINT: &str = "ripr-plus.json";

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ShieldsEndpointBadge {
    #[serde(rename = "schemaVersion")]
    schema_version: u8,
    label: String,
    message: String,
    color: String,
}

pub(crate) fn run(check: bool) -> Result<()> {
    let workspace_root = workspace_root_path()?;
    let target_dir = workspace_root.join(BADGE_ENDPOINT_TARGET_DIR);
    fs::create_dir_all(&target_dir)
        .wrap_err_with(|| format!("creating badge target dir {}", target_dir.display()))?;

    let ripr_plus = ripr_plus_badge(&workspace_root)?;
    validate_shields_badge(&ripr_plus, Some("ripr+"))?;
    write_json_pretty(&target_dir.join(RIPR_PLUS_ENDPOINT), &ripr_plus)?;

    if check {
        let committed_dir = workspace_root.join(BADGE_ENDPOINT_DIR);
        compare_files(
            &committed_dir.join(RIPR_PLUS_ENDPOINT),
            &target_dir.join(RIPR_PLUS_ENDPOINT),
        )?;
        println!("badges: committed endpoints are current");
        return Ok(());
    }

    let committed_dir = workspace_root.join(BADGE_ENDPOINT_DIR);
    fs::create_dir_all(&committed_dir)
        .wrap_err_with(|| format!("creating badge endpoint dir {}", committed_dir.display()))?;
    fs::copy(target_dir.join(RIPR_PLUS_ENDPOINT), committed_dir.join(RIPR_PLUS_ENDPOINT))
        .wrap_err("copying ripr+ badge endpoint into badges/")?;

    println!("badges: refreshed public endpoint JSON under badges/");
    Ok(())
}

fn workspace_root_path() -> Result<PathBuf> {
    let xtask_manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = xtask_manifest_dir.parent().ok_or_else(|| {
        eyre!("xtask manifest dir has no parent: {}", xtask_manifest_dir.display())
    })?;
    Ok(workspace_root.to_path_buf())
}

fn ripr_plus_badge(workspace_root: &Path) -> Result<ShieldsEndpointBadge> {
    let ripr_bin = std::env::var("RIPR_BIN").unwrap_or_else(|_| "ripr".to_string());

    // Public README badge: repo-scoped, not PR/diff scoped.
    let output = Command::new(&ripr_bin)
        .arg("check")
        .arg("--root")
        .arg(workspace_root)
        .arg("--format")
        .arg("repo-badge-plus-shields")
        .current_dir(workspace_root)
        .output()
        .wrap_err_with(|| format!("running {ripr_bin} for repo-scoped badge evidence"))?;

    if !output.status.success() {
        return Err(eyre!(
            "{ripr_bin} repo-badge-plus-shields failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    serde_json::from_slice(&output.stdout)
        .wrap_err_with(|| format!("{ripr_bin} emitted invalid Shields endpoint JSON"))
}

pub(crate) fn validate_shields_badge(
    badge: &ShieldsEndpointBadge,
    expected_label: Option<&str>,
) -> Result<()> {
    if badge.schema_version != 1 {
        return Err(eyre!("badge `{}` has unsupported schemaVersion", badge.label));
    }

    if let Some(expected_label) = expected_label {
        if badge.label != expected_label {
            return Err(eyre!(
                "badge label drifted: got `{}`, expected `{expected_label}`",
                badge.label
            ));
        }
    }

    if badge.message.trim().is_empty() {
        return Err(eyre!("badge `{}` has empty message", badge.label));
    }

    if badge.color.trim().is_empty() {
        return Err(eyre!("badge `{}` has empty color", badge.label));
    }

    Ok(())
}

fn write_json_pretty(path: &Path, badge: &ShieldsEndpointBadge) -> Result<()> {
    let payload =
        serde_json::to_string_pretty(badge).wrap_err("serializing badge endpoint JSON")?;
    fs::write(path, format!("{payload}\n"))
        .wrap_err_with(|| format!("writing badge endpoint {}", path.display()))
}

fn compare_files(committed: &Path, generated: &Path) -> Result<()> {
    let committed_bytes = fs::read(committed)
        .wrap_err_with(|| format!("reading committed badge endpoint {}", committed.display()))?;
    let generated_bytes = fs::read(generated)
        .wrap_err_with(|| format!("reading generated badge endpoint {}", generated.display()))?;

    if committed_bytes != generated_bytes {
        return Err(eyre!(
            "badge endpoint drift: {} differs from {}; run `cargo xtask badges`",
            committed.display(),
            generated.display()
        ));
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
    fn badge_shape_rejects_empty_message() -> Result<()> {
        let badge = ShieldsEndpointBadge {
            schema_version: 1,
            label: "ripr+".to_string(),
            message: " ".to_string(),
            color: "brightgreen".to_string(),
        };

        if validate_shields_badge(&badge, Some("ripr+")).is_ok() {
            return Err(eyre!("empty badge messages must be rejected"));
        }

        Ok(())
    }
}
