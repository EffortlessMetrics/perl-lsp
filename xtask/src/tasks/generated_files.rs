use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::utils::project_root;

const MANIFEST_PATH: &str = ".ci/generated-files.toml";

#[derive(Debug, Clone, Deserialize)]
struct Manifest {
    generated: Vec<GeneratedRule>,
}

#[derive(Debug, Clone, Deserialize)]
struct GeneratedRule {
    path: String,
    command: String,
    owner: String,
    allow_manual_edits: bool,
}

#[derive(Debug, Deserialize)]
struct FixtureInput {
    changed_files: Vec<String>,
    #[serde(default)]
    generator_receipts: Vec<GeneratorReceipt>,
}

#[derive(Debug, Clone, Deserialize)]
struct GeneratorReceipt {
    owner: String,
    #[serde(default)]
    files: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CheckReceipt {
    verdict: String,
    changed_files: Vec<String>,
    expected_command: Option<String>,
    missing_receipts: Vec<String>,
}

pub fn list() -> Result<()> {
    let root = project_root()?;
    let manifest = load_manifest(&root)?;

    for entry in manifest.generated {
        println!(
            "path={} owner={} allow_manual_edits={} command={}",
            entry.path, entry.owner, entry.allow_manual_edits, entry.command
        );
    }

    Ok(())
}

pub fn check(receipt_path: PathBuf, fixture: Option<PathBuf>, allow_override: bool) -> Result<()> {
    let root = project_root()?;
    let manifest = load_manifest(&root)?;

    let (changed_files, generator_receipts) = if let Some(fixture_path) = fixture {
        let raw = fs::read_to_string(&fixture_path)
            .with_context(|| format!("failed to read fixture {}", fixture_path.display()))?;
        let fixture: FixtureInput = serde_json::from_str(&raw)
            .with_context(|| format!("invalid fixture JSON {}", fixture_path.display()))?;
        (fixture.changed_files, fixture.generator_receipts)
    } else {
        (collect_changed_files(&root)?, Vec::new())
    };

    let mut generated_changed = Vec::new();
    let mut missing_receipts = Vec::new();
    let mut expected_command = None;

    for file in &changed_files {
        if let Some(rule) = manifest
            .generated
            .iter()
            .find(|rule| !rule.allow_manual_edits && matches_glob(&rule.path, file))
        {
            generated_changed.push(file.clone());

            let has_receipt = generator_receipts.iter().any(|receipt| {
                receipt.owner == rule.owner && receipt.files.iter().any(|f| f == file)
            });

            if !has_receipt {
                missing_receipts.push(file.clone());
                if expected_command.is_none() {
                    expected_command = Some(rule.command.clone());
                }
            }
        }
    }

    generated_changed.sort();
    generated_changed.dedup();
    missing_receipts.sort();
    missing_receipts.dedup();

    let verdict = if missing_receipts.is_empty() || allow_override { "pass" } else { "fail" };

    let receipt = CheckReceipt {
        verdict: verdict.to_string(),
        changed_files: generated_changed,
        expected_command,
        missing_receipts,
    };

    write_receipt(&receipt_path, &receipt)?;

    if receipt.verdict == "fail" {
        bail!(
            "generated-files check failed: missing generator receipt for {} file(s)",
            receipt.missing_receipts.len()
        );
    }

    Ok(())
}

fn load_manifest(root: &Path) -> Result<Manifest> {
    let manifest_path = root.join(MANIFEST_PATH);
    let raw = fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read {}", manifest_path.display()))?;
    toml::from_str(&raw).with_context(|| format!("invalid TOML in {}", manifest_path.display()))
}

fn collect_changed_files(root: &Path) -> Result<Vec<String>> {
    let diff = std::process::Command::new("git")
        .args(["diff", "--name-only", "HEAD"])
        .current_dir(root)
        .output()
        .context("failed to run git diff --name-only HEAD")?;

    if !diff.status.success() {
        bail!("git diff --name-only HEAD failed with status {}", diff.status);
    }

    let mut files: BTreeSet<String> = String::from_utf8(diff.stdout)
        .context("git diff output was not valid UTF-8")?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(ToOwned::to_owned)
        .collect();

    let untracked = std::process::Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard"])
        .current_dir(root)
        .output()
        .context("failed to run git ls-files --others --exclude-standard")?;

    if !untracked.status.success() {
        bail!("git ls-files --others --exclude-standard failed with status {}", untracked.status);
    }

    for line in String::from_utf8(untracked.stdout)
        .context("git ls-files output was not valid UTF-8")?
        .lines()
    {
        if !line.trim().is_empty() {
            files.insert(line.to_string());
        }
    }

    Ok(files.into_iter().collect())
}

fn write_receipt(path: &Path, receipt: &CheckReceipt) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }

    let payload = serde_json::to_string_pretty(receipt).context("failed to serialize receipt")?;
    fs::write(path, payload).with_context(|| format!("failed to write receipt {}", path.display()))
}

fn matches_glob(pattern: &str, candidate: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return candidate == prefix || candidate.starts_with(&format!("{prefix}/"));
    }

    pattern == candidate
}

#[cfg(test)]
mod tests {
    use super::matches_glob;

    #[test]
    fn supports_double_star_directory_pattern() {
        assert!(matches_glob("docs/project/status/**", "docs/project/status/parser.md"));
        assert!(!matches_glob("docs/project/status/**", "docs/project/other.md"));
    }
}
