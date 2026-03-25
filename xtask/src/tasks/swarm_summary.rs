//! Swarm metrics summary task implementation.
use std::{
    borrow::Cow,
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

use color_eyre::eyre::{bail, Context, Result};
use serde_json::Value;

pub fn run(ops_dir: PathBuf) -> Result<()> {
    let metrics_path = ops_dir.join("swarm-metrics.jsonl");
    if !metrics_path.exists() {
        bail!("No metrics file found at {}", metrics_path.display());
    }

    let file = File::open(&metrics_path)
        .with_context(|| format!("Failed to open {}", metrics_path.display()))?;
    let reader = BufReader::new(file);

    let mut total_entries = 0usize;
    let mut by_event: HashMap<String, usize> = HashMap::new();
    let mut by_agent: HashMap<String, usize> = HashMap::new();
    let mut task_completed: Vec<(String, String)> = Vec::new();
    let mut subagent_stops: Vec<(String, String, String)> = Vec::new();

    for line in reader.lines() {
        let line = line.context("Failed to read swarm metrics line")?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let value: Value = serde_json::from_str(line)
            .with_context(|| format!("Failed to parse JSON entry: {line}"))?;

        total_entries += 1;

        let event = pick_str_field(&value, &["event", "action"]);
        let agent_type = pick_str_field(&value, &["agent_type", "type"]);
        let ts = pick_str_field(&value, &["ts"]);
        let cwd = pick_str_field(&value, &["cwd"]);
        let worktree_path = pick_str_field(&value, &["worktree_path"]);

        *by_event.entry(event.clone().into_owned()).or_insert(0) += 1;
        *by_agent.entry(agent_type.clone().into_owned()).or_insert(0) += 1;

        if event.as_ref() == "task_completed" {
            task_completed.push((ts.to_string(), cwd.to_string()));
        } else if event.as_ref() == "subagent_stop" {
            subagent_stops.push((ts.to_string(), agent_type.to_string(), worktree_path.to_string()));
        }
    }

    println!("=== Swarm Metrics Summary ===");
    println!("Total entries: {total_entries}");
    println!();
    print_counts("By event type:", &by_event);
    print_counts("By agent type:", &by_agent);

    println!("Recent completions (last 5):");
    print_last_pairs(&task_completed, 5, |ts, cwd| {
        println!("{ts}\t{cwd}");
    });
    println!();

    println!("Recent stops (last 5):");
    print_last_triplets(&subagent_stops, 5, |ts, agent_type, worktree_path| {
        println!("{ts}\t{agent_type}\t{worktree_path}");
    });

    Ok(())
}

fn pick_str_field<'a>(value: &'a Value, names: &[&str]) -> Cow<'a, str> {
    for name in names {
        if let Some(v) = value.get(name).and_then(Value::as_str) {
            return Cow::Borrowed(v);
        }
    }
    Cow::Borrowed("(none)")
}

fn print_counts(label: &str, counts: &HashMap<String, usize>) {
    println!("{label}");
    let mut rows: Vec<(&String, &usize)> = counts.iter().collect();
    rows.sort_by(|(a_label, a_count), (b_label, b_count)| {
        b_count
            .cmp(a_count)
            .then_with(|| a_label.cmp(b_label))
    });

    for (key, count) in rows {
        println!("{count:>5} {key}");
    }
    println!();
}

fn print_last_pairs<T>(
    rows: &[(T, T)],
    limit: usize,
    mut printer: impl FnMut(&T, &T),
) {
    if rows.is_empty() {
        return;
    }

    let start = rows.len().saturating_sub(limit);
    for (first, second) in &rows[start..] {
        printer(first, second);
    }
}

fn print_last_triplets<T>(
    rows: &[(T, T, T)],
    limit: usize,
    mut printer: impl FnMut(&T, &T, &T),
) {
    if rows.is_empty() {
        return;
    }

    let start = rows.len().saturating_sub(limit);
    for (first, second, third) in &rows[start..] {
        printer(first, second, third);
    }
}
