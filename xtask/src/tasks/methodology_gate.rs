use color_eyre::eyre::{Context, Result, bail};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

use crate::utils::project_root;

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum MethodologyOutputFormat {
    Human,
    Json,
}

#[derive(Debug, Clone)]
pub struct MethodologyGateConfig {
    pub fixture: Option<PathBuf>,
    pub pr: Option<u64>,
    pub receipt: PathBuf,
    pub dry_run: bool,
    pub enforce: bool,
    pub format: MethodologyOutputFormat,
}

#[derive(Debug, Deserialize)]
struct PolicyFile {
    #[serde(default)]
    forbidden: Vec<ForbiddenLabelSet>,
    #[serde(default)]
    forbidden_pattern: Vec<ForbiddenPattern>,
}

#[derive(Debug, Deserialize)]
struct ForbiddenLabelSet {
    labels: Vec<String>,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct ForbiddenPattern {
    required: String,
    forbidden_glob: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct FixtureInput {
    #[serde(default)]
    event_name: Option<String>,
    #[serde(default)]
    pull_request: Option<FixturePullRequest>,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    body: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FixturePullRequest {
    number: u64,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    labels: Vec<LabelObject>,
}

#[derive(Debug, Deserialize)]
struct LabelObject {
    name: String,
}

#[derive(Debug, Serialize)]
struct MethodologyReceipt {
    schema_version: String,
    classification: String,
    mode: String,
    summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pr_number: Option<u64>,
    labels: Vec<String>,
    contradictions: Vec<Violation>,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
struct Violation {
    rule_type: String,
    labels: Vec<String>,
    reason: String,
}

#[derive(Debug)]
struct InputState {
    pr_number: Option<u64>,
    labels: Vec<String>,
    body: Option<String>,
    event_name: Option<String>,
}

pub fn run(config: MethodologyGateConfig) -> Result<()> {
    let root = project_root()?;
    let policy_path = root.join(".ci/policies/label-contradictions.toml");
    let policy_contents = fs::read_to_string(&policy_path)
        .with_context(|| format!("failed to read {}", policy_path.display()))?;
    let policy: PolicyFile = toml::from_str(&policy_contents)
        .with_context(|| format!("failed to parse {}", policy_path.display()))?;

    let input = load_input(&config)?;
    let mut labels_set = BTreeSet::new();
    for label in input.labels {
        labels_set.insert(label);
    }
    let labels: Vec<String> = labels_set.into_iter().collect();

    let mut warnings = Vec::new();
    if should_warn_closeout_hygiene(input.body.as_deref()) {
        warnings.push(
            "PR body appears partial/scaffold/umbrella while using Closes/Fixes/Resolves; prefer Refs or Part of".to_string(),
        );
    }

    let mut contradictions = Vec::new();
    let labels_lookup: BTreeSet<&str> = labels.iter().map(String::as_str).collect();

    if input.event_name.as_deref() == Some("merge_group") && labels.is_empty() {
        let receipt = MethodologyReceipt {
            schema_version: "1".to_string(),
            classification: "unknown".to_string(),
            mode: mode_name(config.enforce),
            summary:
                "merge_group payload did not expose labels; enforcement deferred to pull_request"
                    .to_string(),
            pr_number: input.pr_number,
            labels,
            contradictions,
            warnings,
        };
        write_receipt(&config, &receipt)?;
        print_receipt(&config, &receipt)?;
        return Ok(());
    }

    for forbidden in &policy.forbidden {
        if forbidden.labels.iter().all(|label| labels_lookup.contains(label.as_str())) {
            contradictions.push(Violation {
                rule_type: "forbidden".to_string(),
                labels: forbidden.labels.clone(),
                reason: forbidden.reason.clone(),
            });
        }
    }

    for pattern in &policy.forbidden_pattern {
        if labels_lookup.contains(pattern.required.as_str()) {
            let mut matches = labels
                .iter()
                .filter(|label| glob_matches(&pattern.forbidden_glob, label))
                .cloned()
                .collect::<Vec<_>>();
            if !matches.is_empty() {
                matches.sort();
                matches.dedup();
                let mut labels_for_violation = vec![pattern.required.clone()];
                labels_for_violation.extend(matches);
                contradictions.push(Violation {
                    rule_type: "forbidden_pattern".to_string(),
                    labels: labels_for_violation,
                    reason: pattern.reason.clone(),
                });
            }
        }
    }

    let has_contradictions = !contradictions.is_empty();
    let classification =
        if has_contradictions { if config.enforce { "failed" } else { "warn" } } else { "pass" };
    let summary = if has_contradictions {
        format!("detected {} contradictory label state(s)", contradictions.len())
    } else {
        "no contradictory label states detected".to_string()
    };

    let receipt = MethodologyReceipt {
        schema_version: "1".to_string(),
        classification: classification.to_string(),
        mode: mode_name(config.enforce),
        summary,
        pr_number: input.pr_number,
        labels,
        contradictions,
        warnings,
    };

    write_receipt(&config, &receipt)?;
    print_receipt(&config, &receipt)?;

    if config.enforce && has_contradictions {
        bail!("methodology gate failed: contradictory PR state detected");
    }

    Ok(())
}

fn mode_name(enforce: bool) -> String {
    if enforce { "enforced".to_string() } else { "advisory".to_string() }
}

fn print_receipt(config: &MethodologyGateConfig, receipt: &MethodologyReceipt) -> Result<()> {
    match config.format {
        MethodologyOutputFormat::Human => {
            println!("Methodology Gate [{}]: {}", receipt.mode, receipt.summary);
            if !receipt.contradictions.is_empty() {
                for contradiction in &receipt.contradictions {
                    println!(
                        " - {}: {} ({})",
                        contradiction.rule_type,
                        contradiction.labels.join(", "),
                        contradiction.reason
                    );
                }
            }
            for warning in &receipt.warnings {
                println!(" - warning: {warning}");
            }
            Ok(())
        }
        MethodologyOutputFormat::Json => {
            let rendered = serde_json::to_string_pretty(receipt)?;
            println!("{rendered}");
            Ok(())
        }
    }
}

fn write_receipt(config: &MethodologyGateConfig, receipt: &MethodologyReceipt) -> Result<()> {
    if config.dry_run {
        return Ok(());
    }

    if let Some(parent) = config.receipt.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let receipt_json = serde_json::to_string_pretty(receipt)?;
    fs::write(&config.receipt, receipt_json)
        .with_context(|| format!("failed to write receipt {}", config.receipt.display()))
}

fn load_input(config: &MethodologyGateConfig) -> Result<InputState> {
    match (&config.fixture, config.pr) {
        (Some(_), Some(_)) => bail!("use either --fixture or --pr, not both"),
        (None, None) => bail!("one of --fixture or --pr is required"),
        (Some(path), None) => load_input_from_fixture(path),
        (None, Some(pr_number)) => load_input_from_pr(pr_number),
    }
}

fn load_input_from_fixture(path: &PathBuf) -> Result<InputState> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read fixture {}", path.display()))?;
    let fixture: FixtureInput = serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse fixture {}", path.display()))?;

    if let Some(pr) = fixture.pull_request {
        let labels = pr.labels.into_iter().map(|label| label.name).collect::<Vec<_>>();
        Ok(InputState {
            pr_number: Some(pr.number),
            labels,
            body: pr.body,
            event_name: fixture.event_name,
        })
    } else {
        Ok(InputState {
            pr_number: None,
            labels: fixture.labels,
            body: fixture.body,
            event_name: fixture.event_name,
        })
    }
}

fn load_input_from_pr(pr_number: u64) -> Result<InputState> {
    let output = std::process::Command::new("gh")
        .args(["pr", "view", &pr_number.to_string(), "--json", "number,body,labels"])
        .output()
        .context("failed to execute gh for PR lookup")?;

    if !output.status.success() {
        bail!(
            "gh PR lookup failed with status {}",
            output.status.code().map_or_else(|| "signal".to_string(), |code| code.to_string())
        );
    }

    let view: GhPrView =
        serde_json::from_slice(&output.stdout).context("failed to decode gh pr view JSON")?;

    let labels = view.labels.into_iter().map(|label| label.name).collect();
    Ok(InputState {
        pr_number: Some(view.number),
        labels,
        body: Some(view.body),
        event_name: Some("pull_request".to_string()),
    })
}

#[derive(Debug, Deserialize)]
struct GhPrView {
    number: u64,
    body: String,
    labels: Vec<LabelObject>,
}

fn should_warn_closeout_hygiene(body: Option<&str>) -> bool {
    static CLOSEOUT_RE: LazyLock<Option<Regex>> =
        LazyLock::new(|| Regex::new(r"(?i)\b(closes|fixes|resolves)\s+#\d+\b").ok());
    static PARTIAL_RE: LazyLock<Option<Regex>> =
        LazyLock::new(|| Regex::new(r"(?i)\b(partial|scaffold|umbrella)\b").ok());

    let Some(body) = body else {
        return false;
    };

    CLOSEOUT_RE.as_ref().is_some_and(|regex| regex.is_match(body))
        && PARTIAL_RE
            .as_ref()
            .is_some_and(|regex| regex.is_match(body))
}

fn glob_matches(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    if let Some(prefix) = pattern.strip_suffix('*') {
        return value.starts_with(prefix);
    }

    if let Some(suffix) = pattern.strip_prefix('*') {
        return value.ends_with(suffix);
    }

    value == pattern
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_pattern_prefix_match() {
        assert!(glob_matches("needs-*", "needs-ci-fix"));
    }

    #[test]
    fn glob_pattern_exact_match() {
        assert!(glob_matches("merge-ready", "merge-ready"));
    }

    #[test]
    fn closeout_warning_requires_both_signals() {
        assert!(should_warn_closeout_hygiene(Some("Partial implementation. Fixes #6855")));
        assert!(!should_warn_closeout_hygiene(Some("Fixes #6855")));
    }
}
