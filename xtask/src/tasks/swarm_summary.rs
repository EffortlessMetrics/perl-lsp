//! Swarm metrics summary task implementation.
use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Duration, Utc};
use color_eyre::eyre::{Context, Result, bail};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct SwarmSummaryConfig {
    pub ops_dir: PathBuf,
    pub since: Option<String>,
    pub limit: usize,
    pub format: SwarmSummaryOutputFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum SwarmSummaryOutputFormat {
    Human,
    Json,
}

#[derive(Debug, Default)]
struct Summary {
    file_entries: usize,
    matched_entries: usize,
    earliest_ts: Option<DateTime<Utc>>,
    latest_ts: Option<DateTime<Utc>>,
    by_event: HashMap<String, usize>,
    by_agent_type: HashMap<String, usize>,
    by_agent_name: HashMap<String, usize>,
    by_session: HashMap<String, usize>,
    by_location: HashMap<String, usize>,
    recent_entries: Vec<SummaryEntry>,
}

#[derive(Debug, Clone, Serialize)]
struct SummaryEntry {
    ts: String,
    event: String,
    agent_name: String,
    agent_type: String,
    session_id: String,
    location: String,
}

#[derive(Debug, Serialize)]
struct SerializableSummary {
    metrics_file: String,
    entries_in_file: usize,
    entries_matched: usize,
    window: WindowSummary,
    counts: SummaryCounts,
    recent_entries: Vec<SummaryEntry>,
}

#[derive(Debug, Serialize)]
struct WindowSummary {
    since: Option<String>,
    first_timestamp: Option<String>,
    last_timestamp: Option<String>,
}

#[derive(Debug, Serialize)]
struct SummaryCounts {
    by_event: Vec<CountRow>,
    by_agent_type: Vec<CountRow>,
    by_agent_name: Vec<CountRow>,
    by_session: Vec<CountRow>,
    by_location: Vec<CountRow>,
}

#[derive(Debug, Serialize)]
struct CountRow {
    key: String,
    count: usize,
}

pub fn run(config: SwarmSummaryConfig) -> Result<()> {
    if config.limit == 0 {
        bail!("limit must be at least 1");
    }

    let metrics_path = config.ops_dir.join("swarm-metrics.jsonl");
    if !metrics_path.exists() {
        bail!("No metrics file found at {}", metrics_path.display());
    }

    let cutoff = parse_since_spec(config.since.as_deref())?;
    let summary = summarize_metrics(&metrics_path, cutoff.as_ref())?;

    if matches!(config.format, SwarmSummaryOutputFormat::Json) {
        let payload = SerializableSummary {
            metrics_file: metrics_path.display().to_string(),
            entries_in_file: summary.file_entries,
            entries_matched: summary.matched_entries,
            window: WindowSummary {
                since: cutoff.as_ref().map(|ts| ts.to_rfc3339()),
                first_timestamp: summary.earliest_ts.map(|ts| ts.to_rfc3339()),
                last_timestamp: summary.latest_ts.map(|ts| ts.to_rfc3339()),
            },
            counts: SummaryCounts {
                by_event: sorted_counts(&summary.by_event, config.limit),
                by_agent_type: sorted_counts(&summary.by_agent_type, config.limit),
                by_agent_name: sorted_counts(&summary.by_agent_name, config.limit),
                by_session: sorted_counts(&summary.by_session, config.limit),
                by_location: sorted_counts(&summary.by_location, config.limit),
            },
            recent_entries: recent_entries(&summary.recent_entries, config.limit),
        };

        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    println!("=== Swarm Metrics Summary ===");
    println!("File: {}", metrics_path.display());
    println!("Entries in file: {}", summary.file_entries);
    println!("Entries matched: {}", summary.matched_entries);
    if let Some(cutoff) = cutoff {
        println!("Window: since {}", cutoff.to_rfc3339());
    } else {
        println!("Window: all entries");
    }
    if let Some(first_ts) = summary.earliest_ts {
        println!("First timestamp: {}", first_ts.to_rfc3339());
    }
    if let Some(last_ts) = summary.latest_ts {
        println!("Last timestamp: {}", last_ts.to_rfc3339());
    }
    println!();

    print_counts("By event type:", &summary.by_event, config.limit);
    print_counts("By agent type:", &summary.by_agent_type, config.limit);
    print_counts("By agent name:", &summary.by_agent_name, config.limit);
    print_counts("By session:", &summary.by_session, config.limit);
    print_counts("By location:", &summary.by_location, config.limit);

    println!("Recent matching events:");
    let recent = recent_entries(&summary.recent_entries, config.limit);
    if recent.is_empty() {
        println!("(none)");
    } else {
        for entry in &recent {
            println!(
                "{}\t{}\t{}\t{}\t{}\t{}",
                entry.ts,
                entry.event,
                entry.agent_name,
                entry.agent_type,
                entry.session_id,
                entry.location
            );
        }
    }

    Ok(())
}

fn summarize_metrics(path: &Path, cutoff: Option<&DateTime<Utc>>) -> Result<Summary> {
    let file = File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;
    let reader = BufReader::new(file);

    let mut summary = Summary::default();

    for line in reader.lines() {
        let line = line.context("Failed to read swarm metrics line")?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let value: Value = serde_json::from_str(line)
            .with_context(|| format!("Failed to parse JSON entry: {line}"))?;

        summary.file_entries += 1;

        let ts_str = pick_string(&value, &["ts"]);
        let ts = ts_str
            .as_deref()
            .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
            .map(DateTime::<Utc>::from);
        let event =
            pick_string(&value, &["event", "action"]).unwrap_or_else(|| "(none)".to_string());
        let agent_name = pick_string(&value, &["agent_name", "agent", "teammate_name"])
            .unwrap_or_else(|| "(none)".to_string());
        let agent_type = pick_string(&value, &["agent_type", "type", "matcher"])
            .unwrap_or_else(|| "(none)".to_string());
        let session_id =
            pick_string(&value, &["session_id"]).unwrap_or_else(|| "(none)".to_string());
        let location = pick_string(&value, &["worktree_path", "cwd", "branch"])
            .unwrap_or_else(|| "(none)".to_string());

        if let Some(cutoff) = cutoff
            && ts.is_none_or(|ts| ts < *cutoff)
        {
            continue;
        }

        summary.matched_entries += 1;

        if let Some(ts) = ts {
            summary.earliest_ts = Some(summary.earliest_ts.map_or(ts, |current| current.min(ts)));
            summary.latest_ts = Some(summary.latest_ts.map_or(ts, |current| current.max(ts)));
        }

        *summary.by_event.entry(event.clone()).or_insert(0) += 1;
        *summary.by_agent_type.entry(agent_type.clone()).or_insert(0) += 1;
        *summary.by_agent_name.entry(agent_name.clone()).or_insert(0) += 1;
        *summary.by_session.entry(session_id.clone()).or_insert(0) += 1;
        *summary.by_location.entry(location.clone()).or_insert(0) += 1;

        summary.recent_entries.push(SummaryEntry {
            ts: ts_str.unwrap_or_else(|| "(none)".to_string()),
            event,
            agent_name,
            agent_type,
            session_id,
            location,
        });
    }

    Ok(summary)
}

fn parse_since_spec(spec: Option<&str>) -> Result<Option<DateTime<Utc>>> {
    let Some(spec) = spec.map(str::trim).filter(|spec| !spec.is_empty()) else {
        return Ok(None);
    };

    if spec.eq_ignore_ascii_case("all") || spec == "0" || spec == "0s" {
        return Ok(None);
    }

    let Some(unit_part) = spec.chars().last() else {
        bail!("Invalid --since value `{spec}`; use forms like 24h, 30m, 7d, or all");
    };
    let value_part = &spec[..spec.len() - unit_part.len_utf8()];
    let value: i64 = value_part
        .parse()
        .with_context(|| format!("Invalid --since value `{spec}`; expected a whole number"))?;
    let duration = match unit_part {
        'm' => Duration::minutes(value),
        'h' => Duration::hours(value),
        'd' => Duration::days(value),
        'w' => Duration::weeks(value),
        _ => bail!("Invalid --since value `{spec}`; use forms like 24h, 30m, 7d, or all"),
    };

    Ok(Some(Utc::now() - duration))
}

fn pick_string(value: &Value, names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| value.get(*name).and_then(Value::as_str))
        .map(ToOwned::to_owned)
}

fn print_counts(label: &str, counts: &HashMap<String, usize>, limit: usize) {
    println!("{label}");
    let rows = sorted_counts(counts, limit);
    if rows.is_empty() {
        println!("(none)");
        println!();
        return;
    }

    for row in rows {
        println!("{:>5} {}", row.count, row.key);
    }
    println!();
}

fn sorted_counts(counts: &HashMap<String, usize>, limit: usize) -> Vec<CountRow> {
    let mut rows: Vec<(&String, &usize)> = counts.iter().collect();
    rows.sort_by(|(a_label, a_count), (b_label, b_count)| {
        b_count.cmp(a_count).then_with(|| a_label.cmp(b_label))
    });

    rows.into_iter()
        .take(limit)
        .map(|(key, count)| CountRow {
            key: key.clone(),
            count: *count,
        })
        .collect()
}

fn recent_entries(entries: &[SummaryEntry], limit: usize) -> Vec<SummaryEntry> {
    let start = entries.len().saturating_sub(limit);
    entries[start..].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_cmd::cargo::cargo_bin;
    use serde_json::Value as JsonValue;
    use std::fs;
    use std::io::Write;
    use std::process::Command;
    use tempfile::{NamedTempFile, TempDir};

    fn sample_file() -> Result<NamedTempFile> {
        let mut file = NamedTempFile::new()?;
        writeln!(
            file,
            "{{\"ts\":\"2026-03-27T10:00:00Z\",\"event\":\"task_completed\",\"agent_name\":\"ops\",\"agent_type\":\"reviewer\",\"session_id\":\"a\",\"cwd\":\"/tmp/a\"}}"
        )?;
        writeln!(
            file,
            "{{\"ts\":\"2026-03-28T10:00:00Z\",\"event\":\"subagent_stop\",\"agent_name\":\"builder\",\"agent_type\":\"builder\",\"session_id\":\"b\",\"worktree_path\":\"/tmp/b\"}}"
        )?;
        Ok(file)
    }

    fn sample_ops_dir() -> Result<TempDir> {
        let dir = TempDir::new()?;
        let mut file = fs::File::create(dir.path().join("swarm-metrics.jsonl"))?;
        writeln!(
            file,
            "{{\"ts\":\"2026-03-27T10:00:00Z\",\"event\":\"task_completed\",\"agent_name\":\"ops\",\"agent_type\":\"reviewer\",\"session_id\":\"a\",\"cwd\":\"/tmp/a\"}}"
        )?;
        writeln!(
            file,
            "{{\"ts\":\"2026-03-28T10:00:00Z\",\"event\":\"subagent_stop\",\"agent_name\":\"builder\",\"agent_type\":\"builder\",\"session_id\":\"b\",\"worktree_path\":\"/tmp/b\"}}"
        )?;
        Ok(dir)
    }

    #[test]
    fn parses_since_window() -> Result<()> {
        let Some(cutoff) = parse_since_spec(Some("24h"))? else {
            return Err(std::io::Error::other("expected cutoff").into());
        };
        assert!(cutoff < Utc::now());
        Ok(())
    }

    #[test]
    fn summarizes_and_filters_metrics() -> Result<()> {
        let file = sample_file()?;
        let summary = summarize_metrics(file.path(), None)?;
        assert_eq!(summary.file_entries, 2);
        assert_eq!(summary.matched_entries, 2);
        assert_eq!(summary.by_event.get("task_completed"), Some(&1));
        assert_eq!(summary.by_event.get("subagent_stop"), Some(&1));
        assert_eq!(summary.by_location.get("/tmp/b"), Some(&1));

        let cutoff = DateTime::parse_from_rfc3339("2026-03-28T00:00:00Z")?.with_timezone(&Utc);
        let filtered = summarize_metrics(file.path(), Some(&cutoff))?;
        assert_eq!(filtered.file_entries, 2);
        assert_eq!(filtered.matched_entries, 1);
        assert_eq!(filtered.by_event.get("subagent_stop"), Some(&1));
        assert_eq!(filtered.by_event.get("task_completed"), None);
        Ok(())
    }

    #[test]
    fn json_output_serializes_summary_shape() -> Result<()> {
        let file = sample_file()?;
        let summary = summarize_metrics(file.path(), None)?;
        let payload = SerializableSummary {
            metrics_file: file.path().display().to_string(),
            entries_in_file: summary.file_entries,
            entries_matched: summary.matched_entries,
            window: WindowSummary {
                since: None,
                first_timestamp: summary.earliest_ts.map(|ts| ts.to_rfc3339()),
                last_timestamp: summary.latest_ts.map(|ts| ts.to_rfc3339()),
            },
            counts: SummaryCounts {
                by_event: sorted_counts(&summary.by_event, 10),
                by_agent_type: sorted_counts(&summary.by_agent_type, 10),
                by_agent_name: sorted_counts(&summary.by_agent_name, 10),
                by_session: sorted_counts(&summary.by_session, 10),
                by_location: sorted_counts(&summary.by_location, 10),
            },
            recent_entries: recent_entries(&summary.recent_entries, 10),
        };

        let json = serde_json::to_string_pretty(&payload)?;
        let parsed: JsonValue = serde_json::from_str(&json)?;
        assert_eq!(parsed["entries_in_file"], 2);
        assert_eq!(parsed["entries_matched"], 2);
        assert_eq!(parsed["counts"]["by_event"][0]["key"], "subagent_stop");
        assert_eq!(
            parsed["recent_entries"].as_array().map(|a| a.len()),
            Some(2)
        );
        Ok(())
    }

    #[test]
    fn cli_json_mode_emits_machine_readable_summary() -> Result<()> {
        let ops_dir = sample_ops_dir()?;
        let ops_dir_path = ops_dir
            .path()
            .to_str()
            .ok_or_else(|| std::io::Error::other("ops dir path was not UTF-8"))?;

        let output = Command::new(cargo_bin("xtask"))
            .args([
                "swarm-summary",
                ops_dir_path,
                "--since",
                "all",
                "--limit",
                "1",
                "--format",
                "json",
            ])
            .output()?;

        assert!(output.status.success(), "xtask swarm-summary failed");
        let stdout = String::from_utf8(output.stdout)?;
        let parsed: JsonValue = serde_json::from_str(&stdout)?;
        assert_eq!(parsed["entries_in_file"], 2);
        assert_eq!(parsed["entries_matched"], 2);
        assert_eq!(parsed["counts"]["by_event"][0]["key"], "subagent_stop");
        Ok(())
    }
}
