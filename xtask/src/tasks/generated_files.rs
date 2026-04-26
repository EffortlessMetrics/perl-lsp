use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Context, Result, bail};
use glob::Pattern;
use serde::{Deserialize, Serialize};

use crate::utils::project_root;

const GENERATED_MANIFEST: &str = ".ci/generated-files.toml";

#[derive(Debug, Clone, Deserialize)]
pub struct GeneratedManifest {
    #[serde(default)]
    generated: Vec<GeneratedRule>,
}

#[derive(Debug, Clone, Deserialize)]
struct GeneratedRule {
    path: String,
    command: String,
    owner: String,
    #[serde(default)]
    allow_manual_edits: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct GeneratorReceipt {
    owner: String,
    command: String,
}

#[derive(Debug, Clone, Deserialize)]
struct FixtureData {
    #[serde(default)]
    changed_files: Vec<String>,
    #[serde(default)]
    generator_receipts: Vec<GeneratorReceipt>,
}

#[derive(Debug, Serialize)]
struct CheckReceipt {
    verdict: String,
    changed_files: Vec<String>,
    expected_command: Vec<String>,
    missing_receipts: Vec<String>,
}

pub fn list() -> Result<()> {
    let root = project_root()?;
    let manifest = read_manifest(&root)?;

    for rule in manifest.generated {
        println!(
            "path={} owner={} command=\"{}\" allow_manual_edits={}",
            rule.path, rule.owner, rule.command, rule.allow_manual_edits
        );
    }

    Ok(())
}

pub fn check(
    receipt: Option<PathBuf>,
    fixture: Option<PathBuf>,
    allow_missing_receipt: bool,
) -> Result<()> {
    let root = project_root()?;
    let manifest = read_manifest(&root)?;
    let receipt_path = receipt.unwrap_or_else(|| root.join("target/receipts/generated-files.json"));

    let (changed_files, generator_receipts) = load_inputs(&root, fixture)?;

    let changed_generated = collect_changed_generated_files(&manifest.generated, &changed_files)?;
    let receipt_lookup: BTreeSet<(String, String)> = generator_receipts
        .into_iter()
        .map(|item| (item.owner, item.command))
        .collect();

    let mut missing_receipts = Vec::new();
    let mut expected_commands = BTreeSet::new();

    for (owner, command) in changed_generated.values() {
        expected_commands.insert(command.clone());
        if !receipt_lookup.contains(&(owner.clone(), command.clone())) {
            missing_receipts.push(format!("{owner}:{command}"));
        }
    }

    let verdict = if missing_receipts.is_empty() || allow_missing_receipt {
        "pass"
    } else {
        "fail"
    };

    let receipt_payload = CheckReceipt {
        verdict: verdict.to_string(),
        changed_files: changed_generated.keys().cloned().collect(),
        expected_command: expected_commands.into_iter().collect(),
        missing_receipts: missing_receipts.clone(),
    };

    if let Some(parent) = receipt_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating receipt directory {}", parent.display()))?;
    }

    let receipt_json =
        serde_json::to_string_pretty(&receipt_payload).context("serialize generated-file receipt")?;
    fs::write(&receipt_path, receipt_json)
        .with_context(|| format!("writing receipt {}", receipt_path.display()))?;

    if verdict == "fail" {
        bail!(
            "generated file ownership check failed: missing receipts for {}",
            missing_receipts.join(", ")
        );
    }

    println!("generated-file ownership check passed");
    Ok(())
}

fn collect_changed_generated_files(
    rules: &[GeneratedRule],
    changed_files: &[String],
) -> Result<BTreeMap<String, (String, String)>> {
    let mut matches = BTreeMap::new();

    for file in changed_files {
        for rule in rules {
            if rule.allow_manual_edits {
                continue;
            }
            let pattern = Pattern::new(&rule.path)
                .with_context(|| format!("invalid generated file glob pattern: {}", rule.path))?;
            if pattern.matches(file) {
                matches.insert(file.clone(), (rule.owner.clone(), rule.command.clone()));
            }
        }
    }

    Ok(matches)
}

fn load_inputs(root: &Path, fixture: Option<PathBuf>) -> Result<(Vec<String>, Vec<GeneratorReceipt>)> {
    if let Some(path) = fixture {
        let fixture_path = resolve_path(root, &path);
        let bytes = fs::read(&fixture_path)
            .with_context(|| format!("reading fixture {}", fixture_path.display()))?;
        let fixture_data: FixtureData = serde_json::from_slice(&bytes)
            .with_context(|| format!("parsing fixture {}", fixture_path.display()))?;
        return Ok((fixture_data.changed_files, fixture_data.generator_receipts));
    }

    let output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()
        .context("running git status --porcelain")?;

    if !output.status.success() {
        bail!("git status --porcelain exited with non-zero status");
    }

    let stdout = String::from_utf8(output.stdout).context("git output was not utf8")?;
    let mut changed_files = Vec::new();

    for line in stdout.lines() {
        if line.len() < 4 {
            continue;
        }
        let raw_path = &line[3..];
        if let Some((_, renamed_to)) = raw_path.split_once(" -> ") {
            changed_files.push(renamed_to.to_string());
        } else {
            changed_files.push(raw_path.to_string());
        }
    }

    let generator_receipts = collect_generator_receipts_from_changed_files(root, &changed_files)?;

    Ok((changed_files, generator_receipts))
}

fn resolve_path(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    root.join(path)
}

fn read_manifest(root: &Path) -> Result<GeneratedManifest> {
    let manifest_path = root.join(GENERATED_MANIFEST);
    let content = fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading {}", manifest_path.display()))?;
    let manifest: GeneratedManifest = toml::from_str(&content)
        .with_context(|| format!("parsing {}", manifest_path.display()))?;
    Ok(manifest)
}

fn collect_generator_receipts_from_changed_files(
    root: &Path,
    changed_files: &[String],
) -> Result<Vec<GeneratorReceipt>> {
    let mut receipts = Vec::new();
    for path in changed_files {
        if !path.ends_with(".json") {
            continue;
        }
        let file_path = root.join(path);
        if !file_path.exists() {
            continue;
        }
        let bytes = fs::read(&file_path)
            .with_context(|| format!("reading potential generator receipt {}", file_path.display()))?;
        let parsed: serde_json::Value = match serde_json::from_slice(&bytes) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if let Some(receipt) = parse_generator_receipt_value(&parsed) {
            receipts.push(receipt);
        }
    }
    Ok(receipts)
}

fn parse_generator_receipt_value(value: &serde_json::Value) -> Option<GeneratorReceipt> {
    let owner = value.get("owner")?.as_str()?.to_string();
    let command = value.get("command")?.as_str()?.to_string();
    Some(GeneratorReceipt { owner, command })
}
