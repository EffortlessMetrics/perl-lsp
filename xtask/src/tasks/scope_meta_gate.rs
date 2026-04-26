//! Scope meta-gate (`cargo xtask scope-meta-gate`).
//!
//! Protects CI gate selection logic against accidental (or silent) narrowing.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Context, Result, bail};
use duct::cmd;
use serde::{Deserialize, Serialize};

use crate::utils::project_root;

const META_TRIGGER_PATHS: &[&str] = &[
    "xtask/src/tasks/ci_scope.rs",
    "xtask/src/tasks/gates.rs",
    ".ci/scope.d/",
    ".ci/gates.d/",
    ".github/workflows/",
    ".ci/parser-ratchet/",
];

const PROTECTED_LANES: &[&str] = &["parser_ratchet", "clippy_scoped", "test_scoped", "security"];

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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeMetaGateReceipt {
    pub schema_version: u32,
    pub mode: String,
    pub old_decision: ScopeDecision,
    pub new_decision: ScopeDecision,
    pub changed_lanes: Vec<String>,
    pub verdict: Verdict,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Fixture {
    old_decision: ScopeDecision,
    new_decision: ScopeDecision,
}

pub fn run(config: ScopeMetaGateConfig) -> Result<()> {
    let receipt = if let Some(fixture_path) = config.fixture {
        run_from_fixture(&fixture_path)?
    } else {
        let Some(base) = config.base.as_deref() else {
            bail!("--base is required when --fixture is not provided");
        };
        let Some(head) = config.head.as_deref() else {
            bail!("--head is required when --fixture is not provided");
        };
        run_from_shas(base, head)?
    };

    if let Some(parent) = config.receipt.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating receipt directory: {}", parent.display()))?;
    }
    let json =
        serde_json::to_string_pretty(&receipt).context("serializing scope-meta-gate receipt")?;
    fs::write(&config.receipt, json)
        .with_context(|| format!("writing receipt: {}", config.receipt.display()))?;

    println!("scope-meta-gate verdict: {:?}", receipt.verdict);
    println!("receipt: {}", config.receipt.display());

    if receipt.verdict == Verdict::Fail {
        bail!(receipt.message);
    }

    Ok(())
}

fn run_from_fixture(path: &Path) -> Result<ScopeMetaGateReceipt> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("reading fixture: {}", path.display()))?;
    let fixture: Fixture = serde_json::from_str(&raw)
        .with_context(|| format!("parsing fixture JSON: {}", path.display()))?;
    Ok(build_receipt("fixture", fixture.old_decision, fixture.new_decision))
}

fn run_from_shas(base: &str, head: &str) -> Result<ScopeMetaGateReceipt> {
    let root = project_root()?;
    let changed_files = git_diff_files(base, head, &root)?;

    if !changed_files.iter().any(|path| is_meta_trigger_path(path)) {
        let empty = ScopeDecision { selected_lanes: Vec::new() };
        return Ok(ScopeMetaGateReceipt {
            schema_version: 1,
            mode: "sha".to_string(),
            old_decision: empty.clone(),
            new_decision: empty,
            changed_lanes: Vec::new(),
            verdict: Verdict::Pass,
            message: "No scope-meta trigger paths changed.".to_string(),
        });
    }

    let old_decision = decision_from_git_snapshot(base, &root)?;
    let new_decision = decision_from_git_snapshot(head, &root)?;
    Ok(build_receipt("sha", old_decision, new_decision))
}

fn build_receipt(
    mode: &str,
    old_decision: ScopeDecision,
    new_decision: ScopeDecision,
) -> ScopeMetaGateReceipt {
    let old: BTreeSet<String> = old_decision.selected_lanes.iter().cloned().collect();
    let new: BTreeSet<String> = new_decision.selected_lanes.iter().cloned().collect();

    let removed: Vec<String> = old.difference(&new).cloned().collect();
    let added: Vec<String> = new.difference(&old).cloned().collect();

    if !removed.is_empty() {
        return ScopeMetaGateReceipt {
            schema_version: 1,
            mode: mode.to_string(),
            old_decision,
            new_decision,
            changed_lanes: removed,
            verdict: Verdict::Fail,
            message: "Selected lanes were narrowed; run affected lanes explicitly or restore scope logic."
                .to_string(),
        };
    }

    if !added.is_empty() {
        return ScopeMetaGateReceipt {
            schema_version: 1,
            mode: mode.to_string(),
            old_decision,
            new_decision,
            changed_lanes: added,
            verdict: Verdict::Warn,
            message: "Scope expanded; review CI spend impact.".to_string(),
        };
    }

    ScopeMetaGateReceipt {
        schema_version: 1,
        mode: mode.to_string(),
        old_decision,
        new_decision,
        changed_lanes: Vec::new(),
        verdict: Verdict::Pass,
        message: "No lane selection narrowing detected.".to_string(),
    }
}

fn decision_from_git_snapshot(sha: &str, root: &Path) -> Result<ScopeDecision> {
    let mut lanes: BTreeSet<String> = BTreeSet::new();

    for path in META_TRIGGER_PATHS {
        if path.ends_with('/') {
            continue;
        }
        if let Ok(content) = git_show(sha, path, root) {
            for lane in lanes_from_content(&content) {
                lanes.insert(lane);
            }
        }
    }

    for lane in PROTECTED_LANES {
        lanes.insert((*lane).to_string());
    }

    Ok(ScopeDecision { selected_lanes: lanes.into_iter().collect() })
}

fn lanes_from_content(content: &str) -> BTreeSet<String> {
    let mut lanes = BTreeSet::new();
    let lower = content.to_lowercase();

    if lower.contains("parser") || lower.contains("ratchet") {
        lanes.insert("parser_ratchet".to_string());
    }
    if lower.contains("clippy") {
        lanes.insert("clippy_scoped".to_string());
    }
    if lower.contains("test") {
        lanes.insert("test_scoped".to_string());
    }
    if lower.contains("security") {
        lanes.insert("security".to_string());
    }

    lanes
}

fn git_show(sha: &str, path: &str, root: &Path) -> Result<String> {
    let object = format!("{sha}:{path}");
    let output = cmd("git", ["show", &object]).dir(root).stderr_null().unchecked().run()?;
    if !output.status.success() {
        bail!("git show failed for {object}");
    }
    String::from_utf8(output.stdout).context("git show output was not UTF-8")
}

fn git_diff_files(base: &str, head: &str, root: &Path) -> Result<Vec<String>> {
    let range = format!("{base}...{head}");
    let output = cmd("git", ["diff", "--name-only", &range]).dir(root).read()?;
    Ok(output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect())
}

fn is_meta_trigger_path(path: &str) -> bool {
    META_TRIGGER_PATHS
        .iter()
        .any(|trigger| path == *trigger || (trigger.ends_with('/') && path.starts_with(trigger)))
}

#[cfg(test)]
mod tests {
    use super::{ScopeDecision, Verdict, build_receipt, is_meta_trigger_path};

    #[test]
    fn narrowing_fails() {
        let old_decision = ScopeDecision {
            selected_lanes: vec!["parser_ratchet".to_string(), "clippy_scoped".to_string()],
        };
        let new_decision = ScopeDecision { selected_lanes: vec!["clippy_scoped".to_string()] };

        let receipt = build_receipt("fixture", old_decision, new_decision);
        assert_eq!(receipt.verdict, Verdict::Fail);
        assert_eq!(receipt.changed_lanes, vec!["parser_ratchet".to_string()]);
    }

    #[test]
    fn expansion_warns() {
        let old_decision = ScopeDecision { selected_lanes: vec!["clippy_scoped".to_string()] };
        let new_decision = ScopeDecision {
            selected_lanes: vec!["clippy_scoped".to_string(), "test_scoped".to_string()],
        };

        let receipt = build_receipt("fixture", old_decision, new_decision);
        assert_eq!(receipt.verdict, Verdict::Warn);
        assert_eq!(receipt.changed_lanes, vec!["test_scoped".to_string()]);
    }

    #[test]
    fn trigger_path_match_works() {
        assert!(is_meta_trigger_path("xtask/src/tasks/ci_scope.rs"));
        assert!(is_meta_trigger_path(".github/workflows/ci.yml"));
        assert!(!is_meta_trigger_path("docs/README.md"));
    }
}
