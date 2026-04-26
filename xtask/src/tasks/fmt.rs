//! Format task implementation

use color_eyre::eyre::{Context, Result, eyre};
use duct::cmd;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

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

    let manifest_paths = workspace_manifest_paths(package_filters.as_deref())?;
    let mut failures = Vec::new();
    for manifest_path in manifest_paths {
        spinner.set_message(format!("{} {}", action, manifest_path));

        let mut args =
            vec!["fmt".to_string(), "--manifest-path".to_string(), manifest_path.clone()];
        if check {
            args.push("--".to_string());
            args.push("--check".to_string());
        }

        if check {
            let command = run_check_command_and_stream_output(&manifest_path)
                .with_context(|| format!("Failed to format {}", manifest_path))?;
            if !command.success {
                let crate_name = crate_name_from_manifest_path(&manifest_path);
                let check_command =
                    format!("cargo fmt --manifest-path {} -- --check", manifest_path);
                let fix_command = format!("cargo fmt --manifest-path {}", manifest_path);
                let mut paths = command.paths;
                if paths.is_empty() {
                    paths.push(manifest_path.clone());
                }
                for path in paths {
                    failures.push(FmtFailure {
                        tool: "rustfmt".to_string(),
                        crate_name: crate_name.clone(),
                        path,
                        check_command: check_command.clone(),
                        fix_command: fix_command.clone(),
                    });
                }
            }
        } else {
            let status = cmd("cargo", &args)
                .run()
                .with_context(|| format!("Failed to format {}", manifest_path))?;
            if !status.status.success() {
                spinner.finish_with_message("❌ Code formatting failed".to_string());
                return Err(eyre!("Code formatting failed for {}", manifest_path));
            }
        }
    }

    if check {
        let receipt = build_fmt_receipt(failures, "cargo xtask fmt --check");
        write_fmt_receipt(&receipt)?;
        if receipt.verdict == "fail" {
            spinner.finish_with_message("❌ Code check failed");
            return Err(eyre!(
                "Code check failed for {} file(s). See target/receipts/fmt.json",
                receipt.failures.len()
            ));
        }
    }

    spinner.finish_with_message(format!(
        "✅ Code {} successfully",
        if check { "check passed" } else { "formatted" }
    ));
    Ok(())
}

#[derive(Debug, Clone)]
struct CheckCommandResult {
    success: bool,
    paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct FmtFailure {
    tool: String,
    #[serde(rename = "crate")]
    crate_name: String,
    path: String,
    check_command: String,
    fix_command: String,
}

#[derive(Debug, Clone, Serialize)]
struct FmtRepro {
    command: String,
}

#[derive(Debug, Clone, Serialize)]
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

fn write_fmt_receipt(receipt: &FmtReceipt) -> Result<()> {
    let path = PathBuf::from("target/receipts/fmt.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create receipt directory {}", parent.display()))?;
    }
    let content =
        serde_json::to_string_pretty(receipt).context("Failed to serialize fmt receipt")?;
    fs::write(&path, content)
        .with_context(|| format!("Failed to write receipt to {}", path.display()))
}

fn build_fmt_receipt(mut failures: Vec<FmtFailure>, repro_command: &str) -> FmtReceipt {
    failures.sort_by(|left, right| {
        left.crate_name
            .cmp(&right.crate_name)
            .then(left.path.cmp(&right.path))
            .then(left.tool.cmp(&right.tool))
    });
    FmtReceipt {
        check: "fmt".to_string(),
        schema_version: "1.0.0".to_string(),
        verdict: if failures.is_empty() { "pass".to_string() } else { "fail".to_string() },
        classification: "fmt_drift".to_string(),
        failures,
        fix_forward_kind: "FMT_ONLY".to_string(),
        safe_auto_fix: true,
        repro: FmtRepro { command: repro_command.to_string() },
    }
}

fn run_check_command_and_stream_output(manifest_path: &str) -> Result<CheckCommandResult> {
    let output = Command::new("cargo")
        .arg("fmt")
        .arg("--manifest-path")
        .arg(manifest_path)
        .arg("--")
        .arg("--check")
        .output()
        .with_context(|| format!("Failed to execute cargo fmt --check for {manifest_path}"))?;

    io::stdout().write_all(&output.stdout).context("Failed to stream cargo fmt stdout")?;
    io::stderr().write_all(&output.stderr).context("Failed to stream cargo fmt stderr")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    Ok(CheckCommandResult {
        success: output.status.success(),
        paths: collect_diff_paths(&stdout, &stderr),
    })
}

fn collect_diff_paths(stdout: &str, stderr: &str) -> Vec<String> {
    let mut paths = BTreeSet::new();
    for source in [stdout, stderr] {
        for line in source.lines() {
            if let Some(path) = extract_diff_path(line) {
                paths.insert(path.to_string());
            }
        }
    }
    paths.into_iter().collect()
}

fn extract_diff_path(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("Diff in ")?;
    let (path, _) = rest.split_once(':')?;
    if path.is_empty() {
        return None;
    }
    Some(path)
}

fn crate_name_from_manifest_path(manifest_path: &str) -> String {
    let path = PathBuf::from(manifest_path);
    path.parent()
        .and_then(|parent| parent.file_name())
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "workspace".to_string())
}

fn workspace_manifest_paths(package_filters: Option<&[String]>) -> Result<Vec<String>> {
    let metadata_json = cmd("cargo", ["metadata", "--format-version", "1", "--no-deps"])
        .read()
        .context("Failed to query cargo metadata for workspace formatting")?;
    let metadata: CargoMetadata =
        serde_json::from_str(&metadata_json).context("Failed to parse cargo metadata JSON")?;

    collect_workspace_manifest_paths(&metadata, package_filters)
}

fn collect_workspace_manifest_paths(
    metadata: &CargoMetadata,
    package_filters: Option<&[String]>,
) -> Result<Vec<String>> {
    let package_by_id: HashMap<_, _> = metadata
        .packages
        .iter()
        .map(|package| (package.id.as_str(), package.manifest_path.clone()))
        .collect();
    let member_name_to_manifest: HashMap<_, _> = metadata
        .packages
        .iter()
        .filter(|package| metadata.workspace_members.iter().any(|member| member == &package.id))
        .map(|package| (package.name.as_str(), package.manifest_path.clone()))
        .collect();

    if let Some(filters) = package_filters {
        let mut selected = Vec::with_capacity(filters.len());
        for package_name in filters {
            if let Some(manifest_path) = member_name_to_manifest.get(package_name.as_str()) {
                selected.push(manifest_path.clone());
            } else {
                // Sort the available list so the error message is stable across runs.
                let mut available: Vec<_> = member_name_to_manifest.keys().copied().collect();
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

fn dedup_preserve_order(paths: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::with_capacity(paths.len());
    let mut deduped = Vec::with_capacity(paths.len());
    for path in paths {
        if seen.insert(path.clone()) {
            deduped.push(path);
        }
    }
    deduped
}

#[cfg(test)]
mod tests {
    use super::{
        CargoMetadata, CargoPackage, FmtFailure, build_fmt_receipt, collect_diff_paths,
        collect_workspace_manifest_paths, extract_diff_path,
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
        let manifests = collect_workspace_manifest_paths(&metadata, Some(&filters))?;
        assert_eq!(manifests, vec!["/repo/crates/perl-parser/Cargo.toml".to_string()]);
        Ok(())
    }

    #[test]
    fn package_filters_are_deduplicated() -> Result<()> {
        let metadata = sample_metadata();
        let filters = vec!["xtask".to_string(), "xtask".to_string()];
        let manifests = collect_workspace_manifest_paths(&metadata, Some(&filters))?;
        assert_eq!(manifests, vec!["/repo/xtask/Cargo.toml".to_string()]);
        Ok(())
    }

    #[test]
    fn package_filters_report_unknown_package() -> Result<()> {
        let metadata = sample_metadata();
        let filters = vec!["missing-package".to_string()];
        let message = match collect_workspace_manifest_paths(&metadata, Some(&filters)) {
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
        let message = match collect_workspace_manifest_paths(&metadata, Some(&filters)) {
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
    fn check_mode_collects_multiple_diff_paths() -> Result<()> {
        let stdout =
            "Diff in crates/perl-parser/src/lib.rs:1:\nDiff in crates/xtask/src/main.rs:2:\n";
        let stderr = "";
        let paths = collect_diff_paths(stdout, stderr);
        assert_eq!(
            paths,
            vec![
                "crates/perl-parser/src/lib.rs".to_string(),
                "crates/xtask/src/main.rs".to_string()
            ]
        );
        Ok(())
    }

    #[test]
    fn extract_diff_path_ignores_non_diff_lines() -> Result<()> {
        assert_eq!(extract_diff_path("warning: something"), None);
        assert_eq!(extract_diff_path("Diff in :"), None);
        assert_eq!(
            extract_diff_path("Diff in crates/a/src/lib.rs:10:"),
            Some("crates/a/src/lib.rs")
        );
        Ok(())
    }

    #[test]
    fn receipt_reports_fail_when_any_failures_exist() -> Result<()> {
        let failures = vec![
            FmtFailure {
                tool: "rustfmt".to_string(),
                crate_name: "xtask".to_string(),
                path: "xtask/src/main.rs".to_string(),
                check_command: "cargo fmt --manifest-path xtask/Cargo.toml -- --check".to_string(),
                fix_command: "cargo fmt --manifest-path xtask/Cargo.toml".to_string(),
            },
            FmtFailure {
                tool: "rustfmt".to_string(),
                crate_name: "perl-parser".to_string(),
                path: "crates/perl-parser/src/lib.rs".to_string(),
                check_command: "cargo fmt --manifest-path crates/perl-parser/Cargo.toml -- --check"
                    .to_string(),
                fix_command: "cargo fmt --manifest-path crates/perl-parser/Cargo.toml".to_string(),
            },
        ];
        let receipt = build_fmt_receipt(failures, "cargo xtask fmt --check");
        assert_eq!(receipt.check, "fmt");
        assert_eq!(receipt.verdict, "fail");
        assert_eq!(receipt.classification, "fmt_drift");
        assert_eq!(receipt.fix_forward_kind, "FMT_ONLY");
        assert!(receipt.safe_auto_fix);
        assert_eq!(receipt.failures.len(), 2);
        assert_eq!(receipt.failures[0].crate_name, "perl-parser");
        Ok(())
    }
}
