//! Format task implementation

use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, eyre};
use duct::cmd;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

const RECEIPT_SCHEMA_VERSION: &str = "1.0.0";
const RECEIPT_PATH: &str = "target/receipts/fmt.json";

#[derive(Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoPackage>,
    workspace_members: Vec<String>,
}

#[derive(Deserialize)]
struct CargoPackage {
    id: String,
    name: String,
    manifest_path: String,
}

#[derive(Clone, Debug)]
struct FmtTarget {
    crate_name: String,
    manifest_path: String,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
struct FmtReceipt {
    check: String,
    schema_version: String,
    verdict: String,
    classification: String,
    failures: Vec<FmtFailure>,
    fix_forward_kind: String,
    safe_auto_fix: bool,
    repro: FmtRepro,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
struct FmtFailure {
    tool: String,
    #[serde(rename = "crate")]
    crate_field: String,
    path: String,
    check_command: String,
    fix_command: String,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
struct FmtRepro {
    command: String,
}

pub fn run(check: bool, package_filters: Option<Vec<String>>) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );

    let action = if check { "Checking" } else { "Formatting" };
    spinner.set_message(format!("{} code", action));

    let targets = workspace_fmt_targets(package_filters.as_deref())?;
    let mut failures = Vec::new();

    for target in &targets {
        spinner.set_message(format!("{} {}", action, target.manifest_path));

        let mut args =
            vec!["fmt".to_string(), "--manifest-path".to_string(), target.manifest_path.clone()];
        if check {
            args.push("--".to_string());
            args.push("--check".to_string());
        }

        let result = cmd("cargo", &args)
            .unchecked()
            .run()
            .with_context(|| format!("Failed to run {args:?}"))?;

        if !result.status.success() {
            if check {
                let check_command =
                    format!("cargo fmt --manifest-path {} -- --check", target.manifest_path);
                let fix_command = format!("cargo fmt --manifest-path {}", target.manifest_path);
                let failed_path = infer_failed_path(&result.stderr, &target.manifest_path)?;
                failures.push(FmtFailure {
                    tool: "rustfmt".to_string(),
                    crate_field: target.crate_name.clone(),
                    path: failed_path,
                    check_command,
                    fix_command,
                });
                continue;
            }
            spinner.finish_with_message(format!(
                "❌ Code {} failed",
                if check { "check" } else { "formatting" }
            ));
            return Err(eyre!(
                "Code {} failed for {}",
                if check { "check" } else { "formatting" },
                target.manifest_path
            ));
        }
    }

    if check {
        let receipt = build_receipt(&failures, package_filters.as_deref());
        write_receipt(&receipt)?;
        if !failures.is_empty() {
            spinner.finish_with_message("❌ Code check failed".to_string());
            return Err(eyre!(
                "Formatting check failed in {} crate(s); see {}",
                failures.len(),
                RECEIPT_PATH
            ));
        }
    }

    spinner.finish_with_message(format!(
        "✅ Code {} successfully",
        if check { "check passed" } else { "formatted" }
    ));
    Ok(())
}

fn workspace_fmt_targets(package_filters: Option<&[String]>) -> Result<Vec<FmtTarget>> {
    let metadata_json = cmd("cargo", ["metadata", "--format-version", "1", "--no-deps"])
        .read()
        .context("Failed to query cargo metadata for workspace formatting")?;
    let metadata: CargoMetadata =
        serde_json::from_str(&metadata_json).context("Failed to parse cargo metadata JSON")?;

    collect_workspace_fmt_targets(&metadata, package_filters)
}

fn collect_workspace_fmt_targets(
    metadata: &CargoMetadata,
    package_filters: Option<&[String]>,
) -> Result<Vec<FmtTarget>> {
    let package_by_id: HashMap<_, _> = metadata
        .packages
        .iter()
        .map(|package| {
            (
                package.id.as_str(),
                FmtTarget {
                    crate_name: package.name.clone(),
                    manifest_path: package.manifest_path.clone(),
                },
            )
        })
        .collect();
    let member_name_to_target: HashMap<_, _> = metadata
        .packages
        .iter()
        .filter(|package| metadata.workspace_members.iter().any(|member| member == &package.id))
        .map(|package| {
            (
                package.name.as_str(),
                FmtTarget {
                    crate_name: package.name.clone(),
                    manifest_path: package.manifest_path.clone(),
                },
            )
        })
        .collect();

    if let Some(filters) = package_filters {
        let mut selected = Vec::with_capacity(filters.len());
        for package_name in filters {
            if let Some(target) = member_name_to_target.get(package_name.as_str()) {
                selected.push(target.clone());
            } else {
                // Sort the available list so the error message is stable across runs.
                let mut available: Vec<_> = member_name_to_target.keys().copied().collect();
                available.sort_unstable();
                return Err(eyre!(
                    "Unknown package `{package_name}`. Available workspace packages: {}",
                    available.join(", ")
                ));
            }
        }
        return Ok(dedup_preserve_order(selected));
    }

    metadata
        .workspace_members
        .iter()
        .map(|member_id| {
            package_by_id
                .get(member_id.as_str())
                .cloned()
                .ok_or_else(|| eyre!("Workspace member not found in cargo metadata: {member_id}"))
        })
        .collect()
}

fn dedup_preserve_order(targets: Vec<FmtTarget>) -> Vec<FmtTarget> {
    let mut seen = HashSet::with_capacity(targets.len());
    let mut deduped = Vec::with_capacity(targets.len());
    for target in targets {
        if seen.insert(target.manifest_path.clone()) {
            deduped.push(target);
        }
    }
    deduped
}

fn infer_failed_path(stderr: &[u8], manifest_path: &str) -> Result<String> {
    let stderr_text = String::from_utf8_lossy(stderr);
    let manifest_parent = Path::new(manifest_path)
        .parent()
        .ok_or_else(|| eyre!("Manifest path has no parent: {manifest_path}"))?;

    for line in stderr_text.lines() {
        if let Some(path) = line.strip_prefix("Diff in ") {
            return to_relative_workspace_path(path.trim());
        }
    }

    Ok(manifest_parent.to_string_lossy().to_string())
}

fn to_relative_workspace_path(path: &str) -> Result<String> {
    let root = project_root()?;
    let candidate = Path::new(path);
    if let Ok(relative) = candidate.strip_prefix(&root) {
        return Ok(relative.to_string_lossy().to_string());
    }
    Ok(path.to_string())
}

fn build_receipt(failures: &[FmtFailure], package_filters: Option<&[String]>) -> FmtReceipt {
    let base_command = "cargo xtask fmt --check";
    let command = match package_filters {
        Some(filters) if !filters.is_empty() => {
            let joined = filters.join(",");
            format!("{base_command} --package {joined}")
        }
        _ => base_command.to_string(),
    };

    FmtReceipt {
        check: "fmt".to_string(),
        schema_version: RECEIPT_SCHEMA_VERSION.to_string(),
        verdict: if failures.is_empty() { "pass" } else { "fail" }.to_string(),
        classification: "fmt_drift".to_string(),
        failures: failures.to_vec(),
        fix_forward_kind: "FMT_ONLY".to_string(),
        safe_auto_fix: true,
        repro: FmtRepro { command },
    }
}

fn write_receipt(receipt: &FmtReceipt) -> Result<()> {
    let root = project_root()?;
    let path = root.join(RECEIPT_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create receipt directory {}", parent.display()))?;
    }
    let content = serde_json::to_string_pretty(receipt)?;
    fs::write(&path, content).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        CargoMetadata, CargoPackage, FmtFailure, build_receipt, collect_workspace_fmt_targets,
    };
    use color_eyre::eyre::Result;

    fn sample_metadata() -> CargoMetadata {
        CargoMetadata {
            packages: vec![
                CargoPackage {
                    id: "path+file:///repo/xtask#0.1.0".to_string(),
                    name: "xtask".to_string(),
                    manifest_path: "/repo/xtask/Cargo.toml".to_string(),
                },
                CargoPackage {
                    id: "path+file:///repo/crates/perl-parser#0.1.0".to_string(),
                    name: "perl-parser".to_string(),
                    manifest_path: "/repo/crates/perl-parser/Cargo.toml".to_string(),
                },
            ],
            workspace_members: vec![
                "path+file:///repo/xtask#0.1.0".to_string(),
                "path+file:///repo/crates/perl-parser#0.1.0".to_string(),
            ],
        }
    }

    #[test]
    fn package_filters_select_requested_manifest_paths() -> Result<()> {
        let metadata = sample_metadata();
        let filters = vec!["perl-parser".to_string()];
        let manifests = collect_workspace_fmt_targets(&metadata, Some(&filters))?;
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].manifest_path, "/repo/crates/perl-parser/Cargo.toml");
        Ok(())
    }

    #[test]
    fn package_filters_are_deduplicated() -> Result<()> {
        let metadata = sample_metadata();
        let filters = vec!["xtask".to_string(), "xtask".to_string()];
        let manifests = collect_workspace_fmt_targets(&metadata, Some(&filters))?;
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].manifest_path, "/repo/xtask/Cargo.toml");
        Ok(())
    }

    #[test]
    fn package_filters_report_unknown_package() -> Result<()> {
        let metadata = sample_metadata();
        let filters = vec!["missing-package".to_string()];
        let message = match collect_workspace_fmt_targets(&metadata, Some(&filters)) {
            Ok(paths) => {
                return Err(color_eyre::eyre::eyre!("expected error, got paths: {paths:?}"));
            }
            Err(err) => format!("{err}"),
        };
        assert!(message.contains("missing-package"));
        assert!(message.contains("Available workspace packages"));
        Ok(())
    }

    #[test]
    fn package_filters_error_lists_packages_in_stable_sorted_order() -> Result<()> {
        let metadata = sample_metadata();
        let filters = vec!["nonexistent".to_string()];
        let message = match collect_workspace_fmt_targets(&metadata, Some(&filters)) {
            Ok(paths) => {
                return Err(color_eyre::eyre::eyre!("expected error, got paths: {paths:?}"));
            }
            Err(err) => format!("{err}"),
        };
        // The available list must be sorted — both packages appear in alphabetical order.
        let perl_pos = message.find("perl-parser").expect("perl-parser in error");
        let xtask_pos = message.find("xtask").expect("xtask in error");
        assert!(perl_pos < xtask_pos, "available packages must be listed in sorted order");
        Ok(())
    }

    #[test]
    fn receipt_records_multiple_failures() -> Result<()> {
        let failures = vec![
            FmtFailure {
                tool: "rustfmt".to_string(),
                crate_field: "xtask".to_string(),
                path: "xtask/src/tasks/fmt.rs".to_string(),
                check_command: "cargo fmt --manifest-path xtask/Cargo.toml -- --check".to_string(),
                fix_command: "cargo fmt --manifest-path xtask/Cargo.toml".to_string(),
            },
            FmtFailure {
                tool: "rustfmt".to_string(),
                crate_field: "perl-parser".to_string(),
                path: "crates/perl-parser/src/lib.rs".to_string(),
                check_command: "cargo fmt --manifest-path crates/perl-parser/Cargo.toml -- --check"
                    .to_string(),
                fix_command: "cargo fmt --manifest-path crates/perl-parser/Cargo.toml".to_string(),
            },
        ];

        let receipt = build_receipt(&failures, None);
        assert_eq!(receipt.check, "fmt");
        assert_eq!(receipt.classification, "fmt_drift");
        assert_eq!(receipt.fix_forward_kind, "FMT_ONLY");
        assert_eq!(receipt.failures.len(), 2);
        Ok(())
    }
}
