use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use clap::ValueEnum;
use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::utils::project_root;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum MethodologyGateFormat {
    Human,
    Json,
}

#[derive(Debug, Deserialize)]
struct Policy {
    #[serde(default)]
    forbidden: Vec<ForbiddenRule>,
    #[serde(default)]
    forbidden_pattern: Vec<ForbiddenPatternRule>,
}

#[derive(Debug, Deserialize)]
struct ForbiddenRule {
    labels: Vec<String>,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct ForbiddenPatternRule {
    required: String,
    forbidden_glob: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct PullRequestApiResponse {
    number: u64,
    #[serde(default)]
    labels: Vec<LabelObject>,
    body: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LabelObject {
    name: String,
}

#[derive(Debug, Deserialize)]
struct FixtureInput {
    number: Option<u64>,
    event_name: Option<String>,
    body: Option<String>,
    labels: Option<Vec<FixtureLabel>>,
    pull_request: Option<FixturePullRequest>,
    merge_group: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct FixturePullRequest {
    number: Option<u64>,
    body: Option<String>,
    labels: Option<Vec<FixtureLabel>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum FixtureLabel {
    NameOnly(String),
    NamedObject { name: String },
}

impl FixtureLabel {
    fn name(&self) -> &str {
        match self {
            FixtureLabel::NameOnly(name) => name.as_str(),
            FixtureLabel::NamedObject { name } => name.as_str(),
        }
    }
}

#[derive(Debug)]
struct InputState {
    pr_number: Option<u64>,
    body: String,
    labels: BTreeSet<String>,
    source_kind: String,
    merge_group_label_unavailable: bool,
}

#[derive(Debug, Serialize)]
struct Receipt {
    gate: &'static str,
    classification: Classification,
    mode: &'static str,
    source: String,
    pr_number: Option<u64>,
    labels: Vec<String>,
    violations: Vec<Violation>,
    closeout_warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
struct Violation {
    labels: Vec<String>,
    reason: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Classification {
    Pass,
    Warn,
    Fail,
    Unknown,
}

pub fn run(
    fixture: Option<PathBuf>,
    pr: Option<u64>,
    receipt: Option<PathBuf>,
    enforce: bool,
    dry_run: bool,
    format: MethodologyGateFormat,
) -> Result<()> {
    if fixture.is_some() == pr.is_some() {
        bail!("exactly one of --fixture or --pr must be supplied");
    }

    let root = project_root()?;
    let policy_path = root.join(".ci/policies/label-contradictions.toml");
    let policy = load_policy(&policy_path)?;
    let state = if let Some(fixture_path) = fixture {
        load_fixture(&fixture_path)?
    } else if let Some(pr_number) = pr {
        load_pr(pr_number)?
    } else {
        bail!("internal argument handling failure");
    };

    let mut violations = find_violations(&policy, &state.labels);
    let closeout_warnings = find_closeout_warnings(&state.body);
    let classification = if state.merge_group_label_unavailable {
        Classification::Unknown
    } else if !violations.is_empty() {
        if enforce { Classification::Fail } else { Classification::Warn }
    } else if !closeout_warnings.is_empty() {
        Classification::Warn
    } else {
        Classification::Pass
    };

    if state.merge_group_label_unavailable {
        violations.clear();
    }

    let receipt_body = Receipt {
        gate: "methodology-gate",
        classification,
        mode: if enforce { "enforce" } else { "advisory" },
        source: state.source_kind,
        pr_number: state.pr_number,
        labels: state.labels.into_iter().collect(),
        violations,
        closeout_warnings,
    };

    if let Some(path) = receipt {
        write_receipt(&path, &receipt_body, dry_run)?;
    }

    print_output(&receipt_body, format)?;

    if matches!(receipt_body.classification, Classification::Fail) {
        bail!("Methodology Gate found contradictory PR labels");
    }

    Ok(())
}

fn load_policy(path: &Path) -> Result<Policy> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read policy file {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("failed to parse policy file {}", path.display()))
}

fn load_fixture(path: &Path) -> Result<InputState> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read fixture {}", path.display()))?;
    let fixture: FixtureInput = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse fixture {}", path.display()))?;

    let event_name = fixture.event_name.unwrap_or_else(|| {
        if fixture.merge_group.is_some() {
            "merge_group".to_string()
        } else if fixture.pull_request.is_some() {
            "pull_request".to_string()
        } else {
            "push".to_string()
        }
    });
    let pull_request = fixture.pull_request;
    let merged_labels = pull_request
        .as_ref()
        .and_then(|pr| pr.labels.as_ref())
        .or(fixture.labels.as_ref())
        .map(|labels| labels.iter().map(|label| label.name().to_string()).collect::<BTreeSet<_>>())
        .unwrap_or_default();
    let merge_group_label_unavailable = event_name == "merge_group" && merged_labels.is_empty();

    Ok(InputState {
        pr_number: pull_request.as_ref().and_then(|pr| pr.number).or(fixture.number),
        body: pull_request
            .as_ref()
            .and_then(|pr| pr.body.clone())
            .or(fixture.body)
            .unwrap_or_default(),
        labels: merged_labels,
        source_kind: format!("fixture:{event_name}"),
        merge_group_label_unavailable,
    })
}

fn load_pr(pr_number: u64) -> Result<InputState> {
    let output = std::process::Command::new("gh")
        .args(["api", &format!("repos/{{owner}}/{{repo}}/pulls/{pr_number}")])
        .output()
        .context("failed to execute gh api for PR metadata")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh api request failed for PR {pr_number}: {stderr}");
    }

    let response: PullRequestApiResponse =
        serde_json::from_slice(&output.stdout).context("failed to parse gh api PR response")?;

    let labels = response.labels.into_iter().map(|label| label.name).collect();
    Ok(InputState {
        pr_number: Some(response.number),
        body: response.body.unwrap_or_default(),
        labels,
        source_kind: "github-pr".to_string(),
        merge_group_label_unavailable: false,
    })
}

fn find_violations(policy: &Policy, labels: &BTreeSet<String>) -> Vec<Violation> {
    let mut violations = Vec::new();

    for rule in &policy.forbidden {
        if rule.labels.iter().all(|label| labels.contains(label)) {
            violations.push(Violation { labels: rule.labels.clone(), reason: rule.reason.clone() });
        }
    }

    for rule in &policy.forbidden_pattern {
        if labels.contains(&rule.required) {
            let wildcard =
                rule.forbidden_glob.strip_suffix('*').unwrap_or(rule.forbidden_glob.as_str());
            for label in labels {
                if label.starts_with(wildcard) {
                    violations.push(Violation {
                        labels: vec![rule.required.clone(), label.clone()],
                        reason: rule.reason.clone(),
                    });
                }
            }
        }
    }

    violations
}

fn find_closeout_warnings(body: &str) -> Vec<String> {
    let lowercase = body.to_lowercase();
    let has_closeout_verb =
        ["closes #", "fixes #", "resolves #"].iter().any(|token| lowercase.contains(token));
    let has_partial_marker = ["partial", "scaffold", "umbrella", "follow-up", "phase", "advisory"]
        .iter()
        .any(|token| lowercase.contains(token));

    if has_closeout_verb && has_partial_marker {
        return vec![
            "PR body appears partial/scaffold/umbrella but uses Closes/Fixes/Resolves; prefer Refs or Part of until full delivery."
                .to_string(),
        ];
    }

    Vec::new()
}

fn write_receipt(path: &Path, receipt: &Receipt, dry_run: bool) -> Result<()> {
    if dry_run {
        return Ok(());
    }

    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create receipt directory {}", parent.display()))?;
    }

    let output = serde_json::to_string_pretty(receipt).context("failed to serialize receipt")?;
    fs::write(path, output)
        .with_context(|| format!("failed to write receipt file {}", path.display()))
}

fn print_output(receipt: &Receipt, format: MethodologyGateFormat) -> Result<()> {
    match format {
        MethodologyGateFormat::Human => {
            println!(
                "Methodology Gate [{}] classification={:?} labels={} violations={} warnings={}",
                receipt.mode,
                receipt.classification,
                receipt.labels.len(),
                receipt.violations.len(),
                receipt.closeout_warnings.len()
            );

            for violation in &receipt.violations {
                println!(
                    "  contradiction: {} ({})",
                    violation.labels.join(" + "),
                    violation.reason
                );
            }
            for warning in &receipt.closeout_warnings {
                println!("  warning: {warning}");
            }
            if matches!(receipt.classification, Classification::Unknown) {
                println!(
                    "  note: merge_group label data unavailable; pull_request event remains enforcement source"
                );
            }
            Ok(())
        }
        MethodologyGateFormat::Json => {
            let json =
                serde_json::to_string_pretty(receipt).context("failed to serialize JSON output")?;
            println!("{json}");
            Ok(())
        }
    }
}
