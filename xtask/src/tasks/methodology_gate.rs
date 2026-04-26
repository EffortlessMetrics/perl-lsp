use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};

const DEFAULT_POLICY_PATH: &str = ".ci/policies/label-contradictions.toml";

#[derive(Debug, Clone)]
pub struct MethodologyGateConfig {
    pub fixture: Option<PathBuf>,
    pub pr: Option<u64>,
    pub receipt: Option<PathBuf>,
    pub dry_run: bool,
    pub enforce: bool,
    pub format_json: bool,
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
struct FixturePayload {
    #[serde(default)]
    pr_number: Option<u64>,
    #[serde(default)]
    event_name: Option<String>,
    #[serde(default)]
    body: String,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    labels_available: bool,
}

#[derive(Debug, Clone, Serialize)]
struct MethodologyGateReceipt {
    schema_version: u32,
    classification: String,
    mode: String,
    source: String,
    pr_number: Option<u64>,
    event_name: Option<String>,
    labels_available: bool,
    labels: Vec<String>,
    violations: Vec<Violation>,
    closeout_hygiene_warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct Violation {
    kind: String,
    labels: Vec<String>,
    reason: String,
}

pub fn run(config: MethodologyGateConfig) -> Result<()> {
    if config.fixture.is_some() == config.pr.is_some() {
        bail!("pass exactly one of --fixture <json> or --pr <number>");
    }

    let root = crate::utils::project_root()?;
    let policy = load_policy(&root.join(DEFAULT_POLICY_PATH))?;
    let context = load_context(&root, &config)?;

    let unknown = is_unknown_context(&context);
    let violations = if unknown { vec![] } else { find_violations(&policy, &context.labels) };
    let closeout_hygiene_warnings = find_closeout_hygiene_warnings(&context.body);

    let classification = if unknown {
        "unknown"
    } else if violations.is_empty() {
        "clean"
    } else {
        "contradiction"
    };

    let receipt = MethodologyGateReceipt {
        schema_version: 1,
        classification: classification.to_string(),
        mode: if config.enforce { "enforce" } else { "advisory" }.to_string(),
        source: context.source,
        pr_number: context.pr_number,
        event_name: context.event_name,
        labels_available: context.labels_available,
        labels: sorted_labels(context.labels),
        violations,
        closeout_hygiene_warnings,
    };

    if let Some(path) = config.receipt.as_ref() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating receipt directory {}", parent.display()))?;
        }
        let payload = serde_json::to_string_pretty(&receipt)?;
        fs::write(path, format!("{payload}\n"))
            .with_context(|| format!("writing receipt {}", path.display()))?;
    }

    emit_output(&receipt, config.format_json)?;

    if config.dry_run {
        return Ok(());
    }

    if config.enforce && !unknown && !receipt.violations.is_empty() {
        bail!("methodology gate failed: contradictory labels detected");
    }

    Ok(())
}

fn emit_output(receipt: &MethodologyGateReceipt, format_json: bool) -> Result<()> {
    if format_json {
        println!("{}", serde_json::to_string_pretty(receipt)?);
        return Ok(());
    }

    println!("Methodology Gate");
    println!("  classification: {}", receipt.classification);
    println!("  mode: {}", receipt.mode);
    println!("  source: {}", receipt.source);
    if let Some(pr_number) = receipt.pr_number {
        println!("  pr: #{}", pr_number);
    }
    if let Some(event_name) = receipt.event_name.as_ref() {
        println!("  event: {}", event_name);
    }

    if receipt.violations.is_empty() {
        println!("  contradictions: none");
    } else {
        println!("  contradictions: {}", receipt.violations.len());
        for violation in &receipt.violations {
            println!("    - {} ({})", violation.labels.join(" + "), violation.reason);
        }
    }

    if !receipt.closeout_hygiene_warnings.is_empty() {
        println!("  closeout hygiene warnings:");
        for warning in &receipt.closeout_hygiene_warnings {
            println!("    - {warning}");
        }
    }

    Ok(())
}

#[derive(Debug)]
struct GateContext {
    source: String,
    pr_number: Option<u64>,
    event_name: Option<String>,
    body: String,
    labels: Vec<String>,
    labels_available: bool,
}

fn is_unknown_context(context: &GateContext) -> bool {
    matches!(context.event_name.as_deref(), Some("merge_group")) && !context.labels_available
}

fn load_context(root: &Path, config: &MethodologyGateConfig) -> Result<GateContext> {
    if let Some(fixture_path) = config.fixture.as_ref() {
        return load_fixture_context(fixture_path);
    }

    if let Some(pr_number) = config.pr {
        return load_pr_context(root, pr_number);
    }

    bail!("either --fixture or --pr is required")
}

fn load_fixture_context(path: &Path) -> Result<GateContext> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("reading fixture {}", path.display()))?;
    let payload: FixturePayload = serde_json::from_str(&raw)
        .with_context(|| format!("parsing fixture {}", path.display()))?;

    Ok(GateContext {
        source: format!("fixture:{}", path.display()),
        pr_number: payload.pr_number,
        event_name: payload.event_name,
        body: payload.body,
        labels: payload.labels,
        labels_available: payload.labels_available,
    })
}

fn load_pr_context(root: &Path, pr_number: u64) -> Result<GateContext> {
    let output = Command::new("gh")
        .args(["pr", "view", &pr_number.to_string(), "--json", "number,body,labels"])
        .current_dir(root)
        .output()
        .context("failed to run `gh pr view`")?;

    if !output.status.success() {
        bail!(
            "failed to query PR #{pr_number}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    #[derive(Deserialize)]
    struct GhResponse {
        number: u64,
        #[serde(default)]
        body: String,
        #[serde(default)]
        labels: Vec<GhLabel>,
    }

    #[derive(Deserialize)]
    struct GhLabel {
        name: String,
    }

    let parsed: GhResponse =
        serde_json::from_slice(&output.stdout).context("parsing gh pr view output")?;
    let labels = parsed.labels.into_iter().map(|entry| entry.name).collect();

    Ok(GateContext {
        source: format!("pr:{pr_number}"),
        pr_number: Some(parsed.number),
        event_name: std::env::var("GITHUB_EVENT_NAME").ok(),
        body: parsed.body,
        labels,
        labels_available: true,
    })
}

fn load_policy(path: &Path) -> Result<Policy> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("reading policy {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("parsing policy {}", path.display()))
}

fn find_violations(policy: &Policy, labels: &[String]) -> Vec<Violation> {
    let mut violations = Vec::new();

    for rule in &policy.forbidden {
        if rule.labels.iter().all(|needle| labels.iter().any(|label| label == needle)) {
            violations.push(Violation {
                kind: "forbidden".to_string(),
                labels: rule.labels.clone(),
                reason: rule.reason.clone(),
            });
        }
    }

    for rule in &policy.forbidden_pattern {
        if !labels.iter().any(|label| label == &rule.required) {
            continue;
        }

        let forbidden_labels: Vec<String> = labels
            .iter()
            .filter(|label| label_matches_glob(label, &rule.forbidden_glob))
            .cloned()
            .collect();

        if !forbidden_labels.is_empty() {
            let mut labels_with_required = vec![rule.required.clone()];
            labels_with_required.extend(forbidden_labels);
            violations.push(Violation {
                kind: "forbidden_pattern".to_string(),
                labels: labels_with_required,
                reason: rule.reason.clone(),
            });
        }
    }

    violations
}

fn find_closeout_hygiene_warnings(body: &str) -> Vec<String> {
    let body_lower = body.to_ascii_lowercase();
    if !body_lower.contains("partial")
        && !body_lower.contains("scaffold")
        && !body_lower.contains("umbrella")
    {
        return vec![];
    }

    if body_lower.contains("closes #")
        || body_lower.contains("fixes #")
        || body_lower.contains("resolves #")
    {
        return vec![
            "PR body appears partial/scaffold/umbrella but uses Closes/Fixes/Resolves; prefer Refs/Part of until fully complete"
                .to_string(),
        ];
    }

    vec![]
}

fn label_matches_glob(label: &str, glob: &str) -> bool {
    if let Some(prefix) = glob.strip_suffix('*') {
        return label.starts_with(prefix);
    }
    label == glob
}

fn sorted_labels(mut labels: Vec<String>) -> Vec<String> {
    labels.sort();
    labels
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::Result;

    #[test]
    fn detects_fixture_contradiction() -> Result<()> {
        let root = crate::utils::project_root()?;
        let policy = load_policy(&root.join(DEFAULT_POLICY_PATH))?;
        let fixture_path =
            root.join("xtask/tests/fixtures/methodology/review-plus-needs-builder.json");
        let context = load_fixture_context(&fixture_path)?;
        let violations = find_violations(&policy, &context.labels);
        assert!(!violations.is_empty());
        Ok(())
    }

    #[test]
    fn clean_fixture_has_no_violations() -> Result<()> {
        let root = crate::utils::project_root()?;
        let policy = load_policy(&root.join(DEFAULT_POLICY_PATH))?;
        let fixture_path = root.join("xtask/tests/fixtures/methodology/clean.json");
        let context = load_fixture_context(&fixture_path)?;
        let violations = find_violations(&policy, &context.labels);
        assert!(violations.is_empty());
        Ok(())
    }

    #[test]
    fn partial_closeout_warns_on_fixes() {
        let warnings = find_closeout_hygiene_warnings(
            "This is a partial implementation. Fixes #6855 because reasons.",
        );
        assert_eq!(warnings.len(), 1);
    }
}
