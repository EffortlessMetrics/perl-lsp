use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const POLICY_PATH: &str = ".ci/policies/intent-diff-rules.toml";

#[derive(Debug, Deserialize)]
struct Policy {
    severity: SeverityPolicy,
    #[serde(default)]
    issue_targets: HashMap<String, Vec<String>>,
    #[serde(default)]
    component_expectations: HashMap<String, ComponentPolicy>,
    #[serde(default = "default_override_markers")]
    override_markers: Vec<String>,
    #[serde(default = "default_test_path_markers")]
    test_path_markers: Vec<String>,
    #[serde(default = "default_behavior_receipt_markers")]
    behavior_receipt_markers: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SeverityPolicy {
    docs_only_code_claim: RuleLevel,
    docs_title_code_change: RuleLevel,
    scaffold_closeout: RuleLevel,
}

#[derive(Debug, Deserialize)]
struct ComponentPolicy {
    keywords: Vec<String>,
    expected_paths: Vec<String>,
    #[serde(default)]
    level: Option<RuleLevel>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum RuleLevel {
    Warn,
    Fail,
}

#[derive(Debug, Deserialize)]
struct FixtureInput {
    title: String,
    body: String,
    actual_paths: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Receipt {
    claimed_component: Vec<String>,
    claimed_closeout_issues: Vec<u64>,
    expected_paths: Vec<String>,
    actual_paths: Vec<String>,
    evidence: Evidence,
    verdict: String,
    violations: Vec<Violation>,
}

#[derive(Debug, Serialize)]
struct Evidence {
    target_path_touched: bool,
    test_updated: bool,
    behavior_receipt: bool,
    explicit_override: bool,
}

#[derive(Debug, Serialize)]
struct Violation {
    rule: String,
    level: String,
    message: String,
}

#[derive(Debug)]
struct PrData {
    title: String,
    body: String,
    actual_paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GhPrResponse {
    title: String,
    body: String,
    files: Vec<GhFile>,
}

#[derive(Debug, Deserialize)]
struct GhFile {
    path: String,
}

pub fn run(pr: Option<u64>, fixture: Option<PathBuf>, receipt: Option<PathBuf>) -> Result<()> {
    if pr.is_some() && fixture.is_some() {
        bail!("pass only one input source: --pr or --fixture");
    }
    if pr.is_none() && fixture.is_none() {
        bail!("one input source is required: --pr or --fixture");
    }

    let root = project_root()?;
    let policy_path = root.join(POLICY_PATH);
    let policy = load_policy(&policy_path)?;

    let pr_data = match (pr, fixture) {
        (Some(number), None) => fetch_pr(number)?,
        (None, Some(path)) => load_fixture(&path)?,
        _ => bail!("invalid input arguments"),
    };

    let report = evaluate(&policy, pr_data);

    if let Some(path) = receipt {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create receipt dir {}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(&report)?;
        fs::write(&path, json)
            .with_context(|| format!("failed to write receipt {}", path.display()))?;
    }

    if report.verdict == "fail" {
        bail!("intent-diff-gate failed");
    }

    Ok(())
}

fn load_policy(path: &Path) -> Result<Policy> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read policy file {}", path.display()))?;
    let parsed: Policy =
        toml::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(parsed)
}

fn load_fixture(path: &Path) -> Result<PrData> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read fixture {}", path.display()))?;
    let input: FixtureInput = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse fixture {}", path.display()))?;
    Ok(PrData { title: input.title, body: input.body, actual_paths: input.actual_paths })
}

fn fetch_pr(pr_number: u64) -> Result<PrData> {
    let output = Command::new("gh")
        .arg("pr")
        .arg("view")
        .arg(pr_number.to_string())
        .arg("--json")
        .arg("title,body,files")
        .output()
        .context("failed to execute gh; install gh cli or use --fixture")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh pr view failed for #{}: {}", pr_number, stderr.trim());
    }

    let parsed: GhPrResponse = serde_json::from_slice(&output.stdout)
        .context("failed to parse gh pr view json response")?;

    Ok(PrData {
        title: parsed.title,
        body: parsed.body,
        actual_paths: parsed.files.into_iter().map(|f| f.path).collect(),
    })
}

fn evaluate(policy: &Policy, pr: PrData) -> Receipt {
    let normalized_paths: Vec<String> = pr.actual_paths.iter().map(|p| p.to_lowercase()).collect();
    let title_lc = pr.title.to_lowercase();
    let body_lc = pr.body.to_lowercase();
    let merged_text = format!("{}\n{}", title_lc, body_lc);

    let closeout_issues = parse_closing_issues(&merged_text);
    let docs_only = is_docs_only(&pr.actual_paths);
    let docs_claim = is_docs_claim(&title_lc);
    let code_fix_claim = is_code_fix_claim(&merged_text);
    let partial_claim = is_partial_or_scaffold(&merged_text);

    let mut expected_paths: BTreeSet<String> = BTreeSet::new();
    for issue in &closeout_issues {
        if let Some(paths) = policy.issue_targets.get(&issue.to_string()) {
            for path in paths {
                expected_paths.insert(path.clone());
            }
        }
    }

    let mut claimed_components = Vec::new();
    for (name, rule) in &policy.component_expectations {
        let claimed =
            rule.keywords.iter().any(|keyword| merged_text.contains(&keyword.to_lowercase()));
        if claimed {
            claimed_components.push(name.clone());
            for path in &rule.expected_paths {
                expected_paths.insert(path.clone());
            }
        }
    }

    let target_path_touched = expected_paths
        .iter()
        .any(|expected| matches_any_path(&normalized_paths, &expected.to_lowercase()));
    let test_updated = normalized_paths.iter().any(|path| {
        policy.test_path_markers.iter().any(|marker| path.contains(&marker.to_lowercase()))
    });
    let behavior_receipt = normalized_paths.iter().any(|path| {
        policy.behavior_receipt_markers.iter().any(|marker| path.contains(&marker.to_lowercase()))
    });
    let explicit_override =
        policy.override_markers.iter().any(|marker| merged_text.contains(&marker.to_lowercase()));

    let evidence =
        Evidence { target_path_touched, test_updated, behavior_receipt, explicit_override };

    let mut violations: Vec<Violation> = Vec::new();

    if code_fix_claim && docs_only {
        violations.push(Violation {
            rule: "docs_only_code_claim".to_string(),
            level: level_str(policy.severity.docs_only_code_claim).to_string(),
            message: "PR claims a fix but only docs changed".to_string(),
        });
    }

    if docs_claim && !docs_only {
        violations.push(Violation {
            rule: "docs_title_code_change".to_string(),
            level: level_str(policy.severity.docs_title_code_change).to_string(),
            message: "PR title claims docs scope but production code changed".to_string(),
        });
    }

    if (!closeout_issues.is_empty() || !claimed_components.is_empty())
        && !evidence.target_path_touched
        && !evidence.test_updated
        && !evidence.behavior_receipt
        && !evidence.explicit_override
    {
        violations.push(Violation {
            rule: "closeout_requires_evidence".to_string(),
            level: "fail".to_string(),
            message:
                "Closeout claim requires target path, tests, behavior receipt, or explicit override"
                    .to_string(),
        });
    }

    if partial_claim && !closeout_issues.is_empty() {
        violations.push(Violation {
            rule: "scaffold_closeout".to_string(),
            level: level_str(policy.severity.scaffold_closeout).to_string(),
            message: "Scaffold/partial PR must not use closing keywords".to_string(),
        });
    }

    for (name, rule) in &policy.component_expectations {
        let claimed =
            rule.keywords.iter().any(|keyword| merged_text.contains(&keyword.to_lowercase()));
        if claimed
            && !rule.expected_paths.is_empty()
            && !rule
                .expected_paths
                .iter()
                .any(|expected| matches_any_path(&normalized_paths, &expected.to_lowercase()))
            && !evidence.explicit_override
        {
            let level = rule.level.unwrap_or(RuleLevel::Fail);
            violations.push(Violation {
                rule: format!("component_path_expectation:{name}"),
                level: level_str(level).to_string(),
                message: format!("Component '{name}' claim missing expected path evidence"),
            });
        }
    }

    let fail = violations.iter().any(|v| v.level == "fail");
    let warn = !fail && !violations.is_empty();

    Receipt {
        claimed_component: claimed_components,
        claimed_closeout_issues: closeout_issues,
        expected_paths: expected_paths.into_iter().collect(),
        actual_paths: pr.actual_paths,
        evidence,
        verdict: if fail {
            "fail".to_string()
        } else if warn {
            "warn".to_string()
        } else {
            "pass".to_string()
        },
        violations,
    }
}

fn parse_closing_issues(text: &str) -> Vec<u64> {
    let mut issues = BTreeSet::new();
    let tokens: Vec<&str> = text.split_whitespace().collect();
    for pair in tokens.windows(2) {
        let keyword = sanitize_token(pair[0]);
        let issue_token = sanitize_token(pair[1]);
        if is_closeout_keyword(&keyword)
            && let Some(stripped) = issue_token.strip_prefix('#')
            && let Ok(issue) = stripped.parse::<u64>()
        {
            issues.insert(issue);
        }
    }
    issues.into_iter().collect()
}

fn is_closeout_keyword(token: &str) -> bool {
    matches!(
        token,
        "close"
            | "closes"
            | "closed"
            | "fix"
            | "fixes"
            | "fixed"
            | "resolve"
            | "resolves"
            | "resolved"
    )
}

fn sanitize_token(token: &str) -> String {
    token
        .trim_matches(|c: char| {
            c == ':' || c == ',' || c == ';' || c == '.' || c == ')' || c == '('
        })
        .to_lowercase()
}

fn is_docs_only(paths: &[String]) -> bool {
    !paths.is_empty()
        && paths.iter().all(|path| {
            let lc = path.to_lowercase();
            lc.starts_with("docs/")
                || lc.ends_with(".md")
                || lc.ends_with(".mdx")
                || lc.ends_with(".rst")
                || lc.starts_with(".github/")
        })
}

fn is_docs_claim(title_lc: &str) -> bool {
    title_lc.starts_with("docs") || title_lc.contains("documentation")
}

fn is_code_fix_claim(text: &str) -> bool {
    text.contains("fix") || text.contains("bug") || text.contains("regression")
}

fn is_partial_or_scaffold(text: &str) -> bool {
    text.contains("scaffold") || text.contains("partial") || text.contains("wip")
}

fn matches_any_path(actual_paths: &[String], expected: &str) -> bool {
    actual_paths.iter().any(|path| path == expected || path.contains(expected))
}

fn level_str(level: RuleLevel) -> &'static str {
    match level {
        RuleLevel::Warn => "warn",
        RuleLevel::Fail => "fail",
    }
}

fn default_override_markers() -> Vec<String> {
    vec!["intent-diff-override".to_string(), "intent-diff: override".to_string()]
}

fn default_test_path_markers() -> Vec<String> {
    vec!["/tests/".to_string(), "_test".to_string(), "/spec/".to_string()]
}

fn default_behavior_receipt_markers() -> Vec<String> {
    vec![
        "target/receipts/".to_string(),
        ".ci/receipts/".to_string(),
        "review/receipts/".to_string(),
    ]
}
