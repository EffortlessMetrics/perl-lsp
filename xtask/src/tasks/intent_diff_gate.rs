//! Intent/diff closeout evidence gate.

use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

const DEFAULT_POLICY_PATH: &str = ".ci/policies/intent-diff-rules.toml";
const DEFAULT_RECEIPT_PATH: &str = "target/receipts/intent-diff-gate.json";

static CODE_FIX_RE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?i)\b(fix|bugfix|regression|activation)\b"));
static DOCS_RE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?i)\b(docs?|documentation|readme)\b"));
static SCAFFOLD_RE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?i)\b(scaffold|partial|wip|follow[- ]up)\b"));
static CLOSES_RE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?i)\b(?:close[sd]?|fix(?:e[sd])?|resolve[sd]?)\s*#(\d+)\b"));

#[derive(Debug, Clone)]
pub struct IntentDiffGateConfig {
    pub pr: Option<u64>,
    pub fixture: Option<PathBuf>,
    pub receipt: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct PolicyFile {
    #[serde(default)]
    defaults: PolicyDefaults,
    #[serde(default)]
    issues: BTreeMap<String, IssueRule>,
    #[serde(default)]
    components: BTreeMap<String, ComponentRule>,
}

#[derive(Debug, Deserialize)]
struct PolicyDefaults {
    #[serde(default = "default_fail")]
    docs_only_code_fix: GateLevel,
    #[serde(default = "default_fail")]
    scaffold_closeout: GateLevel,
    #[serde(default = "default_warn")]
    docs_claim_code_change: GateLevel,
}

impl Default for PolicyDefaults {
    fn default() -> Self {
        Self {
            docs_only_code_fix: default_fail(),
            scaffold_closeout: default_fail(),
            docs_claim_code_change: default_warn(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct IssueRule {
    #[serde(default)]
    expected_paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ComponentRule {
    #[serde(default)]
    expected_paths: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum GateLevel {
    Warn,
    Fail,
}

fn default_fail() -> GateLevel {
    GateLevel::Fail
}

fn default_warn() -> GateLevel {
    GateLevel::Warn
}

#[derive(Debug, Clone, Deserialize)]
struct FixtureInput {
    title: String,
    body: String,
    changed_files: Vec<String>,
    #[serde(default)]
    evidence: FixtureEvidence,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct FixtureEvidence {
    #[serde(default)]
    test_updated: bool,
    #[serde(default)]
    behavior_receipt: bool,
    override_approved: bool,
}

#[derive(Debug)]
struct PrInput {
    title: String,
    body: String,
    changed_files: Vec<String>,
    evidence: FixtureEvidence,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
enum Verdict {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Serialize)]
struct Receipt {
    claimed_component: Option<String>,
    claimed_closeout_issues: Vec<u64>,
    expected_paths: Vec<String>,
    actual_paths: Vec<String>,
    evidence: ReceiptEvidence,
    verdict: Verdict,
    violations: Vec<Violation>,
}

#[derive(Debug, Clone, Serialize)]
struct ReceiptEvidence {
    target_path_touched: bool,
    test_updated: bool,
    behavior_receipt: bool,
    override_approved: bool,
}

#[derive(Debug, Clone, Serialize)]
struct Violation {
    code: String,
    level: GateLevel,
    message: String,
}

pub fn run(config: IntentDiffGateConfig) -> Result<()> {
    if config.pr.is_some() == config.fixture.is_some() {
        bail!("Provide exactly one input mode: --pr <N> or --fixture <json>");
    }

    let root = project_root()?;
    let policy = read_policy(&root.join(DEFAULT_POLICY_PATH))?;

    let input = if let Some(pr) = config.pr {
        load_pr_from_gh(pr)?
    } else {
        load_fixture(
            config
                .fixture
                .as_ref()
                .ok_or_else(|| color_eyre::eyre::eyre!("missing fixture path"))?,
        )?
    };

    let receipt = evaluate(&input, &policy);
    let receipt_path = config.receipt.unwrap_or_else(|| root.join(DEFAULT_RECEIPT_PATH));
    write_receipt(&receipt_path, &receipt)?;

    println!("intent-diff-gate verdict: {:?}", receipt.verdict);
    println!("receipt: {}", receipt_path.display());
    for violation in &receipt.violations {
        println!("- [{:?}] {} ({})", violation.level, violation.message, violation.code);
    }

    if matches!(receipt.verdict, Verdict::Fail) {
        bail!("intent-diff-gate failed");
    }

    Ok(())
}

fn read_policy(path: &Path) -> Result<PolicyFile> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}

fn load_fixture(path: &Path) -> Result<PrInput> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("reading fixture {}", path.display()))?;
    let fixture: FixtureInput = serde_json::from_str(&raw)
        .with_context(|| format!("parsing fixture {}", path.display()))?;
    Ok(PrInput {
        title: fixture.title,
        body: fixture.body,
        changed_files: fixture.changed_files,
        evidence: fixture.evidence,
    })
}

#[derive(Debug, Deserialize)]
struct GhPr {
    title: String,
    body: String,
    files: Vec<GhFile>,
}

#[derive(Debug, Deserialize)]
struct GhFile {
    path: String,
}

fn load_pr_from_gh(pr: u64) -> Result<PrInput> {
    let output = Command::new("gh")
        .args(["pr", "view", &pr.to_string(), "--json", "title,body,files"])
        .output()
        .context("running gh pr view")?;

    if !output.status.success() {
        bail!("gh pr view failed: {}", String::from_utf8_lossy(&output.stderr).trim());
    }

    let parsed: GhPr = serde_json::from_slice(&output.stdout).context("parsing gh PR payload")?;
    Ok(PrInput {
        title: parsed.title,
        body: parsed.body,
        changed_files: parsed.files.into_iter().map(|f| f.path).collect(),
        evidence: FixtureEvidence::default(),
    })
}

fn evaluate(input: &PrInput, policy: &PolicyFile) -> Receipt {
    let combined = format!("{}\n{}", input.title, input.body);
    let claimed_code_fix = regex_matches(&CODE_FIX_RE, &combined);
    let claimed_docs = regex_matches(&DOCS_RE, &input.title);
    let scaffold_claim = regex_matches(&SCAFFOLD_RE, &combined);

    let actual_paths = normalize_paths(&input.changed_files);
    let docs_only = actual_paths.iter().all(|p| is_doc_path(p));
    let production_changed = actual_paths.iter().any(|p| !is_doc_path(p) && !is_test_path(p));
    let test_updated = input.evidence.test_updated || actual_paths.iter().any(|p| is_test_path(p));

    let closing_issues = extract_closing_issues(&combined);
    let claimed_component = infer_component(&combined);

    let mut expected = BTreeSet::new();
    for issue in &closing_issues {
        if let Some(rule) = policy.issues.get(&issue.to_string()) {
            for path in &rule.expected_paths {
                expected.insert(path.clone());
            }
        }
    }
    if let Some(component) = claimed_component.as_deref()
        && let Some(rule) = policy.components.get(component)
    {
        for path in &rule.expected_paths {
            expected.insert(path.clone());
        }
    }
    let expected_paths: Vec<String> = expected.into_iter().collect();
    let target_path_touched = expected_paths
        .iter()
        .any(|needle| actual_paths.iter().any(|actual| path_matches(actual, needle)));

    let mut violations = Vec::new();

    if claimed_code_fix && docs_only {
        violations.push(Violation {
            code: "docs_only_code_fix_claim".to_string(),
            level: policy.defaults.docs_only_code_fix,
            message: "PR claims a code fix but only docs changed".to_string(),
        });
    }

    if claimed_docs && production_changed {
        violations.push(Violation {
            code: "docs_claim_with_prod_changes".to_string(),
            level: policy.defaults.docs_claim_code_change,
            message: "Docs-focused title but production code changed".to_string(),
        });
    }

    if !(closing_issues.is_empty()
        || target_path_touched
        || test_updated
        || input.evidence.behavior_receipt
        || input.evidence.override_approved)
    {
        violations.push(Violation {
            code: "closeout_without_evidence".to_string(),
            level: GateLevel::Fail,
            message: "Closeout keyword used without target-path/test/receipt/override evidence"
                .to_string(),
        });
    }

    if scaffold_claim && !closing_issues.is_empty() {
        violations.push(Violation {
            code: "scaffold_with_closing_keyword".to_string(),
            level: policy.defaults.scaffold_closeout,
            message: "Scaffold/partial PR should not use closing keywords".to_string(),
        });
    }

    let verdict = if violations.iter().any(|v| matches!(v.level, GateLevel::Fail)) {
        Verdict::Fail
    } else if violations.iter().any(|v| matches!(v.level, GateLevel::Warn)) {
        Verdict::Warn
    } else {
        Verdict::Pass
    };

    Receipt {
        claimed_component,
        claimed_closeout_issues: closing_issues,
        expected_paths,
        actual_paths,
        evidence: ReceiptEvidence {
            target_path_touched,
            test_updated,
            behavior_receipt: input.evidence.behavior_receipt,
            override_approved: input.evidence.override_approved,
        },
        verdict,
        violations,
    }
}

fn normalize_paths(paths: &[String]) -> Vec<String> {
    let mut set: BTreeSet<String> = BTreeSet::new();
    for path in paths {
        let normalized = path.replace('\\', "/");
        set.insert(normalized);
    }
    set.into_iter().collect()
}

fn extract_closing_issues(text: &str) -> Vec<u64> {
    let mut issues = BTreeSet::new();
    if let Ok(regex) = &*CLOSES_RE {
        for caps in regex.captures_iter(text) {
            if let Some(m) = caps.get(1)
                && let Ok(value) = m.as_str().parse::<u64>()
            {
                issues.insert(value);
            }
        }
    }
    issues.into_iter().collect()
}

fn regex_matches(regex: &LazyLock<Result<Regex, regex::Error>>, text: &str) -> bool {
    match &**regex {
        Ok(compiled) => compiled.is_match(text),
        Err(_) => false,
    }
}

fn infer_component(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    if lower.contains("vs code") && lower.contains("activation") {
        return Some("vscode_activation".to_string());
    }
    None
}

fn is_doc_path(path: &str) -> bool {
    path.starts_with("docs/") || path.ends_with(".md")
}

fn is_test_path(path: &str) -> bool {
    path.contains("/test") || path.contains("/tests/")
}

fn path_matches(actual: &str, expected: &str) -> bool {
    actual == expected || actual.starts_with(expected)
}

fn write_receipt(path: &Path, receipt: &Receipt) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    let payload = serde_json::to_string_pretty(receipt).context("serializing receipt")?;
    fs::write(path, format!("{payload}\n")).with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> Result<PolicyFile> {
        toml::from_str(
            r#"
[defaults]
docs_only_code_fix = "fail"
scaffold_closeout = "fail"
docs_claim_code_change = "warn"

[issues."6747"]
expected_paths = ["vscode-extension/package.json", "crates/perl-lsp-rs/tests/"]

[components.vscode_activation]
expected_paths = ["vscode-extension/package.json", "crates/perl-lsp-rs/tests/"]
"#,
        )
        .context("valid inline policy")
    }

    #[test]
    fn docs_only_fix_claim_fails() -> Result<()> {
        let input = PrInput {
            title: "fix(vscode): VS Code activation bug".to_string(),
            body: "Fixes #6747".to_string(),
            changed_files: vec!["docs/notes.md".to_string()],
            evidence: FixtureEvidence::default(),
        };

        let receipt = evaluate(&input, &policy()?);
        assert!(matches!(receipt.verdict, Verdict::Fail));
        Ok(())
    }

    #[test]
    fn partial_refs_passes() -> Result<()> {
        let input = PrInput {
            title: "feat(ci): partial scaffold".to_string(),
            body: "Refs #6747".to_string(),
            changed_files: vec!["docs/ci/new-gate.md".to_string()],
            evidence: FixtureEvidence::default(),
        };

        let receipt = evaluate(&input, &policy()?);
        assert!(matches!(receipt.verdict, Verdict::Pass));
        Ok(())
    }

    #[test]
    fn closeout_with_target_path_passes() -> Result<()> {
        let input = PrInput {
            title: "fix(vscode): activation regression".to_string(),
            body: "Closes #6747".to_string(),
            changed_files: vec!["vscode-extension/package.json".to_string()],
            evidence: FixtureEvidence::default(),
        };

        let receipt = evaluate(&input, &policy()?);
        assert!(matches!(receipt.verdict, Verdict::Pass));
        Ok(())
    }
}
