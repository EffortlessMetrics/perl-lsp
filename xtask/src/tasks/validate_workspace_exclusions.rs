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

const REQUIRED_EXCLUDED_DIRECTORIES: &[&str] = &["tree-sitter-perl", "fuzz"];
const OPTIONAL_EXCLUDED_DIRECTORIES: &[&str] = &["archive"];
const EXCLUDED_CRATES: &[&str] = &["tree-sitter-perl", "perl-parser-fuzz"];
const PROJECT_CARGO_TOML: &str = "Cargo.toml";

#[derive(Deserialize)]
struct Metadata {
    workspace_members: Vec<String>,
    packages: Vec<MetadataPackage>,
}

#[derive(Deserialize)]
struct MetadataPackage {
    id: String,
    name: String,
    manifest_path: String,
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
    println!(
        "  - {} directories excluded from workspace",
        known_excluded_directories().count()
    );
    println!("  - Exclusion strategy clearly documented");
    println!("  - No accidental dependencies on excluded crates");
    println!("  - workspace.dependencies clean");

    Ok(())
}

fn check_excluded_directories_exist(root: &Path) -> Result<()> {
    println!("✓ Checking excluded directories exist...");

    let missing = REQUIRED_EXCLUDED_DIRECTORIES
        .iter()
        .filter(|excluded| !root.join(excluded).exists())
        .map(|excluded| excluded.to_string())
        .collect::<Vec<_>>();

    if !missing.is_empty() {
        bail!(
            "❌ ERROR: Excluded directories do not exist: {}",
            missing.join(", ")
        );
    }

    println!("  All required excluded directories exist");
    println!();
    Ok(())
}

fn check_exclusion_documentation(root: &Path) -> Result<()> {
    println!("✓ Checking exclusion documentation...");

    let cargo_toml = root.join(PROJECT_CARGO_TOML);
    let content = fs::read_to_string(&cargo_toml).with_context(|| {
        format!(
            "Failed to read workspace Cargo.toml at {}",
            cargo_toml.display()
        )
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
        format!(
            "Failed to read workspace Cargo.toml at {}",
            cargo_toml.display()
        )
    })?;
    let manifest: Value =
        toml::from_str(&content).context("Failed to parse workspace Cargo.toml")?;

    let workspace_dependencies = manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(Value::as_table);

    if let Some(workspace_dependencies) = workspace_dependencies {
        let excluded = excluded_set();
        let offending = workspace_dependencies
            .iter()
            .filter_map(|(name, _value)| {
                if !excluded.contains(name.as_str()) {
                    return None;
                }

                Some(name.as_str())
            })
            .collect::<Vec<_>>();

        if !offending.is_empty() {
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
        format!(
            "Failed to read workspace Cargo.toml at {}",
            cargo_toml.display()
        )
    })?;
    let manifest: Value =
        toml::from_str(&content).context("Failed to parse workspace Cargo.toml")?;

    let exclude_values = workspace_exclude_values(&manifest)?;
    let missing = missing_required_excluded_directories(&exclude_values);

    if !missing.is_empty() {
        bail!(
            "❌ ERROR: Excluded paths missing from [workspace].exclude: {}",
            missing.join(", ")
        );
    }

    println!("  Required excluded paths are in [workspace].exclude");
    println!();
    Ok(())
}

fn check_workspace_members(root: &Path) -> Result<()> {
    println!("✓ Checking workspace members don't include excluded crates...");

    let metadata = load_cargo_metadata(root)?;
    let offending = excluded_workspace_member_names(&metadata);
    let member_count = metadata.workspace_members.len();

    if !offending.is_empty() {
        bail!(
            "❌ ERROR: Excluded crates found in workspace members: {}",
            offending.join(", ")
        );
    }

    println!(
        "  Workspace has {} members (excluded crates not included)",
        member_count
    );
    println!();
    Ok(())
}

fn check_member_dependencies(root: &Path) -> Result<()> {
    println!("✓ Checking for dependencies on excluded crates...");

    let excluded = excluded_set();
    let crate_pattern = format!(
        r"(?m)^\s*({})\s*=",
        excluded
            .iter()
            .map(|entry| regex::escape(entry))
            .collect::<Vec<_>>()
            .join("|")
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

        let crate_name = entry
            .file_name()
            .into_string()
            .unwrap_or_else(|_| String::from("<invalid>"));
        let content = fs::read_to_string(&manifest)
            .with_context(|| format!("Failed to read {}", manifest.display()))?;

        if has_excluded_dependency_reference(&content, &exclusion_re, &excluded) {
            offenders.push(crate_name);
        }
    }

    if !offenders.is_empty() {
        bail!(
            "❌ ERROR: Dependencies on excluded crates found in: {}",
            offenders.join(", ")
        );
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

        if !pattern.is_match(trimmed) {
            continue;
        }

        let Some(capture) = pattern.captures(trimmed).and_then(|c| c.get(1)) else {
            continue;
        };

        if excluded.contains(capture.as_str()) {
            return true;
        }
    }

    false
}

fn excluded_set() -> HashSet<&'static str> {
    EXCLUDED_CRATES.iter().copied().collect()
}

fn known_excluded_directories() -> impl Iterator<Item = &'static str> {
    REQUIRED_EXCLUDED_DIRECTORIES
        .iter()
        .chain(OPTIONAL_EXCLUDED_DIRECTORIES.iter())
        .copied()
}

fn workspace_exclude_values(manifest: &Value) -> Result<Vec<&str>> {
    let exclude = manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("exclude"))
        .and_then(Value::as_array)
        .ok_or_else(|| eyre!("Workspace has no [workspace].exclude array"))?;

    Ok(exclude.iter().filter_map(Value::as_str).collect())
}

fn missing_required_excluded_directories(exclude_values: &[&str]) -> Vec<String> {
    REQUIRED_EXCLUDED_DIRECTORIES
        .iter()
        .filter(|entry| !exclude_values.contains(entry))
        .map(|entry| entry.to_string())
        .collect()
}

fn manifest_is_in_excluded_directory(manifest_path: &str) -> bool {
    let manifest_dir = Path::new(manifest_path)
        .parent()
        .unwrap_or_else(|| Path::new(manifest_path));

    known_excluded_directories().any(|entry| manifest_dir.ends_with(entry))
}

fn excluded_workspace_member_names(metadata: &Metadata) -> Vec<String> {
    let workspace_member_ids = metadata.workspace_members.iter().collect::<HashSet<_>>();

    metadata
        .packages
        .iter()
        .filter(|pkg| workspace_member_ids.contains(&pkg.id))
        .filter(|pkg| manifest_is_in_excluded_directory(&pkg.manifest_path))
        .map(|pkg| pkg.name.clone())
        .collect()
}

fn load_cargo_metadata(root: &Path) -> Result<Metadata> {
    let output = Command::new("cargo")
        .current_dir(root)
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .context("Failed to execute `cargo metadata`")?;

    if !output.status.success() {
        bail!(
            "`cargo metadata` failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let stdout =
        String::from_utf8(output.stdout).context("`cargo metadata` output was not valid UTF-8")?;
    serde_json::from_str(&stdout).context("Failed to parse `cargo metadata` JSON")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_exclude_values_reads_workspace_table() -> Result<()> {
        let manifest: Value = toml::from_str(
            r#"
exclude = ["wrong-root-level-entry"]

[workspace]
exclude = ["tree-sitter-perl", "fuzz"]
"#,
        )?;

        let exclude = workspace_exclude_values(&manifest)?;

        assert_eq!(exclude, vec!["tree-sitter-perl", "fuzz"]);
        Ok(())
    }

    #[test]
    fn missing_required_excluded_paths_ignores_optional_entries() {
        let exclude_values = vec!["tree-sitter-perl", "fuzz"];

        let missing = missing_required_excluded_directories(&exclude_values);

        assert!(missing.is_empty());
    }

    #[test]
    fn missing_required_excluded_paths_flags_missing_required_path() {
        let exclude_values = vec!["tree-sitter-perl"];

        let missing = missing_required_excluded_directories(&exclude_values);

        assert_eq!(missing, vec!["fuzz".to_string()]);
    }

    #[test]
    fn manifest_is_in_excluded_directory_uses_manifest_parent() -> Result<()> {
        let manifest = Path::new("/repo/tree-sitter-perl/Cargo.toml");
        let manifest_dir = manifest
            .parent()
            .ok_or_else(|| eyre!("Expected manifest path to have a parent"))?;

        assert!(manifest_dir.ends_with("tree-sitter-perl"));
        assert!(!manifest.ends_with("tree-sitter-perl"));
        assert!(manifest_is_in_excluded_directory(
            "/repo/tree-sitter-perl/Cargo.toml"
        ));
        Ok(())
    }

    #[test]
    fn excluded_workspace_member_names_flags_members_by_manifest_directory() {
        let excluded_id = String::from("perl-parser-fuzz 0.1.0 (path+file:///repo/fuzz)");
        let metadata = Metadata {
            workspace_members: vec![excluded_id.clone()],
            packages: vec![
                MetadataPackage {
                    id: excluded_id,
                    name: String::from("perl-parser-fuzz"),
                    manifest_path: String::from("/repo/fuzz/Cargo.toml"),
                },
                MetadataPackage {
                    id: String::from("other 0.1.0 (path+file:///repo/crates/other)"),
                    name: String::from("other"),
                    manifest_path: String::from("/repo/crates/other/Cargo.toml"),
                },
            ],
        };

        let offending = excluded_workspace_member_names(&metadata);

        assert_eq!(offending, vec!["perl-parser-fuzz".to_string()]);
    }
}
