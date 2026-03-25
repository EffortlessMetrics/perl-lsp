//! crates.io launch-preparation helper.

use color_eyre::eyre::{Context, Result, bail};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::utils::project_root;

const CORE_LAUNCH_CRATES: &[&str] =
    &["perl-parser", "perl-lexer", "perl-lsp", "perl-dap", "perl-corpus"];

#[derive(Deserialize)]
struct RootCargoManifest {
    workspace: RootWorkspace,
}

#[derive(Deserialize)]
struct RootWorkspace {
    metadata: Option<RootWorkspaceMetadata>,
}

#[derive(Deserialize)]
struct RootWorkspaceMetadata {
    publish: Option<RootPublishMetadata>,
}

#[derive(Deserialize)]
struct RootPublishMetadata {
    allow: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct CargoMetadata {
    workspace_root: String,
    packages: Vec<MetadataPackage>,
}

#[derive(Deserialize)]
struct MetadataPackage {
    name: String,
    manifest_path: String,
    publish: Option<JsonValue>,
}

pub fn run(all: bool) -> Result<()> {
    let root = project_root()?;
    let launch_crates = if all {
        load_publish_allowlist(&root)?
    } else {
        CORE_LAUNCH_CRATES.iter().map(|name| (*name).to_string()).collect()
    };

    let metadata = load_cargo_metadata(&root)?;
    let patch_args = package_patch_args(&metadata);
    let package_names: HashSet<_> =
        metadata.packages.iter().map(|package| package.name.as_str()).collect();

    let unknown = launch_crates
        .iter()
        .filter(|name| !package_names.contains(name.as_str()))
        .collect::<Vec<_>>();

    if !unknown.is_empty() {
        let unknown_list = unknown.iter().map(|name| name.as_str()).collect::<Vec<_>>().join(", ");
        bail!("unknown crates for launch prep: {unknown_list}");
    }

    println!(
        "🚀 crates.io launch prep ({})",
        if all { "all publish-allowlist crates" } else { "core launch crates" }
    );
    println!(
        "📦 Running cargo check + cargo package (dry-run) for {} crate(s)",
        launch_crates.len()
    );

    for crate_name in launch_crates {
        println!();
        println!("==> {crate_name}");
        run_cargo_check(&root, &crate_name)?;
        run_cargo_package_dry_run(&root, &crate_name, &patch_args)?;
    }

    println!();
    println!("✅ crates.io launch prep completed ({})", if all { "all" } else { "core" });

    Ok(())
}

fn package_patch_args(metadata: &CargoMetadata) -> Vec<String> {
    let mut args = Vec::new();
    let workspace_root = Path::new(&metadata.workspace_root);

    for package in &metadata.packages {
        if !is_publish_candidate(&package.publish) {
            continue;
        }

        let manifest_path = Path::new(&package.manifest_path);
        let crate_dir = match manifest_path.parent() {
            Some(dir) => dir,
            None => continue,
        };
        let rel_dir = crate_dir.strip_prefix(workspace_root).unwrap_or(crate_dir);

        args.push(format!("--config=patch.crates-io.{}.path={}", package.name, rel_dir.display()));
    }

    args
}

fn is_publish_candidate(publish: &Option<JsonValue>) -> bool {
    match publish {
        Some(value) => match value.as_array() {
            Some(entries) => !entries.is_empty(),
            None => true,
        },
        None => true,
    }
}

fn load_cargo_metadata(root: &Path) -> Result<CargoMetadata> {
    let output = Command::new("cargo")
        .current_dir(root)
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .context("failed to run cargo metadata")?;

    if !output.status.success() {
        bail!("cargo metadata failed: {}", String::from_utf8_lossy(&output.stderr).trim_end());
    }

    let metadata: CargoMetadata =
        serde_json::from_slice(&output.stdout).context("failed to parse cargo metadata output")?;
    Ok(metadata)
}

fn load_publish_allowlist(root: &Path) -> Result<Vec<String>> {
    let manifest_text = fs::read_to_string(root.join("Cargo.toml"))
        .context("failed to read workspace Cargo.toml")?;
    let manifest: RootCargoManifest =
        toml::from_str(&manifest_text).context("failed to parse workspace Cargo.toml")?;

    let allowlist = manifest
        .workspace
        .metadata
        .and_then(|metadata| metadata.publish)
        .and_then(|publish| publish.allow)
        .ok_or_else(|| {
            color_eyre::eyre::eyre!("[workspace.metadata.publish.allow] is missing from Cargo.toml")
        })?;

    if allowlist.is_empty() {
        bail!("publish allowlist is empty");
    }

    Ok(allowlist)
}

fn run_cargo_check(root: &Path, crate_name: &str) -> Result<()> {
    let output = Command::new("cargo")
        .current_dir(root)
        .args(["check", "--locked", "-p"])
        .arg(crate_name)
        .output()
        .context("failed to run cargo check")?;

    if !output.status.success() {
        bail!(
            "cargo check failed for {crate_name}: {}",
            String::from_utf8_lossy(&output.stderr).trim_end()
        );
    }

    Ok(())
}

fn run_cargo_package_dry_run(root: &Path, crate_name: &str, patch_args: &[String]) -> Result<()> {
    let mut args = vec!["package".to_string(), "-p".to_string(), crate_name.to_string()];
    args.push("--no-verify".to_string());
    args.extend(patch_args.iter().cloned());

    let output = Command::new("cargo")
        .current_dir(root)
        .args(args)
        .output()
        .context("failed to run cargo package")?;

    if !output.status.success() {
        bail!(
            "cargo package dry-run failed for {crate_name}: {}",
            String::from_utf8_lossy(&output.stderr).trim_end()
        );
    }

    Ok(())
}
