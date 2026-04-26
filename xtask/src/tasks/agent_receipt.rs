use chrono::{DateTime, Utc};
use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReceipt {
    pub schema_version: u32,
    pub receipt_id: String,
    pub task_id: String,
    pub snapshot_id: String,
    pub head_sha: String,
    pub received_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub mutation: String,
    pub allowed_mutations: Vec<String>,
    pub forbidden_mutations: Vec<String>,
    pub output_schema: String,
    pub supersedes: Option<String>,
}

pub fn validate(receipt_path: &Path) -> Result<()> {
    let raw = fs::read_to_string(receipt_path)
        .with_context(|| format!("failed to read receipt file: {}", receipt_path.display()))?;
    let receipt: AgentReceipt = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse receipt file: {}", receipt_path.display()))?;

    validate_receipt(&receipt)?;
    println!("receipt valid: {}", receipt.receipt_id);
    Ok(())
}

fn validate_receipt(receipt: &AgentReceipt) -> Result<()> {
    if receipt.task_id.trim().is_empty() {
        bail!("task_id cannot be empty");
    }
    if receipt.receipt_id.trim().is_empty() {
        bail!("receipt_id cannot be empty");
    }
    if receipt.received_at > Utc::now() {
        bail!("received_at cannot be in the future");
    }
    if receipt.expires_at <= receipt.received_at {
        bail!("expires_at must be later than received_at");
    }
    if !receipt.allowed_mutations.iter().any(|m| m == &receipt.mutation) {
        bail!("mutation `{}` is not in allowed_mutations", receipt.mutation);
    }
    let forbidden: HashSet<&str> =
        receipt.forbidden_mutations.iter().map(std::string::String::as_str).collect();
    if forbidden.contains(receipt.mutation.as_str()) {
        bail!("mutation `{}` is forbidden", receipt.mutation);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_receipt() -> AgentReceipt {
        AgentReceipt {
            schema_version: 1,
            receipt_id: "r-1".to_string(),
            task_id: "t-1".to_string(),
            snapshot_id: "s-1".to_string(),
            head_sha: "abc".to_string(),
            received_at: Utc::now() - chrono::Duration::minutes(1),
            expires_at: Utc::now() + chrono::Duration::minutes(10),
            mutation: "comment.update".to_string(),
            allowed_mutations: vec!["comment.update".to_string()],
            forbidden_mutations: vec!["label.set".to_string()],
            output_schema: "agent-receipt-v1".to_string(),
            supersedes: None,
        }
    }

    #[test]
    fn validates_allowed_mutation() -> Result<()> {
        validate_receipt(&base_receipt())?;
        Ok(())
    }

    #[test]
    fn rejects_forbidden_mutation() {
        let mut receipt = base_receipt();
        receipt.forbidden_mutations.push("comment.update".to_string());
        let result = validate_receipt(&receipt);
        assert!(result.is_err(), "forbidden mutation must fail");
    }
}
