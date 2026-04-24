use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

const SCORECARD_PATH: &str = "docs/project/status/parser-performance-scorecard.json";
const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct ParserPerformanceScorecard {
    pub schema_version: u32,
    pub measurements: BTreeMap<String, BenchmarkMeasurement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BenchmarkMeasurement {
    pub regime: String,
    pub benchmark: String,
    pub iterations: u32,
    pub median_ns: u64,
    pub p95_ns: u64,
}

pub(crate) fn measure_and_record<F>(
    key: &str,
    regime: &str,
    benchmark: &str,
    iterations: u32,
    mut workload: F,
) where
    F: FnMut(),
{
    let mut samples = Vec::with_capacity(iterations as usize);

    for _ in 0..3 {
        workload();
    }

    for _ in 0..iterations {
        let start = Instant::now();
        workload();
        samples.push(start.elapsed().as_nanos() as u64);
    }

    samples.sort_unstable();
    let median_ns = percentile(&samples, 0.5);
    let p95_ns = percentile(&samples, 0.95);

    let mut scorecard = read_existing_scorecard().unwrap_or_default();
    if scorecard.schema_version == 0 {
        scorecard.schema_version = SCHEMA_VERSION;
    }

    scorecard.measurements.insert(
        key.to_string(),
        BenchmarkMeasurement {
            regime: regime.to_string(),
            benchmark: benchmark.to_string(),
            iterations,
            median_ns,
            p95_ns,
        },
    );

    let _ = write_scorecard(&scorecard);
}

fn percentile(values: &[u64], p: f64) -> u64 {
    if values.is_empty() {
        return 0;
    }
    let last = values.len().saturating_sub(1);
    let idx = (last as f64 * p).round() as usize;
    values[idx.min(last)]
}

fn read_existing_scorecard() -> Option<ParserPerformanceScorecard> {
    let path = scorecard_path();
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn write_scorecard(scorecard: &ParserPerformanceScorecard) -> Result<(), std::io::Error> {
    let path = scorecard_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let serialized = serde_json::to_string_pretty(scorecard).map_err(std::io::Error::other)?;
    fs::write(path, format!("{serialized}\n"))
}

fn scorecard_path() -> PathBuf {
    // CARGO_MANIFEST_DIR points to crates/perl-parser for these benches.
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    crate_root.join("../..").join(SCORECARD_PATH)
}
