use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_POLICY_PATH: &str = ".ci/policies/label-contradictions.toml";

#[derive(Debug, Clone)]
pub struct MethodologyGateConfig {
    pub fixture: Option<PathBuf>,
    pub pr: Option<u64>,
    pub receipt: Option<PathBuf>,
    pub enforce: bool,
    pub dry_run: bool,
    pub format: MethodologyGateOutputFormat,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum MethodologyGateOutputFormat {
    Human,
    Json,
}

#[derive(Debug, Deserialize)]
struct LabelPolicy {
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

#[derive(Debug, Serialize)]
struct MethodologyReceipt {
    gate: &'static str,
    mode: &'static str,
    classification: &'static str,
    source: &'static str,
    enforce: bool,
    contradictions: Vec<Contradiction>,
    warnings: Vec<String>,
    labels: Vec<String>,
}

#[derive(Debug, Serialize)]
struct Contradiction {
    rule: String,
    reason: String,
    labels: Vec<String>,
}

#[derive(Debug)]
struct InputState {
    labels: BTreeSet<String>,
    body: Option<String>,
    merge_group_lookup_unavailable: bool,
}

pub fn run(config: MethodologyGateConfig) -> Result<()> {
    if config.fixture.is_some() == config.pr.is_some() {
        bail!("provide exactly one of --fixture or --pr");
    }

    let policy_path = crate::utils::project_root()?.join(DEFAULT_POLICY_PATH);
    let policy = read_policy(&policy_path)?;

    let (source, state) = if let Some(fixture_path) = &config.fixture {
        ("fixture", read_input_state_from_fixture(fixture_path)?)
    } else if let Some(pr_number) = config.pr {
        ("pr", read_input_state_from_pr(pr_number)?)
    } else {
        bail!("either --fixture or --pr is required");
    };

    let receipt = evaluate(&policy, state, source, config.enforce);
    emit_output(&receipt, config.format)?;
    write_receipt_if_requested(&receipt, config.receipt.as_deref(), config.dry_run)?;

    if config.enforce && !receipt.contradictions.is_empty() {
        bail!("methodology-gate found contradictory methodology labels");
    }

    Ok(())
}

fn read_policy(path: &Path) -> Result<LabelPolicy> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read policy file {}", path.display()))?;
    let parsed = toml::from_str::<LabelPolicy>(&raw)
        .with_context(|| format!("failed to parse policy file {}", path.display()))?;
    Ok(parsed)
}

fn read_input_state_from_fixture(path: &Path) -> Result<InputState> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read fixture {}", path.display()))?;
    let value: Value = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse fixture {}", path.display()))?;
    input_state_from_value(&value)
}

fn read_input_state_from_pr(pr_number: u64) -> Result<InputState> {
    let repo = std::env::var("GITHUB_REPOSITORY")
        .context("GITHUB_REPOSITORY must be set (example: EffortlessMetrics/perl-lsp)")?;
    let endpoint = format!("repos/{repo}/pulls/{pr_number}");
    let output = Command::new("gh")
        .args(["api", endpoint.as_str()])
        .output()
        .context("failed to execute `gh api` to read PR labels")?;

    if !output.status.success() {
        bail!(
            "gh api request failed while reading PR {}: {}",
            pr_number,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let body = String::from_utf8(output.stdout)
        .context("gh api returned non-UTF8 output while reading PR")?;
    let value: Value = serde_json::from_str(&body)
        .context("failed to parse gh api JSON payload for PR")?;
    input_state_from_value(&value)
}

fn input_state_from_value(value: &Value) -> Result<InputState> {
    let labels = extract_labels(value)?;
    let body = value.get("body").and_then(Value::as_str).map(ToString::to_string);
    let merge_group_lookup_unavailable = value
        .get("merge_group_lookup_unavailable")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    Ok(InputState {
        labels,
        body,
        merge_group_lookup_unavailable,
    })
}

fn extract_labels(value: &Value) -> Result<BTreeSet<String>> {
    let labels_value = value
        .get("labels")
        .ok_or_else(|| color_eyre::eyre::eyre!("input must include a top-level `labels` array"))?;
    let labels = labels_value
        .as_array()
        .ok_or_else(|| color_eyre::eyre::eyre!("input field `labels` must be an array"))?;

    let mut out = BTreeSet::new();
    for label in labels {
        if let Some(name) = label.as_str() {
            out.insert(name.to_string());
            continue;
        }
        if let Some(name) = label.get("name").and_then(Value::as_str) {
            out.insert(name.to_string());
            continue;
        }
        bail!("label entries must be strings or objects with a `name` field");
    }
    Ok(out)
}

fn evaluate(
    policy: &LabelPolicy,
    state: InputState,
    source: &'static str,
    enforce: bool,
) -> MethodologyReceipt {
    if state.merge_group_lookup_unavailable {
        return MethodologyReceipt {
            gate: "methodology-gate",
            mode: if enforce { "enforce" } else { "advisory" },
            classification: "unknown",
            source,
            enforce,
            contradictions: Vec::new(),
            warnings: vec![
                "merge_group label lookup unavailable; enforcement is deferred to pull_request runs"
                    .to_string(),
            ],
            labels: state.labels.into_iter().collect(),
        };
    }

    let mut contradictions = Vec::new();
    for rule in &policy.forbidden {
        let matched: Vec<String> = rule
            .labels
            .iter()
            .filter(|label| state.labels.contains(*label))
            .cloned()
            .collect();
        if matched.len() == rule.labels.len() {
            contradictions.push(Contradiction {
                rule: format!("all({})", rule.labels.join(", ")),
                reason: rule.reason.clone(),
                labels: matched,
            });
        }
    }

    for rule in &policy.forbidden_pattern {
        if !state.labels.contains(&rule.required) {
            continue;
        }
        let prefix = rule.forbidden_glob.strip_suffix('*');
        let matched: Vec<String> = state
            .labels
            .iter()
            .filter(|label| {
                if let Some(prefix) = prefix {
                    label.starts_with(prefix)
                } else {
                    *label == &rule.forbidden_glob
                }
            })
            .cloned()
            .collect();

        if !matched.is_empty() {
            let mut labels = Vec::with_capacity(matched.len() + 1);
            labels.push(rule.required.clone());
            labels.extend(matched);
            contradictions.push(Contradiction {
                rule: format!("{} + {}", rule.required, rule.forbidden_glob),
                reason: rule.reason.clone(),
                labels,
            });
        }
    }

    let mut warnings = Vec::new();
    if let Some(body) = &state.body
        && has_closeout_hygiene_risk(body)
    {
        warnings.push(
            "PR body uses closing keywords with partial/scaffold/umbrella language; prefer Refs/Part of for partial work".to_string(),
        );
    }

    let classification = if contradictions.is_empty() {
        "pass"
    } else if enforce {
        "fail"
    } else {
        "warn"
    };

    MethodologyReceipt {
        gate: "methodology-gate",
        mode: if enforce { "enforce" } else { "advisory" },
        classification,
        source,
        enforce,
        contradictions,
        warnings,
        labels: state.labels.into_iter().collect(),
    }
}

fn has_closeout_hygiene_risk(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    let has_closer = ["closes", "fixes", "resolves"].iter().any(|token| lower.contains(token));
    let has_partial = ["partial", "scaffold", "umbrella"]
        .iter()
        .any(|token| lower.contains(token));
    has_closer && has_partial
}

fn emit_output(receipt: &MethodologyReceipt, format: MethodologyGateOutputFormat) -> Result<()> {
    match format {
        MethodologyGateOutputFormat::Human => {
            println!(
                "Methodology Gate ({}) => {}",
                receipt.mode,
                receipt.classification
            );
            if receipt.contradictions.is_empty() {
                println!("No contradictory label states detected.");
            } else {
                println!("Contradictions:");
                for contradiction in &receipt.contradictions {
                    println!(
                        "- {} [{}]",
                        contradiction.reason,
                        contradiction.labels.join(", ")
                    );
                }
            }
            for warning in &receipt.warnings {
                println!("warning: {warning}");
            }
        }
        MethodologyGateOutputFormat::Json => {
            let json = serde_json::to_string_pretty(receipt)
                .context("failed to serialize methodology gate output JSON")?;
            println!("{json}");
        }
    }

    Ok(())
}

fn write_receipt_if_requested(
    receipt: &MethodologyReceipt,
    receipt_path: Option<&Path>,
    dry_run: bool,
) -> Result<()> {
    if let Some(path) = receipt_path {
        if dry_run {
            println!("dry-run: skipping receipt write to {}", path.display());
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create receipt directory {}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(receipt)
            .context("failed to serialize methodology gate receipt")?;
        fs::write(path, format!("{json}\n"))
            .with_context(|| format!("failed to write receipt {}", path.display()))?;
        println!("receipt: {}", path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_pair_and_pattern_contradictions() -> Result<()> {
        let policy: LabelPolicy = toml::from_str(
            r#"
            [[forbidden]]
            labels = ["review-reviewed", "needs-builder-fix"]
            reason = "mutually exclusive"

            [[forbidden_pattern]]
            required = "merge-ready"
            forbidden_glob = "needs-*"
            reason = "cannot merge with blockers"
            "#,
        )?;

        let state = InputState {
            labels: BTreeSet::from([
                "review-reviewed".to_string(),
                "needs-builder-fix".to_string(),
                "merge-ready".to_string(),
                "needs-ci-fix".to_string(),
            ]),
            body: None,
            merge_group_lookup_unavailable: false,
        };

        let receipt = evaluate(&policy, state, "fixture", true);
        assert_eq!(receipt.classification, "fail");
        assert_eq!(receipt.contradictions.len(), 2);
        Ok(())
    }

    #[test]
    fn unknown_when_merge_group_lookup_is_unavailable() -> Result<()> {
        let policy: LabelPolicy = toml::from_str("")?;
        let state = InputState {
            labels: BTreeSet::new(),
            body: None,
            merge_group_lookup_unavailable: true,
        };

        let receipt = evaluate(&policy, state, "fixture", false);
        assert_eq!(receipt.classification, "unknown");
        Ok(())
    }
}
