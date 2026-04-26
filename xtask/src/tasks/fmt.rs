//! Format task implementation

use color_eyre::eyre::{Context, Result, eyre};
use duct::cmd;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const FMT_SCHEMA_VERSION: &str = "1.0.0";
const FMT_RECEIPT_RELATIVE_PATH: &str = "target/receipts/fmt.json";
const FMT_CHECK_REPRO_COMMAND: &str = "cargo xtask fmt --check";
const FMT_CHECK_TOOL: &str = "rustfmt";

#[derive(Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoPackage>,
    workspace_members: Vec<String>,
}

#[derive(Clone, Deserialize)]
struct CargoPackage {
    id: String,
    name: String,
    manifest_path: String,
}

#[derive(Clone, Debug)]
struct FormatTarget {
    crate_name: String,
    manifest_path: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
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

#[derive(Debug, Serialize, PartialEq, Eq)]
struct FmtFailure {
    tool: String,
    #[serde(rename = "crate")]
    crate_name: String,
    path: String,
    check_command: String,
    fix_command: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
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

    let targets = workspace_format_targets(package_filters.as_deref())?;

    if check {
        let failures = run_check_targets(&spinner, &targets)?;
        let receipt = build_receipt(failures);
        write_receipt(&receipt)?;

        if receipt.verdict == "fail" {
            spinner.finish_with_message("❌ Code check failed");
            return Err(eyre!(
                "Code check failed for {} crate(s). See {} for full failure list.",
                receipt.failures.len(),
                FMT_RECEIPT_RELATIVE_PATH
            ));
        }

        spinner.finish_with_message("✅ Code check passed");
        return Ok(());
    }

    run_format_targets(&spinner, &targets)?;
    spinner.finish_with_message("✅ Code formatted successfully");
    Ok(())
}

fn run_check_targets(spinner: &ProgressBar, targets: &[FormatTarget]) -> Result<Vec<FmtFailure>> {
    let mut failures = Vec::new();

    for target in targets {
        spinner.set_message(format!("Checking {}", target.manifest_path));
        let check_command =
            format!("cargo fmt --manifest-path {} -- --check", target.manifest_path);
        let fix_command = format!("cargo fmt --manifest-path {}", target.manifest_path);

        let status = Command::new("cargo")
            .arg("fmt")
            .arg("--manifest-path")
            .arg(&target.manifest_path)
            .arg("--")
            .arg("--check")
            .status()
            .with_context(|| {
                format!("Failed to execute formatting check for {}", target.crate_name)
            })?;

        if !status.success() {
            failures.push(FmtFailure {
                tool: FMT_CHECK_TOOL.to_string(),
                crate_name: target.crate_name.clone(),
                path: target.manifest_path.clone(),
                check_command,
                fix_command,
            });
        }
    }

    Ok(failures)
}

fn run_format_targets(spinner: &ProgressBar, targets: &[FormatTarget]) -> Result<()> {
    for target in targets {
        spinner.set_message(format!("Formatting {}", target.manifest_path));

        let args = ["fmt", "--manifest-path", target.manifest_path.as_str()];
        let status = cmd("cargo", args)
            .run()
            .with_context(|| format!("Failed to format {}", target.manifest_path))?;

        if !status.status.success() {
            return Err(eyre!("Code formatting failed for {}", target.manifest_path));
        }
    }

    Ok(())
}

fn build_receipt(failures: Vec<FmtFailure>) -> FmtReceipt {
    let verdict = if failures.is_empty() { "pass" } else { "fail" };
    FmtReceipt {
        check: "fmt".to_string(),
        schema_version: FMT_SCHEMA_VERSION.to_string(),
        verdict: verdict.to_string(),
        classification: "fmt_drift".to_string(),
        failures,
        fix_forward_kind: "FMT_ONLY".to_string(),
        safe_auto_fix: true,
        repro: FmtRepro { command: FMT_CHECK_REPRO_COMMAND.to_string() },
    }
}

fn write_receipt(receipt: &FmtReceipt) -> Result<()> {
    let receipt_path = PathBuf::from(FMT_RECEIPT_RELATIVE_PATH);
    if let Some(parent) = receipt_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create receipt directory {}", parent.display()))?;
    }

    let payload =
        serde_json::to_string_pretty(receipt).context("Failed to serialize fmt receipt JSON")?;
    fs::write(&receipt_path, payload)
        .with_context(|| format!("Failed to write receipt {}", receipt_path.display()))?;
    println!("fmt receipt: {}", receipt_path.display());
    Ok(())
}

fn workspace_format_targets(package_filters: Option<&[String]>) -> Result<Vec<FormatTarget>> {
    let metadata_json = cmd("cargo", ["metadata", "--format-version", "1", "--no-deps"])
        .read()
        .context("Failed to query cargo metadata for workspace formatting")?;
    let metadata: CargoMetadata =
        serde_json::from_str(&metadata_json).context("Failed to parse cargo metadata JSON")?;

    collect_workspace_format_targets(&metadata, package_filters)
}

fn collect_workspace_format_targets(
    metadata: &CargoMetadata,
    package_filters: Option<&[String]>,
) -> Result<Vec<FormatTarget>> {
    let package_by_id: HashMap<_, _> =
        metadata.packages.iter().map(|package| (package.id.as_str(), package)).collect();
    let member_name_to_package: HashMap<_, _> = metadata
        .packages
        .iter()
        .filter(|package| metadata.workspace_members.iter().any(|member| member == &package.id))
        .map(|package| (package.name.as_str(), package))
        .collect();

    if let Some(filters) = package_filters {
        let mut selected = Vec::with_capacity(filters.len());
        for package_name in filters {
            if let Some(package) = member_name_to_package.get(package_name.as_str()) {
                selected.push(FormatTarget {
                    crate_name: package.name.clone(),
                    manifest_path: package.manifest_path.clone(),
                });
            } else {
                let mut available: Vec<_> = member_name_to_package.keys().copied().collect();
                available.sort_unstable();
                return Err(eyre!(
                    "Unknown package `{package_name}`. Available workspace packages: {}",
                    available.join(", ")
                ));
            }
        }
        return Ok(dedup_targets_preserve_order(selected));
    }

    metadata
        .workspace_members
        .iter()
        .map(|member_id| {
            let package = package_by_id.get(member_id.as_str()).ok_or_else(|| {
                eyre!("Workspace member not found in cargo metadata: {member_id}")
            })?;
            Ok(FormatTarget {
                crate_name: package.name.clone(),
                manifest_path: package.manifest_path.clone(),
            })
        })
        .collect()
}

fn dedup_targets_preserve_order(targets: Vec<FormatTarget>) -> Vec<FormatTarget> {
    let mut seen = HashSet::with_capacity(targets.len());
    let mut deduped = Vec::with_capacity(targets.len());
    for target in targets {
        if seen.insert(target.manifest_path.clone()) {
            deduped.push(target);
        }
    }
    deduped
}

#[cfg(test)]
mod tests {
    use super::{
        CargoMetadata, CargoPackage, FmtFailure, build_receipt, collect_workspace_format_targets,
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
        let manifests = collect_workspace_format_targets(&metadata, Some(&filters))?;
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].crate_name, "perl-parser");
        assert_eq!(manifests[0].manifest_path, "/repo/crates/perl-parser/Cargo.toml");
        Ok(())
    }

    #[test]
    fn package_filters_are_deduplicated() -> Result<()> {
        let metadata = sample_metadata();
        let filters = vec!["xtask".to_string(), "xtask".to_string()];
        let manifests = collect_workspace_format_targets(&metadata, Some(&filters))?;
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].manifest_path, "/repo/xtask/Cargo.toml");
        Ok(())
    }

    #[test]
    fn package_filters_report_unknown_package() -> Result<()> {
        let metadata = sample_metadata();
        let filters = vec!["missing-package".to_string()];
        let message = match collect_workspace_format_targets(&metadata, Some(&filters)) {
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
        let message = match collect_workspace_format_targets(&metadata, Some(&filters)) {
            Ok(paths) => {
                return Err(color_eyre::eyre::eyre!("expected error, got paths: {paths:?}"));
            }
            Err(err) => format!("{err}"),
        };
        let perl_pos = message
            .find("perl-parser")
            .ok_or_else(|| color_eyre::eyre::eyre!("perl-parser missing from error"))?;
        let xtask_pos = message
            .find("xtask")
            .ok_or_else(|| color_eyre::eyre::eyre!("xtask missing from error"))?;
        assert!(perl_pos < xtask_pos, "available packages must be listed in sorted order");
        Ok(())
    }

    #[test]
    fn receipt_collects_multiple_failures() {
        let receipt = build_receipt(vec![
            FmtFailure {
                tool: "rustfmt".to_string(),
                crate_name: "xtask".to_string(),
                path: "xtask/Cargo.toml".to_string(),
                check_command: "cargo fmt --manifest-path xtask/Cargo.toml -- --check".to_string(),
                fix_command: "cargo fmt --manifest-path xtask/Cargo.toml".to_string(),
            },
            FmtFailure {
                tool: "rustfmt".to_string(),
                crate_name: "perl-parser".to_string(),
                path: "crates/perl-parser/Cargo.toml".to_string(),
                check_command: "cargo fmt --manifest-path crates/perl-parser/Cargo.toml -- --check"
                    .to_string(),
                fix_command: "cargo fmt --manifest-path crates/perl-parser/Cargo.toml".to_string(),
            },
        ]);

        assert_eq!(receipt.check, "fmt");
        assert_eq!(receipt.verdict, "fail");
        assert_eq!(receipt.classification, "fmt_drift");
        assert_eq!(receipt.fix_forward_kind, "FMT_ONLY");
        assert!(receipt.safe_auto_fix);
        assert_eq!(receipt.failures.len(), 2);
    }
}
