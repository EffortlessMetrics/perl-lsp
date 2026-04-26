use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

const PLAYBOOKS_PATH: &str = ".ci/fix-forward/playbooks.toml";

#[derive(Debug, Deserialize)]
struct PlaybookCatalog {
    playbooks: Vec<Playbook>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Playbook {
    kind: String,
    safe_auto_fix: bool,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    route: Option<String>,
    #[serde(default)]
    mutation: Option<String>,
    #[serde(default)]
    branch_prefix: Option<String>,
    #[serde(default)]
    next_agent: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FixForwardReceipt {
    classification: String,
    fix_forward_kind: String,
    safe_auto_fix: bool,
    command: Option<String>,
    route: Option<String>,
    evidence: Vec<String>,
    next_agent: String,
}

pub fn classify(receipt: PathBuf, output: PathBuf) -> Result<()> {
    let raw = fs::read_to_string(&receipt)
        .with_context(|| format!("reading receipt {}", receipt.display()))?;
    let value: Value =
        serde_json::from_str(&raw).with_context(|| format!("parsing {}", receipt.display()))?;

    let catalog = load_playbooks()?;
    let (kind, evidence) = classify_kind(&value);
    let Some(playbook) = catalog.playbooks.iter().find(|entry| entry.kind == kind) else {
        bail!("no playbook configured for classified kind: {kind}");
    };

    let classified = FixForwardReceipt {
        classification: kind.clone(),
        fix_forward_kind: kind,
        safe_auto_fix: playbook.safe_auto_fix,
        command: playbook.command.clone(),
        route: playbook.route.clone(),
        evidence,
        next_agent: resolve_next_agent(playbook),
    };

    write_json(&output, &classified)
}

pub fn list_playbooks() -> Result<()> {
    let catalog = load_playbooks()?;
    for playbook in catalog.playbooks {
        println!(
            "{}\tsafe_auto_fix={}\tcommand={}\troute={}\tmutation={}\tbranch_prefix={}",
            playbook.kind,
            playbook.safe_auto_fix,
            playbook.command.as_deref().unwrap_or("-"),
            playbook.route.as_deref().unwrap_or("-"),
            playbook.mutation.as_deref().unwrap_or("-"),
            playbook.branch_prefix.as_deref().unwrap_or("-"),
        );
    }
    Ok(())
}

fn resolve_next_agent(playbook: &Playbook) -> String {
    if let Some(agent) = &playbook.next_agent {
        return agent.clone();
    }
    if playbook.safe_auto_fix {
        "fix-forward-agent".to_string()
    } else {
        "triage-agent".to_string()
    }
}

fn write_json(path: &Path, payload: &FixForwardReceipt) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    let rendered =
        serde_json::to_string_pretty(payload).context("serializing fix-forward output")?;
    fs::write(path, format!("{rendered}\n")).with_context(|| format!("writing {}", path.display()))
}

fn load_playbooks() -> Result<PlaybookCatalog> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| color_eyre::eyre::eyre!("cannot resolve repository root"))?;
    let path = root.join(PLAYBOOKS_PATH);
    let raw = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}

fn classify_kind(receipt: &Value) -> (String, Vec<String>) {
    if is_stale_base(receipt) {
        return (
            "STALE_BASE_CASCADE".to_string(),
            vec!["agent_receipt indicates stale base or stale-base lane failure".to_string()],
        );
    }

    let failed_gates = failed_gates(receipt);
    if is_fmt_only_failure(&failed_gates) {
        return (
            "FMT_ONLY".to_string(),
            vec!["only fmt gate failed while all other required gates passed/skipped".to_string()],
        );
    }

    if is_title_failure(&failed_gates) {
        return (
            "TITLE_FIX".to_string(),
            vec!["failing gate output references title validation format".to_string()],
        );
    }

    if is_generated_docs_regen(&failed_gates) {
        return (
            "GENERATED_DOC_REGEN".to_string(),
            vec!["failure points to generated docs/status doc drift".to_string()],
        );
    }

    if is_parser_ratchet_regression(&failed_gates) {
        return (
            "PARSER_RATCHET_REGRESSION".to_string(),
            vec!["parser-related gate output references ratchet regression".to_string()],
        );
    }

    (
        "INFRA_ADVISORY_DEMOTION".to_string(),
        vec!["defaulted to infra advisory lane after no narrower classifier matched".to_string()],
    )
}

fn failed_gates(receipt: &Value) -> Vec<&Value> {
    receipt
        .get("gates")
        .and_then(Value::as_array)
        .map(|gates| {
            gates
                .iter()
                .filter(|gate| gate.get("status").and_then(Value::as_str) == Some("fail"))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn is_fmt_only_failure(failed_gates: &[&Value]) -> bool {
    if failed_gates.len() != 1 {
        return false;
    }
    let gate = failed_gates[0];
    gate.get("gate_name")
        .and_then(Value::as_str)
        .map(|name| name == "fmt" || name.contains("fmt"))
        .unwrap_or(false)
}

fn is_stale_base(receipt: &Value) -> bool {
    if receipt.pointer("/agent_receipt/is_latest").and_then(Value::as_bool) == Some(false) {
        return true;
    }

    let Some(failures) = receipt.pointer("/agent_receipt/failures").and_then(Value::as_array)
    else {
        return false;
    };

    failures.iter().any(|failure| {
        let lane = failure.get("lane").and_then(Value::as_str).unwrap_or_default();
        let summary = failure.get("summary").and_then(Value::as_str).unwrap_or_default();
        lane.contains("stale-base") || summary.to_lowercase().contains("stale base")
    })
}

fn is_title_failure(failed_gates: &[&Value]) -> bool {
    failed_gates.iter().any(|gate| {
        let name = gate.get("gate_name").and_then(Value::as_str).unwrap_or_default();
        let summary = gate.get("output_summary").and_then(Value::as_str).unwrap_or_default();
        name.contains("title") || summary.to_lowercase().contains("validate-title")
    })
}

fn is_generated_docs_regen(failed_gates: &[&Value]) -> bool {
    failed_gates.iter().any(|gate| {
        let name = gate.get("gate_name").and_then(Value::as_str).unwrap_or_default();
        let summary = gate.get("output_summary").and_then(Value::as_str).unwrap_or_default();
        name.contains("docs")
            || summary.to_lowercase().contains("generated docs")
            || summary.to_lowercase().contains("status doc")
    })
}

fn is_parser_ratchet_regression(failed_gates: &[&Value]) -> bool {
    failed_gates.iter().any(|gate| {
        let name = gate.get("gate_name").and_then(Value::as_str).unwrap_or_default();
        let summary = gate.get("output_summary").and_then(Value::as_str).unwrap_or_default();
        (name.contains("parser") || name.contains("ratchet"))
            && summary.to_lowercase().contains("regression")
    })
}
