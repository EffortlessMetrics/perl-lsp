use color_eyre::eyre::{Context, Result, eyre};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const PLAYBOOKS_PATH: &str = ".ci/fix-forward/playbooks.toml";

#[derive(Debug, Deserialize)]
struct PlaybookCatalog {
    #[serde(flatten)]
    playbooks: BTreeMap<String, Playbook>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Playbook {
    pub safe_auto_fix: bool,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub route: Option<String>,
    #[serde(default)]
    pub mutation: Option<String>,
    #[serde(default)]
    pub branch_prefix: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FixForwardReceipt {
    pub classification: String,
    pub fix_forward_kind: String,
    pub safe_auto_fix: bool,
    pub command: Option<String>,
    pub route: Option<String>,
    pub evidence: Vec<String>,
    pub next_agent: String,
}

pub fn classify(receipt: PathBuf, output: PathBuf) -> Result<()> {
    let raw = fs::read_to_string(&receipt)
        .with_context(|| format!("Failed to read receipt {}", receipt.display()))?;
    let receipt_json: Value = serde_json::from_str(&raw)
        .with_context(|| format!("Failed to parse receipt JSON {}", receipt.display()))?;

    let catalog = load_playbooks()?;
    let (classification, evidence) = classify_kind(&receipt_json);
    let playbook = catalog.playbooks.get(&classification).ok_or_else(|| {
        eyre!("No playbook found for classification `{classification}` in {}", PLAYBOOKS_PATH)
    })?;

    let next_agent = if playbook.safe_auto_fix {
        "builder".to_string()
    } else {
        playbook.route.clone().unwrap_or_else(|| "triage".to_string())
    };

    let fix_forward_receipt = FixForwardReceipt {
        classification: classification.clone(),
        fix_forward_kind: classification,
        safe_auto_fix: playbook.safe_auto_fix,
        command: playbook.command.clone(),
        route: playbook.route.clone(),
        evidence,
        next_agent,
    };

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory {}", parent.display()))?;
    }
    let rendered = serde_json::to_string_pretty(&fix_forward_receipt)
        .context("Failed to render fix-forward receipt JSON")?;
    fs::write(&output, format!("{rendered}\n"))
        .with_context(|| format!("Failed to write {}", output.display()))?;

    Ok(())
}

pub fn list_playbooks() -> Result<()> {
    let catalog = load_playbooks()?;
    for (kind, playbook) in catalog.playbooks {
        let action = playbook
            .command
            .clone()
            .or(playbook.route.clone())
            .or(playbook.mutation.clone())
            .unwrap_or_else(|| "manual".to_string());
        println!("{kind}\tsafe_auto_fix={}\taction={action}", playbook.safe_auto_fix);
    }
    Ok(())
}

fn load_playbooks() -> Result<PlaybookCatalog> {
    let root = crate::utils::project_root()?;
    let path = root.join(PLAYBOOKS_PATH);
    load_playbooks_from_path(&path)
}

fn load_playbooks_from_path(path: &Path) -> Result<PlaybookCatalog> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read playbooks from {}", path.display()))?;
    toml::from_str(&raw)
        .with_context(|| format!("Failed to parse playbooks TOML {}", path.display()))
}

fn classify_kind(receipt: &Value) -> (String, Vec<String>) {
    if let Some(classification) = receipt.get("classification").and_then(Value::as_str) {
        return (classification.to_string(), vec!["input.classification".to_string()]);
    }

    let mut evidence = Vec::new();

    let blocking_failures = receipt
        .get("summary")
        .and_then(|summary| summary.get("blocking_failures"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if blocking_failures
        .iter()
        .any(|gate| gate.as_str().is_some_and(|name| name == "stale-base-classifier"))
    {
        evidence.push("summary.blocking_failures contains stale-base-classifier".to_string());
        return ("STALE_BASE_CASCADE".to_string(), evidence);
    }

    let failing_gates = receipt
        .get("gates")
        .and_then(Value::as_array)
        .map(|gates| {
            gates
                .iter()
                .filter(|gate| {
                    gate.get("status").and_then(Value::as_str).is_some_and(|status| {
                        status == "fail" || status == "error" || status == "timeout"
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    for gate in &failing_gates {
        let gate_name = gate.get("gate_name").and_then(Value::as_str).unwrap_or_default();
        let output_summary = gate
            .get("output_summary")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_ascii_lowercase();

        if gate_name == "fmt" || output_summary.contains("cargo fmt") {
            evidence.push(format!("failing gate `{gate_name}` indicates formatting drift"));
            return ("FMT_ONLY".to_string(), evidence);
        }

        if gate_name.contains("title") {
            evidence.push(format!("failing gate `{gate_name}` validates title"));
            return ("TITLE_FIX".to_string(), evidence);
        }

        if gate_name.contains("doc") || output_summary.contains("generated docs") {
            evidence.push(format!("failing gate `{gate_name}` requires generated docs refresh"));
            return ("GENERATED_DOC_REGEN".to_string(), evidence);
        }

        if gate_name.contains("infra") {
            evidence.push(format!("failing gate `{gate_name}` marked infrastructure advisory"));
            return ("INFRA_ADVISORY_DEMOTION".to_string(), evidence);
        }

        if gate_name.contains("parser") && gate_name.contains("ratchet") {
            evidence.push(format!("failing gate `{gate_name}` is parser ratchet"));
            return ("PARSER_RATCHET_REGRESSION".to_string(), evidence);
        }
    }

    evidence.push("no typed classifier matched; defaulting to STALE_BASE_CASCADE".to_string());
    ("STALE_BASE_CASCADE".to_string(), evidence)
}

#[cfg(test)]
mod tests {
    use super::classify_kind;

    #[test]
    fn classify_prefers_explicit_classification() {
        let receipt = serde_json::json!({"classification": "STALE_BASE_CASCADE"});
        let (kind, evidence) = classify_kind(&receipt);
        assert_eq!(kind, "STALE_BASE_CASCADE");
        assert_eq!(evidence.first().map(String::as_str), Some("input.classification"));
    }

    #[test]
    fn classify_fmt_gate() {
        let receipt = serde_json::json!({
            "gates": [
                {"gate_name": "fmt", "status": "fail", "output_summary": ""}
            ]
        });
        let (kind, _) = classify_kind(&receipt);
        assert_eq!(kind, "FMT_ONLY");
    }

    #[test]
    fn classify_generated_doc_gate() {
        let receipt = serde_json::json!({
            "gates": [
                {"gate_name": "status-docs", "status": "fail", "output_summary": "generated docs changed"}
            ]
        });
        let (kind, _) = classify_kind(&receipt);
        assert_eq!(kind, "GENERATED_DOC_REGEN");
    }
}
