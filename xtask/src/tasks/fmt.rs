//! Format task implementation

use color_eyre::eyre::{Context, Result, eyre};
use duct::cmd;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoPackage>,
    workspace_members: Vec<String>,
}

#[derive(Deserialize)]
struct CargoPackage {
    id: String,
    manifest_path: String,
}

pub fn run(check: bool) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );

    let action = if check { "Checking" } else { "Formatting" };
    spinner.set_message(format!("{} code", action));

    for manifest_path in workspace_manifest_paths()? {
        spinner.set_message(format!("{} {}", action, manifest_path));

        let mut args = vec![
            "fmt".to_string(),
            "--manifest-path".to_string(),
            manifest_path,
        ];
        if check {
            args.push("--".to_string());
            args.push("--check".to_string());
        }

        let status = cmd("cargo", &args)
            .run()
            .with_context(|| format!("Failed to format {}", args[2]))?;

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

fn workspace_manifest_paths() -> Result<Vec<String>> {
    let metadata_json = cmd("cargo", ["metadata", "--format-version", "1", "--no-deps"])
        .read()
        .context("Failed to query cargo metadata for workspace formatting")?;
    let metadata: CargoMetadata =
        serde_json::from_str(&metadata_json).context("Failed to parse cargo metadata JSON")?;

    let package_by_id: HashMap<_, _> = metadata
        .packages
        .into_iter()
        .map(|package| (package.id, package.manifest_path))
        .collect();

    metadata
        .workspace_members
        .into_iter()
        .map(|member_id| {
            package_by_id
                .get(&member_id)
                .cloned()
                .ok_or_else(|| eyre!("Workspace member not found in cargo metadata: {member_id}"))
        })
        .collect()
}
