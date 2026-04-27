//! Generated-file ownership checks.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Result, bail, eyre};
use glob::Pattern;
use serde::{Deserialize, Serialize};

use crate::utils::project_root;

const MANIFEST_PATH: &str = ".ci/generated-files.toml";

#[derive(Debug, Deserialize)]
struct GeneratedManifest {
    generated: Vec<GeneratedEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GeneratedEntry {
    path: String,
    command: String,
    owner: String,
    #[serde(default)]
    allow_manual_edits: bool,
}

#[derive(Debug, Deserialize, Default)]
struct FixtureInput {
    #[serde(default)]
    changed_files: Vec<String>,
    #[serde(default)]
    receipts: Vec<GeneratorReceipt>,
}

#[derive(Debug, Clone, Deserialize)]
struct GeneratorReceipt {
    owner: String,
    command: String,
}

#[derive(Debug, Serialize)]
struct GeneratedFilesReceipt {
    schema_version: u8,
    verdict: String,
    changed_files: Vec<String>,
    expected_command: Vec<String>,
    missing_receipts: Vec<String>,
}

pub fn list(fixture: Option<PathBuf>) -> Result<()> {
    let manifest = load_manifest()?;
    let changed_files = load_changed_files(fixture)?;

    for entry in &manifest.generated {
        println!("{} => {} ({})", entry.path, entry.command, entry.owner);
    }

    if !changed_files.is_empty() {
        println!("\nchanged files:");
        for file in changed_files {
            println!("- {file}");
        }
    }

    Ok(())
}

pub fn check(
    receipt_path: PathBuf,
    fixture: Option<PathBuf>,
    generator_receipt: Vec<PathBuf>,
    allow_manual_edits: bool,
) -> Result<()> {
    let manifest = load_manifest()?;
    let fixture_input = load_fixture_input(fixture)?;
    let mut available_receipts = fixture_input.receipts;
    available_receipts.extend(load_generator_receipts(&generator_receipt)?);

    let changed_files = if fixture_input.changed_files.is_empty() {
        detect_changed_files_from_git()?
    } else {
        fixture_input.changed_files
    };

    let mut expected_commands = BTreeSet::new();
    let mut missing_receipts = BTreeSet::new();
    let mut override_used = false;

    for changed_file in &changed_files {
        for entry in &manifest.generated {
            if !matches_pattern(&entry.path, changed_file)? {
                continue;
            }

            expected_commands.insert(entry.command.clone());

            if entry.allow_manual_edits {
                continue;
            }
            if allow_manual_edits {
                override_used = true;
                continue;
            }

            let has_receipt = available_receipts
                .iter()
                .any(|receipt| receipt.owner == entry.owner || receipt.command == entry.command);

            if !has_receipt {
                missing_receipts.insert(entry.owner.clone());
            }
        }
    }

    let verdict = if !missing_receipts.is_empty() {
        "fail"
    } else if override_used {
        "override"
    } else {
        "pass"
    };

    let receipt = GeneratedFilesReceipt {
        schema_version: 1,
        verdict: verdict.to_string(),
        changed_files,
        expected_command: expected_commands.into_iter().collect(),
        missing_receipts: missing_receipts.into_iter().collect(),
    };

    write_receipt(&receipt_path, &receipt)?;

    if receipt.verdict == "fail" {
        bail!(
            "generated-files ownership check failed: missing receipts for {:?}",
            receipt.missing_receipts
        );
    }

    println!("generated-files verdict: {}", receipt.verdict);
    Ok(())
}

fn load_manifest() -> Result<GeneratedManifest> {
    let root = project_root()?;
    let manifest_path = root.join(MANIFEST_PATH);
    let raw = fs::read_to_string(&manifest_path)?;
    let parsed: GeneratedManifest = toml::from_str(&raw)
        .map_err(|err| eyre!("failed to parse {}: {err}", manifest_path.to_string_lossy()))?;
    Ok(parsed)
}

fn load_changed_files(fixture: Option<PathBuf>) -> Result<Vec<String>> {
    let fixture_input = load_fixture_input(fixture)?;
    if fixture_input.changed_files.is_empty() {
        detect_changed_files_from_git()
    } else {
        Ok(fixture_input.changed_files)
    }
}

fn load_fixture_input(fixture: Option<PathBuf>) -> Result<FixtureInput> {
    match fixture {
        Some(path) => {
            let raw = fs::read_to_string(path)?;
            let parsed = serde_json::from_str::<FixtureInput>(&raw)?;
            Ok(parsed)
        }
        None => Ok(FixtureInput::default()),
    }
}

fn load_generator_receipts(paths: &[PathBuf]) -> Result<Vec<GeneratorReceipt>> {
    let mut receipts = Vec::new();
    for path in paths {
        let raw = fs::read_to_string(path)?;
        let value: serde_json::Value = serde_json::from_str(&raw)?;
        receipts.extend(parse_receipt_value(&value)?);
    }
    Ok(receipts)
}

fn parse_receipt_value(value: &serde_json::Value) -> Result<Vec<GeneratorReceipt>> {
    if let Ok(single) = serde_json::from_value::<GeneratorReceipt>(value.clone()) {
        return Ok(vec![single]);
    }

    if let Ok(list) = serde_json::from_value::<Vec<GeneratorReceipt>>(value.clone()) {
        return Ok(list);
    }

    if let Some(array) = value.get("receipts") {
        let list: Vec<GeneratorReceipt> = serde_json::from_value(array.clone())?;
        return Ok(list);
    }

    bail!("unsupported generator receipt JSON format")
}

fn detect_changed_files_from_git() -> Result<Vec<String>> {
    let root = project_root()?;
    let diff = std::process::Command::new("git")
        .arg("diff")
        .arg("--name-only")
        .current_dir(&root)
        .output()?;

    if !diff.status.success() {
        bail!("git diff --name-only failed")
    }

    let mut files = parse_lines_to_set(&String::from_utf8(diff.stdout)?);

    let staged = std::process::Command::new("git")
        .arg("diff")
        .arg("--name-only")
        .arg("--cached")
        .current_dir(&root)
        .output()?;

    if staged.status.success() {
        files.extend(parse_lines_to_set(&String::from_utf8(staged.stdout)?));
    }

    let untracked = std::process::Command::new("git")
        .arg("ls-files")
        .arg("--others")
        .arg("--exclude-standard")
        .current_dir(&root)
        .output()?;

    if untracked.status.success() {
        files.extend(parse_lines_to_set(&String::from_utf8(untracked.stdout)?));
    }

    let mut sorted: Vec<String> = files.into_iter().collect();
    sorted.sort();
    Ok(sorted)
}

fn parse_lines_to_set(raw: &str) -> BTreeSet<String> {
    raw.lines().map(str::trim).filter(|line| !line.is_empty()).map(ToString::to_string).collect()
}

fn matches_pattern(pattern: &str, candidate: &str) -> Result<bool> {
    let compiled = Pattern::new(pattern)?;
    Ok(compiled.matches(candidate))
}

fn write_receipt(path: &Path, receipt: &GeneratedFilesReceipt) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let payload = serde_json::json!({
        "$schema": ".ci/receipts/schemas/generated-files.schema.json",
        "schema_version": receipt.schema_version,
        "verdict": receipt.verdict,
        "changed_files": receipt.changed_files,
        "expected_command": receipt.expected_command,
        "missing_receipts": receipt.missing_receipts,
    });

    fs::write(path, serde_json::to_string_pretty(&payload)?)?;
    Ok(())
}
