use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const DEFAULT_SAMPLES: usize = 25;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ScorecardMetric {
    pub(crate) median_us: f64,
    pub(crate) p95_us: f64,
    pub(crate) samples: usize,
    pub(crate) unit: String,
    pub(crate) source_bench: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ParserPerformanceScorecard {
    pub(crate) schema_version: u32,
    pub(crate) metrics: BTreeMap<String, ScorecardMetric>,
}

impl Default for ParserPerformanceScorecard {
    fn default() -> Self {
        Self { schema_version: 1, metrics: BTreeMap::new() }
    }
}

pub(crate) fn load_or_default(path: &Path) -> ParserPerformanceScorecard {
    let Ok(raw) = fs::read_to_string(path) else {
        return ParserPerformanceScorecard::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub(crate) fn scorecard_path() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| manifest_dir.to_path_buf());
    workspace_root.join("target/receipts/parser-performance-scorecard.json")
}

pub(crate) fn upsert_metric(
    path: &Path,
    metric_name: &str,
    source_bench: &str,
    sample_micros: Vec<f64>,
) -> std::io::Result<()> {
    if sample_micros.is_empty() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut scorecard = load_or_default(path);
    scorecard.metrics.insert(metric_name.to_string(), summarize(sample_micros, source_bench));

    let json = serde_json::to_string_pretty(&scorecard)?;
    fs::write(path, json)
}

fn summarize(mut sample_micros: Vec<f64>, source_bench: &str) -> ScorecardMetric {
    sample_micros.sort_by(|a, b| a.total_cmp(b));

    let len = sample_micros.len();
    let median_idx = len / 2;
    let p95_idx = ((len as f64 * 0.95).ceil() as usize).saturating_sub(1).min(len - 1);

    ScorecardMetric {
        median_us: sample_micros[median_idx],
        p95_us: sample_micros[p95_idx],
        samples: len,
        unit: "us".to_string(),
        source_bench: source_bench.to_string(),
    }
}

pub(crate) fn measure_samples_us(mut f: impl FnMut()) -> Vec<f64> {
    let mut sample_micros = Vec::with_capacity(DEFAULT_SAMPLES);

    for _ in 0..DEFAULT_SAMPLES {
        let start = Instant::now();
        f();
        let elapsed = start.elapsed();
        sample_micros.push(duration_to_micros(elapsed));
    }

    sample_micros
}

fn duration_to_micros(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000_000.0
}
