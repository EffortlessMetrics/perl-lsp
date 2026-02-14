//! Gate execution harness for CI gates
//!
//! This module implements a structured gate runner that:
//! - Reads gate definitions from `.ci/gate-policy.yaml`
//! - Executes gates with proper environment setup
//! - Captures timing, output, and status for each gate
//! - Generates receipts following the receipt.schema.json format
//!
//! # Usage
//!
//! ```bash
//! cargo xtask gates                    # Run all merge_gate tier
//! cargo xtask gates --tier pr-fast     # Run pr_fast tier only
//! cargo xtask gates --gate fmt         # Run single gate
//! cargo xtask gates --list             # List available gates
//! cargo xtask gates --receipt          # Output receipt to stdout
//! cargo xtask gates --diff baseline.json  # Compare against baseline
//! ```

use chrono::{DateTime, Utc};
use color_eyre::eyre::{Context, Result, bail};
use console::{Style, Term};
use duct::cmd;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

use crate::utils::project_root;

// =============================================================================
// CLI Types
// =============================================================================

/// Gate tier for filtering
#[derive(Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum GateTier {
    /// Fast checks for every PR iteration (~1-2 min)
    PrFast,
    /// Full verification before merge (~3-8 min)
    MergeGate,
    /// Scheduled comprehensive tests (~15-60 min)
    Nightly,
    /// All tiers combined
    All,
}

impl std::fmt::Display for GateTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GateTier::PrFast => write!(f, "pr_fast"),
            GateTier::MergeGate => write!(f, "merge_gate"),
            GateTier::Nightly => write!(f, "nightly"),
            GateTier::All => write!(f, "all"),
        }
    }
}

/// Output format for gate results
#[derive(Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    /// Human-readable terminal output (default)
    Human,
    /// JSON receipt format
    Json,
    /// Minimal summary for CI logs
    Summary,
}

// =============================================================================
// Gate Policy Schema (from .ci/gate-policy.yaml)
// =============================================================================
// Note: Some fields are parsed for future use (budgets, matrix, etc.)
// and are intentionally unused in the current implementation.

/// Top-level gate policy configuration
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct GatePolicy {
    pub schema_version: u32,
    pub global: GlobalSettings,
    pub tiers: HashMap<String, TierDefinition>,
    pub gates: Vec<GateDefinition>,
    #[serde(default)]
    pub flake_policy: Option<FlakePolicy>,
    #[serde(default)]
    pub audit: Option<AuditConfig>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct GlobalSettings {
    pub default_timeout_seconds: u64,
    #[serde(default)]
    pub artifact_retention_days: u32,
    #[serde(default)]
    pub default_retry_count: u32,
    #[serde(default)]
    pub environment: HashMap<String, String>,
    #[serde(default)]
    pub toolchain: Option<ToolchainConfig>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ToolchainConfig {
    pub msrv: Option<String>,
    #[serde(default)]
    pub components: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct TierDefinition {
    pub description: String,
    pub target_duration_seconds: u64,
    pub enforcement: String,
    #[serde(default)]
    pub trigger: Vec<serde_yaml_ng::Value>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct GateDefinition {
    pub name: String,
    pub tier: String,
    pub description: String,
    #[serde(default = "default_true")]
    pub required: bool,
    pub command: String,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default)]
    pub retry_count: u32,
    #[serde(default)]
    pub budgets: Option<GateBudgets>,
    #[serde(default)]
    pub quarantine: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub matrix: Option<serde_yaml_ng::Value>,
}

fn default_true() -> bool {
    true
}

fn default_timeout() -> u64 {
    300
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct GateBudgets {
    pub max_duration_ms: Option<u64>,
    pub max_warnings: Option<u32>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct FlakePolicy {
    pub max_retries: u32,
    pub auto_quarantine_threshold: u32,
    pub quarantine_duration_days: u32,
    #[serde(default)]
    pub quarantined_gates: Vec<QuarantinedGate>,
    #[serde(default)]
    pub known_flaky_patterns: Vec<FlakyPattern>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct QuarantinedGate {
    pub gate: String,
    pub reason: String,
    pub quarantined_at: String,
    pub issue: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct FlakyPattern {
    pub pattern: String,
    pub reason: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct AuditConfig {
    pub receipt_path: String,
    pub log_directory: String,
    pub retention_days: u32,
}

// =============================================================================
// Receipt Schema (from .ci/receipt.schema.json)
// =============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Receipt {
    pub schema_version: String,
    pub metadata: ReceiptMetadata,
    pub gates: Vec<GateResult>,
    pub summary: ReceiptSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_config: Option<DiffConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReceiptMetadata {
    pub timestamp: String,
    pub git_sha: String,
    pub git_sha_short: String,
    pub git_branch: String,
    pub git_dirty: bool,
    pub toolchain: ToolchainInfo,
    pub platform: PlatformInfo,
    pub environment: EnvironmentInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolchainInfo {
    pub rustc_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rustc_channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rustc_semver: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cargo_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nix_version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlatformInfo {
    pub os: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_version: Option<String>,
    pub arch: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_cores: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_wsl: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnvironmentInfo {
    #[serde(rename = "type")]
    pub env_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ci_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ci_run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ci_run_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_number: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nix_shell: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GateResult {
    pub gate_name: String,
    pub tier: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    pub duration_ms: u64,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<GateMetrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GateMetrics {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tests_total: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tests_passed: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tests_failed: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tests_skipped: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tests_ignored: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coverage_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_peak_mb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_checked: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReceiptSummary {
    pub total_gates: u32,
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<u32>,
    pub total_duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier_results: Option<HashMap<String, TierSummary>>,
    pub overall_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocking_failures: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregate_metrics: Option<AggregateMetrics>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TierSummary {
    pub total: u32,
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
    pub duration_ms: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AggregateMetrics {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tests: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tests_passed: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tests_failed: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_warnings: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_memory_mb: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiffConfig {
    pub comparable_fields: Vec<String>,
    pub ignored_fields: Vec<String>,
    pub threshold_fields: HashMap<String, f64>,
}

// =============================================================================
// Diff Result Types
// =============================================================================

#[derive(Debug, Serialize)]
pub struct DiffResult {
    pub baseline_timestamp: String,
    pub current_timestamp: String,
    pub gates_added: Vec<String>,
    pub gates_removed: Vec<String>,
    pub status_changes: Vec<StatusChange>,
    pub metric_changes: Vec<MetricChange>,
    pub overall_regression: bool,
}

#[derive(Debug, Serialize)]
pub struct StatusChange {
    pub gate_name: String,
    pub old_status: String,
    pub new_status: String,
    pub is_regression: bool,
}

#[derive(Debug, Serialize)]
pub struct MetricChange {
    pub gate_name: String,
    pub metric_name: String,
    pub old_value: f64,
    pub new_value: f64,
    pub delta_percent: f64,
    pub exceeds_threshold: bool,
}

// =============================================================================
// Gate Runner Implementation
// =============================================================================

/// Configuration for the gate runner
pub struct GateRunnerConfig {
    pub tier: GateTier,
    pub gate_filter: Option<String>,
    pub output_format: OutputFormat,
    pub emit_receipt: bool,
    pub receipt_path: Option<PathBuf>,
    pub diff_baseline: Option<PathBuf>,
    pub list_only: bool,
    pub fail_fast: bool,
    /// For future parallel execution support
    #[allow(dead_code)]
    pub parallel: bool,
    pub verbose: bool,
}

impl Default for GateRunnerConfig {
    fn default() -> Self {
        Self {
            tier: GateTier::MergeGate,
            gate_filter: None,
            output_format: OutputFormat::Human,
            emit_receipt: false,
            receipt_path: None,
            diff_baseline: None,
            list_only: false,
            fail_fast: false,
            parallel: false,
            verbose: false,
        }
    }
}

/// Main entry point for gate execution
pub fn run(config: GateRunnerConfig) -> Result<()> {
    let root = project_root()?;
    std::env::set_current_dir(&root).context("Failed to change to project root")?;

    // Load gate policy
    let policy_path = root.join(".ci/gate-policy.yaml");
    let policy = load_policy(&policy_path)?;

    // Filter gates based on tier and gate filter
    let gates_to_run = filter_gates(&policy, &config)?;

    // Handle list mode
    if config.list_only {
        return list_gates(&gates_to_run, &policy);
    }

    // Handle diff mode
    if let Some(baseline_path) = &config.diff_baseline {
        let baseline = load_receipt(baseline_path)?;
        let current = run_gates(&gates_to_run, &policy, &config)?;
        let diff = compare_receipts(&baseline, &current)?;
        return output_diff(&diff, &config);
    }

    // Run gates
    let receipt = run_gates(&gates_to_run, &policy, &config)?;

    // Output results
    output_results(&receipt, &config)?;

    // Write receipt if requested
    if config.emit_receipt {
        let receipt_path = config
            .receipt_path
            .clone()
            .unwrap_or_else(|| root.join("target/receipts/receipt.json"));
        write_receipt(&receipt, &receipt_path)?;
    }

    // Exit with appropriate code
    if receipt.summary.failed > 0 && has_blocking_failures(&receipt) {
        bail!("One or more required gates failed");
    }

    Ok(())
}

/// Load gate policy from YAML file
fn load_policy(path: &PathBuf) -> Result<GatePolicy> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read gate policy from {}", path.display()))?;
    let policy: GatePolicy = serde_yaml_ng::from_str(&content)
        .with_context(|| format!("Failed to parse gate policy from {}", path.display()))?;
    Ok(policy)
}

/// Filter gates based on tier and gate name filter
fn filter_gates<'a>(
    policy: &'a GatePolicy,
    config: &GateRunnerConfig,
) -> Result<Vec<&'a GateDefinition>> {
    let mut gates: Vec<&GateDefinition> = policy.gates.iter().collect();

    // Filter by specific gate name
    if let Some(gate_name) = &config.gate_filter {
        gates.retain(|g| g.name == *gate_name);
        if gates.is_empty() {
            bail!("No gate found with name '{}'", gate_name);
        }
        return Ok(gates);
    }

    // Filter by tier
    match config.tier {
        GateTier::PrFast => {
            gates.retain(|g| g.tier == "pr_fast");
        }
        GateTier::MergeGate => {
            // merge_gate includes pr_fast gates plus merge_gate gates
            gates.retain(|g| g.tier == "pr_fast" || g.tier == "merge_gate");
        }
        GateTier::Nightly => {
            // nightly includes all tiers
            // Keep all gates
        }
        GateTier::All => {
            // Keep all gates
        }
    }

    // Sort by tier priority (pr_fast first, then merge_gate, then nightly)
    gates.sort_by_key(|g| match g.tier.as_str() {
        "pr_fast" => 0,
        "merge_gate" => 1,
        "nightly" => 2,
        _ => 3,
    });

    Ok(gates)
}

/// List available gates
fn list_gates(gates: &[&GateDefinition], policy: &GatePolicy) -> Result<()> {
    let mut term = Term::stdout();
    let bold = Style::new().bold();
    let dim = Style::new().dim();

    writeln!(term, "{}", bold.apply_to("Available Gates"))?;
    writeln!(term, "{}", "=".repeat(60))?;
    writeln!(term)?;

    // Group by tier
    let mut by_tier: HashMap<&str, Vec<&&GateDefinition>> = HashMap::new();
    for gate in gates {
        by_tier.entry(gate.tier.as_str()).or_default().push(gate);
    }

    for tier_name in &["pr_fast", "merge_gate", "nightly"] {
        if let Some(tier_gates) = by_tier.get(tier_name) {
            let tier_def = policy.tiers.get(*tier_name);
            let tier_desc = tier_def.map(|t| t.description.as_str()).unwrap_or("Unknown tier");

            writeln!(
                term,
                "{} {}",
                bold.apply_to(tier_name),
                dim.apply_to(format!("({})", tier_desc))
            )?;
            writeln!(term, "{}", "-".repeat(60))?;

            for gate in tier_gates {
                let required_indicator = if gate.required { "*" } else { " " };
                let quarantine_indicator = if gate.quarantine { " [Q]" } else { "" };
                writeln!(
                    term,
                    "  {}{} {}{}",
                    required_indicator,
                    bold.apply_to(&gate.name),
                    dim.apply_to(&gate.description),
                    quarantine_indicator
                )?;
            }
            writeln!(term)?;
        }
    }

    writeln!(term, "{}", dim.apply_to("* = required gate, [Q] = quarantined"))?;

    Ok(())
}

/// Run gates and collect results
fn run_gates(
    gates: &[&GateDefinition],
    policy: &GatePolicy,
    config: &GateRunnerConfig,
) -> Result<Receipt> {
    let root = project_root()?;
    let start_time = Instant::now();
    let timestamp: DateTime<Utc> = Utc::now();

    // Collect metadata
    let metadata = collect_metadata(timestamp)?;

    // Create log directory
    let log_dir = root.join("target/receipts/logs");
    fs::create_dir_all(&log_dir).context("Failed to create log directory")?;

    // Run each gate
    let mut results: Vec<GateResult> = Vec::new();
    let mut tier_summaries: HashMap<String, TierSummary> = HashMap::new();

    let spinner = if config.output_format == OutputFormat::Human {
        let pb = ProgressBar::new(gates.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {wide_msg}")
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .progress_chars("#>-"),
        );
        Some(pb)
    } else {
        None
    };

    for (idx, gate) in gates.iter().enumerate() {
        if let Some(ref pb) = spinner {
            pb.set_position(idx as u64);
            pb.set_message(format!("Running {}...", gate.name));
        }

        let result = run_single_gate(gate, policy, &log_dir, config)?;

        // Update tier summary
        let tier_summary = tier_summaries.entry(gate.tier.clone()).or_default();
        tier_summary.total += 1;
        tier_summary.duration_ms += result.duration_ms;
        match result.status.as_str() {
            "pass" => tier_summary.passed += 1,
            "fail" => tier_summary.failed += 1,
            "skip" => tier_summary.skipped += 1,
            _ => {}
        }

        // Print result in human mode
        if let Some(ref pb) = spinner {
            let status_icon = match result.status.as_str() {
                "pass" => "PASS",
                "fail" => "FAIL",
                "skip" => "SKIP",
                "timeout" => "TIME",
                _ => "ERR",
            };
            pb.println(format!(
                "[{:>4}] {} ({:.1}s)",
                status_icon,
                gate.name,
                result.duration_ms as f64 / 1000.0
            ));
        }

        // Check for fail-fast
        if config.fail_fast && result.status == "fail" && gate.required {
            if let Some(ref pb) = spinner {
                pb.finish_with_message("Gate failed, stopping (fail-fast mode)");
            }
            results.push(result);
            break;
        }

        results.push(result);
    }

    if let Some(ref pb) = spinner {
        pb.finish_and_clear();
    }

    // Build summary
    let total_duration_ms = start_time.elapsed().as_millis() as u64;
    let passed = results.iter().filter(|r| r.status == "pass").count() as u32;
    let failed = results.iter().filter(|r| r.status == "fail").count() as u32;
    let skipped = results.iter().filter(|r| r.status == "skip").count() as u32;
    let timeout = results.iter().filter(|r| r.status == "timeout").count() as u32;
    let error = results.iter().filter(|r| r.status == "error").count() as u32;

    let blocking_failures: Vec<String> = results
        .iter()
        .filter(|r| r.status == "fail" && r.required.unwrap_or(true))
        .map(|r| r.gate_name.clone())
        .collect();

    let overall_status = if blocking_failures.is_empty() {
        if failed > 0 { "partial" } else { "pass" }
    } else {
        "fail"
    };

    let summary = ReceiptSummary {
        total_gates: results.len() as u32,
        passed,
        failed,
        skipped,
        timeout: if timeout > 0 { Some(timeout) } else { None },
        error: if error > 0 { Some(error) } else { None },
        total_duration_ms,
        tier_results: if tier_summaries.is_empty() { None } else { Some(tier_summaries) },
        overall_status: overall_status.to_string(),
        blocking_failures: if blocking_failures.is_empty() {
            None
        } else {
            Some(blocking_failures)
        },
        aggregate_metrics: None, // Could aggregate test counts etc.
    };

    Ok(Receipt {
        schema_version: "1.0.0".to_string(),
        metadata,
        gates: results,
        summary,
        diff_config: None,
    })
}

/// Run a single gate and capture its result
fn run_single_gate(
    gate: &GateDefinition,
    policy: &GatePolicy,
    log_dir: &std::path::Path,
    config: &GateRunnerConfig,
) -> Result<GateResult> {
    let start = Instant::now();
    let log_path = log_dir.join(format!("{}.log", gate.name));

    // Apply global environment variables
    for (key, value) in &policy.global.environment {
        // SAFETY: Single-threaded xtask binary
        unsafe {
            std::env::set_var(key, value);
        }
    }

    // Determine timeout
    let timeout_secs = gate.timeout_seconds;
    // Note: timeout enforcement could be added using process timeout

    // Execute command
    let command = gate.command.trim();

    // Handle quarantined gates
    if gate.quarantine && !config.verbose {
        // Skip quarantined gates unless verbose mode
        return Ok(GateResult {
            gate_name: gate.name.clone(),
            tier: gate.tier.clone(),
            status: "skip".to_string(),
            required: Some(gate.required),
            duration_ms: 0,
            command: command.to_string(),
            exit_code: None,
            output_summary: Some("Quarantined - skipped".to_string()),
            log_path: None,
            metrics: None,
            artifacts: None,
        });
    }

    // Run the command
    let result = cmd!("bash", "-c", command).stderr_to_stdout().stdout_capture().unchecked().run();

    let duration_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(output) => {
            let exit_code = output.status.code().unwrap_or(-1);
            let stdout = String::from_utf8_lossy(&output.stdout);

            // Write log file
            if let Err(e) = fs::write(&log_path, stdout.as_bytes()) {
                eprintln!("Warning: Failed to write log file: {}", e);
            }

            // Check if timed out
            let timed_out = duration_ms > (timeout_secs * 1000);

            let status = if timed_out {
                "timeout".to_string()
            } else if exit_code == 0 {
                "pass".to_string()
            } else {
                "fail".to_string()
            };

            // Extract output summary (last 10 lines or error message)
            let output_summary = extract_output_summary(&stdout, 10);

            // Parse metrics if this is a test gate
            let metrics = if gate.tags.contains(&"test".to_string()) {
                parse_test_metrics(&stdout)
            } else {
                None
            };

            Ok(GateResult {
                gate_name: gate.name.clone(),
                tier: gate.tier.clone(),
                status,
                required: Some(gate.required),
                duration_ms,
                command: command.to_string(),
                exit_code: Some(exit_code),
                output_summary: Some(output_summary),
                log_path: Some(format!("logs/{}.log", gate.name)),
                metrics,
                artifacts: if gate.artifacts.is_empty() {
                    None
                } else {
                    Some(gate.artifacts.clone())
                },
            })
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            Ok(GateResult {
                gate_name: gate.name.clone(),
                tier: gate.tier.clone(),
                status: "error".to_string(),
                required: Some(gate.required),
                duration_ms,
                command: command.to_string(),
                exit_code: None,
                output_summary: Some(format!("Execution error: {}", e)),
                log_path: None,
                metrics: None,
                artifacts: None,
            })
        }
    }
}

/// Collect system metadata for the receipt
fn collect_metadata(timestamp: DateTime<Utc>) -> Result<ReceiptMetadata> {
    // Git info
    let git_sha = cmd!("git", "rev-parse", "HEAD")
        .read()
        .unwrap_or_else(|_| "UNVERIFIED".to_string())
        .trim()
        .to_string();

    let git_sha_short =
        if git_sha.len() >= 7 { git_sha[..7].to_string() } else { "UNVERIF".to_string() };

    let git_branch = cmd!("git", "rev-parse", "--abbrev-ref", "HEAD")
        .read()
        .unwrap_or_else(|_| "unknown".to_string())
        .trim()
        .to_string();

    let git_dirty =
        cmd!("git", "status", "--porcelain").read().map(|s| !s.trim().is_empty()).unwrap_or(false);

    // Toolchain info
    let rustc_version = cmd!("rustc", "--version")
        .read()
        .unwrap_or_else(|_| "unknown".to_string())
        .trim()
        .to_string();

    let rustc_semver = rustc_version.split_whitespace().nth(1).map(|s| s.to_string());

    let rustc_channel = rustc_version
        .split_whitespace()
        .nth(2)
        .and_then(|s| {
            if s.starts_with('(') {
                s.strip_prefix('(').and_then(|s| s.strip_suffix(')'))
            } else {
                Some(s)
            }
        })
        .map(|s| s.to_string());

    let cargo_version = cmd!("cargo", "--version").read().ok().map(|s| s.trim().to_string());

    let nix_version = cmd!("nix", "--version").read().ok().map(|s| s.trim().to_string());

    // Platform info
    let os = std::env::consts::OS.to_string();
    let arch = std::env::consts::ARCH.to_string();

    #[cfg(target_os = "linux")]
    let os_version = { cmd!("uname", "-r").read().ok().map(|s| s.trim().to_string()) };

    #[cfg(not(target_os = "linux"))]
    let os_version = None;

    let is_wsl = os_version
        .as_ref()
        .map(|v| v.to_lowercase().contains("microsoft") || v.to_lowercase().contains("wsl"))
        .unwrap_or(false);

    let cpu_cores = std::thread::available_parallelism().map(|p| p.get() as u32).ok();

    // Memory (Linux only for now)
    #[cfg(target_os = "linux")]
    let memory_gb = {
        fs::read_to_string("/proc/meminfo").ok().and_then(|content| {
            content
                .lines()
                .find(|l| l.starts_with("MemTotal:"))
                .and_then(|l| l.split_whitespace().nth(1).and_then(|s| s.parse::<u64>().ok()))
                .map(|kb| kb as f64 / 1024.0 / 1024.0)
        })
    };

    #[cfg(not(target_os = "linux"))]
    let memory_gb = None;

    // Environment detection
    let env_type = if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
        "ci".to_string()
    } else {
        "local".to_string()
    };

    let ci_provider = if std::env::var("GITHUB_ACTIONS").is_ok() {
        Some("github-actions".to_string())
    } else {
        None
    };

    let ci_run_id = std::env::var("GITHUB_RUN_ID").ok();

    let ci_run_url = ci_run_id.as_ref().and_then(|run_id| {
        std::env::var("GITHUB_REPOSITORY")
            .ok()
            .map(|repo| format!("https://github.com/{}/actions/runs/{}", repo, run_id))
    });

    let pr_number = std::env::var("GITHUB_EVENT_NUMBER").ok().and_then(|s| s.parse().ok());

    let nix_shell = std::env::var("IN_NIX_SHELL").is_ok();

    let trigger = std::env::var("CI_TRIGGER").ok().or_else(|| {
        if env_type == "ci" { Some("ci-pr".to_string()) } else { Some("manual".to_string()) }
    });

    Ok(ReceiptMetadata {
        timestamp: timestamp.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        git_sha,
        git_sha_short,
        git_branch,
        git_dirty,
        toolchain: ToolchainInfo {
            rustc_version,
            rustc_channel,
            rustc_semver,
            cargo_version,
            node_version: None,
            nix_version,
        },
        platform: PlatformInfo { os, os_version, arch, cpu_cores, memory_gb, is_wsl: Some(is_wsl) },
        environment: EnvironmentInfo {
            env_type,
            ci_provider,
            ci_run_id,
            ci_run_url,
            pr_number,
            nix_shell: Some(nix_shell),
        },
        trigger,
    })
}

/// Extract summary from command output
fn extract_output_summary(output: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = output.lines().collect();
    let start = if lines.len() > max_lines { lines.len() - max_lines } else { 0 };
    lines[start..].join("\n")
}

/// Parse test metrics from cargo test output
fn parse_test_metrics(output: &str) -> Option<GateMetrics> {
    // Look for "test result: ok. X passed; Y failed; Z ignored"
    for line in output.lines() {
        if line.contains("test result:") {
            let mut metrics = GateMetrics::default();

            // Parse passed count
            if let Some(passed) = extract_number(line, "passed") {
                metrics.tests_passed = Some(passed);
            }

            // Parse failed count
            if let Some(failed) = extract_number(line, "failed") {
                metrics.tests_failed = Some(failed);
            }

            // Parse ignored count
            if let Some(ignored) = extract_number(line, "ignored") {
                metrics.tests_ignored = Some(ignored);
            }

            // Calculate total
            let total = metrics.tests_passed.unwrap_or(0)
                + metrics.tests_failed.unwrap_or(0)
                + metrics.tests_ignored.unwrap_or(0);
            if total > 0 {
                metrics.tests_total = Some(total);
                return Some(metrics);
            }
        }
    }
    None
}

fn extract_number(line: &str, suffix: &str) -> Option<u32> {
    let pattern = format!(" {}", suffix);
    line.find(&pattern).and_then(|idx| {
        // Look backwards for the number
        let before = &line[..idx];
        before.split_whitespace().last().and_then(|s| s.parse().ok())
    })
}

/// Output results in the requested format
fn output_results(receipt: &Receipt, config: &GateRunnerConfig) -> Result<()> {
    match config.output_format {
        OutputFormat::Human => output_human(receipt),
        OutputFormat::Json => output_json(receipt),
        OutputFormat::Summary => output_summary(receipt),
    }
}

fn output_human(receipt: &Receipt) -> Result<()> {
    let mut term = Term::stdout();
    let bold = Style::new().bold();
    let green = Style::new().green();
    let red = Style::new().red();
    let yellow = Style::new().yellow();
    let dim = Style::new().dim();

    writeln!(term)?;
    writeln!(term, "{}", "=".repeat(60))?;
    writeln!(term, "{}", bold.apply_to("Gate Execution Summary"))?;
    writeln!(term, "{}", "=".repeat(60))?;
    writeln!(term)?;

    // Metadata
    writeln!(term, "{} {}", bold.apply_to("Git:"), receipt.metadata.git_sha_short)?;
    writeln!(term, "{} {}", bold.apply_to("Branch:"), receipt.metadata.git_branch)?;
    writeln!(term, "{} {}", bold.apply_to("Rust:"), receipt.metadata.toolchain.rustc_version)?;
    writeln!(term)?;

    // Results by tier
    if let Some(ref tier_results) = receipt.summary.tier_results {
        for tier in &["pr_fast", "merge_gate", "nightly"] {
            if let Some(summary) = tier_results.get(*tier) {
                let status_style = if summary.failed > 0 { red.clone() } else { green.clone() };
                writeln!(
                    term,
                    "{}: {} passed, {} failed, {} skipped ({:.1}s)",
                    bold.apply_to(tier),
                    status_style.apply_to(summary.passed),
                    status_style.apply_to(summary.failed),
                    dim.apply_to(summary.skipped),
                    summary.duration_ms as f64 / 1000.0
                )?;
            }
        }
        writeln!(term)?;
    }

    // Overall status
    let status_style = match receipt.summary.overall_status.as_str() {
        "pass" => green.clone(),
        "fail" => red.clone(),
        "partial" => yellow,
        _ => dim.clone(),
    };

    writeln!(
        term,
        "{}: {}",
        bold.apply_to("Overall"),
        status_style.apply_to(receipt.summary.overall_status.to_uppercase())
    )?;
    writeln!(
        term,
        "{}: {:.1}s",
        bold.apply_to("Total time"),
        receipt.summary.total_duration_ms as f64 / 1000.0
    )?;

    if let Some(ref failures) = receipt.summary.blocking_failures
        && !failures.is_empty()
    {
        writeln!(term)?;
        writeln!(term, "{}", red.apply_to("Blocking failures:"))?;
        for gate in failures {
            writeln!(term, "  - {}", gate)?;
        }
    }

    writeln!(term)?;
    writeln!(term, "{}", "=".repeat(60))?;

    Ok(())
}

fn output_json(receipt: &Receipt) -> Result<()> {
    let json = serde_json::to_string_pretty(receipt)?;
    println!("{}", json);
    Ok(())
}

fn output_summary(receipt: &Receipt) -> Result<()> {
    println!(
        "[{}] {}/{} passed in {:.1}s",
        receipt.summary.overall_status.to_uppercase(),
        receipt.summary.passed,
        receipt.summary.total_gates,
        receipt.summary.total_duration_ms as f64 / 1000.0
    );
    Ok(())
}

/// Write receipt to file
fn write_receipt(receipt: &Receipt, path: &PathBuf) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(receipt)?;
    fs::write(path, json)?;
    eprintln!("Receipt written to: {}", path.display());
    Ok(())
}

/// Load existing receipt for comparison
fn load_receipt(path: &PathBuf) -> Result<Receipt> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read baseline receipt from {}", path.display()))?;
    let receipt: Receipt = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse baseline receipt from {}", path.display()))?;
    Ok(receipt)
}

/// Compare two receipts and generate diff
fn compare_receipts(baseline: &Receipt, current: &Receipt) -> Result<DiffResult> {
    let baseline_gates: HashMap<&str, &GateResult> =
        baseline.gates.iter().map(|g| (g.gate_name.as_str(), g)).collect();

    let current_gates: HashMap<&str, &GateResult> =
        current.gates.iter().map(|g| (g.gate_name.as_str(), g)).collect();

    // Find added gates
    let gates_added: Vec<String> = current_gates
        .keys()
        .filter(|k| !baseline_gates.contains_key(*k))
        .map(|k| k.to_string())
        .collect();

    // Find removed gates
    let gates_removed: Vec<String> = baseline_gates
        .keys()
        .filter(|k| !current_gates.contains_key(*k))
        .map(|k| k.to_string())
        .collect();

    // Find status changes
    let mut status_changes = Vec::new();
    for (name, current_gate) in &current_gates {
        if let Some(baseline_gate) = baseline_gates.get(name)
            && baseline_gate.status != current_gate.status
        {
            let is_regression = baseline_gate.status == "pass" && current_gate.status == "fail";
            status_changes.push(StatusChange {
                gate_name: name.to_string(),
                old_status: baseline_gate.status.clone(),
                new_status: current_gate.status.clone(),
                is_regression,
            });
        }
    }

    // Find metric changes
    let mut metric_changes = Vec::new();
    for (name, current_gate) in &current_gates {
        if let (Some(_baseline_gate), Some(current_metrics), Some(baseline_metrics)) = (
            baseline_gates.get(name),
            &current_gate.metrics,
            baseline_gates.get(name).and_then(|g| g.metrics.as_ref()),
        ) {
            // Compare tests_total
            if let (Some(old), Some(new)) =
                (baseline_metrics.tests_total, current_metrics.tests_total)
                && old != new
            {
                let delta = ((new as f64 - old as f64) / old as f64) * 100.0;
                metric_changes.push(MetricChange {
                    gate_name: name.to_string(),
                    metric_name: "tests_total".to_string(),
                    old_value: old as f64,
                    new_value: new as f64,
                    delta_percent: delta,
                    exceeds_threshold: delta.abs() > 10.0,
                });
            }
        }
    }

    let overall_regression = status_changes.iter().any(|c| c.is_regression);

    Ok(DiffResult {
        baseline_timestamp: baseline.metadata.timestamp.clone(),
        current_timestamp: current.metadata.timestamp.clone(),
        gates_added,
        gates_removed,
        status_changes,
        metric_changes,
        overall_regression,
    })
}

/// Output diff results
fn output_diff(diff: &DiffResult, config: &GateRunnerConfig) -> Result<()> {
    if config.output_format == OutputFormat::Json {
        println!("{}", serde_json::to_string_pretty(diff)?);
        return Ok(());
    }

    let mut term = Term::stdout();
    let bold = Style::new().bold();
    let green = Style::new().green();
    let red = Style::new().red();
    let yellow = Style::new().yellow();

    writeln!(term, "{}", bold.apply_to("Receipt Comparison"))?;
    writeln!(term, "{}", "=".repeat(60))?;
    writeln!(term, "Baseline: {}", diff.baseline_timestamp)?;
    writeln!(term, "Current:  {}", diff.current_timestamp)?;
    writeln!(term)?;

    if !diff.gates_added.is_empty() {
        writeln!(term, "{}", green.apply_to("Gates Added:"))?;
        for gate in &diff.gates_added {
            writeln!(term, "  + {}", gate)?;
        }
        writeln!(term)?;
    }

    if !diff.gates_removed.is_empty() {
        writeln!(term, "{}", red.apply_to("Gates Removed:"))?;
        for gate in &diff.gates_removed {
            writeln!(term, "  - {}", gate)?;
        }
        writeln!(term)?;
    }

    if !diff.status_changes.is_empty() {
        writeln!(term, "{}", bold.apply_to("Status Changes:"))?;
        for change in &diff.status_changes {
            let indicator = if change.is_regression {
                red.apply_to("REGRESSION")
            } else {
                green.apply_to("IMPROVEMENT")
            };
            writeln!(
                term,
                "  {} {}: {} -> {}",
                indicator, change.gate_name, change.old_status, change.new_status
            )?;
        }
        writeln!(term)?;
    }

    if !diff.metric_changes.is_empty() {
        writeln!(term, "{}", bold.apply_to("Metric Changes:"))?;
        for change in &diff.metric_changes {
            let delta_str = if change.delta_percent > 0.0 {
                format!("+{:.1}%", change.delta_percent)
            } else {
                format!("{:.1}%", change.delta_percent)
            };
            let style = if change.exceeds_threshold { yellow.clone() } else { Style::new() };
            writeln!(
                term,
                "  {} [{}]: {} -> {} ({})",
                change.gate_name,
                change.metric_name,
                change.old_value,
                change.new_value,
                style.apply_to(delta_str)
            )?;
        }
    }

    writeln!(term)?;
    if diff.overall_regression {
        writeln!(term, "{}", red.apply_to("OVERALL: REGRESSION DETECTED"))?;
    } else {
        writeln!(term, "{}", green.apply_to("OVERALL: No regressions"))?;
    }

    Ok(())
}

/// Check if there are any blocking failures
fn has_blocking_failures(receipt: &Receipt) -> bool {
    receipt.summary.blocking_failures.as_ref().map(|f| !f.is_empty()).unwrap_or(false)
}
