use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfMeasurement {
    pub iterations: u32,
    pub median_us: f64,
    pub p95_us: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ParserPerformanceScorecard {
    schema_version: u32,
    measured_at_unix_secs: u64,
    metrics: BTreeMap<String, PerfMeasurement>,
}

pub fn measure_iterations<F>(iterations: u32, mut f: F) -> PerfMeasurement
where
    F: FnMut(),
{
    let mut samples: Vec<f64> = Vec::with_capacity(iterations as usize);
    for _ in 0..iterations {
        let started = Instant::now();
        f();
        samples.push(started.elapsed().as_secs_f64() * 1_000_000.0);
    }

    samples.sort_by(f64::total_cmp);

    let len = samples.len();
    let median_idx = len / 2;
    let p95_idx = ((len.saturating_sub(1) as f64) * 0.95).round() as usize;

    PerfMeasurement {
        iterations,
        median_us: samples.get(median_idx).copied().unwrap_or(0.0),
        p95_us: samples.get(p95_idx).copied().unwrap_or(0.0),
    }
}

pub fn write_scorecard(entries: Vec<(&'static str, PerfMeasurement)>) {
    if entries.is_empty() {
        return;
    }

    if let Err(err) = try_write_scorecard(entries) {
        eprintln!("parser scorecard emission failed: {err}");
    }
}

fn try_write_scorecard(entries: Vec<(&'static str, PerfMeasurement)>) -> Result<(), String> {
    let root = project_root()?;
    let out_path = root.join("target/metrics/parser-performance-scorecard.json");

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("create_dir_all({}): {e}", parent.display()))?;
    }

    let mut current = fs::read_to_string(&out_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<ParserPerformanceScorecard>(&raw).ok())
        .unwrap_or_else(|| ParserPerformanceScorecard {
            schema_version: 1,
            measured_at_unix_secs: 0,
            metrics: BTreeMap::new(),
        });

    for (name, measurement) in entries {
        current.metrics.insert(name.to_string(), measurement);
    }

    current.schema_version = 1;
    current.measured_at_unix_secs =
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

    let payload = serde_json::to_string_pretty(&current)
        .map_err(|e| format!("serialize scorecard JSON: {e}"))?;
    fs::write(&out_path, payload).map_err(|e| format!("write {}: {e}", out_path.display()))?;
    Ok(())
}

fn project_root() -> Result<PathBuf, String> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .ok_or_else(|| "failed to resolve workspace root from CARGO_MANIFEST_DIR".to_string())
}
