use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

const ARTIFACT_RELATIVE_PATH: &str = "target/metrics/parser-performance-scorecard.json";
const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScorecardMetric {
    pub iterations: usize,
    pub median_ns: u64,
    pub p95_ns: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserPerformanceScorecard {
    pub schema_version: u32,
    pub generator: String,
    pub measured_at_unix_secs: u64,
    pub metrics: BTreeMap<String, ScorecardMetric>,
}

impl Default for ParserPerformanceScorecard {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            generator: "perl-parser benches".to_string(),
            measured_at_unix_secs: unix_seconds_now(),
            metrics: BTreeMap::new(),
        }
    }
}

pub fn measure<F>(iterations: usize, mut op: F) -> ScorecardMetric
where
    F: FnMut(),
{
    let mut samples: Vec<u64> = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        op();
        let nanos_u128 = start.elapsed().as_nanos();
        let nanos = u64::try_from(nanos_u128).unwrap_or(u64::MAX);
        samples.push(nanos);
    }

    samples.sort_unstable();
    let median_idx = samples.len() / 2;
    let p95_idx = (samples.len() * 95 / 100).min(samples.len().saturating_sub(1));

    ScorecardMetric {
        iterations,
        median_ns: *samples.get(median_idx).unwrap_or(&0),
        p95_ns: *samples.get(p95_idx).unwrap_or(&0),
    }
}

pub fn upsert_metric(name: &str, metric: ScorecardMetric) {
    let path = artifact_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let mut artifact = read_artifact(&path).unwrap_or_default();
    artifact.schema_version = SCHEMA_VERSION;
    artifact.measured_at_unix_secs = unix_seconds_now();
    artifact.metrics.insert(name.to_string(), metric);

    if let Ok(json) = serde_json::to_string_pretty(&artifact) {
        let _ = fs::write(&path, json);
    }
}

fn read_artifact(path: &Path) -> Option<ParserPerformanceScorecard> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn artifact_path() -> PathBuf {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_root.join("../..").join(ARTIFACT_RELATIVE_PATH)
}

fn unix_seconds_now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}
