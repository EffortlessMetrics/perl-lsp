//! Format task implementation

use color_eyre::eyre::{Context, Result, eyre};
use duct::cmd;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap};

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

    for manifest_path in workspace_manifest_paths(package_filters.as_deref())? {
        spinner.set_message(format!("{} {}", action, manifest_path));

        let mut args = vec!["fmt".to_string(), "--manifest-path".to_string(), manifest_path];
        if check {
            args.push("--".to_string());
            args.push("--check".to_string());
        }

        let status =
            cmd("cargo", &args).run().with_context(|| format!("Failed to format {}", args[2]))?;

        if !status.status.success() {
            spinner.finish_with_message(format!(
                "❌ Code {} failed",
                if check { "check" } else { "formatting" }
            ));
            return Err(eyre!(
                "Code {} failed for {}",
                if check { "check" } else { "formatting" },
                args[2]
            ));
        }
    }

    spinner.finish_with_message(format!(
        "✅ Code {} successfully",
        if check { "check passed" } else { "formatted" }
    ));
    Ok(())
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
                let available: Vec<_> = member_name_to_manifest.keys().copied().collect();
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
    let mut seen = BTreeSet::new();
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
    use super::{CargoMetadata, CargoPackage, collect_workspace_manifest_paths};
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
}
