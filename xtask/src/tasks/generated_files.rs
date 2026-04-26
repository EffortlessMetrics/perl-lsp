use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};

const DEFAULT_MANIFEST: &str = ".ci/generated-files.toml";

#[derive(Debug, Deserialize)]
struct GeneratedManifest {
    generated: Vec<GeneratedRule>,
}

#[derive(Debug, Deserialize, Clone)]
struct GeneratedRule {
    path: String,
    command: String,
    owner: String,
    #[serde(default)]
    allow_manual_edits: bool,
}

#[derive(Debug, Serialize)]
struct CheckReceipt {
    schema_version: u32,
    verdict: String,
    changed_files: Vec<String>,
    expected_command: Vec<String>,
    missing_receipts: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct FixtureInput {
    changed_files: Vec<String>,
    #[serde(default)]
    receipt_owners: Vec<String>,
}

pub fn check(
    manifest_path: Option<PathBuf>,
    receipt_path: PathBuf,
    allow_manual_edits_override: bool,
    fixture: Option<PathBuf>,
) -> Result<()> {
    let root = crate::utils::project_root()?;
    let manifest_path = manifest_path.unwrap_or_else(|| root.join(DEFAULT_MANIFEST));
    let manifest = read_manifest(&manifest_path)?;

    let (changed_files, receipt_owners) = if let Some(fixture_path) = fixture {
        let fixture = read_fixture(&fixture_path)?;
        (fixture.changed_files, fixture.receipt_owners.into_iter().collect::<BTreeSet<_>>())
    } else {
        (collect_git_changed_files()?, BTreeSet::new())
    };

    let mut missing_receipts = BTreeSet::new();
    let mut expected_commands = BTreeSet::new();

    for changed_file in &changed_files {
        for rule in &manifest.generated {
            if !path_matches_rule(changed_file, &rule.path) {
                continue;
            }
            if allow_manual_edits_override || rule.allow_manual_edits {
                continue;
            }
            if !receipt_owners.contains(&rule.owner) {
                missing_receipts.insert(rule.owner.clone());
                expected_commands.insert(rule.command.clone());
            }
        }
    }

    let verdict = if missing_receipts.is_empty() { "pass" } else { "fail" };
    let receipt = CheckReceipt {
        schema_version: 1,
        verdict: verdict.to_string(),
        changed_files,
        expected_command: expected_commands.into_iter().collect(),
        missing_receipts: missing_receipts.into_iter().collect(),
    };

    if let Some(parent) = receipt_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating receipt directory {}", parent.display()))?;
    }
    let payload =
        serde_json::to_string_pretty(&receipt).context("serializing generated file receipt")?;
    fs::write(&receipt_path, format!("{payload}\n"))
        .with_context(|| format!("writing {}", receipt_path.display()))?;

    if receipt.verdict == "fail" {
        bail!(
            "generated-file ownership check failed: missing receipts for owners [{}]",
            receipt.missing_receipts.join(", ")
        );
    }

    Ok(())
}

pub fn list(manifest_path: Option<PathBuf>) -> Result<()> {
    let root = crate::utils::project_root()?;
    let manifest_path = manifest_path.unwrap_or_else(|| root.join(DEFAULT_MANIFEST));
    let manifest = read_manifest(&manifest_path)?;

    for rule in manifest.generated {
        println!(
            "{}\t{}\t{}\tallow_manual_edits={}",
            rule.owner, rule.path, rule.command, rule.allow_manual_edits
        );
    }

    Ok(())
}

fn read_manifest(path: &Path) -> Result<GeneratedManifest> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("reading manifest {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("parsing manifest {}", path.display()))
}

fn read_fixture(path: &Path) -> Result<FixtureInput> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("reading fixture {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parsing fixture {}", path.display()))
}

fn collect_git_changed_files() -> Result<Vec<String>> {
    let mut changed = BTreeSet::new();

    collect_from_command(&["diff", "--name-only"])?.into_iter().for_each(|path| {
        changed.insert(path);
    });
    collect_from_command(&["diff", "--name-only", "--cached"])?.into_iter().for_each(|path| {
        changed.insert(path);
    });
    collect_from_command(&["ls-files", "--others", "--exclude-standard"])?.into_iter().for_each(
        |path| {
            changed.insert(path);
        },
    );

    Ok(changed.into_iter().collect())
}

fn collect_from_command(args: &[&str]) -> Result<Vec<String>> {
    let output = std::process::Command::new("git")
        .args(args)
        .output()
        .with_context(|| format!("running git {}", args.join(" ")))?;
    if !output.status.success() {
        bail!("git {} failed: {}", args.join(" "), String::from_utf8_lossy(&output.stderr));
    }

    let stdout = String::from_utf8(output.stdout).context("git output was not UTF-8")?;
    Ok(stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn path_matches_rule(file_path: &str, rule_path: &str) -> bool {
    if let Some(prefix) = rule_path.strip_suffix("**") {
        return file_path.starts_with(prefix);
    }

    file_path == rule_path
}

#[cfg(test)]
mod tests {
    use super::path_matches_rule;

    #[test]
    fn matches_recursive_prefix_rules() {
        assert!(path_matches_rule("docs/project/status/parser.md", "docs/project/status/**"));
    }

    #[test]
    fn does_not_match_outside_prefix() {
        assert!(!path_matches_rule("docs/project/ROADMAP.md", "docs/project/status/**"));
    }
}
