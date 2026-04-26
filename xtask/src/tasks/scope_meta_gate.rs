//! Scope meta-gate: protects CI gate-selection logic from silent regressions.
//!
//! `scope-meta-gate` compares an `old_decision` and `new_decision` and fails
//! when lanes are dropped by scope-logic changes in protected files.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Context, Result, bail};
use duct::cmd;
use serde::{Deserialize, Serialize};

use crate::tasks::ci_scope::{self, ScopeOutput};

const META_SCOPE_PREFIXES: &[&str] = &[
    "xtask/src/tasks/ci_scope.rs",
    "xtask/src/tasks/gates.rs",
    ".ci/scope.d/",
    ".ci/gates.d/",
    ".github/workflows/",
    ".ci/parser-ratchet/",
];

#[derive(Debug, Clone)]
pub struct ScopeMetaGateConfig {
    pub base: Option<String>,
    pub head: Option<String>,
    pub fixture: Option<PathBuf>,
    pub receipt: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeDecision {
    pub selected_lanes: Vec<String>,
    #[serde(default)]
    pub selected_heavy_lanes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FixtureInput {
    #[serde(default)]
    changed_files: Vec<String>,
    old_decision: ScopeDecision,
    new_decision: ScopeDecision,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeMetaGateReceipt {
    pub schema_version: u32,
    pub base: Option<String>,
    pub head: Option<String>,
    pub changed_files: Vec<String>,
    pub old_decision: ScopeDecision,
    pub new_decision: ScopeDecision,
    pub changed_lanes: ChangedLanes,
    pub verdict: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedLanes {
    pub removed: Vec<String>,
    pub added: Vec<String>,
    pub unchanged: Vec<String>,
}

pub fn run(config: ScopeMetaGateConfig) -> Result<()> {
    let (base, head, changed_files, old_decision, new_decision) =
        load_decisions(&config).context("Failed to load scope decisions")?;

    let changed_lanes = diff_lanes(&old_decision, &new_decision);
    let meta_touched = touches_meta_scope_files(&changed_files);

    let (verdict, reason) = if !meta_touched {
        ("pass".to_string(), "No protected scope-meta files changed".to_string())
    } else if !changed_lanes.removed.is_empty() {
        (
            "fail".to_string(),
            format!(
                "Protected scope logic changed and lane(s) were removed: {}",
                changed_lanes.removed.join(", ")
            ),
        )
    } else if !changed_lanes.added.is_empty() {
        (
            "warn".to_string(),
            format!(
                "Protected scope logic changed and lane coverage expanded: {}",
                changed_lanes.added.join(", ")
            ),
        )
    } else {
        (
            "pass".to_string(),
            "Protected scope logic changed but lane selection is unchanged".to_string(),
        )
    };

    let receipt = ScopeMetaGateReceipt {
        schema_version: 1,
        base,
        head,
        changed_files,
        old_decision,
        new_decision,
        changed_lanes,
        verdict: verdict.clone(),
        reason,
    };

    if let Some(parent) = config.receipt.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed creating receipt dir {}", parent.display()))?;
    }

    let json = serde_json::to_string_pretty(&receipt)?;
    fs::write(&config.receipt, json)
        .with_context(|| format!("Failed writing receipt to {}", config.receipt.display()))?;

    println!("scope-meta-gate verdict: {}", receipt.verdict);
    println!("receipt: {}", config.receipt.display());

    if verdict == "fail" {
        bail!("scope-meta-gate failed: lane selection narrowed in protected scope files");
    }

    Ok(())
}

fn load_decisions(
    config: &ScopeMetaGateConfig,
) -> Result<(Option<String>, Option<String>, Vec<String>, ScopeDecision, ScopeDecision)> {
    if let Some(fixture) = &config.fixture {
        let raw = fs::read_to_string(fixture)
            .with_context(|| format!("Failed reading fixture {}", fixture.display()))?;
        let parsed: FixtureInput = serde_json::from_str(&raw)
            .with_context(|| format!("Fixture JSON is invalid: {}", fixture.display()))?;

        return Ok((None, None, parsed.changed_files, parsed.old_decision, parsed.new_decision));
    }

    let (base, head) = match (&config.base, &config.head) {
        (Some(base), Some(head)) => (base.clone(), head.clone()),
        _ => bail!("Provide either --fixture <path> OR both --base <sha> --head <sha>"),
    };

    let root = crate::utils::project_root()?;
    let changed_files = changed_files_between(&root, &base, &head)?;

    let metadata = load_metadata(&root)?;
    let workspace_root = root.to_string_lossy().replace('\\', "/");

    // `old_decision` is evaluated against non-meta paths to represent prior
    // lane intent. `new_decision` includes all changed files under new logic.
    let non_meta_files: Vec<String> =
        changed_files.iter().filter(|f| !is_meta_scope_file(f)).cloned().collect();

    let old_scope = ci_scope::classify_files(&non_meta_files, &metadata, &workspace_root)?;
    let new_scope = ci_scope::classify_files(&changed_files, &metadata, &workspace_root)?;

    Ok((
        Some(base),
        Some(head),
        changed_files,
        scope_to_decision(&old_scope),
        scope_to_decision(&new_scope),
    ))
}

fn scope_to_decision(scope: &ScopeOutput) -> ScopeDecision {
    ScopeDecision {
        selected_lanes: scope.selected_lanes.iter().map(|lane| lane.lane.clone()).collect(),
        selected_heavy_lanes: scope
            .selected_heavy_lanes
            .iter()
            .map(|lane| lane.lane.clone())
            .collect(),
    }
}

fn diff_lanes(old_decision: &ScopeDecision, new_decision: &ScopeDecision) -> ChangedLanes {
    let old: BTreeSet<String> = old_decision
        .selected_lanes
        .iter()
        .chain(old_decision.selected_heavy_lanes.iter())
        .cloned()
        .collect();
    let new: BTreeSet<String> = new_decision
        .selected_lanes
        .iter()
        .chain(new_decision.selected_heavy_lanes.iter())
        .cloned()
        .collect();

    let removed: Vec<String> = old.difference(&new).cloned().collect();
    let added: Vec<String> = new.difference(&old).cloned().collect();
    let unchanged: Vec<String> = old.intersection(&new).cloned().collect();

    ChangedLanes { removed, added, unchanged }
}

fn touches_meta_scope_files(files: &[String]) -> bool {
    files.iter().any(|f| is_meta_scope_file(f))
}

fn is_meta_scope_file(path: &str) -> bool {
    META_SCOPE_PREFIXES.iter().any(|prefix| path == *prefix || path.starts_with(prefix))
}

fn changed_files_between(root: &Path, base: &str, head: &str) -> Result<Vec<String>> {
    let diff_spec = format!("{base}...{head}");
    let output = cmd("git", ["diff", "--name-only", &diff_spec])
        .dir(root)
        .stdout_capture()
        .stderr_capture()
        .unchecked()
        .run()
        .context("Failed to run git diff")?;

    let stdout = if output.status.success() {
        String::from_utf8(output.stdout).context("git diff output was not valid UTF-8")?
    } else {
        let fallback_diff = format!("{base}..{head}");
        let fallback = cmd("git", ["diff", "--name-only", &fallback_diff])
            .dir(root)
            .stdout_capture()
            .stderr_capture()
            .run()
            .context("Failed to run fallback git diff")?;
        String::from_utf8(fallback.stdout)
            .context("fallback git diff output was not valid UTF-8")?
    };

    Ok(stdout.lines().map(ToString::to_string).collect())
}

fn load_metadata(root: &Path) -> Result<serde_json::Value> {
    let output = cmd("cargo", ["metadata", "--format-version", "1"])
        .dir(root)
        .stdout_capture()
        .stderr_capture()
        .run()
        .context("Failed to run cargo metadata")?;

    let stdout =
        String::from_utf8(output.stdout).context("cargo metadata output was not valid UTF-8")?;
    serde_json::from_str(&stdout).context("Failed to parse cargo metadata JSON")
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::Result;

    #[test]
    fn lane_diff_detects_removed_lanes() {
        let old = ScopeDecision {
            selected_lanes: vec!["parser_ratchet".to_string(), "test_scoped".to_string()],
            selected_heavy_lanes: vec![],
        };
        let new = ScopeDecision {
            selected_lanes: vec!["test_scoped".to_string()],
            selected_heavy_lanes: vec![],
        };

        let diff = diff_lanes(&old, &new);
        assert_eq!(diff.removed, vec!["parser_ratchet".to_string()]);
        assert!(diff.added.is_empty());
    }

    #[test]
    fn meta_path_matcher_handles_prefixes() {
        assert!(is_meta_scope_file("xtask/src/tasks/ci_scope.rs"));
        assert!(is_meta_scope_file(".ci/scope.d/parser.yaml"));
        assert!(is_meta_scope_file(".github/workflows/pr-smoke.yml"));
        assert!(!is_meta_scope_file("crates/perl-parser/src/lib.rs"));
    }

    #[test]
    fn fixture_roundtrip_parses() -> Result<()> {
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/scope-meta-gate/remove-parser-lane-fail.json");
        let raw = fs::read_to_string(&fixture_path)?;
        let fixture: FixtureInput = serde_json::from_str(&raw)?;
        assert!(fixture.changed_files.iter().any(|f| f == "xtask/src/tasks/ci_scope.rs"));
        assert!(fixture.old_decision.selected_lanes.iter().any(|l| l == "parser_ratchet"));
        Ok(())
    }
}
