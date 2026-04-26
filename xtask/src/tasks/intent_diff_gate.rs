use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_POLICY_PATH: &str = ".ci/policies/intent-diff-rules.toml";

#[derive(Debug, Deserialize)]
struct Policy {
    severity: SeverityPolicy,
    close_keywords: CloseKeywords,
    scaffold_markers: ScaffoldMarkers,
    issue_targets: HashMap<String, Vec<String>>,
    components: HashMap<String, ComponentRule>,
    override_marker: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SeverityPolicy {
    docs_only_code_fix: String,
    docs_title_code_change: String,
    closeout_evidence_missing: String,
    scaffold_with_closing: String,
    vscode_activation_missing_evidence: String,
}

#[derive(Debug, Deserialize)]
struct CloseKeywords {
    words: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ScaffoldMarkers {
    words: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ComponentRule {
    title_contains_any: Vec<String>,
    expected_paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct InputFixture {
    title: String,
    body: String,
    changed_paths: Vec<String>,
    explicit_override: Option<bool>,
    behavior_receipt: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct GhPrView {
    title: String,
    body: String,
    files: Vec<GhPrFile>,
}

#[derive(Debug, Deserialize)]
struct GhPrFile {
    path: String,
}

#[derive(Debug, Serialize)]
pub struct IntentDiffReceipt {
    claimed_component: Option<String>,
    claimed_closeout_issues: Vec<u64>,
    expected_paths: Vec<String>,
    actual_paths: Vec<String>,
    evidence: Evidence,
    verdict: String,
    violations: Vec<Violation>,
}

#[derive(Debug, Serialize)]
pub struct Evidence {
    target_path_touched: bool,
    test_updated: bool,
    behavior_receipt: bool,
    explicit_override: bool,
    docs_only_diff: bool,
    scaffold_or_partial: bool,
}

#[derive(Debug, Serialize)]
pub struct Violation {
    code: String,
    severity: String,
    message: String,
}

struct EvaluationInput {
    title: String,
    body: String,
    changed_paths: Vec<String>,
    explicit_override: bool,
    behavior_receipt: bool,
}

pub fn run(pr: Option<u64>, fixture: Option<PathBuf>, receipt: Option<PathBuf>) -> Result<()> {
    let root = project_root()?;
    let policy = load_policy(&root)?;

    let input = match (pr, fixture) {
        (Some(_), Some(_)) => bail!("provide either --pr or --fixture, not both"),
        (None, None) => bail!("provide one of --pr <N> or --fixture <json>"),
        (Some(pr_number), None) => load_pr_input(pr_number)?,
        (None, Some(path)) => load_fixture_input(&path)?,
    };

    let receipt_data = evaluate(&policy, input);
    let json =
        serde_json::to_string_pretty(&receipt_data).context("serializing intent-diff receipt")?;

    if let Some(out_path) = receipt {
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating receipt dir {}", parent.display()))?;
        }
        fs::write(&out_path, format!("{json}\n"))
            .with_context(|| format!("writing receipt {}", out_path.display()))?;
        println!("intent-diff receipt written to {}", out_path.display());
    } else {
        println!("{json}");
    }

    if receipt_data.verdict == "fail" {
        bail!("intent-diff gate failed");
    }

    Ok(())
}

fn load_policy(root: &Path) -> Result<Policy> {
    let path = root.join(DEFAULT_POLICY_PATH);
    let raw = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}

fn load_fixture_input(path: &Path) -> Result<EvaluationInput> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("reading fixture {}", path.display()))?;
    let fixture: InputFixture = serde_json::from_str(&raw)
        .with_context(|| format!("parsing fixture {}", path.display()))?;
    Ok(EvaluationInput {
        title: fixture.title,
        body: fixture.body,
        changed_paths: fixture.changed_paths,
        explicit_override: fixture.explicit_override.unwrap_or(false),
        behavior_receipt: fixture.behavior_receipt.unwrap_or(false),
    })
}

fn load_pr_input(pr_number: u64) -> Result<EvaluationInput> {
    let output = Command::new("gh")
        .args(["pr", "view", &pr_number.to_string(), "--json", "title,body,files"])
        .output()
        .context("running gh pr view")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh pr view failed: {stderr}");
    }

    let payload: GhPrView =
        serde_json::from_slice(&output.stdout).context("parsing gh pr view JSON")?;
    let changed_paths = payload.files.into_iter().map(|f| f.path).collect();

    Ok(EvaluationInput {
        title: payload.title,
        body: payload.body,
        changed_paths,
        explicit_override: false,
        behavior_receipt: false,
    })
}

fn evaluate(policy: &Policy, input: EvaluationInput) -> IntentDiffReceipt {
    let title_lower = input.title.to_lowercase();
    let body_lower = input.body.to_lowercase();
    let actual_paths = input.changed_paths;

    let claimed_component = policy
        .components
        .iter()
        .find(|(_, rule)| {
            rule.title_contains_any
                .iter()
                .any(|needle| title_lower.contains(&needle.to_lowercase()))
        })
        .map(|(name, _)| name.clone());

    let closeout_issues = extract_closeout_issues(&input.body, &policy.close_keywords.words);

    let mut expected_paths: BTreeSet<String> = BTreeSet::new();
    for issue in &closeout_issues {
        if let Some(paths) = policy.issue_targets.get(&issue.to_string()) {
            for path in paths {
                expected_paths.insert(path.clone());
            }
        }
    }

    if let Some(component_name) = &claimed_component {
        if let Some(rule) = policy.components.get(component_name) {
            for path in &rule.expected_paths {
                expected_paths.insert(path.clone());
            }
        }
    }

    let expected_paths_vec: Vec<String> = expected_paths.into_iter().collect();
    let docs_only_diff = actual_paths.iter().all(|path| is_docs_path(path));
    let test_updated = actual_paths.iter().any(|path| is_test_path(path));
    let target_path_touched = touches_expected(&actual_paths, &expected_paths_vec);
    let scaffold_or_partial = contains_any(&title_lower, &policy.scaffold_markers.words)
        || contains_any(&body_lower, &policy.scaffold_markers.words);

    let override_marker =
        policy.override_marker.as_deref().unwrap_or("[intent-diff-override]").to_lowercase();
    let explicit_override = input.explicit_override || body_lower.contains(&override_marker);
    let behavior_receipt = input.behavior_receipt
        || actual_paths
            .iter()
            .any(|path| path.contains("receipt") || path.contains("fixtures/behavior"));

    let evidence = Evidence {
        target_path_touched,
        test_updated,
        behavior_receipt,
        explicit_override,
        docs_only_diff,
        scaffold_or_partial,
    };

    let mut violations = Vec::new();
    let is_code_fix_claim = is_code_fix_claim(&title_lower, &body_lower);
    let docs_title_claim = title_lower.starts_with("docs") || title_lower.contains("docs:");
    let production_code_changed = actual_paths.iter().any(|path| is_production_code_path(path));

    if is_code_fix_claim && docs_only_diff {
        violations.push(Violation {
            code: "docs_only_code_fix_claim".to_string(),
            severity: policy.severity.docs_only_code_fix.clone(),
            message: "PR claims code fix intent but diff is docs-only".to_string(),
        });
    }

    if docs_title_claim && production_code_changed {
        violations.push(Violation {
            code: "docs_title_with_code_diff".to_string(),
            severity: policy.severity.docs_title_code_change.clone(),
            message: "Docs-scoped title but production code changed".to_string(),
        });
    }

    if !closeout_issues.is_empty() {
        if scaffold_or_partial {
            violations.push(Violation {
                code: "scaffold_with_closing_keyword".to_string(),
                severity: policy.severity.scaffold_with_closing.clone(),
                message: "Scaffold/partial PR uses closing keyword".to_string(),
            });
        }

        let has_evidence = evidence.target_path_touched
            || evidence.test_updated
            || evidence.behavior_receipt
            || evidence.explicit_override;
        if !has_evidence {
            violations.push(Violation {
                code: "closeout_missing_evidence".to_string(),
                severity: policy.severity.closeout_evidence_missing.clone(),
                message: "Closing keyword used without required closeout evidence".to_string(),
            });
        }
    }

    if claimed_component.as_deref() == Some("vscode_activation") {
        let has_vscode_evidence =
            evidence.target_path_touched || evidence.test_updated || evidence.explicit_override;
        if !has_vscode_evidence {
            violations.push(Violation {
                code: "vscode_activation_missing_evidence".to_string(),
                severity: policy.severity.vscode_activation_missing_evidence.clone(),
                message: "VS Code activation fix claim requires vscode-extension path/tests/override evidence"
                    .to_string(),
            });
        }
    }

    let verdict = if violations.iter().any(|v| v.severity.eq_ignore_ascii_case("fail")) {
        "fail"
    } else if violations.is_empty() {
        "pass"
    } else {
        "warn"
    }
    .to_string();

    IntentDiffReceipt {
        claimed_component,
        claimed_closeout_issues: closeout_issues,
        expected_paths: expected_paths_vec,
        actual_paths,
        evidence,
        verdict,
        violations,
    }
}

fn extract_closeout_issues(body: &str, close_keywords: &[String]) -> Vec<u64> {
    let escaped_words =
        close_keywords.iter().map(|word| regex::escape(word)).collect::<Vec<String>>().join("|");
    let pattern = format!(r"(?i)\b(?:{})\b\s+#(?P<issue>\d+)", escaped_words);

    let Ok(re) = Regex::new(&pattern) else {
        return Vec::new();
    };

    re.captures_iter(body)
        .filter_map(|capture| capture.name("issue"))
        .filter_map(|m| m.as_str().parse::<u64>().ok())
        .collect()
}

fn contains_any(haystack: &str, needles: &[String]) -> bool {
    needles.iter().any(|needle| haystack.contains(&needle.to_lowercase()))
}

fn is_docs_path(path: &str) -> bool {
    path.starts_with("docs/") || path.ends_with(".md")
}

fn is_test_path(path: &str) -> bool {
    path.starts_with("tests/")
        || path.contains("/tests/")
        || path.contains("fixtures")
        || path.ends_with("_test.rs")
}

fn is_production_code_path(path: &str) -> bool {
    (path.starts_with("crates/") || path.starts_with("xtask/"))
        && !is_test_path(path)
        && !path.ends_with(".md")
}

fn is_code_fix_claim(title_lower: &str, body_lower: &str) -> bool {
    ["fix", "bug", "regression", "activation"].iter().any(|w| title_lower.contains(w))
        || ["fix", "bug", "regression", "activation"].iter().any(|w| body_lower.contains(w))
}

fn touches_expected(actual_paths: &[String], expected_paths: &[String]) -> bool {
    actual_paths
        .iter()
        .any(|actual| expected_paths.iter().any(|expected| actual.starts_with(expected)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_policy() -> Result<Policy> {
        let raw = r#"
[severity]
docs_only_code_fix = "fail"
docs_title_code_change = "warn"
closeout_evidence_missing = "fail"
scaffold_with_closing = "fail"
vscode_activation_missing_evidence = "fail"

[close_keywords]
words = ["closes", "fixes", "resolves"]

[scaffold_markers]
words = ["scaffold", "partial", "follow-up"]

override_marker = "[intent-diff-override]"

[issue_targets]
"6747" = ["vscode-extension/package.json"]

[components.vscode_activation]
title_contains_any = ["vs code activation", "vscode activation"]
expected_paths = ["vscode-extension/package.json", "crates/perl-lsp-ux-tests/tests"]
"#;

        let parsed = toml::from_str(raw)?;
        Ok(parsed)
    }

    #[test]
    fn fixture_6780_style_fails() -> Result<()> {
        let policy = sample_policy()?;
        let receipt = evaluate(
            &policy,
            EvaluationInput {
                title: "fix(vscode): fix VS Code activation startup".to_string(),
                body: "Fixes #6747".to_string(),
                changed_paths: vec!["docs/notes/vscode.md".to_string()],
                explicit_override: false,
                behavior_receipt: false,
            },
        );

        assert_eq!(receipt.verdict, "fail");
        assert!(!receipt.violations.is_empty());
        Ok(())
    }

    #[test]
    fn partial_refs_passes() -> Result<()> {
        let policy = sample_policy()?;
        let receipt = evaluate(
            &policy,
            EvaluationInput {
                title: "feat(ci): scaffold gate".to_string(),
                body: "Refs #6853".to_string(),
                changed_paths: vec!["xtask/src/tasks/intent_diff_gate.rs".to_string()],
                explicit_override: false,
                behavior_receipt: false,
            },
        );

        assert_eq!(receipt.verdict, "pass");
        Ok(())
    }

    #[test]
    fn closeout_with_target_path_passes() -> Result<()> {
        let policy = sample_policy()?;
        let receipt = evaluate(
            &policy,
            EvaluationInput {
                title: "fix(vscode): fix VS Code activation".to_string(),
                body: "Closes #6747".to_string(),
                changed_paths: vec!["vscode-extension/package.json".to_string()],
                explicit_override: false,
                behavior_receipt: false,
            },
        );

        assert_eq!(receipt.verdict, "pass");
        Ok(())
    }
}
