use color_eyre::eyre::{Context, ContextCompat, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use crate::utils::project_root;

const PLAYBOOKS_PATH: &str = ".ci/fix-forward/playbooks.toml";

#[derive(Debug, Clone, Deserialize)]
struct PlaybooksFile {
    playbook: Vec<Playbook>,
}

#[derive(Debug, Clone, Deserialize)]
struct Playbook {
    kind: String,
    safe_auto_fix: bool,
    command: Option<String>,
    route: Option<String>,
    mutation: Option<String>,
    branch_prefix: Option<String>,
    next_agent: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ClassifyConfig {
    pub receipt: PathBuf,
    pub output: PathBuf,
}

#[derive(Debug, Serialize)]
struct FixForwardReceipt {
    classification: String,
    fix_forward_kind: String,
    safe_auto_fix: bool,
    command: Option<String>,
    route: Option<String>,
    evidence: Vec<String>,
    next_agent: Option<String>,
}

pub fn classify(config: ClassifyConfig) -> Result<()> {
    let receipt_text = fs::read_to_string(&config.receipt)
        .with_context(|| format!("Failed reading receipt {}", config.receipt.display()))?;
    let receipt: Value = serde_json::from_str(&receipt_text)
        .with_context(|| format!("Failed parsing receipt JSON {}", config.receipt.display()))?;

    let playbooks = load_playbooks()?;
    let (kind, evidence) = classify_receipt(&receipt);
    let playbook =
        playbooks.get(&kind).with_context(|| format!("Missing fix-forward playbook for {kind}"))?;

    let out = FixForwardReceipt {
        classification: kind.clone(),
        fix_forward_kind: kind,
        safe_auto_fix: playbook.safe_auto_fix,
        command: playbook.command.clone(),
        route: playbook.route.clone(),
        evidence,
        next_agent: playbook.next_agent.clone(),
    };

    let parent = config
        .output
        .parent()
        .with_context(|| format!("Output path has no parent: {}", config.output.display()))?;
    fs::create_dir_all(parent)
        .with_context(|| format!("Failed creating output directory {}", parent.display()))?;

    let out_json =
        serde_json::to_string_pretty(&out).context("Failed serializing fix-forward receipt")?;
    fs::write(&config.output, out_json)
        .with_context(|| format!("Failed writing output {}", config.output.display()))?;

    Ok(())
}

pub fn list_playbooks() -> Result<()> {
    let playbooks = load_playbooks()?;
    for playbook in playbooks.values() {
        println!(
            "{}\tsafe_auto_fix={}\tcommand={}\troute={}\tmutation={}\tbranch_prefix={}\tnext_agent={}",
            playbook.kind,
            playbook.safe_auto_fix,
            playbook.command.as_deref().unwrap_or("-"),
            playbook.route.as_deref().unwrap_or("-"),
            playbook.mutation.as_deref().unwrap_or("-"),
            playbook.branch_prefix.as_deref().unwrap_or("-"),
            playbook.next_agent.as_deref().unwrap_or("-")
        );
    }
    Ok(())
}

fn load_playbooks() -> Result<BTreeMap<String, Playbook>> {
    let root = project_root()?;
    let path = root.join(PLAYBOOKS_PATH);
    let text = fs::read_to_string(&path)
        .with_context(|| format!("Failed reading playbooks {}", path.display()))?;
    let parsed: PlaybooksFile =
        toml::from_str(&text).with_context(|| format!("Failed parsing {}", path.display()))?;

    let mut map = BTreeMap::new();
    for playbook in parsed.playbook {
        if map.insert(playbook.kind.clone(), playbook).is_some() {
            bail!("Duplicate playbook kind in {}", path.display());
        }
    }

    Ok(map)
}

fn classify_receipt(receipt: &Value) -> (String, Vec<String>) {
    let mut evidence = Vec::new();

    if has_fmt_only_failure(receipt, &mut evidence) {
        return ("FMT_ONLY".to_string(), evidence);
    }

    if has_title_failure(receipt, &mut evidence) {
        return ("TITLE_FIX".to_string(), evidence);
    }

    if has_stale_base_cascade(receipt, &mut evidence) {
        return ("STALE_BASE_CASCADE".to_string(), evidence);
    }

    if has_generated_docs_failure(receipt, &mut evidence) {
        return ("GENERATED_DOC_REGEN".to_string(), evidence);
    }

    if has_infra_advisory_failure(receipt, &mut evidence) {
        return ("INFRA_ADVISORY_DEMOTION".to_string(), evidence);
    }

    if has_parser_ratchet_regression(receipt, &mut evidence) {
        return ("PARSER_RATCHET_REGRESSION".to_string(), evidence);
    }

    evidence.push("No typed classifier matched; defaulting to INFRA_ADVISORY_DEMOTION".to_string());
    ("INFRA_ADVISORY_DEMOTION".to_string(), evidence)
}

fn has_fmt_only_failure(receipt: &Value, evidence: &mut Vec<String>) -> bool {
    let failed = failed_gates(receipt);
    if failed.len() != 1 {
        return false;
    }

    let gate = failed.first();
    let Some(gate) = gate else {
        return false;
    };

    if has_keyword(gate, &["fmt"]) {
        evidence.push("Single failing gate matches fmt".to_string());
        return true;
    }

    false
}

fn has_title_failure(receipt: &Value, evidence: &mut Vec<String>) -> bool {
    let failed = failed_gates(receipt);
    if failed.iter().any(|gate| has_keyword(gate, &["title", "validate-title"])) {
        evidence.push("Detected title validation failure".to_string());
        return true;
    }

    false
}

fn has_stale_base_cascade(receipt: &Value, evidence: &mut Vec<String>) -> bool {
    let failed = failed_gates(receipt);
    if failed.iter().any(|gate| {
        has_keyword(gate, &["stale base", "stale-base", "merge-base", "out of date", "rebase"])
    }) {
        evidence.push("Detected stale-base or merge-base drift failure signal".to_string());
        return true;
    }

    false
}

fn has_generated_docs_failure(receipt: &Value, evidence: &mut Vec<String>) -> bool {
    let failed = failed_gates(receipt);
    if failed.iter().any(|gate| {
        has_keyword(gate, &["status-docs", "generated docs", "status docs", "doc regen"])
    }) {
        evidence.push("Detected generated docs/status docs drift".to_string());
        return true;
    }

    false
}

fn has_infra_advisory_failure(receipt: &Value, evidence: &mut Vec<String>) -> bool {
    let failed = failed_gates(receipt);
    if failed.iter().any(|gate| {
        has_keyword(gate, &["network", "github api", "timeout", "rate limit", "runner"])
    }) {
        evidence.push("Detected infrastructure/transient failure signal".to_string());
        return true;
    }

    false
}

fn has_parser_ratchet_regression(receipt: &Value, evidence: &mut Vec<String>) -> bool {
    let failed = failed_gates(receipt);
    if failed
        .iter()
        .any(|gate| has_keyword(gate, &["parser", "ratchet", "corpus", "ga_alignment", "nodekind"]))
    {
        evidence.push("Detected parser ratchet/corpus regression signal".to_string());
        return true;
    }

    false
}

fn failed_gates(receipt: &Value) -> Vec<&Map<String, Value>> {
    let Some(gates) = receipt.get("gates").and_then(Value::as_array) else {
        return Vec::new();
    };

    gates
        .iter()
        .filter_map(Value::as_object)
        .filter(|gate| {
            gate.get("status")
                .and_then(Value::as_str)
                .is_some_and(|status| matches!(status, "fail" | "error" | "timeout"))
        })
        .collect()
}

fn has_keyword(gate: &Map<String, Value>, keywords: &[&str]) -> bool {
    let mut haystacks = Vec::new();
    for key in ["gate_name", "command", "output_summary"] {
        if let Some(value) = gate.get(key).and_then(Value::as_str) {
            haystacks.push(value.to_ascii_lowercase());
        }
    }

    keywords.iter().any(|keyword| haystacks.iter().any(|field| field.contains(keyword)))
}
