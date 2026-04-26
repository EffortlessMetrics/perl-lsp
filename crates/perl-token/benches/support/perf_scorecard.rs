use perl_parser_core::percentile::nearest_rank_percentile;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const ARTIFACT_RELATIVE_PATH: &str = "docs/project/status/token_performance_scorecard.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ScoreMetric {
    pub(crate) iterations: usize,
    pub(crate) median_ns: u64,
    pub(crate) p95_ns: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenPerformanceScorecard {
    schema_version: u32,
    generated_at_epoch_s: u64,
    metrics: BTreeMap<String, ScoreMetric>,
}

impl Default for TokenPerformanceScorecard {
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

    // Discard warmup rounds so allocator and cache stabilization does not
    // pollute scored samples.
    let warmup = 2usize.min(rounds);
    for _ in 0..warmup {
        run();
    }

    let mut samples: Vec<u64> = Vec::with_capacity(rounds);
    for _ in 0..rounds {
        let start = Instant::now();
        run();
        let elapsed_ns = start.elapsed().as_nanos();
        let sample = u64::try_from(elapsed_ns).map_or(u64::MAX, |value| value);
        samples.push(sample);
    }

    samples.sort_unstable();
    let n = samples.len();
    let median_ns = samples.get(n / 2).copied().unwrap_or_default();
    let p95_ns = nearest_rank_percentile(&samples, 95);

    ScoreMetric { iterations: rounds, median_ns, p95_ns }
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
