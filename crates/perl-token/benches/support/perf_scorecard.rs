use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use perl_parser_core::percentile::nearest_rank_percentile;

const ARTIFACT_RELATIVE_PATH: &str = "docs/project/status/token_performance_scorecard.json";
const WARMUP_ROUNDS: usize = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ScoreMetric {
    pub(crate) name: String,
    pub(crate) iterations: usize,
    pub(crate) median_ns: u128,
    pub(crate) p95_ns: u128,
    pub(crate) mean_ns: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TokenPerformanceScorecard {
    pub(crate) schema_version: u32,
    pub(crate) generated_at_epoch_s: u64,
    pub(crate) metrics: BTreeMap<String, ScoreMetric>,
}

impl Default for TokenPerformanceScorecard {
    fn default() -> Self {
        Self { schema_version: 1, generated_at_epoch_s: 0, metrics: BTreeMap::new() }
    }
}

pub(crate) fn sample_metric<F>(name: &str, iterations: usize, mut run: F) -> ScoreMetric
where
    F: FnMut(),
{
    let scored_rounds = iterations.max(5);
    let warmup = WARMUP_ROUNDS.min(scored_rounds);

    for _ in 0..warmup {
        run();
    }

    let mut samples: Vec<u128> = Vec::with_capacity(scored_rounds);
    for _ in 0..scored_rounds {
        let start = Instant::now();
        run();
        samples.push(start.elapsed().as_nanos());
    }

    samples.sort_unstable();
    let total: u128 = samples.iter().copied().sum();
    let samples_u64: Vec<u64> = samples
        .iter()
        .map(|ns| (*ns).min(u64::MAX as u128) as u64)
        .collect();
    let median_ns = u128::from(nearest_rank_percentile(&samples_u64, 50));
    let p95_ns = u128::from(nearest_rank_percentile(&samples_u64, 95));

    ScoreMetric {
        name: name.to_string(),
        iterations: scored_rounds,
        median_ns,
        p95_ns,
        mean_ns: if scored_rounds == 0 { 0 } else { total / scored_rounds as u128 },
    }
}

pub(crate) fn record_metric(metric: ScoreMetric) {
    let Some(path) = find_artifact_path() else {
        return;
    };

    let mut scorecard = read_scorecard(&path).unwrap_or_default();
    scorecard.generated_at_epoch_s = now_epoch_seconds();
    scorecard.metrics.insert(metric.name.clone(), metric);

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let Ok(json) = serde_json::to_string_pretty(&scorecard) else {
        return;
    };
    let _ = fs::write(path, json);
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

fn read_scorecard(path: &Path) -> Option<TokenPerformanceScorecard> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn now_epoch_seconds() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}
