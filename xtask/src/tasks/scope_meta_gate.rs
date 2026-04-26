use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Context, Result, bail};
use duct::cmd;
use serde::{Deserialize, Serialize};

use crate::tasks::ci_scope::{self, ScopeOutput};
use crate::utils::project_root;

const TRIGGER_PATH_PREFIXES: &[&str] = &[
    "xtask/src/tasks/ci_scope.rs",
    "xtask/src/tasks/gates.rs",
    ".ci/scope.d/",
    ".ci/gates.d/",
    ".github/workflows/",
    ".ci/parser-ratchet/",
];

#[derive(Debug)]
pub struct ScopeMetaGateConfig {
    pub base: Option<String>,
    pub head: Option<String>,
    pub receipt: Option<PathBuf>,
    pub fixture: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeMetaGateReceipt {
    pub schema_version: u32,
    pub mode: String,
    pub old_decision: ScopeOutput,
    pub new_decision: ScopeOutput,
    pub changed_lanes: ChangedLanes,
    pub verdict: Verdict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedLanes {
    pub dropped: Vec<String>,
    pub added: Vec<String>,
    pub unchanged: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verdict {
    pub status: String,
    pub reason: String,
    pub required_lanes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct FixtureDoc {
    old_decision: ScopeOutput,
    new_decision: ScopeOutput,
}

pub fn run(config: ScopeMetaGateConfig) -> Result<()> {
    let root = project_root()?;

    let (old_decision, new_decision, mode) = if let Some(fixture_path) = config.fixture.as_ref() {
        let fixture = load_fixture(fixture_path)?;
        (fixture.old_decision, fixture.new_decision, "fixture".to_string())
    } else {
        let base = config.base.clone().ok_or_else(|| {
            color_eyre::eyre::eyre!("--base is required when --fixture is not provided")
        })?;
        let head = config.head.clone().ok_or_else(|| {
            color_eyre::eyre::eyre!("--head is required when --fixture is not provided")
        })?;
        let old = compute_decision_for_sha(&root, &base, &head)?;
        let new = compute_decision_for_sha(&root, &head, &head)?;
        (old, new, "git".to_string())
    };

    let changed_lanes = compare_lanes(&old_decision, &new_decision);
    let verdict = evaluate_verdict(&old_decision, &new_decision, &changed_lanes);

    let receipt = ScopeMetaGateReceipt {
        schema_version: 1,
        mode,
        old_decision,
        new_decision,
        changed_lanes,
        verdict,
    };

    let receipt_json = serde_json::to_string_pretty(&receipt)
        .context("Failed to serialize scope-meta-gate receipt")?;
    validate_receipt_registry_if_present(&root, &receipt)?;

    if let Some(path) = config.receipt.as_ref() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed creating receipt dir {}", parent.display()))?;
        }
        fs::write(path, &receipt_json)
            .with_context(|| format!("Failed writing receipt {}", path.display()))?;
    }

    println!("{receipt_json}");

    if receipt.verdict.status == "fail" {
        bail!("scope-meta-gate failed: {}", receipt.verdict.reason);
    }

    Ok(())
}

fn load_fixture(path: &Path) -> Result<FixtureDoc> {
    let payload = fs::read_to_string(path)
        .with_context(|| format!("Failed reading fixture {}", path.display()))?;
    serde_json::from_str(&payload)
        .with_context(|| format!("Failed parsing fixture {}", path.display()))
}

fn compute_decision_for_sha(root: &Path, base: &str, head: &str) -> Result<ScopeOutput> {
    let changed_files = changed_files_between(root, base, head)?;
    let metadata = cmd("cargo", ["metadata", "--format-version", "1"])
        .dir(root)
        .stdout_capture()
        .stderr_capture()
        .run()
        .context("Failed to run cargo metadata")?;
    let metadata_json: serde_json::Value =
        serde_json::from_slice(&metadata.stdout).context("Failed to parse cargo metadata JSON")?;
    let workspace_root = root.to_string_lossy().replace('\\', "/");
    let mut decision = ci_scope::classify_files(&changed_files, &metadata_json, &workspace_root)?;
    decision.base = base.to_string();
    decision.head_sha = head.to_string();
    decision.changed_files = changed_files;
    Ok(decision)
}

fn changed_files_between(root: &Path, base: &str, head: &str) -> Result<Vec<String>> {
    let diff_spec = format!("{base}...{head}");
    let output = cmd("git", ["diff", "--name-only", &diff_spec])
        .dir(root)
        .stdout_capture()
        .stderr_capture()
        .unchecked()
        .run()
        .context("Failed running git diff for scope-meta-gate")?;

    let stdout_bytes = if output.status.success() {
        output.stdout
    } else {
        let fallback_spec = format!("{base}..{head}");
        cmd("git", ["diff", "--name-only", &fallback_spec])
            .dir(root)
            .stdout_capture()
            .run()
            .context("Failed running git diff fallback for scope-meta-gate")?
            .stdout
    };

    let stdout = String::from_utf8(stdout_bytes).context("git diff output was not UTF-8")?;
    Ok(stdout.lines().map(|line| line.to_string()).collect())
}

fn validate_receipt_registry_if_present(root: &Path, receipt: &ScopeMetaGateReceipt) -> Result<()> {
    let schema_path = root.join(".ci/receipts/schemas/scope-meta-gate.schema.json");
    if !schema_path.exists() {
        return Ok(());
    }

    let schema_doc = fs::read_to_string(&schema_path)
        .with_context(|| format!("Failed reading schema {}", schema_path.display()))?;
    let schema_json: serde_json::Value = serde_json::from_str(&schema_doc)
        .with_context(|| format!("Failed parsing schema {}", schema_path.display()))?;

    let required = schema_json
        .get("required")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let receipt_json =
        serde_json::to_value(receipt).context("Failed converting receipt to JSON")?;

    for field in required {
        if let Some(field_name) = field.as_str()
            && receipt_json.get(field_name).is_none()
        {
            bail!("scope-meta-gate receipt missing required field `{field_name}`");
        }
    }

    Ok(())
}

fn compare_lanes(old_decision: &ScopeOutput, new_decision: &ScopeOutput) -> ChangedLanes {
    let old: BTreeSet<String> = lane_set(old_decision);
    let new: BTreeSet<String> = lane_set(new_decision);

    let dropped = old.difference(&new).cloned().collect();
    let added = new.difference(&old).cloned().collect();
    let unchanged = old.intersection(&new).cloned().collect();

    ChangedLanes { dropped, added, unchanged }
}

fn lane_set(decision: &ScopeOutput) -> BTreeSet<String> {
    decision
        .selected_lanes
        .iter()
        .map(|lane| lane.lane.clone())
        .chain(decision.selected_heavy_lanes.iter().map(|lane| lane.lane.clone()))
        .collect()
}

fn evaluate_verdict(
    old_decision: &ScopeOutput,
    new_decision: &ScopeOutput,
    changed_lanes: &ChangedLanes,
) -> Verdict {
    let meta_sensitive_change =
        old_decision.changed_files.iter().chain(new_decision.changed_files.iter()).any(|path| {
            TRIGGER_PATH_PREFIXES.iter().any(|prefix| path == *prefix || path.starts_with(prefix))
        });

    if !changed_lanes.dropped.is_empty() && meta_sensitive_change {
        return Verdict {
            status: "fail".to_string(),
            reason: "lane selection shrank after scope/gate policy changes".to_string(),
            required_lanes: changed_lanes.dropped.clone(),
        };
    }

    if !changed_lanes.dropped.is_empty() {
        return Verdict {
            status: "warn".to_string(),
            reason: "lane selection shrank, but no scope-meta trigger files changed".to_string(),
            required_lanes: changed_lanes.dropped.clone(),
        };
    }

    if !changed_lanes.added.is_empty() {
        return Verdict {
            status: "warn".to_string(),
            reason: "lane selection expanded safely".to_string(),
            required_lanes: vec![],
        };
    }

    Verdict {
        status: "pass".to_string(),
        reason: "lane selection unchanged".to_string(),
        required_lanes: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::Result;

    fn scope_output(files: &[&str], lanes: &[&str]) -> ScopeOutput {
        ScopeOutput {
            schema_version: 2,
            base: "base".to_string(),
            head_sha: "head".to_string(),
            changed_files: files.iter().map(|f| (*f).to_string()).collect(),
            diff_class: "code".to_string(),
            direct_crates: vec![],
            reverse_dep_closure: vec![],
            architecture_wideners: vec![],
            risk_tags: vec![],
            platform_overrides: Default::default(),
            selected_lanes: lanes
                .iter()
                .map(|lane| ci_scope::LaneEntry {
                    lane: (*lane).to_string(),
                    scope: vec![],
                    reason: "fixture".to_string(),
                })
                .collect(),
            selected_heavy_lanes: vec![],
            explanations: Default::default(),
        }
    }

    #[test]
    fn fixture_drop_with_scope_logic_change_fails() -> Result<()> {
        let old_decision =
            scope_output(&["xtask/src/tasks/ci_scope.rs"], &["parser_ratchet", "clippy_scoped"]);
        let new_decision = scope_output(&["xtask/src/tasks/ci_scope.rs"], &["clippy_scoped"]);

        let changed = compare_lanes(&old_decision, &new_decision);
        let verdict = evaluate_verdict(&old_decision, &new_decision, &changed);

        assert_eq!(verdict.status, "fail");
        assert!(verdict.required_lanes.contains(&"parser_ratchet".to_string()));
        Ok(())
    }

    #[test]
    fn fixture_docs_expansion_warns() -> Result<()> {
        let old_decision = scope_output(&["docs/ci/LOCAL_CI_PROTOCOL.md"], &["clippy_scoped"]);
        let new_decision =
            scope_output(&["docs/ci/LOCAL_CI_PROTOCOL.md"], &["clippy_scoped", "docs"]);

        let changed = compare_lanes(&old_decision, &new_decision);
        let verdict = evaluate_verdict(&old_decision, &new_decision, &changed);

        assert!(verdict.status == "warn" || verdict.status == "pass");
        Ok(())
    }
}
