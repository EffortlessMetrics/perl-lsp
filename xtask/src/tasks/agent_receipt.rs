use crate::tasks::agent_lease::{AgentLease, read_lease};
use chrono::{DateTime, Utc};
use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentReceipt {
    pub schema_version: u32,
    pub task_id: String,
    pub snapshot_id: String,
    pub head_sha: String,
    pub lease_path: String,
    pub required_output_schema: String,
    pub received_at: DateTime<Utc>,
    pub idempotency_key: String,
    pub mutation: String,
    pub status: String,
}

pub fn validate(receipt_path: &Path) -> Result<()> {
    let receipt = read_receipt(receipt_path)?;
    validate_core_fields(&receipt)?;

    let lease = read_lease(Path::new(&receipt.lease_path))?;
    validate_against_lease(&receipt, &lease)?;

    println!("Receipt validation succeeded for task {}", receipt.task_id);
    Ok(())
}

fn read_receipt(path: &Path) -> Result<AgentReceipt> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("reading receipt JSON from {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("parsing receipt JSON from {}", path.display()))
}

fn validate_core_fields(receipt: &AgentReceipt) -> Result<()> {
    if receipt.task_id.trim().is_empty() {
        bail!("task_id must not be empty");
    }
    if receipt.idempotency_key.trim().is_empty() {
        bail!("idempotency_key must not be empty");
    }
    if receipt.mutation.trim().is_empty() {
        bail!("mutation must not be empty");
    }
    Ok(())
}

fn validate_against_lease(receipt: &AgentReceipt, lease: &AgentLease) -> Result<()> {
    if receipt.task_id != lease.task.task_id {
        bail!("task_id mismatch: receipt={}, lease={}", receipt.task_id, lease.task.task_id);
    }
    if receipt.snapshot_id != lease.task.snapshot_id {
        bail!(
            "snapshot_id mismatch: receipt={}, lease={}",
            receipt.snapshot_id,
            lease.task.snapshot_id
        );
    }
    if receipt.head_sha != lease.task.head_sha {
        bail!("stale head: receipt={}, lease={}", receipt.head_sha, lease.task.head_sha);
    }
    if receipt.required_output_schema != lease.task.required_output_schema {
        bail!(
            "required_output_schema mismatch: receipt={}, lease={}",
            receipt.required_output_schema,
            lease.task.required_output_schema
        );
    }

    let allowed = lease.task.allowed_mutations.iter().collect::<HashSet<_>>();
    if !allowed.contains(&receipt.mutation) {
        bail!(
            "mutation '{}' is not in allowed_mutations [{}]",
            receipt.mutation,
            lease.task.allowed_mutations.join(", ")
        );
    }

    let forbidden = lease.task.forbidden_mutations.iter().collect::<HashSet<_>>();
    if forbidden.contains(&receipt.mutation) {
        bail!("mutation '{}' is forbidden", receipt.mutation);
    }

    let now = Utc::now();
    if now > lease.task.expires_at {
        bail!(
            "lease expired at {}; mutation '{}' rejected",
            lease.task.expires_at.to_rfc3339(),
            receipt.mutation
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use serde_json::json;

    #[test]
    fn agent_receipt_lease_fixture_preserves_pr_and_expiry() -> Result<()> {
        let expires_at = Utc::now() + Duration::days(1);
        let task = valid_lease(expires_at)?.task;

        assert_eq!(format!("pr: {}", task.pr), "pr: 6853");
        assert_eq!(task.expires_at, expires_at);
        assert!(
            format!("{task:?}").contains("pr: 6853"),
            "AgentTask {{ ... }} fixture should expose the PR field"
        );

        Ok(())
    }

    #[test]
    fn agent_receipt_fixture_builders_match_contract() -> Result<()> {
        let before_receipt = Utc::now();
        let receipt = valid_receipt("comment_upsert");
        let after_receipt = Utc::now();

        assert_eq!(receipt.schema_version, 1);
        assert_eq!(receipt.task_id, "task-6853");
        assert_eq!(receipt.snapshot_id, "snap-001");
        assert_eq!(receipt.head_sha, "abc123");
        assert_eq!(receipt.lease_path, "lease.json");
        assert_eq!(
            receipt.required_output_schema,
            ".ci/receipts/schemas/agent-receipt.schema.json"
        );
        assert_eq!(receipt.idempotency_key, "key-1");
        assert_eq!(receipt.mutation, "comment_upsert");
        assert_eq!(receipt.status, "pass");
        assert!(
            receipt.received_at >= before_receipt && receipt.received_at <= after_receipt,
            "receipt timestamp should be captured while building the fixture"
        );

        let expires_at = Utc::now() + Duration::days(1);
        let before_lease = Utc::now();
        let lease = valid_lease(expires_at)?;
        let after_lease = Utc::now();

        assert_eq!(lease.schema_version, 1);
        assert!(
            lease.issued_at >= before_lease && lease.issued_at <= after_lease,
            "lease timestamp should be captured while building the fixture"
        );
        assert_eq!(lease.task.task_id, "task-6853");
        assert_eq!(lease.task.snapshot_id, "snap-001");
        assert_eq!(lease.task.lane, "ci-modernization");
        assert_eq!(lease.task.pr, 6853);
        assert_eq!(lease.task.head_sha, "abc123");
        assert_eq!(lease.task.base_sha, "def456");
        assert_eq!(lease.task.canonical_state, "open");
        assert_eq!(lease.task.allowed_mutations, vec!["comment_upsert".to_string()]);
        assert_eq!(lease.task.forbidden_mutations, vec!["label_mutation".to_string()]);
        assert_eq!(
            lease.task.required_output_schema,
            ".ci/receipts/schemas/agent-receipt.schema.json"
        );
        assert_eq!(
            lease.task.expires_at, expires_at,
            "AgentTask {{ ... }} fixture must preserve expires_at"
        );

        Ok(())
    }

    #[test]
    fn agent_receipt_core_fields_reject_blank_values() -> Result<()> {
        for (field, receipt) in [
            (
                "task_id",
                AgentReceipt { task_id: " ".to_string(), ..valid_receipt("comment_upsert") },
            ),
            (
                "idempotency_key",
                AgentReceipt {
                    idempotency_key: " ".to_string(),
                    ..valid_receipt("comment_upsert")
                },
            ),
            (
                "mutation",
                AgentReceipt { mutation: " ".to_string(), ..valid_receipt("comment_upsert") },
            ),
        ] {
            let err = validate_core_fields(&receipt)
                .err()
                .ok_or_else(|| color_eyre::eyre::eyre!("{field} should be rejected"))?;
            assert!(
                err.to_string().contains(field),
                "expected {field} validation error, got {err}"
            );
        }

        Ok(())
    }

    #[test]
    fn agent_receipt_accepts_matching_allowed_mutation() -> Result<()> {
        let receipt = valid_receipt("comment_upsert");
        let lease = valid_lease(Utc::now() + Duration::days(1))?;

        validate_against_lease(&receipt, &lease)
    }

    #[test]
    fn agent_receipt_rejects_lease_identity_mismatches() -> Result<()> {
        for (field, expected, receipt) in [
            (
                "task_id",
                "task_id mismatch",
                AgentReceipt {
                    task_id: "other-task".to_string(),
                    ..valid_receipt("comment_upsert")
                },
            ),
            (
                "snapshot_id",
                "snapshot_id mismatch",
                AgentReceipt {
                    snapshot_id: "other-snapshot".to_string(),
                    ..valid_receipt("comment_upsert")
                },
            ),
            (
                "head_sha",
                "stale head",
                AgentReceipt {
                    head_sha: "other-head".to_string(),
                    ..valid_receipt("comment_upsert")
                },
            ),
            (
                "required_output_schema",
                "required_output_schema mismatch",
                AgentReceipt {
                    required_output_schema: "other.schema".to_string(),
                    ..valid_receipt("comment_upsert")
                },
            ),
        ] {
            let lease = valid_lease(Utc::now() + Duration::days(1))?;
            let err = validate_against_lease(&receipt, &lease)
                .err()
                .ok_or_else(|| color_eyre::eyre::eyre!("{field} mismatch should be rejected"))?;
            assert!(err.to_string().contains(expected), "expected {field} mismatch, got {err}");
        }

        Ok(())
    }

    #[test]
    fn agent_receipt_rejects_disallowed_forbidden_and_expired_mutations() -> Result<()> {
        let lease = valid_lease(Utc::now() + Duration::days(1))?;

        let disallowed = validate_against_lease(&valid_receipt("close-pr"), &lease)
            .err()
            .ok_or_else(|| color_eyre::eyre::eyre!("disallowed mutation should be rejected"))?;
        assert!(
            disallowed.to_string().contains("not in allowed_mutations"),
            "expected allowed mutation error, got {disallowed}"
        );

        let mut overlapping_lease = lease.clone();
        overlapping_lease.task.forbidden_mutations.push("comment_upsert".to_string());
        let forbidden =
            validate_against_lease(&valid_receipt("comment_upsert"), &overlapping_lease)
                .err()
                .ok_or_else(|| color_eyre::eyre::eyre!("forbidden mutation should be rejected"))?;
        assert!(
            forbidden.to_string().contains("is forbidden"),
            "expected forbidden mutation error, got {forbidden}"
        );

        let expired = validate_against_lease(
            &valid_receipt("comment_upsert"),
            &valid_lease(Utc::now() - Duration::days(1))?,
        )
        .err()
        .ok_or_else(|| color_eyre::eyre::eyre!("expired lease should reject receipt"))?;
        assert!(
            expired.to_string().contains("lease expired"),
            "expected expired lease error, got {expired}"
        );

        Ok(())
    }

    #[test]
    fn agent_receipt_reader_rejects_unknown_fields() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("receipt.json");
        let mut receipt = serde_json::to_value(valid_receipt("comment_upsert"))?;
        receipt["unexpected"] = json!("extra");
        fs::write(&path, serde_json::to_string_pretty(&receipt)?)?;

        let err = read_receipt(&path)
            .err()
            .ok_or_else(|| color_eyre::eyre::eyre!("unknown receipt field should be rejected"))?;
        let debug = format!("{err:?}");
        assert!(debug.contains("unexpected"), "expected unknown field error, got {err}");

        Ok(())
    }

    fn valid_receipt(mutation: &str) -> AgentReceipt {
        AgentReceipt {
            schema_version: 1,
            task_id: "task-6853".to_string(),
            snapshot_id: "snap-001".to_string(),
            head_sha: "abc123".to_string(),
            lease_path: "lease.json".to_string(),
            required_output_schema: ".ci/receipts/schemas/agent-receipt.schema.json".to_string(),
            received_at: Utc::now(),
            idempotency_key: "key-1".to_string(),
            mutation: mutation.to_string(),
            status: "pass".to_string(),
        }
    }

    fn valid_lease(expires_at: DateTime<Utc>) -> Result<AgentLease> {
        let fixture = include_str!("../../tests/fixtures/agent-leases/lease-expired.json");
        let mut lease_json: serde_json::Value = serde_json::from_str(fixture)?;
        lease_json["issued_at"] = json!(Utc::now());
        lease_json["expires_at"] = json!(expires_at);
        let lease = serde_json::from_value(lease_json)?;
        Ok(lease)
    }
}
