use chrono::Utc;
use color_eyre::eyre::{Context, Result, bail};
use serde::Serialize;
use std::fs;
use std::process::Command;
use std::time::Instant;

use crate::utils::project_root;

const LANE_COMMANDS: &[(&str, &[&str])] = &[
    ("ci-format", &["ci-format"]),
    ("ci-docs-check", &["ci-docs-check"]),
    ("ci-clippy-lib", &["ci-clippy-lib"]),
    ("clippy-prod-no-unwrap", &["clippy-prod-no-unwrap"]),
    ("ci-test-lib", &["ci-test-lib"]),
    ("ci-lsp-def", &["ci-lsp-def"]),
    ("status-check", &["status-check"]),
];

#[derive(Serialize)]
struct CiLaneResult {
    name: String,
    seconds: f64,
    returncode: i32,
}

#[derive(Serialize)]
struct CiMeasurePayload {
    generated_at: String,
    lanes: Vec<CiLaneResult>,
    total_seconds: f64,
}

pub fn run() -> Result<()> {
    let root = project_root()?;

    let artifacts_dir = root.join("artifacts");
    fs::create_dir_all(&artifacts_dir).context("Failed to create artifacts directory")?;

    let mut lanes = Vec::new();
    let mut total_seconds = 0.0;
    let mut ndjson = String::new();

    for &(name, args) in LANE_COMMANDS.iter() {
        println!("==> {}", name);

        let start = Instant::now();
        let status = Command::new("just")
            .current_dir(&root)
            .args(args)
            .status()
            .with_context(|| format!("Failed to run '{name}' lane"))?;

        let elapsed = (start.elapsed().as_secs_f64() * 1000.0).round() / 1000.0;
        let returncode = status.code().unwrap_or(-1);
        let record = CiLaneResult { name: name.to_string(), seconds: elapsed, returncode };
        ndjson.push_str(&serde_json::to_string(&record)?);
        ndjson.push('\n');

        if !status.success() {
            bail!("CI lane '{name}' failed with exit code {returncode}");
        }

        total_seconds += elapsed;
        lanes.push(record);
    }

    let payload = CiMeasurePayload { generated_at: Utc::now().to_rfc3339(), lanes, total_seconds };

    let ndjson_path = artifacts_dir.join("ci-time.ndjson");
    let json_path = artifacts_dir.join("ci-time.json");
    let md_path = artifacts_dir.join("ci-time.md");

    fs::write(&ndjson_path, ndjson).context("Failed to write ci-time.ndjson")?;
    fs::write(&json_path, serde_json::to_string_pretty(&payload)?.as_bytes())
        .context("Failed to write ci-time.json")?;

    let mut markdown = String::new();
    markdown.push_str("# CI Timing Baseline\n\n");
    markdown.push_str(&format!("- Generated at: `{}`\n", payload.generated_at));
    markdown.push_str(&format!("- Total: `{}s`\n\n", payload.total_seconds));
    markdown.push_str("| Lane | Seconds | RC |\n|------|---------|----|\n");
    for lane in &payload.lanes {
        markdown
            .push_str(&format!("| `{}` | {} | {} |\n", lane.name, lane.seconds, lane.returncode));
    }
    fs::write(&md_path, markdown).context("Failed to write ci-time.md")?;

    println!("Wrote:");
    println!("  - {}", json_path.display());
    println!("  - {}", md_path.display());
    println!("  - {}", ndjson_path.display());

    Ok(())
}
