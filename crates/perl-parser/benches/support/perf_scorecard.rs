use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const ARTIFACT_RELATIVE_PATH: &str = "docs/project/status/parser_performance_scorecard.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ScoreMetric {
    pub(crate) iterations: usize,
    pub(crate) median_ns: u128,
    pub(crate) p95_ns: u128,
    pub(crate) mean_ns: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ParserPerformanceScorecard {
    pub(crate) schema_version: u32,
    pub(crate) generated_at_epoch_s: u64,
    pub(crate) metrics: BTreeMap<String, ScoreMetric>,
}

impl Default for ParserPerformanceScorecard {
    fn default() -> Self {
        Self { schema_version: 1, generated_at_epoch_s: 0, metrics: BTreeMap::new() }
    }
}

pub(crate) fn record_metric(name: &str, metric: ScoreMetric) {
    let Some(path) = find_artifact_path() else {
        return;
    };

    let mut scorecard = read_scorecard(&path).unwrap_or_default();
    scorecard.generated_at_epoch_s = now_epoch_seconds();
    scorecard.metrics.insert(name.to_string(), metric);

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let Ok(json) = serde_json::to_string_pretty(&scorecard) else {
        return;
    };
    let _ = fs::write(path, json);
}

pub(crate) fn sample_metric<F>(iterations: usize, mut run: F) -> ScoreMetric
where
    F: FnMut(),
{
    let rounds = iterations.max(5);
    let mut samples: Vec<u128> = Vec::with_capacity(rounds);

    for _ in 0..rounds {
        let start = Instant::now();
        run();
        samples.push(start.elapsed().as_nanos());
    }

    samples.sort_unstable();
    let median_idx = samples.len() / 2;
    let p95_idx = ((samples.len() * 95) / 100).min(samples.len().saturating_sub(1));
    let total: u128 = samples.iter().copied().sum();

    ScoreMetric {
        iterations: rounds,
        median_ns: samples.get(median_idx).copied().unwrap_or_default(),
        p95_ns: samples.get(p95_idx).copied().unwrap_or_default(),
        mean_ns: total / rounds as u128,
    }
}

fn find_artifact_path() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join(ARTIFACT_RELATIVE_PATH);
        if candidate.parent().is_some_and(|parent| parent.exists()) {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn read_scorecard(path: &Path) -> Option<ParserPerformanceScorecard> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn now_epoch_seconds() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}
