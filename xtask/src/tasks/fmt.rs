//! Format task implementation

use color_eyre::eyre::{Context, Result, eyre};
use duct::cmd;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

const FMT_RECEIPT_PATH: &str = "target/receipts/fmt.json";
const FMT_SCHEMA_VERSION: &str = "1.0.0";

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

pub fn run(check: bool, package_filters: Option<Vec<String>>) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );

    let action = if check { "Checking" } else { "Formatting" };
    spinner.set_message(format!("{} code", action));

    let mut failures = Vec::new();
    let packages = workspace_packages(package_filters.as_deref())?;
    for package in &packages {
        spinner.set_message(format!("{} {}", action, package.manifest_path));

        let check_command = fmt_check_command(&package.manifest_path);
        let fix_command = fmt_fix_command(&package.manifest_path);
        let args = fmt_args(&package.manifest_path, check);
        let status = cmd("cargo", &args)
            .run()
            .with_context(|| format!("Failed to run {}", check_command))?;

        if !status.status.success() {
            failures.push(FmtFailure {
                tool: "cargo fmt".to_string(),
                crate_name: package.name.clone(),
                path: package.manifest_path.clone(),
                check_command,
                fix_command,
            });
        }
    }

    if check {
        let receipt = build_fmt_receipt(&failures);
        write_fmt_receipt(&receipt)?;
        eprintln!("Fmt receipt written to: {FMT_RECEIPT_PATH}");
    }

    if !failures.is_empty() {
        spinner.finish_with_message(format!(
            "❌ Code {} failed",
            if check { "check" } else { "formatting" }
        ));
        return Err(eyre!(
            "Code {} failed for {} package(s)",
            if check { "check" } else { "formatting" },
            failures.len()
        ));
    }

    spinner.finish_with_message(format!(
        "✅ Code {} successfully",
        if check { "check passed" } else { "formatted" }
    ));
    Ok(())
}

fn workspace_packages(package_filters: Option<&[String]>) -> Result<Vec<WorkspacePackage>> {
    let metadata_json = cmd("cargo", ["metadata", "--format-version", "1", "--no-deps"])
        .read()
        .context("Failed to query cargo metadata for workspace formatting")?;
    let metadata: CargoMetadata =
        serde_json::from_str(&metadata_json).context("Failed to parse cargo metadata JSON")?;

    collect_workspace_packages(&metadata, package_filters)
}

fn collect_workspace_packages(
    metadata: &CargoMetadata,
    package_filters: Option<&[String]>,
) -> Result<Vec<WorkspacePackage>> {
    let package_by_id: HashMap<_, _> = metadata
        .packages
        .iter()
        .map(|package| {
            (
                package.id.as_str(),
                WorkspacePackage {
                    name: package.name.clone(),
                    manifest_path: package.manifest_path.clone(),
                },
            )
        })
        .collect();
    let member_name_to_package: HashMap<_, _> = metadata
        .packages
        .iter()
        .filter(|package| metadata.workspace_members.iter().any(|member| member == &package.id))
        .map(|package| {
            (
                package.name.as_str(),
                WorkspacePackage {
                    name: package.name.clone(),
                    manifest_path: package.manifest_path.clone(),
                },
            )
        })
        .collect();

    if let Some(filters) = package_filters {
        let mut selected = Vec::with_capacity(filters.len());
        for package_name in filters {
            if let Some(package) = member_name_to_package.get(package_name.as_str()) {
                selected.push(package.clone());
            } else {
                // Sort the available list so the error message is stable across runs.
                let mut available: Vec<_> = member_name_to_package.keys().copied().collect();
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

fn dedup_preserve_order(packages: Vec<WorkspacePackage>) -> Vec<WorkspacePackage> {
    let mut seen = HashSet::with_capacity(packages.len());
    let mut deduped = Vec::with_capacity(packages.len());
    for package in packages {
        if seen.insert(package.manifest_path.clone()) {
            deduped.push(package);
        }
    }
    deduped
}

fn fmt_args(manifest_path: &str, check: bool) -> Vec<String> {
    let mut args =
        vec!["fmt".to_string(), "--manifest-path".to_string(), manifest_path.to_string()];
    if check {
        args.push("--".to_string());
        args.push("--check".to_string());
    }
    args
}

fn fmt_check_command(manifest_path: &str) -> String {
    format!("cargo fmt --manifest-path {manifest_path} -- --check")
}

fn fmt_fix_command(manifest_path: &str) -> String {
    format!("cargo fmt --manifest-path {manifest_path}")
}

fn write_fmt_receipt(receipt: &FmtReceipt) -> Result<()> {
    let receipt_path = Path::new(FMT_RECEIPT_PATH);
    if let Some(parent) = receipt_path.parent() {
        fs::create_dir_all(parent).context("Failed to create fmt receipt directory")?;
    }
    let json = serde_json::to_string_pretty(receipt).context("Failed to serialize fmt receipt")?;
    fs::write(receipt_path, format!("{json}\n")).context("Failed to write fmt receipt")
}

fn build_fmt_receipt(failures: &[FmtFailure]) -> FmtReceipt {
    let verdict = if failures.is_empty() { "pass" } else { "fail" };
    FmtReceipt {
        check: "fmt".to_string(),
        schema_version: FMT_SCHEMA_VERSION.to_string(),
        verdict: verdict.to_string(),
        classification: "fmt_drift".to_string(),
        failures: failures.to_vec(),
        fix_forward_kind: "FMT_ONLY".to_string(),
        safe_auto_fix: true,
        repro: ReproCommand { command: "cargo xtask fmt --check".to_string() },
    }
}

#[derive(Clone, Debug, Deserialize)]
struct WorkspacePackage {
    name: String,
    manifest_path: String,
}

#[derive(Clone, Debug, Serialize)]
struct FmtFailure {
    tool: String,
    #[serde(rename = "crate")]
    crate_name: String,
    path: String,
    check_command: String,
    fix_command: String,
}

#[derive(Debug, Serialize)]
struct ReproCommand {
    command: String,
}

#[derive(Debug, Serialize)]
struct FmtReceipt {
    check: String,
    schema_version: String,
    verdict: String,
    classification: String,
    failures: Vec<FmtFailure>,
    fix_forward_kind: String,
    safe_auto_fix: bool,
    repro: ReproCommand,
}

#[cfg(test)]
fn temp_receipt_path(dir: &Path) -> PathBuf {
    dir.join("fmt.json")
}

#[cfg(test)]
fn write_fmt_receipt_to_path(receipt: &FmtReceipt, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("Failed to create fmt receipt directory")?;
    }
    let json = serde_json::to_string_pretty(receipt).context("Failed to serialize fmt receipt")?;
    fs::write(path, format!("{json}\n")).context("Failed to write fmt receipt")
}

#[cfg(test)]
mod tests {
    use super::{
        CargoMetadata, CargoPackage, FmtFailure, build_fmt_receipt, collect_workspace_packages,
        temp_receipt_path, write_fmt_receipt_to_path,
    };
    use color_eyre::eyre::Result;
    use serde_json::Value;
    use tempfile::TempDir;

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
        let manifests = collect_workspace_packages(&metadata, Some(&filters))?;
        assert_eq!(manifests[0].manifest_path, "/repo/crates/perl-parser/Cargo.toml".to_string());
        Ok(())
    }

    #[test]
    fn package_filters_are_deduplicated() -> Result<()> {
        let metadata = sample_metadata();
        let filters = vec!["xtask".to_string(), "xtask".to_string()];
        let manifests = collect_workspace_packages(&metadata, Some(&filters))?;
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].manifest_path, "/repo/xtask/Cargo.toml".to_string());
        Ok(())
    }

    #[test]
    fn package_filters_report_unknown_package() -> Result<()> {
        let metadata = sample_metadata();
        let filters = vec!["missing-package".to_string()];
        let message = match collect_workspace_packages(&metadata, Some(&filters)) {
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
        let message = match collect_workspace_packages(&metadata, Some(&filters)) {
            Ok(paths) => {
                return Err(color_eyre::eyre::eyre!("expected error, got paths: {paths:?}"));
            }
            Err(err) => format!("{err}"),
        };
        // The available list must be sorted — both packages appear in alphabetical order.
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
    fn fmt_receipt_captures_multiple_failures() -> Result<()> {
        let receipt = build_fmt_receipt(&[
            FmtFailure {
                tool: "cargo fmt".to_string(),
                crate_name: "xtask".to_string(),
                path: "/repo/xtask/Cargo.toml".to_string(),
                check_command: "cargo fmt --manifest-path /repo/xtask/Cargo.toml -- --check"
                    .to_string(),
                fix_command: "cargo fmt --manifest-path /repo/xtask/Cargo.toml".to_string(),
            },
            FmtFailure {
                tool: "cargo fmt".to_string(),
                crate_name: "perl-parser".to_string(),
                path: "/repo/crates/perl-parser/Cargo.toml".to_string(),
                check_command:
                    "cargo fmt --manifest-path /repo/crates/perl-parser/Cargo.toml -- --check"
                        .to_string(),
                fix_command: "cargo fmt --manifest-path /repo/crates/perl-parser/Cargo.toml"
                    .to_string(),
            },
        ]);
        assert_eq!(receipt.verdict, "fail");
        assert_eq!(receipt.failures.len(), 2);
        assert_eq!(receipt.fix_forward_kind, "FMT_ONLY");
        assert!(receipt.safe_auto_fix);
        Ok(())
    }

    #[test]
    fn fmt_receipt_write_round_trip_contains_required_fields() -> Result<()> {
        let tmp = TempDir::new()?;
        let path = temp_receipt_path(tmp.path());
        let receipt = build_fmt_receipt(&[]);
        write_fmt_receipt_to_path(&receipt, &path)?;
        let raw = std::fs::read_to_string(&path)?;
        let parsed: Value = serde_json::from_str(&raw)?;
        assert_eq!(parsed["check"], "fmt");
        assert_eq!(parsed["schema_version"], "1.0.0");
        assert_eq!(parsed["classification"], "fmt_drift");
        assert_eq!(parsed["fix_forward_kind"], "FMT_ONLY");
        assert_eq!(parsed["safe_auto_fix"], true);
        assert_eq!(parsed["repro"]["command"], "cargo xtask fmt --check");
        Ok(())
    }
}
