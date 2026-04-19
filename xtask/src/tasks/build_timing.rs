//! Build timing collection and comparison automation.

use crate::utils::project_root;
use chrono::Utc;
use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

const ARTIFACTS_DIR: &str = "artifacts";
const TIMING_RECEIPT_FILE: &str = "build-timing-receipt.json";
const TIMING_BASELINE_FILE: &str = "build-timing-baseline.json";
const LSP_PROVIDERS_LIB: &str = "crates/perl-lsp-providers/src/lib.rs";
const PARSER_LIB: &str = "crates/perl-parser/src/lib.rs";

#[derive(Serialize)]
struct BuildTimingReceipt {
    timestamp: String,
    toolchain: String,
    system: SystemInfo,
    measurements: BTreeMap<String, BuildMeasurement>,
}

#[derive(Serialize)]
struct SystemInfo {
    cpu_cores: Value,
    memory_gb: Value,
    os: String,
}

#[derive(Serialize, Deserialize)]
struct BuildMeasurement {
    duration_seconds: f64,
    command: String,
}

#[derive(Deserialize)]
struct MeasurableReceipt {
    timestamp: String,
    toolchain: String,
    measurements: BTreeMap<String, BuildMeasurement>,
}

pub fn run_receipt(
    run_clean: bool,
    run_incremental: bool,
    run_tests: bool,
    output: Option<PathBuf>,
    baseline: bool,
) -> Result<()> {
    let root = project_root()?;
    let mut run_clean = run_clean;
    let mut run_incremental = run_incremental;
    let mut run_tests = run_tests;

    if !run_clean && !run_incremental && !run_tests {
        run_clean = true;
        run_incremental = true;
        run_tests = true;
    }

    let artifacts = root.join(ARTIFACTS_DIR);
    fs::create_dir_all(&artifacts).with_context(|| {
        format!(
            "Failed to create artifacts directory {}",
            artifacts.display()
        )
    })?;

    let mut output_path = if let Some(output) = output {
        if output.is_absolute() {
            output
        } else {
            root.join(output)
        }
    } else {
        artifacts.join(TIMING_RECEIPT_FILE)
    };

    if baseline {
        output_path = artifacts.join(TIMING_BASELINE_FILE);
    }

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory {}", parent.display()))?;
    }

    let system = collect_system_info(&root);
    let mut measurements = BTreeMap::new();

    if run_clean {
        let measurement = measure_command(
            &root,
            "clean_build_workspace",
            &["cargo", "build", "--workspace", "--locked"],
            Some(&["cargo", "clean"]),
        );
        measurements.insert("clean_build_workspace".to_string(), measurement);
    }

    if run_incremental {
        let providers_dir = root.join("crates/perl-lsp-providers");
        run_command_silently(&root, &["cargo", "build", "--workspace", "--locked"]);
        if providers_dir.exists() {
            touch_file(&root.join(LSP_PROVIDERS_LIB))?;
            let measurement = measure_command(
                &root,
                "incremental_build_providers",
                &["cargo", "build", "-p", "perl-lsp-providers", "--locked"],
                None,
            );
            measurements.insert("incremental_build_providers".to_string(), measurement);
        } else {
            touch_file(&root.join(PARSER_LIB))?;
            let measurement = measure_command(
                &root,
                "incremental_build_parser",
                &["cargo", "build", "-p", "perl-parser", "--locked"],
                None,
            );
            measurements.insert("incremental_build_parser".to_string(), measurement);
        }
    }

    if run_tests {
        let measurement = measure_command(
            &root,
            "test_build_workspace",
            &["cargo", "test", "--workspace", "--lib", "--locked"],
            None,
        );
        measurements.insert("test_build_workspace".to_string(), measurement);
    }

    let receipt = BuildTimingReceipt {
        timestamp: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        toolchain: command_output_or_unknown(&["rustc", "--version"]),
        system,
        measurements,
    };

    let payload = serde_json::to_string_pretty(&receipt)
        .context("Failed to serialize build timing receipt")?;
    fs::write(&output_path, format!("{payload}\n")).context("Failed to write timing receipt")?;

    println!("=== Build Timing Receipt Generated ===");
    println!("Output: {}", output_path.display());
    println!();
    println!("{payload}");

    if baseline {
        println!();
        println!("Baseline saved to: {}", output_path.display());
        println!("Use this baseline to compare against future measurements:");
        println!(
            "  cargo xtask compare-build-timing {} <new-measurement.json>",
            output_path.display()
        );
    }

    Ok(())
}

pub fn run_compare(baseline: PathBuf, current: PathBuf) -> Result<()> {
    let baseline_path = resolve_path(&baseline)?;
    let current_path = resolve_path(&current)?;

    let baseline_raw = fs::read_to_string(&baseline_path)
        .with_context(|| format!("Failed to read {}", baseline_path.display()))?;
    let current_raw = fs::read_to_string(&current_path)
        .with_context(|| format!("Failed to read {}", current_path.display()))?;

    let baseline: MeasurableReceipt =
        serde_json::from_str(&baseline_raw).context("Failed to parse baseline receipt")?;
    let current: MeasurableReceipt =
        serde_json::from_str(&current_raw).context("Failed to parse current receipt")?;

    println!("# Build Timing Comparison");
    println!();
    println!("**Generated:** {}", Utc::now().format("%Y-%m-%dT%H:%M:%SZ"));
    println!();
    println!("## Metadata");
    println!("| Property | Baseline | Current |");
    println!("|----------|----------|---------|");
    println!(
        "| Timestamp | {} | {} |",
        baseline.timestamp, current.timestamp
    );
    println!(
        "| Toolchain | {} | {} |",
        baseline.toolchain, current.toolchain
    );
    println!();
    println!("## Build Timing Results");
    println!();
    println!("| Metric | Baseline | Current | Change | Improvement |");
    println!("|--------|----------|---------|---------|-------------|");

    let mut all_keys = BTreeSet::new();
    all_keys.extend(baseline.measurements.keys().cloned());
    all_keys.extend(current.measurements.keys().cloned());

    let mut total_metrics = 0usize;
    let mut improvements = 0usize;
    let mut regressions = 0usize;

    for key in all_keys {
        total_metrics += 1;
        let baseline_measurement = baseline.measurements.get(&key);
        let current_measurement = current.measurements.get(&key);
        match (baseline_measurement, current_measurement) {
            (Some(base), Some(curr)) => {
                let change = curr.duration_seconds - base.duration_seconds;
                let improvement = if base.duration_seconds != 0.0 {
                    (base.duration_seconds - curr.duration_seconds) / base.duration_seconds * 100.0
                } else {
                    0.0
                };

                let mut improvement_label = format!("{improvement:.1}%");
                if improvement > 0.0 {
                    improvements += 1;
                    improvement_label = format!("🟢 {improvement_label}");
                } else if improvement < 0.0 {
                    regressions += 1;
                    improvement_label = format!("🔴 {improvement_label}");
                }

                println!(
                    "| {key} | {} | {} | {} | {} |",
                    format_seconds(base.duration_seconds),
                    format_seconds(curr.duration_seconds),
                    format_signed_seconds(change),
                    improvement_label
                );
            }
            (Some(base), None) => {
                println!(
                    "| {key} | {} | N/A | N/A | N/A |",
                    format_seconds(base.duration_seconds)
                );
            }
            (None, Some(curr)) => {
                println!(
                    "| {key} | N/A | {} | N/A | N/A |",
                    format_seconds(curr.duration_seconds)
                );
            }
            _ => {}
        }
    }

    println!();
    println!("## Summary");
    println!();
    println!("- **Total metrics compared:** {total_metrics}");
    println!("- **Improvements:** {improvements}");
    println!("- **Regressions:** {regressions}");
    println!();

    println!("## Target Validation");
    println!();

    print_target_validation(
        "Full Workspace Build (Target: 40% faster)",
        baseline
            .measurements
            .get("clean_build_workspace")
            .map(|m| m.duration_seconds),
        current
            .measurements
            .get("clean_build_workspace")
            .map(|m| m.duration_seconds),
        Some(40.0),
    );

    let baseline_incremental = baseline
        .measurements
        .get("incremental_build_providers")
        .map(|m| m.duration_seconds)
        .or_else(|| {
            baseline
                .measurements
                .get("incremental_build_parser")
                .map(|m| m.duration_seconds)
        });
    let current_incremental = current
        .measurements
        .get("incremental_build_providers")
        .map(|m| m.duration_seconds)
        .or_else(|| {
            current
                .measurements
                .get("incremental_build_parser")
                .map(|m| m.duration_seconds)
        });

    print_target_validation(
        "Incremental Build (Target: 67% faster)",
        baseline_incremental,
        current_incremental,
        Some(67.0),
    );

    print_target_validation_no_target(
        "Test Build",
        baseline
            .measurements
            .get("test_build_workspace")
            .map(|m| m.duration_seconds),
        current
            .measurements
            .get("test_build_workspace")
            .map(|m| m.duration_seconds),
    );

    Ok(())
}

fn resolve_path(path: &PathBuf) -> Result<PathBuf> {
    let root = project_root()?;
    let candidate = if path.is_absolute() {
        path.clone()
    } else {
        root.join(path)
    };
    Ok(candidate)
}

fn format_seconds(value: f64) -> String {
    format!("{value:.1}s")
}

fn format_signed_seconds(value: f64) -> String {
    format!("{value:+.1}s")
}

fn print_target_validation(
    label: &str,
    baseline: Option<f64>,
    current: Option<f64>,
    target: Option<f64>,
) {
    match (baseline, current) {
        (Some(base), Some(cur)) => {
            let improvement = if base != 0.0 {
                (base - cur) / base * 100.0
            } else {
                0.0
            };

            println!("### {label}");
            println!("- Baseline: {}", format_seconds(base));
            println!("- Current: {}", format_seconds(cur));
            println!("- Improvement: {improvement:.1}%");
            if let Some(target) = target {
                if improvement >= target {
                    println!("- Status: ✅ **Target Met**");
                } else {
                    println!("- Status: ❌ **Target Not Met**");
                }
            }
            println!();
        }
        _ => {
            println!("### {label}");
            println!("- Status: ⚠️ **No data available**");
            println!();
        }
    }
}

fn print_target_validation_no_target(label: &str, baseline: Option<f64>, current: Option<f64>) {
    match (baseline, current) {
        (Some(base), Some(cur)) => {
            let improvement = if base != 0.0 {
                (base - cur) / base * 100.0
            } else {
                0.0
            };

            println!("### {label}");
            println!("- Baseline: {}", format_seconds(base));
            println!("- Current: {}", format_seconds(cur));
            println!("- Improvement: {improvement:.1}%");
            println!();
        }
        _ => {
            println!("### {label}");
            println!("- Status: ⚠️ **No data available**");
            println!();
        }
    }
}

fn collect_system_info(_root: &Path) -> SystemInfo {
    SystemInfo {
        cpu_cores: command_output_parse_or_unknown(&["nproc"])
            .or_else(|| command_output_parse_or_unknown(&["getconf", "_NPROCESSORS_ONLN"]))
            .unwrap_or_else(|| Value::String("unknown".to_string())),
        memory_gb: detect_memory_gb(),
        os: command_output_or_unknown(&["uname", "-s", "-r"]),
    }
}

fn detect_memory_gb() -> Value {
    if let Some(mem) = parse_memory_from_free() {
        return Value::from(mem);
    }

    command_output_parse_f64_or_unknown(&["sysctl", "-n", "hw.memsize"])
        .and_then(|value| {
            if value.is_number()
                && let Some(raw) = value.as_f64()
            {
                return Some(Value::from((raw / 1024.0 / 1024.0 / 1024.0).round() as u64));
            }
            None
        })
        .unwrap_or_else(|| Value::String("unknown".to_string()))
}

fn parse_memory_from_free() -> Option<u64> {
    let output = command_output(&["free", "-g"])?;
    for line in output.lines() {
        if line.starts_with("Mem:") {
            return line
                .split_whitespace()
                .nth(1)
                .and_then(|value| value.parse::<u64>().ok());
        }
    }
    None
}

fn command_output_or_unknown(command: &[&str]) -> String {
    match command_output(command) {
        Some(value) => value,
        None => "unknown".to_string(),
    }
}

fn command_output_parse_or_unknown(command: &[&str]) -> Option<Value> {
    command_output(command).and_then(|value| value.trim().parse::<u64>().ok().map(Value::from))
}

fn command_output_parse_f64_or_unknown(command: &[&str]) -> Option<Value> {
    command_output(command).and_then(|value| value.trim().parse::<f64>().ok().map(Value::from))
}

fn command_output(command: &[&str]) -> Option<String> {
    let (program, args) = command.split_first()?;
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn touch_file(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {} parent directories", parent.display()))?;
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(&[])?;
    Ok(())
}

fn measure_command(
    root: &Path,
    name: &str,
    command: &[&str],
    pre_command: Option<&[&str]>,
) -> BuildMeasurement {
    println!("=== Measuring: {name} ===");
    println!("Command: {}", command_to_string(command));

    if let Some(pre) = pre_command {
        println!("Pre-command: {}", command_to_string(pre));
        run_command_silently(root, pre);
    }

    let duration = run_command_silently(root, command);
    println!("Duration: {duration:.4}s");
    println!();

    BuildMeasurement {
        duration_seconds: duration,
        command: command_to_string(command),
    }
}

fn run_command_silently(root: &Path, command: &[&str]) -> f64 {
    let start = Instant::now();
    let status = Command::new(command[0])
        .current_dir(root)
        .args(&command[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if let Ok(status) = status {
        if !status.success() {
            println!("⚠️ Command exited with status {status}");
        }
    } else {
        println!(
            "⚠️ Command failed to launch: {}",
            command_to_string(command)
        );
    }

    start.elapsed().as_secs_f64()
}

fn command_to_string(command: &[&str]) -> String {
    command.join(" ")
}
