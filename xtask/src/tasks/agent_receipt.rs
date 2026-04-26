use chrono::{DateTime, Utc};
use color_eyre::eyre::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationAction {
    Comment,
    CommentUpdate,
    StateTransition,
    ArtifactUpload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppliedMutation {
    pub action: MutationAction,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentReceipt {
    pub receipt_id: String,
    pub task_id: String,
    pub snapshot_id: String,
    pub canonical_head_sha: String,
    pub observed_head_sha: String,
    pub created_at: DateTime<Utc>,
    pub lease_expires_at: DateTime<Utc>,
    pub allowed_mutations: Vec<String>,
    pub forbidden_mutations: Vec<String>,
    pub idempotency_key: String,
    pub supersedes_receipt_id: Option<String>,
    pub required_output_schema: String,
    pub applied_mutations: Vec<AppliedMutation>,
}

pub fn validate(receipt_path: &Path) -> Result<()> {
    let receipt = read_json::<AgentReceipt>(receipt_path)?;

    ensure!(!receipt.task_id.trim().is_empty(), "task_id must not be empty");
    ensure!(receipt.idempotency_key == receipt.task_id, "idempotency_key must match task_id");
    ensure!(
        receipt.created_at <= receipt.lease_expires_at,
        "receipt created_at is later than lease_expires_at"
    );
    ensure!(
        receipt.canonical_head_sha == receipt.observed_head_sha,
        "stale head: canonical_head_sha ({}) != observed_head_sha ({})",
        receipt.canonical_head_sha,
        receipt.observed_head_sha
    );

    for mutation in &receipt.applied_mutations {
        let action = mutation_name(&mutation.action);
        ensure!(
            receipt.allowed_mutations.iter().any(|allowed| allowed == action),
            "mutation '{}' is not present in allowed_mutations",
            action
        );

        ensure!(
            receipt.forbidden_mutations.iter().all(|forbidden| forbidden != action),
            "mutation '{}' is present in forbidden_mutations",
            action
        );
    }

    if let Some(supersedes_receipt_id) = &receipt.supersedes_receipt_id {
        ensure!(
            !supersedes_receipt_id.trim().is_empty(),
            "supersedes_receipt_id, when provided, must not be empty"
        );
    }

    println!("Receipt is valid for task {}", receipt.task_id);
    Ok(())
}

fn mutation_name(action: &MutationAction) -> &'static str {
    match action {
        MutationAction::Comment => "comment",
        MutationAction::CommentUpdate => "comment_update",
        MutationAction::StateTransition => "state_transition",
        MutationAction::ArtifactUpload => "artifact_upload",
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read JSON file at {}", path.display()))?;
    serde_json::from_str::<T>(&raw)
        .with_context(|| format!("Failed to parse JSON file at {}", path.display()))
}
