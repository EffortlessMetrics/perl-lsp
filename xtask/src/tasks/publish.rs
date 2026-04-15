//! Publishing functionality for crates and VSCode extension

use crate::utils::{project_root, run_cargo_metadata};
use color_eyre::eyre::{Result, bail, eyre};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

pub fn publish_crates(yes: bool, dry_run: bool) -> Result<()> {
    println!("📦 Publishing crates to crates.io");

    let publish_targets = load_publish_targets()?;

    if !yes {
        println!("This will publish:");
        for target in &publish_targets {
            println!("  - {}", target.name);
        }
        println!();
        print!("Continue? [y/N] ");

        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Publishing cancelled.");
            return Ok(());
        }
    }

    let mut args = vec!["publish", "--no-verify"];
    if dry_run {
        args.push("--dry-run");
    }

    for (index, target) in publish_targets.iter().enumerate() {
        println!("Publishing {}...", target.name);
        let crate_dir = target.manifest_path.parent().ok_or_else(|| {
            eyre!(
                "Invalid manifest path for publish target '{}': {:?}",
                target.name,
                target.manifest_path
            )
        })?;

        let output = Command::new("cargo").current_dir(crate_dir).args(&args).output()?;
        if !output.status.success() {
            bail!("Failed to publish {}: {}", target.name, String::from_utf8_lossy(&output.stderr));
        }
        println!("✅ {} published", target.name);

        if !dry_run && index + 1 != publish_targets.len() {
            // Wait for crates.io to process before publishing the next package.
            println!("Waiting 30 seconds for crates.io to process...");
            thread::sleep(Duration::from_secs(30));
        }
    }
    println!();
    println!("✅ All crates published successfully!");

    Ok(())
}

#[derive(Deserialize)]
struct CargoMetadata {
    metadata: Option<WorkspaceMetadata>,
    packages: Vec<MetadataPackage>,
}

#[derive(Deserialize)]
struct WorkspaceMetadata {
    publish: Option<PublishMetadata>,
}

#[derive(Deserialize)]
struct PublishMetadata {
    allow: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct MetadataPackage {
    name: String,
    manifest_path: PathBuf,
}

struct PublishTarget {
    name: String,
    manifest_path: PathBuf,
}

fn load_publish_targets() -> Result<Vec<PublishTarget>> {
    let bytes = run_cargo_metadata(true)?;
    let metadata: CargoMetadata = serde_json::from_slice(&bytes)?;

    let allowlist = metadata
        .metadata
        .and_then(|workspace| workspace.publish)
        .and_then(|publish| publish.allow)
        .ok_or_else(|| {
            eyre!(
                "Publish allowlist missing. Add [workspace.metadata.publish.allow] in the workspace Cargo.toml."
            )
        })?;

    if allowlist.is_empty() {
        bail!("Publish allowlist is empty. Add crates to [workspace.metadata.publish.allow].");
    }

    let mut package_map = HashMap::new();
    for package in metadata.packages {
        package_map.insert(package.name, package.manifest_path);
    }

    let mut seen = HashSet::new();
    let mut targets = Vec::new();

    for crate_name in allowlist {
        if !seen.insert(crate_name.clone()) {
            continue;
        }

        let manifest_path = package_map.get(&crate_name).ok_or_else(|| {
            eyre!(
                "Crate '{}' listed in [workspace.metadata.publish.allow] is not a workspace member.",
                crate_name
            )
        })?;

        targets.push(PublishTarget { name: crate_name, manifest_path: manifest_path.clone() });
    }

    Ok(targets)
}

pub fn publish_vscode(yes: bool, token: Option<String>) -> Result<()> {
    println!("🚀 Publishing VSCode extension to marketplace");

    // Check for token - try argument first, then environment variable
    let token = token.or_else(|| std::env::var("VSCE_PAT").ok());
    if token.is_none() {
        bail!("VSCE_PAT token required. Set via --token or VSCE_PAT environment variable.");
    }

    if !yes {
        println!("This will publish the VSCode extension to the marketplace.");
        println!();
        print!("Continue? [y/N] ");

        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Publishing cancelled.");
            return Ok(());
        }
    }

    // First compile the extension
    println!("Compiling extension...");
    let output =
        Command::new("npm").current_dir("vscode-extension").args(["run", "compile"]).output()?;

    if !output.status.success() {
        bail!("Failed to compile extension: {}", String::from_utf8_lossy(&output.stderr));
    }

    // Publish to marketplace
    println!("Publishing to marketplace...");
    let token = token.ok_or_else(|| {
        color_eyre::eyre::eyre!("VSCE_PAT environment variable is required for publishing")
    })?;
    let output = Command::new("npx")
        .current_dir("vscode-extension")
        .env("VSCE_PAT", token)
        .args(["vsce", "publish"])
        .output()?;

    if !output.status.success() {
        bail!("Failed to publish extension: {}", String::from_utf8_lossy(&output.stderr));
    }

    println!("✅ VSCode extension published successfully!");
    println!();
    println!(
        "View in marketplace: https://marketplace.visualstudio.com/items?itemName=perl.language-server"
    );

    Ok(())
}

pub fn publish_release(version: String, dry_run: bool, git_ref: Option<String>) -> Result<()> {
    let root = project_root()?;
    let ref_name = git_ref.unwrap_or_else(|| format!("v{version}"));

    let status = Command::new("gh")
        .current_dir(&root)
        .args([
            "workflow",
            "run",
            "Publish to crates.io",
            "--ref",
            &ref_name,
            "-f",
            &format!("version={version}"),
            "-f",
            &format!("dry_run={dry_run}"),
        ])
        .status()?;

    if !status.success() {
        bail!("publish-release workflow dispatch failed");
    }

    println!("Dispatched \"Publish to crates.io\" for {version} on ref {ref_name}.");
    Ok(())
}

pub fn smoke_test_release(version: String) -> Result<()> {
    let root = project_root()?;
    // Use a relative path with forward slashes so bash (Git Bash / MSYS2 on
    // Windows) does not interpret backslashes in an absolute Windows path as
    // escape sequences. `current_dir(&root)` below anchors the relative path.
    let script = std::path::PathBuf::from("scripts/smoke-test-release.sh");
    let status = Command::new("bash")
        .arg(&script)
        .arg(version)
        .current_dir(&root)
        .env("XTASK_SMOKE_TEST_RELEASE", "1")
        .status()?;

    if !status.success() {
        bail!("smoke-test-release failed");
    }

    Ok(())
}
