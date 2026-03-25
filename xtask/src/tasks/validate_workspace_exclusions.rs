//! Validate workspace exclusion strategy invariants.

use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail, eyre};
use regex::Regex;
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::Command;
use toml::Value;

const EXCLUDED_DIRECTORIES: &[&str] = &["tree-sitter-perl", "fuzz"];
const EXCLUDED_CRATES: &[&str] = &["tree-sitter-perl-c"];
const PROJECT_CARGO_TOML: &str = "Cargo.toml";

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<MetadataPackage>,
}

#[derive(Deserialize)]
struct MetadataPackage {
    name: String,
}

pub fn run() -> Result<()> {
    let root = project_root()?;

    println!("Validating workspace exclusion strategy...");
    println!();

    check_excluded_directories_exist(&root)?;
    check_exclusion_documentation(&root)?;
    check_workspace_dependencies(&root)?;
    check_exclude_section(&root)?;
    check_workspace_members(&root)?;
    check_member_dependencies(&root)?;

    println!("==========================================");
    println!("✅ All workspace exclusion checks passed!");
    println!("==========================================");
    println!();
    println!("Summary:");
    println!("  - {} directories excluded from workspace", EXCLUDED_DIRECTORIES.len());
    println!("  - Exclusion strategy clearly documented");
    println!("  - No accidental dependencies on excluded crates");
    println!("  - workspace.dependencies clean");

    Ok(())
}

fn check_excluded_directories_exist(root: &Path) -> Result<()> {
    println!("✓ Checking excluded directories exist...");

    let missing = EXCLUDED_DIRECTORIES
        .iter()
        .filter(|excluded| !root.join(excluded).exists())
        .map(|excluded| excluded.to_string())
        .collect::<Vec<_>>();

    if !missing.is_empty() {
        bail!("❌ ERROR: Excluded directories do not exist: {}", missing.join(", "));
    }

    println!("  All excluded directories exist");
    println!();
    Ok(())
}

fn check_exclusion_documentation(root: &Path) -> Result<()> {
    println!("✓ Checking exclusion documentation...");

    let cargo_toml = root.join(PROJECT_CARGO_TOML);
    let content = fs::read_to_string(&cargo_toml).with_context(|| {
        format!("Failed to read workspace Cargo.toml at {}", cargo_toml.display())
    })?;

    if !content.contains("exclude = [") {
        bail!("❌ ERROR: Exclusion strategy not documented in Cargo.toml");
    }

    println!("  Exclusion strategy is documented");
    println!();
    Ok(())
}

fn check_workspace_dependencies(root: &Path) -> Result<()> {
    println!("✓ Checking workspace.dependencies...");

    let cargo_toml = root.join(PROJECT_CARGO_TOML);
    let content = fs::read_to_string(&cargo_toml).with_context(|| {
        format!("Failed to read workspace Cargo.toml at {}", cargo_toml.display())
    })?;
    let manifest: Value =
        toml::from_str(&content).context("Failed to parse workspace Cargo.toml")?;

    let workspace_dependencies = manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(Value::as_table);

    if let Some(workspace_dependencies) = workspace_dependencies {
        let excluded = excluded_set();
        if workspace_dependencies.keys().any(|dep| excluded.contains(dep.as_str())) {
            bail!("❌ ERROR: workspace.dependencies references excluded crates");
        }
    }

    println!("  workspace.dependencies clean (no excluded crate references)");
    println!();
    Ok(())
}

fn check_exclude_section(root: &Path) -> Result<()> {
    println!("✓ Checking exclude section...");

    let cargo_toml = root.join(PROJECT_CARGO_TOML);
    let content = fs::read_to_string(&cargo_toml).with_context(|| {
        format!("Failed to read workspace Cargo.toml at {}", cargo_toml.display())
    })?;
    let manifest: Value =
        toml::from_str(&content).context("Failed to parse workspace Cargo.toml")?;

    let exclude = manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("exclude"))
        .and_then(Value::as_array)
        .ok_or_else(|| eyre!("Workspace has no [workspace].exclude array"))?;

    let exclude_values: Vec<&str> = exclude.iter().filter_map(Value::as_str).collect();
    let missing = EXCLUDED_DIRECTORIES
        .iter()
        .filter(|entry| !exclude_values.contains(entry))
        .map(|entry| entry.to_string())
        .collect::<Vec<_>>();

    if !missing.is_empty() {
        bail!("❌ ERROR: Excluded paths missing from [workspace].exclude: {}", missing.join(", "));
    }

    println!("  All expected paths are in exclude section");
    println!();
    Ok(())
}

fn check_workspace_members(root: &Path) -> Result<()> {
    println!("✓ Checking workspace members don't include excluded crates...");

    let metadata = load_cargo_metadata(root)?;
    let excluded = excluded_set();
    let offending = metadata
        .packages
        .iter()
        .filter(|pkg| excluded.contains(pkg.name.as_str()))
        .map(|pkg| pkg.name.clone())
        .collect::<Vec<_>>();
    let member_count = metadata.packages.len();

    if !offending.is_empty() {
        bail!("❌ ERROR: Excluded crates found in workspace members: {}", offending.join(", "));
    }

    println!("  Workspace has {} members (excluded crates not included)", member_count);
    println!();
    Ok(())
}

fn check_member_dependencies(root: &Path) -> Result<()> {
    println!("✓ Checking for dependencies on excluded crates...");

    let excluded = excluded_set();
    let crate_pattern = format!(
        r"(?m)^\s*({})\s*=",
        excluded.iter().map(|entry| regex::escape(entry)).collect::<Vec<_>>().join("|")
    );
    let exclusion_re = Regex::new(&crate_pattern).context("Failed to compile dependency regex")?;

    let mut offenders = Vec::new();
    let crates_dir = root.join("crates");
    for entry in fs::read_dir(&crates_dir).context("Unable to list crates directory")? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let manifest = entry.path().join("Cargo.toml");
        if !manifest.exists() {
            continue;
        }

        let crate_name =
            entry.file_name().into_string().unwrap_or_else(|_| String::from("<invalid>"));
        let content = fs::read_to_string(&manifest)
            .with_context(|| format!("Failed to read {}", manifest.display()))?;

        if has_excluded_dependency_reference(&content, &exclusion_re, &excluded) {
            offenders.push(crate_name);
        }
    }

    if !offenders.is_empty() {
        bail!("❌ ERROR: Dependencies on excluded crates found in: {}", offenders.join(", "));
    }

    println!("  No workspace members depend on excluded crates");
    println!();
    Ok(())
}

fn has_excluded_dependency_reference(
    manifest_content: &str,
    pattern: &Regex,
    excluded: &HashSet<&str>,
) -> bool {
    for line in manifest_content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if pattern.is_match(trimmed) {
            if let Some(capture) = pattern.captures(trimmed).and_then(|c| c.get(1)) {
                if excluded.contains(capture.as_str()) {
                    return true;
                }
            }
        }
    }

    false
}

fn excluded_set() -> HashSet<&'static str> {
    EXCLUDED_CRATES.iter().copied().collect()
}

fn load_cargo_metadata(root: &Path) -> Result<Metadata> {
    let output = Command::new("cargo")
        .current_dir(root)
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .context("Failed to execute `cargo metadata`")?;

    if !output.status.success() {
        bail!("`cargo metadata` failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    let stdout =
        String::from_utf8(output.stdout).context("`cargo metadata` output was not valid UTF-8")?;
    serde_json::from_str(&stdout).context("Failed to parse `cargo metadata` JSON")
}
