use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ScenarioScore {
    pub sample_count: usize,
    pub median_micros: f64,
    pub p95_micros: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ParserPerformanceScorecard {
    pub schema_version: u32,
    pub benches: Vec<String>,
    pub scenarios: BTreeMap<String, ScenarioScore>,
}

impl Default for ParserPerformanceScorecard {
    fn default() -> Self {
        Self { schema_version: 1, benches: Vec::new(), scenarios: BTreeMap::new() }
    }
}

pub(crate) fn measure_scenario(mut op: impl FnMut(), samples: usize) -> ScenarioScore {
    let sample_count = samples.max(5);
    let mut durations = Vec::with_capacity(sample_count);
    for _ in 0..sample_count {
        let started = Instant::now();
        op();
        let elapsed = started.elapsed();
        durations.push(elapsed.as_secs_f64() * 1_000_000.0);
    }

    durations.sort_by(|a, b| a.total_cmp(b));

    let median = percentile(&durations, 0.50);
    let p95 = percentile(&durations, 0.95);

    ScenarioScore { sample_count, median_micros: median, p95_micros: p95 }
}

pub(crate) fn write_scorecard(
    bench_name: &str,
    scenarios: impl IntoIterator<Item = (&'static str, ScenarioScore)>,
) {
    let path = scorecard_path();
    let parent = match path.parent() {
        Some(parent) => parent,
        None => return,
    };

    if fs::create_dir_all(parent).is_err() {
        return;
    }

    let mut scorecard = fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<ParserPerformanceScorecard>(&raw).ok())
        .unwrap_or_default();

    for (name, score) in scenarios {
        scorecard.scenarios.insert(name.to_string(), score);
    }

    if !scorecard.benches.iter().any(|item| item == bench_name) {
        scorecard.benches.push(bench_name.to_string());
        scorecard.benches.sort();
    }

    if let Ok(json) = serde_json::to_string_pretty(&scorecard) {
        let _ = fs::write(path, json);
    }
}

pub(crate) fn scorecard_path() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir.parent().and_then(|path| path.parent()).map_or_else(
        || PathBuf::from("target/receipts/parser-performance-scorecard.json"),
        |root| root.join("target/receipts/parser-performance-scorecard.json"),
    )
}

fn percentile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }

    let upper_index = sorted.len().saturating_sub(1);
    let index = ((upper_index as f64) * q).round() as usize;
    sorted[index.min(upper_index)]
}
