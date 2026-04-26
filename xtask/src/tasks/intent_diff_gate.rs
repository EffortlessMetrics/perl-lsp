use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail, eyre};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Deserialize)]
pub struct IntentDiffPolicy {
    #[serde(default)]
    pub severity: SeverityPolicy,
    #[serde(default)]
    pub issues: BTreeMap<String, IssueRule>,
    #[serde(default)]
    pub component_expected_paths: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct IssueRule {
    #[serde(default)]
    pub expected_paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SeverityPolicy {
    #[serde(default = "default_fail")]
    pub docs_only_code_fix: String,
    #[serde(default = "default_fail")]
    pub closeout_missing_evidence: String,
    #[serde(default = "default_fail")]
    pub scaffold_with_close_keyword: String,
    #[serde(default = "default_warn")]
    pub docs_claim_but_code_changed: String,
    #[serde(default = "default_fail")]
    pub vscode_activation_missing_path: String,
}

impl Default for SeverityPolicy {
    fn default() -> Self {
        Self {
            docs_only_code_fix: default_fail(),
            closeout_missing_evidence: default_fail(),
            scaffold_with_close_keyword: default_fail(),
            docs_claim_but_code_changed: default_warn(),
            vscode_activation_missing_path: default_fail(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GateInput {
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub actual_paths: Vec<String>,
    #[serde(default)]
    pub behavior_receipt: bool,
    #[serde(default)]
    pub override_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GateReceipt {
    pub claimed_component: String,
    pub claimed_closeout_issues: Vec<String>,
    pub expected_paths: BTreeMap<String, Vec<String>>,
    pub actual_paths: Vec<String>,
    pub evidence: Evidence,
    pub verdict: String,
    pub violations: Vec<Violation>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Evidence {
    pub touched_expected_path: bool,
    pub test_updated: bool,
    pub behavior_receipt: bool,
    pub explicit_override: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Violation {
    pub code: String,
    pub severity: String,
    pub message: String,
}

fn default_fail() -> String {
    "fail".to_string()
}

fn default_warn() -> String {
    "warn".to_string()
}

pub fn run(pr: Option<u64>, fixture: Option<PathBuf>, receipt: Option<PathBuf>) -> Result<()> {
    if pr.is_some() == fixture.is_some() {
        bail!("Provide exactly one of --pr <N> or --fixture <json>");
    }

    let root = project_root()?;
    let policy = load_policy(&root.join(".ci/policies/intent-diff-rules.toml"))?;
    let input = if let Some(path) = fixture {
        load_fixture(&path)?
    } else if let Some(pr_number) = pr {
        load_pr_input(pr_number)?
    } else {
        bail!("Provide exactly one of --pr <N> or --fixture <json>");
    };

    let gate_receipt = evaluate(&input, &policy);
    let serialized = serde_json::to_string_pretty(&gate_receipt)?;

    if let Some(path) = receipt {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create receipt directory {}", parent.display())
            })?;
        }
        fs::write(&path, &serialized)
            .with_context(|| format!("Failed to write receipt to {}", path.display()))?;
    }

    println!("{serialized}");

    if gate_receipt.verdict == "fail" {
        bail!("intent-diff gate failed");
    }

    Ok(())
}

fn load_policy(path: &Path) -> Result<IntentDiffPolicy> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read policy file {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("Failed to parse policy file {}", path.display()))
}

fn load_fixture(path: &Path) -> Result<GateInput> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read fixture {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("Failed to parse fixture {}", path.display()))
}

fn load_pr_input(pr: u64) -> Result<GateInput> {
    let output = Command::new("gh")
        .args([
            "pr",
            "view",
            &pr.to_string(),
            "--json",
            "title,body,files",
            "--repo",
            "EffortlessMetrics/perl-lsp",
        ])
        .output()
        .context("Failed to execute gh CLI")?;

    if !output.status.success() {
        return Err(eyre!("gh pr view failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    #[derive(Deserialize)]
    struct GhFile {
        path: String,
    }
    #[derive(Deserialize)]
    struct GhPr {
        title: String,
        #[serde(default)]
        body: String,
        #[serde(default)]
        files: Vec<GhFile>,
    }

    let parsed: GhPr = serde_json::from_slice(&output.stdout).context("Failed to parse gh JSON")?;
    let actual_paths = parsed.files.into_iter().map(|file| file.path).collect();

    Ok(GateInput {
        title: parsed.title,
        body: parsed.body,
        actual_paths,
        behavior_receipt: false,
        override_reason: None,
    })
}

fn evaluate(input: &GateInput, policy: &IntentDiffPolicy) -> GateReceipt {
    let merged_text = format!("{}\n{}", input.title.to_lowercase(), input.body.to_lowercase());
    let claimed_component = detect_component(&merged_text);
    let closeout_issues = parse_closeout_issues(&input.body);
    let is_docs_only = input.actual_paths.iter().all(|path| is_docs_path(path));
    let test_updated = input.actual_paths.iter().any(|path| is_test_path(path));
    let explicit_override = input.override_reason.is_some();

    let mut expected_paths = BTreeMap::new();
    let mut all_expected_paths = BTreeSet::new();

    for issue in &closeout_issues {
        if let Some(rule) = policy.issues.get(issue) {
            expected_paths.insert(issue.clone(), rule.expected_paths.clone());
            for path in &rule.expected_paths {
                all_expected_paths.insert(path.clone());
            }
        }
    }

    if let Some(paths) = policy.component_expected_paths.get(&claimed_component) {
        expected_paths.insert(claimed_component.clone(), paths.clone());
        for path in paths {
            all_expected_paths.insert(path.clone());
        }
    }

    let touched_expected_path = input
        .actual_paths
        .iter()
        .any(|actual| all_expected_paths.iter().any(|expected| actual.starts_with(expected)));

    let evidence = Evidence {
        touched_expected_path,
        test_updated,
        behavior_receipt: input.behavior_receipt,
        explicit_override,
    };

    let mut violations = Vec::new();
    let claims_fix = merged_text.contains("fix") || merged_text.contains("bug");
    let claims_docs = input.title.to_lowercase().contains("docs");
    let production_changed =
        input.actual_paths.iter().any(|path| !is_docs_path(path) && !is_test_path(path));
    let claims_scaffold_partial =
        merged_text.contains("scaffold") || merged_text.contains("partial");

    if claims_fix && is_docs_only {
        violations.push(Violation {
            code: "docs-only-code-fix-claim".to_string(),
            severity: policy.severity.docs_only_code_fix.clone(),
            message: "PR claims a fix but only docs files changed".to_string(),
        });
    }

    if claims_docs && production_changed {
        violations.push(Violation {
            code: "docs-title-with-code-change".to_string(),
            severity: policy.severity.docs_claim_but_code_changed.clone(),
            message: "PR title claims docs-only scope but production code changed".to_string(),
        });
    }

    if claims_scaffold_partial && !closeout_issues.is_empty() {
        violations.push(Violation {
            code: "partial-pr-uses-close-keyword".to_string(),
            severity: policy.severity.scaffold_with_close_keyword.clone(),
            message: "Scaffold/partial PR should not use closing keywords".to_string(),
        });
    }

    if !(closeout_issues.is_empty()
        || touched_expected_path
        || test_updated
        || input.behavior_receipt
        || explicit_override)
    {
        violations.push(Violation {
            code: "closeout-missing-evidence".to_string(),
            severity: policy.severity.closeout_missing_evidence.clone(),
            message: "Closeout keyword used without target-path touch, tests, receipt, or override"
                .to_string(),
        });
    }

    if claimed_component == "vscode_activation_fix"
        && !(touched_expected_path || test_updated || explicit_override)
    {
        violations.push(Violation {
            code: "vscode-activation-missing-evidence".to_string(),
            severity: policy.severity.vscode_activation_missing_path.clone(),
            message:
                "VS Code activation fix claims must touch expected extension paths or tests (or override)"
                    .to_string(),
        });
    }

    let verdict = if violations.iter().any(|violation| violation.severity == "fail") {
        "fail"
    } else if violations.iter().any(|violation| violation.severity == "warn") {
        "warn"
    } else {
        "pass"
    }
    .to_string();

    GateReceipt {
        claimed_component,
        claimed_closeout_issues: closeout_issues,
        expected_paths,
        actual_paths: input.actual_paths.clone(),
        evidence,
        verdict,
        violations,
    }
}

fn detect_component(merged_text: &str) -> String {
    if merged_text.contains("vs code")
        && merged_text.contains("activation")
        && merged_text.contains("fix")
    {
        return "vscode_activation_fix".to_string();
    }
    if merged_text.contains("docs") {
        return "docs".to_string();
    }
    if merged_text.contains("fix") {
        return "code_fix".to_string();
    }
    "unspecified".to_string()
}

fn parse_closeout_issues(body: &str) -> Vec<String> {
    let mut issues = BTreeSet::new();
    let normalized = body.replace('\n', " ");
    let tokens: Vec<&str> = normalized.split_whitespace().collect();

    for window in tokens.windows(2) {
        if let [keyword, issue_ref] = window {
            let key = keyword.to_ascii_lowercase();
            if (key == "closes" || key == "fixes" || key == "resolves")
                && let Some(stripped) = issue_ref.strip_prefix('#')
            {
                let digits: String =
                    stripped.chars().take_while(|ch| ch.is_ascii_digit()).collect();
                if !digits.is_empty() {
                    issues.insert(digits);
                }
            }
        }
    }

    issues.into_iter().collect()
}

fn is_docs_path(path: &str) -> bool {
    path.starts_with("docs/")
        || path.ends_with(".md")
        || path.ends_with(".txt")
        || path.starts_with(".github/")
}

fn is_test_path(path: &str) -> bool {
    path.contains("/tests/")
        || path.contains("_test")
        || path.ends_with("_tests.rs")
        || path.contains("fixtures")
}
