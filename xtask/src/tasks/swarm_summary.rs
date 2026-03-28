//! Swarm metrics summary task implementation.
use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Duration, Utc};
use color_eyre::eyre::{Context, Result, bail};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct SwarmSummaryConfig {
    pub ops_dir: PathBuf,
    pub since: Option<String>,
    pub limit: usize,
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

#[derive(Debug, Clone)]
struct SummaryEntry {
    ts: String,
    event: String,
    agent_name: String,
    agent_type: String,
    session_id: String,
    location: String,
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
    if summary.recent_entries.is_empty() {
        println!("(none)");
    } else {
        let start = summary.recent_entries.len().saturating_sub(config.limit);
        for entry in &summary.recent_entries[start..] {
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

        if let Some(cutoff) = cutoff {
            if ts.is_none_or(|ts| ts < *cutoff) {
                continue;
            }
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
    names.iter().find_map(|name| value.get(*name).and_then(Value::as_str)).map(ToOwned::to_owned)
}

fn print_counts(label: &str, counts: &HashMap<String, usize>, limit: usize) {
    println!("{label}");
    let mut rows: Vec<(&String, &usize)> = counts.iter().collect();
    rows.sort_by(|(a_label, a_count), (b_label, b_count)| {
        b_count.cmp(a_count).then_with(|| a_label.cmp(b_label))
    });

    if rows.is_empty() {
        println!("(none)");
        println!();
        return;
    }

    for (key, count) in rows.into_iter().take(limit) {
        println!("{count:>5} {key}");
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn parses_since_window() -> Result<()> {
        let cutoff = parse_since_spec(Some("24h"))?.expect("expected cutoff");
        assert!(cutoff < Utc::now());
        Ok(())
    }

    #[test]
    fn summarizes_and_filters_metrics() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(
            file,
            "{{\"ts\":\"2026-03-27T10:00:00Z\",\"event\":\"task_completed\",\"agent_name\":\"ops\",\"agent_type\":\"reviewer\",\"session_id\":\"a\",\"cwd\":\"/tmp/a\"}}"
        )?;
        writeln!(
            file,
            "{{\"ts\":\"2026-03-28T10:00:00Z\",\"event\":\"subagent_stop\",\"agent_name\":\"builder\",\"agent_type\":\"builder\",\"session_id\":\"b\",\"worktree_path\":\"/tmp/b\"}}"
        )?;

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
}
