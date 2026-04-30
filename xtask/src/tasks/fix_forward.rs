use color_eyre::eyre::{Context, Result, eyre};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use crate::utils::project_root;

#[derive(Debug, Deserialize)]
struct PlaybookConfig {
    playbooks: Vec<Playbook>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Playbook {
    kind: String,
    safe_auto_fix: bool,
    command: Option<String>,
    route: Option<String>,
    mutation: Option<String>,
    branch_prefix: Option<String>,
    #[serde(default)]
    match_gate_names: Vec<String>,
    #[serde(default)]
    match_output_contains: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Receipt {
    #[serde(default)]
    gates: Vec<ReceiptGate>,
}

#[derive(Debug, Deserialize)]
struct ReceiptGate {
    gate_name: String,
    status: String,
    #[serde(default)]
    output_summary: Option<String>,
    #[serde(default)]
    command: Option<String>,
}

#[derive(Debug, Serialize)]
struct FixForwardReceipt {
    classification: String,
    fix_forward_kind: String,
    safe_auto_fix: bool,
    command: Option<String>,
    route: Option<String>,
    evidence: Vec<String>,
    next_agent: String,
}

pub fn classify(receipt_path: PathBuf, output_path: PathBuf) -> Result<()> {
    let playbooks = load_playbooks()?;

    let receipt_text = fs::read_to_string(&receipt_path)
        .with_context(|| format!("Failed to read receipt {}", receipt_path.display()))?;
    let receipt: Receipt = serde_json::from_str(&receipt_text)
        .with_context(|| format!("Failed to parse receipt {}", receipt_path.display()))?;

    let (playbook, evidence) = select_playbook(&playbooks, &receipt)
        .ok_or_else(|| eyre!("No matching fix-forward playbook found for failed gates"))?;

    let fix_forward_receipt = FixForwardReceipt {
        classification: "typed_fix_forward".to_string(),
        fix_forward_kind: playbook.kind.clone(),
        safe_auto_fix: playbook.safe_auto_fix,
        command: playbook.command.clone(),
        route: playbook.route.clone(),
        evidence,
        next_agent: next_agent(playbook),
    };

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory {}", parent.display()))?;
    }

    let output_json = serde_json::to_string_pretty(&fix_forward_receipt)
        .context("Failed to serialize fix-forward receipt")?;
    fs::write(&output_path, output_json)
        .with_context(|| format!("Failed to write receipt {}", output_path.display()))?;

    Ok(())
}

pub fn list_playbooks() -> Result<()> {
    let playbooks = load_playbooks()?;
    for playbook in playbooks {
        println!(
            "{}\tsafe_auto_fix={}\tcommand={}\troute={}\tmutation={}\tbranch_prefix={}",
            playbook.kind,
            playbook.safe_auto_fix,
            playbook.command.as_deref().unwrap_or("-"),
            playbook.route.as_deref().unwrap_or("-"),
            playbook.mutation.as_deref().unwrap_or("-"),
            playbook.branch_prefix.as_deref().unwrap_or("-")
        );
    }
    Ok(())
}

fn load_playbooks() -> Result<Vec<Playbook>> {
    let root = project_root()?;
    let playbook_path = root.join(".ci/fix-forward/playbooks.toml");
    let content = fs::read_to_string(&playbook_path)
        .with_context(|| format!("Failed to read {}", playbook_path.display()))?;
    let config: PlaybookConfig = toml::from_str(&content)
        .with_context(|| format!("Failed to parse {}", playbook_path.display()))?;
    Ok(config.playbooks)
}

fn select_playbook<'a>(
    playbooks: &'a [Playbook],
    receipt: &Receipt,
) -> Option<(&'a Playbook, Vec<String>)> {
    let failed_gates =
        receipt.gates.iter().filter(|gate| gate.status == "fail").collect::<Vec<_>>();

    for playbook in playbooks {
        let mut evidence = BTreeSet::new();
        let mut matched = false;

        for gate in &failed_gates {
            for gate_pattern in &playbook.match_gate_names {
                if gate.gate_name.contains(gate_pattern) {
                    evidence.insert(format!("gate:{} matched {}", gate.gate_name, gate_pattern));
                    matched = true;
                }
            }

            if let Some(command) = &gate.command {
                for output_pattern in &playbook.match_output_contains {
                    if command.contains(output_pattern) {
                        evidence.insert(format!("command:{} matched {}", command, output_pattern));
                        matched = true;
                    }
                }
            }

            if let Some(output_summary) = &gate.output_summary {
                for output_pattern in &playbook.match_output_contains {
                    if output_summary.contains(output_pattern) {
                        evidence.insert(format!(
                            "output_summary in {} matched {}",
                            gate.gate_name, output_pattern
                        ));
                        matched = true;
                    }
                }
            }
        }

        if matched {
            return Some((playbook, evidence.into_iter().collect()));
        }
    }

    None
}

fn next_agent(playbook: &Playbook) -> String {
    if playbook.safe_auto_fix {
        return "fix-forward-bot".to_string();
    }

    if let Some(route) = &playbook.route {
        return route.clone();
    }

    "human-review".to_string()
}
