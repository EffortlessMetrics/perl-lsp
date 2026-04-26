use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::tasks::ci_scope::{self, ScopeOutput};

const SENSITIVE_PATH_PREFIXES: &[&str] = &[
    "xtask/src/tasks/ci_scope.rs",
    "xtask/src/tasks/gates.rs",
    ".ci/scope.d/",
    ".ci/gates.d/",
    ".github/workflows/",
    ".ci/parser-ratchet/",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeMetaGateConfig {
    pub base: Option<String>,
    pub head: Option<String>,
    pub receipt: Option<PathBuf>,
    pub fixture: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeMetaGateReceipt {
    pub schema_version: u32,
    pub old_decision: LaneDecision,
    pub new_decision: LaneDecision,
    pub changed_lanes: ChangedLanes,
    pub verdict: String,
    pub advisory: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneDecision {
    pub selected_lanes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedLanes {
    pub removed: Vec<String>,
    pub added: Vec<String>,
    pub unchanged: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FixtureInput {
    old_decision: LaneDecision,
    new_decision: LaneDecision,
}

pub fn run(config: ScopeMetaGateConfig) -> Result<()> {
    let receipt = if let Some(fixture) = &config.fixture {
        run_fixture(fixture)?
    } else {
        run_from_git(&config)?
    };

    if let Some(path) = &config.receipt {
        write_receipt(path, &receipt)?;
    }

    println!("{}", serde_json::to_string_pretty(&receipt)?);

    if receipt.verdict == "fail" {
        bail!("scope-meta-gate failed: one or more lanes became unselected");
    }

    Ok(())
}

fn run_fixture(path: &Path) -> Result<ScopeMetaGateReceipt> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read fixture {}", path.display()))?;
    let fixture: FixtureInput = serde_json::from_str(&raw)
        .with_context(|| format!("Failed to parse fixture {}", path.display()))?;
    Ok(build_receipt(fixture.old_decision, fixture.new_decision))
}

fn run_from_git(config: &ScopeMetaGateConfig) -> Result<ScopeMetaGateReceipt> {
    let root = crate::utils::project_root()?;
    let base = config.base.as_deref().unwrap_or("HEAD~1");
    let head = config.head.as_deref().unwrap_or("HEAD");

    let touched_paths = changed_files_between(base, head, &root)?;
    if !select_scope_meta_gate(&touched_paths) {
        return Ok(ScopeMetaGateReceipt {
            schema_version: 1,
            old_decision: LaneDecision { selected_lanes: vec![] },
            new_decision: LaneDecision { selected_lanes: vec![] },
            changed_lanes: ChangedLanes { removed: vec![], added: vec![], unchanged: vec![] },
            verdict: "pass".to_string(),
            advisory: vec!["scope-meta-gate not selected for this diff".to_string()],
        });
    }

    let metadata = load_metadata(&root)?;
    let workspace_root = root.to_string_lossy().replace('\\', "/");

    let old_files = changed_files_between(&format!("{base}^"), base, &root)?;
    let new_files = changed_files_between(base, head, &root)?;

    let old_scope = ci_scope::classify_files(&old_files, &metadata, &workspace_root)?;
    let new_scope = ci_scope::classify_files(&new_files, &metadata, &workspace_root)?;

    let old_decision = LaneDecision { selected_lanes: lanes_from_scope(&old_scope) };
    let new_decision = LaneDecision { selected_lanes: lanes_from_scope(&new_scope) };

    Ok(build_receipt(old_decision, new_decision))
}

fn lanes_from_scope(scope: &ScopeOutput) -> Vec<String> {
    let mut lanes: BTreeSet<String> = BTreeSet::new();
    for lane in &scope.selected_lanes {
        lanes.insert(lane.lane.clone());
    }
    for lane in &scope.selected_heavy_lanes {
        lanes.insert(lane.lane.clone());
    }
    lanes.into_iter().collect()
}

fn build_receipt(old_decision: LaneDecision, new_decision: LaneDecision) -> ScopeMetaGateReceipt {
    let old: BTreeSet<String> = old_decision.selected_lanes.iter().cloned().collect();
    let new: BTreeSet<String> = new_decision.selected_lanes.iter().cloned().collect();

    let removed = old.difference(&new).cloned().collect::<Vec<_>>();
    let added = new.difference(&old).cloned().collect::<Vec<_>>();
    let unchanged = new.intersection(&old).cloned().collect::<Vec<_>>();

    let (verdict, advisory) = if !removed.is_empty() {
        (
            "fail".to_string(),
            vec![format!(
                "Lane selection regressed; affected lanes must run explicitly: {}",
                removed.join(", ")
            )],
        )
    } else if !added.is_empty() {
        (
            "warn".to_string(),
            vec!["Scope expanded safely; additional lanes are selected".to_string()],
        )
    } else {
        ("pass".to_string(), vec!["No lane-selection regression detected".to_string()])
    };

    ScopeMetaGateReceipt {
        schema_version: 1,
        old_decision,
        new_decision,
        changed_lanes: ChangedLanes { removed, added, unchanged },
        verdict,
        advisory,
    }
}

fn select_scope_meta_gate(changed_files: &[String]) -> bool {
    changed_files.iter().any(|path| {
        SENSITIVE_PATH_PREFIXES.iter().any(|prefix| path == *prefix || path.starts_with(prefix))
    })
}

fn changed_files_between(base: &str, head: &str, root: &Path) -> Result<Vec<String>> {
    let diff_spec = format!("{base}...{head}");
    let output = duct::cmd("git", ["diff", "--name-only", &diff_spec])
        .dir(root)
        .stdout_capture()
        .stderr_capture()
        .unchecked()
        .run()
        .context("Failed to run git diff for scope-meta-gate")?;

    let stdout = if output.status.success() {
        String::from_utf8(output.stdout).context("git diff output was not valid UTF-8")?
    } else {
        let diff_spec_two = format!("{base}..{head}");
        let output2 = duct::cmd("git", ["diff", "--name-only", &diff_spec_two])
            .dir(root)
            .stdout_capture()
            .stderr_capture()
            .run()
            .context("Failed to run git diff two-dot fallback for scope-meta-gate")?;
        String::from_utf8(output2.stdout).context("git diff output was not valid UTF-8")?
    };

    Ok(stdout.lines().map(ToOwned::to_owned).collect())
}

fn load_metadata(root: &Path) -> Result<serde_json::Value> {
    let output = duct::cmd("cargo", ["metadata", "--format-version", "1"])
        .dir(root)
        .stdout_capture()
        .stderr_capture()
        .run()
        .context("Failed to run cargo metadata")?;

    let stdout =
        String::from_utf8(output.stdout).context("cargo metadata output was not valid UTF-8")?;
    serde_json::from_str(&stdout).context("Failed to parse cargo metadata JSON")
}

fn write_receipt(path: &Path, receipt: &ScopeMetaGateReceipt) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create receipt directory {}", parent.display()))?;
    }
    let payload = serde_json::to_string_pretty(receipt).context("Failed to serialize receipt")?;
    fs::write(path, format!("{payload}\n"))
        .with_context(|| format!("Failed to write receipt {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removed_lane_fails() {
        let old = LaneDecision {
            selected_lanes: vec!["parser_ratchet".to_string(), "test_scoped".to_string()],
        };
        let new = LaneDecision { selected_lanes: vec!["test_scoped".to_string()] };
        let receipt = build_receipt(old, new);
        assert_eq!(receipt.verdict, "fail");
        assert_eq!(receipt.changed_lanes.removed, vec!["parser_ratchet".to_string()]);
    }

    #[test]
    fn added_lane_warns() {
        let old = LaneDecision { selected_lanes: vec!["test_scoped".to_string()] };
        let new =
            LaneDecision { selected_lanes: vec!["test_scoped".to_string(), "docs".to_string()] };
        let receipt = build_receipt(old, new);
        assert_eq!(receipt.verdict, "warn");
        assert_eq!(receipt.changed_lanes.added, vec!["docs".to_string()]);
    }

    #[test]
    fn fixture_parser_rule_removed_fails() -> Result<()> {
        let receipt =
            run_fixture(Path::new("tests/fixtures/scope-meta-gate/parser-lane-removed.json"))?;
        assert_eq!(receipt.verdict, "fail");
        Ok(())
    }

    #[test]
    fn fixture_docs_expands_warns_or_passes() -> Result<()> {
        let receipt =
            run_fixture(Path::new("tests/fixtures/scope-meta-gate/docs-scope-expanded.json"))?;
        assert!(receipt.verdict == "warn" || receipt.verdict == "pass");
        Ok(())
    }

    #[test]
    fn selector_matches_sensitive_paths() {
        let files = vec![".github/workflows/ci.yml".to_string()];
        assert!(select_scope_meta_gate(&files));
    }
}
