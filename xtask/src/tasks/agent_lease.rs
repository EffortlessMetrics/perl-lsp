use chrono::{DateTime, Utc};
use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentTask {
    pub task_id: String,
    pub snapshot_id: String,
    pub lane: String,
    pub pr: u64,
    pub head_sha: String,
    pub base_sha: String,
    pub canonical_state: String,
    pub allowed_mutations: Vec<String>,
    pub forbidden_mutations: Vec<String>,
    pub required_output_schema: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentLease {
    pub schema_version: u32,
    pub issued_at: DateTime<Utc>,
    #[serde(flatten)]
    pub task: AgentTask,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CurrentSnapshot {
    pub snapshot_id: String,
    pub head_sha: String,
}

pub fn acquire(task_path: &Path, out_path: &Path) -> Result<()> {
    let task = read_task(task_path)?;
    validate_task(&task)?;

    let lease = AgentLease { schema_version: 1, issued_at: Utc::now(), task };

    write_json(out_path, &lease)?;
    println!("Lease written to {}", out_path.display());
    Ok(())
}

pub fn verify(lease_path: &Path, current_path: &Path) -> Result<()> {
    let lease = read_lease(lease_path)?;
    validate_task(&lease.task)?;

    let now = Utc::now();
    if now > lease.task.expires_at {
        bail!("Lease expired at {} (now {})", lease.task.expires_at.to_rfc3339(), now.to_rfc3339());
    }

    let current = read_snapshot(current_path)?;
    if current.snapshot_id != lease.task.snapshot_id {
        bail!(
            "snapshot mismatch: lease={}, current={}",
            lease.task.snapshot_id,
            current.snapshot_id
        );
    }

    if current.head_sha != lease.task.head_sha {
        bail!("stale head: lease={}, current={}", lease.task.head_sha, current.head_sha);
    }

    println!("Lease verification succeeded for task {}", lease.task.task_id);
    Ok(())
}

pub fn read_lease(path: &Path) -> Result<AgentLease> {
    read_json(path, "lease")
}

fn read_task(path: &Path) -> Result<AgentTask> {
    read_json(path, "task")
}

fn read_snapshot(path: &Path) -> Result<CurrentSnapshot> {
    read_json(path, "snapshot")
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path, label: &str) -> Result<T> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("reading {label} JSON from {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("parsing {label} JSON from {}", path.display()))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating parent directory {}", parent.display()))?;
    }

    let raw = serde_json::to_string_pretty(value).context("serializing lease JSON")?;
    fs::write(path, format!("{raw}\n"))
        .with_context(|| format!("writing JSON output to {}", path.display()))
}

fn validate_task(task: &AgentTask) -> Result<()> {
    if task.task_id.trim().is_empty() {
        bail!("task_id must not be empty");
    }
    if task.snapshot_id.trim().is_empty() {
        bail!("snapshot_id must not be empty");
    }
    if task.head_sha.trim().is_empty() {
        bail!("head_sha must not be empty");
    }
    if task.required_output_schema.trim().is_empty() {
        bail!("required_output_schema must not be empty");
    }
    if task.allowed_mutations.is_empty() {
        bail!("allowed_mutations must not be empty");
    }

    let duplicates = task
        .allowed_mutations
        .iter()
        .filter(|mutation| task.forbidden_mutations.contains(*mutation))
        .cloned()
        .collect::<Vec<_>>();
    if !duplicates.is_empty() {
        bail!("allowed_mutations and forbidden_mutations overlap: {}", duplicates.join(", "));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::{ensure, eyre};

    fn parse_time(raw: &str) -> Result<DateTime<Utc>> {
        raw.parse().with_context(|| format!("parsing fixture timestamp {raw}"))
    }

    fn valid_task() -> Result<AgentTask> {
        Ok(AgentTask {
            task_id: "task-1".to_owned(),
            snapshot_id: "snap-1".to_owned(),
            lane: "proof".to_owned(),
            pr: 42,
            head_sha: "head-a".to_owned(),
            base_sha: "base-a".to_owned(),
            canonical_state: "open".to_owned(),
            allowed_mutations: vec!["comment_upsert".to_owned()],
            forbidden_mutations: vec!["label_mutation".to_owned()],
            required_output_schema: ".ci/receipts/schemas/agent-receipt.schema.json".to_owned(),
            expires_at: parse_time("2030-01-01T00:00:00Z")?,
        })
    }

    fn valid_lease() -> Result<AgentLease> {
        Ok(AgentLease {
            schema_version: 1,
            issued_at: parse_time("2026-06-20T00:00:00Z")?,
            task: valid_task()?,
        })
    }

    fn write_fixture_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))?;
        Ok(())
    }

    fn validation_error(
        mut task: AgentTask,
        mutate: impl FnOnce(&mut AgentTask),
    ) -> Result<String> {
        mutate(&mut task);
        let err = validate_task(&task).err().ok_or_else(|| eyre!("task should fail validation"))?;
        Ok(err.to_string())
    }

    #[test]
    fn agent_lease_acquire_writes_nested_lease_and_verify_accepts_snapshot() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let task_path = tmp.path().join("task.json");
        let lease_path = tmp.path().join("nested").join("lease.json");
        let current_path = tmp.path().join("current.json");
        write_fixture_json(&task_path, &valid_task()?)?;
        write_fixture_json(
            &current_path,
            &CurrentSnapshot { snapshot_id: "snap-1".to_owned(), head_sha: "head-a".to_owned() },
        )?;

        acquire(&task_path, &lease_path)?;
        let lease = read_lease(&lease_path)?;
        verify(&lease_path, &current_path)?;

        ensure!(lease.schema_version == 1, "got lease: {lease:?}");
        ensure!(lease.task.task_id == "task-1", "got lease: {lease:?}");

        Ok(())
    }

    #[test]
    fn agent_lease_verify_reports_expired_snapshot_and_head_mismatches() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let lease_path = tmp.path().join("lease.json");
        let current_path = tmp.path().join("current.json");

        let mut expired = valid_lease()?;
        expired.task.expires_at = parse_time("2020-01-01T00:00:00Z")?;
        write_fixture_json(&lease_path, &expired)?;
        write_fixture_json(
            &current_path,
            &CurrentSnapshot { snapshot_id: "snap-1".to_owned(), head_sha: "head-a".to_owned() },
        )?;
        let err = verify(&lease_path, &current_path)
            .err()
            .ok_or_else(|| eyre!("expired lease should fail"))?;
        ensure!(err.to_string().contains("Lease expired at"), "got error: {err}");

        write_fixture_json(&lease_path, &valid_lease()?)?;
        write_fixture_json(
            &current_path,
            &CurrentSnapshot { snapshot_id: "snap-2".to_owned(), head_sha: "head-a".to_owned() },
        )?;
        let err = verify(&lease_path, &current_path)
            .err()
            .ok_or_else(|| eyre!("snapshot mismatch should fail"))?;
        ensure!(err.to_string().contains("snapshot mismatch"), "got error: {err}");

        write_fixture_json(
            &current_path,
            &CurrentSnapshot { snapshot_id: "snap-1".to_owned(), head_sha: "head-b".to_owned() },
        )?;
        let err = verify(&lease_path, &current_path)
            .err()
            .ok_or_else(|| eyre!("stale head should fail"))?;
        ensure!(err.to_string().contains("stale head"), "got error: {err}");

        Ok(())
    }

    #[test]
    fn agent_lease_validate_task_reports_required_field_and_mutation_contracts() -> Result<()> {
        let cases: [(&str, fn(&mut AgentTask)); 6] = [
            ("task_id must not be empty", |task: &mut AgentTask| task.task_id = " ".to_owned()),
            ("snapshot_id must not be empty", |task: &mut AgentTask| task.snapshot_id.clear()),
            ("head_sha must not be empty", |task: &mut AgentTask| task.head_sha.clear()),
            ("required_output_schema must not be empty", |task: &mut AgentTask| {
                task.required_output_schema = "\t".to_owned()
            }),
            ("allowed_mutations must not be empty", |task: &mut AgentTask| {
                task.allowed_mutations.clear()
            }),
            (
                "allowed_mutations and forbidden_mutations overlap: label_mutation",
                |task: &mut AgentTask| task.allowed_mutations.push("label_mutation".to_owned()),
            ),
        ];
        for (expected, mutate) in cases {
            let err = validation_error(valid_task()?, mutate)?;
            ensure!(err.contains(expected), "expected {expected:?}; got {err}");
        }

        Ok(())
    }

    #[test]
    fn agent_lease_read_json_reports_missing_and_malformed_files() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let missing = tmp.path().join("missing.json");
        let err = read_lease(&missing).err().ok_or_else(|| eyre!("missing lease should fail"))?;
        ensure!(err.to_string().contains("reading lease JSON"), "got error: {err}");

        let malformed = tmp.path().join("malformed.json");
        fs::write(&malformed, "{")?;
        let err =
            read_lease(&malformed).err().ok_or_else(|| eyre!("malformed lease should fail"))?;
        ensure!(err.to_string().contains("parsing lease JSON"), "got error: {err}");

        Ok(())
    }
}
