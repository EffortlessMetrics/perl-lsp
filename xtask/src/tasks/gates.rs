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
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;
use std::time::Instant;

use crate::tasks::ci_scope::{self, ScopeOutput};
use crate::utils::project_root;

mod first_failure;
mod planning_types;

pub use first_failure::{is_cargo_test_command, parse_first_failure};

use planning_types::{GatePlan, PackageTargetIndex, PlannedGate, SkippedGate};

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
#[derive(Debug, Clone, Deserialize)]
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
#[derive(Debug, Clone, Deserialize)]
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
#[derive(Debug, Clone, Deserialize)]
pub struct ToolchainConfig {
    pub msrv: Option<String>,
    #[serde(default)]
    pub components: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct TierDefinition {
    pub description: String,
    pub target_duration_seconds: u64,
    pub enforcement: String,
    #[serde(default)]
    pub trigger: Vec<serde_yaml_ng::Value>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
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
    #[serde(default)]
    pub planning: Option<GatePlanningConfig>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct GatePlanningConfig {
    pub role: GatePlanningRole,
    #[serde(default)]
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatePlanningRole {
    AlwaysOn,
    RustScoped,
    RustFallback,
    RustPackageScoped,
    Static,
}

impl std::fmt::Display for GatePlanningRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GatePlanningRole::AlwaysOn => write!(f, "always_on"),
            GatePlanningRole::RustScoped => write!(f, "rust_scoped"),
            GatePlanningRole::RustFallback => write!(f, "rust_fallback"),
            GatePlanningRole::RustPackageScoped => write!(f, "rust_package_scoped"),
            GatePlanningRole::Static => write!(f, "static"),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_timeout() -> u64 {
    300
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct GateBudgets {
    pub max_duration_ms: Option<u64>,
    pub max_warnings: Option<u32>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
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
#[derive(Debug, Clone, Deserialize)]
pub struct QuarantinedGate {
    pub gate: String,
    pub reason: String,
    pub quarantined_at: String,
    pub issue: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct FlakyPattern {
    pub pattern: String,
    pub reason: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
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
    pub agent_receipt: Option<AgentReceipt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_config: Option<DiffConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentReceipt {
    pub sha: String,
    pub is_latest: bool,
    pub tier: String,
    pub scope: AgentScope,
    pub selected_lanes: Vec<AgentLane>,
    pub failures: Vec<AgentFailure>,
    pub suggested_next_actions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<AgentPlanReceipt>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AgentScope {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff_class: Option<String>,
    pub direct_crates: Vec<String>,
    pub reverse_deps: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub architecture_wideners: Vec<String>,
    pub risk_tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentPlanReceipt {
    pub base: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_class: Option<String>,
    pub scope_ok: bool,
    pub fallback_used: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<String>,
    pub package_args: Vec<String>,
    pub selected: Vec<AgentPlannedGate>,
    pub skipped: Vec<AgentSkippedGate>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentPlannedGate {
    pub name: String,
    pub role: GatePlanningRole,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentSkippedGate {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<GatePlanningRole>,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentLane {
    pub name: String,
    pub reason: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentFailure {
    pub lane: String,
    pub summary: String,
    pub repro: String,
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

/// First failing test extracted from `cargo test` output.
///
/// Populated only when a `cargo test`-class gate exits non-zero.
/// Used by followers and curators to repair without re-running gates locally.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct FirstFailure {
    /// Full test path, e.g. `module::submod::tests::test_name`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test: Option<String>,
    /// Panic location as `file:line`, e.g. `src/lib.rs:42`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site: Option<String>,
    /// Panic / assertion message (first non-empty line after the `panicked at` line)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Process exit code
    pub exit_code: i32,
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
    /// First failing test details for `cargo test`-class gates that exit non-zero.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_failure: Option<FirstFailure>,
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
    pub base_ref: Option<String>,
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
            base_ref: None,
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
    let policy = load_policy_for_inspection(&policy_path)?;

    // Handle list mode against the static policy catalog. Dynamic PR-fast scope
    // planning is run only for actual execution/diff receipts.
    if config.list_only {
        let gates = filter_gates(&policy, &config)?;
        return list_gates(&gates, &policy);
    }

    // Build the executable plan. PR-fast uses the shared xtask runner plus
    // ci-scope planning so local `just pr-fast` and CI execute the same lane
    // decisions instead of duplicating shell/YAML logic.
    let plan = plan_gates(&root, &policy, &config)?;

    // Handle diff mode
    if let Some(baseline_path) = &config.diff_baseline {
        let baseline = load_receipt(baseline_path)?;
        let current = run_gate_plan(&plan, &policy, &config)?;
        let diff = compare_receipts(&baseline, &current)?;
        return output_diff(&diff, &config);
    }

    // Run gates
    let receipt = run_gate_plan(&plan, &policy, &config)?;

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
    if has_blocking_failures(&receipt) {
        bail!("One or more required gates failed, timed out, or errored");
    }

    Ok(())
}

/// Load gate policy from YAML file
pub(crate) fn load_policy_for_inspection(path: &Path) -> Result<GatePolicy> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read gate policy from {}", path.display()))?;
    let policy: GatePolicy = serde_yaml_ng::from_str(&content)
        .with_context(|| format!("Failed to parse gate policy from {}", path.display()))?;
    Ok(policy)
}

/// Filter gates based on tier and gate name filter
fn filter_gates(policy: &GatePolicy, config: &GateRunnerConfig) -> Result<Vec<GateDefinition>> {
    let mut gates: Vec<GateDefinition> = policy.gates.clone();

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
        "release" => 3,
        _ => 4,
    });

    Ok(gates)
}

fn plan_gates(root: &Path, policy: &GatePolicy, config: &GateRunnerConfig) -> Result<GatePlan> {
    let base = config.base_ref.clone().unwrap_or_else(|| select_scope_base(root));

    if config.gate_filter.is_some() {
        return Ok(static_gate_plan(config.tier.clone(), base, filter_gates(policy, config)?));
    }

    match config.tier {
        GateTier::PrFast => plan_pr_fast_gates(root, gates_for_tier(policy, "pr_fast"), base),
        GateTier::MergeGate => {
            let mut plan = plan_pr_fast_gates(root, gates_for_tier(policy, "pr_fast"), base)?;
            plan.tier = GateTier::MergeGate;
            extend_plan_with_static_tiers(&mut plan, policy, &["merge_gate"]);
            Ok(plan)
        }
        GateTier::Nightly => {
            let mut plan = plan_pr_fast_gates(root, gates_for_tier(policy, "pr_fast"), base)?;
            plan.tier = GateTier::Nightly;
            extend_plan_with_static_tiers(&mut plan, policy, &["merge_gate", "nightly"]);
            Ok(plan)
        }
        GateTier::All => {
            let mut plan = plan_pr_fast_gates(root, gates_for_tier(policy, "pr_fast"), base)?;
            plan.tier = GateTier::All;
            extend_plan_with_non_pr_fast_static_gates(&mut plan, policy);
            Ok(plan)
        }
    }
}

fn static_gate_plan(tier: GateTier, base: String, gates: Vec<GateDefinition>) -> GatePlan {
    let selected = gates.into_iter().map(static_gate).collect();

    GatePlan {
        tier,
        base,
        scope: None,
        scope_ok: true,
        fallback_used: false,
        fallback_reason: None,
        package_args: Vec::new(),
        selected,
        skipped: Vec::new(),
    }
}

fn gates_for_tier(policy: &GatePolicy, tier: &str) -> Vec<GateDefinition> {
    policy.gates.iter().filter(|gate| gate.tier == tier).cloned().collect()
}

fn extend_plan_with_static_tiers(plan: &mut GatePlan, policy: &GatePolicy, tiers: &[&str]) {
    let tier_set: HashSet<&str> = tiers.iter().copied().collect();
    plan.selected.extend(
        policy
            .gates
            .iter()
            .filter(|gate| tier_set.contains(gate.tier.as_str()))
            .cloned()
            .map(static_gate),
    );
}

fn extend_plan_with_non_pr_fast_static_gates(plan: &mut GatePlan, policy: &GatePolicy) {
    plan.selected.extend(
        policy.gates.iter().filter(|gate| gate.tier != "pr_fast").cloned().map(static_gate),
    );
}

fn static_gate(gate: GateDefinition) -> PlannedGate {
    PlannedGate {
        role: gate
            .planning
            .as_ref()
            .map(|planning| planning.role)
            .unwrap_or(GatePlanningRole::Static),
        reason: "selected by static policy filter".to_string(),
        gate,
    }
}

/// Plan PR-fast from policy planning roles plus ci-scope output.
fn plan_pr_fast_gates(root: &Path, gates: Vec<GateDefinition>, base: String) -> Result<GatePlan> {
    let scope = match compute_scope_output(root, &base) {
        Ok(scope) => scope,
        Err(err) => {
            let reason =
                format!("ci-scope failed for base '{base}'; falling back to rust_fallback gates");
            eprintln!("warning: {reason}: {err:#}");
            return build_pr_fast_plan_from_scope_with_targets(
                GateTier::PrFast,
                base,
                gates,
                None,
                false,
                true,
                Some(reason),
                None,
            );
        }
    };

    let non_rust_diff = is_non_rust_diff(&scope);
    let fallback_used = !non_rust_diff && selected_package_names(&scope).is_empty();
    let fallback_reason = if fallback_used {
        Some("ci-scope produced no package scope for a Rust-relevant diff".to_string())
    } else {
        None
    };

    let target_index = if !non_rust_diff && !fallback_used {
        match load_package_target_index(root) {
            Ok(index) => Some(index),
            Err(err) => {
                let reason =
                    "cargo metadata target indexing failed; falling back to rust_fallback gates"
                        .to_string();
                eprintln!("warning: {reason}: {err:#}");
                return build_pr_fast_plan_from_scope_with_targets(
                    GateTier::PrFast,
                    base,
                    gates,
                    Some(scope),
                    true,
                    true,
                    Some(reason),
                    None,
                );
            }
        }
    } else {
        None
    };

    build_pr_fast_plan_from_scope_with_targets(
        GateTier::PrFast,
        base,
        gates,
        Some(scope),
        true,
        fallback_used,
        fallback_reason,
        target_index.as_ref(),
    )
}

#[cfg(test)]
fn build_pr_fast_plan_from_scope(
    tier: GateTier,
    base: String,
    gates: Vec<GateDefinition>,
    scope: Option<ScopeOutput>,
    scope_ok: bool,
    fallback_used: bool,
    fallback_reason: Option<String>,
) -> Result<GatePlan> {
    build_pr_fast_plan_from_scope_with_targets(
        tier,
        base,
        gates,
        scope,
        scope_ok,
        fallback_used,
        fallback_reason,
        None,
    )
}

fn build_pr_fast_plan_from_scope_with_targets(
    tier: GateTier,
    base: String,
    gates: Vec<GateDefinition>,
    scope: Option<ScopeOutput>,
    scope_ok: bool,
    fallback_used: bool,
    fallback_reason: Option<String>,
    target_index: Option<&PackageTargetIndex>,
) -> Result<GatePlan> {
    let non_rust_diff = scope.as_ref().is_some_and(is_non_rust_diff);
    let package_names = scope.as_ref().map(selected_package_names).unwrap_or_default();
    let package_args = package_args_from_names(&package_names);

    let mut selected = Vec::new();
    let mut skipped = Vec::new();

    for gate in gates {
        let role = pr_fast_role(&gate)?;
        match role {
            GatePlanningRole::AlwaysOn => {
                selected.push(PlannedGate {
                    role,
                    reason: "always-on pr_fast gate".to_string(),
                    gate,
                });
            }
            GatePlanningRole::RustScoped => {
                if fallback_used {
                    skipped.push(SkippedGate {
                        name: gate.name,
                        role: Some(role),
                        reason: fallback_reason
                            .clone()
                            .unwrap_or_else(|| "rust fallback selected".to_string()),
                    });
                } else if non_rust_diff {
                    skipped.push(SkippedGate {
                        name: gate.name,
                        role: Some(role),
                        reason: scope_skip_reason(scope.as_ref()),
                    });
                } else {
                    let gate_package_names =
                        package_names_for_gate(&gate, &package_names, target_index);
                    let gate_package_args = package_args_from_names(&gate_package_names);
                    if gate_package_args.is_empty() {
                        skipped.push(SkippedGate {
                            name: gate.name.clone(),
                            role: Some(role),
                            reason: no_eligible_package_reason(&gate),
                        });
                    } else {
                        selected.push(PlannedGate {
                            role,
                            reason: format!(
                                "code diff selected packages: {}",
                                gate_package_names.join(", ")
                            ),
                            gate: render_package_args(gate, &gate_package_args)?,
                        });
                    }
                }
            }
            GatePlanningRole::RustFallback => {
                if fallback_used {
                    selected.push(PlannedGate {
                        role,
                        reason: fallback_reason
                            .clone()
                            .unwrap_or_else(|| "rust fallback selected".to_string()),
                        gate,
                    });
                } else if non_rust_diff {
                    skipped.push(SkippedGate {
                        name: gate.name,
                        role: Some(role),
                        reason: scope_skip_reason(scope.as_ref()),
                    });
                } else {
                    skipped.push(SkippedGate {
                        name: gate.name,
                        role: Some(role),
                        reason: "rust scoped plan selected".to_string(),
                    });
                }
            }
            GatePlanningRole::RustPackageScoped => {
                if fallback_used {
                    selected.push(PlannedGate {
                        role,
                        reason: fallback_reason
                            .clone()
                            .unwrap_or_else(|| "rust fallback selected".to_string()),
                        gate,
                    });
                } else if let Some(reason) = package_scoped_reason(&gate, &package_names) {
                    selected.push(PlannedGate { role, reason, gate });
                } else {
                    skipped.push(SkippedGate {
                        name: gate.name.clone(),
                        role: Some(role),
                        reason: package_scoped_skip_reason(&gate, scope.as_ref()),
                    });
                }
            }
            GatePlanningRole::Static => {
                bail!(
                    "Gate '{}' in pr_fast must declare planning.role; static is not valid for pr_fast planning",
                    gate.name
                );
            }
        }
    }

    Ok(GatePlan {
        tier,
        base,
        scope,
        scope_ok,
        fallback_used,
        fallback_reason,
        package_args,
        selected,
        skipped,
    })
}

fn pr_fast_role(gate: &GateDefinition) -> Result<GatePlanningRole> {
    gate.planning.as_ref().map(|planning| planning.role).ok_or_else(|| {
        color_eyre::eyre::eyre!("Gate '{}' in pr_fast is missing planning.role", gate.name)
    })
}

fn is_non_rust_diff(scope: &ScopeOutput) -> bool {
    matches!(scope.diff_class.as_str(), "prose_only" | "docs_as_code" | "ci_config")
}

fn scope_skip_reason(scope: Option<&ScopeOutput>) -> String {
    let diff_class = scope.map(|scope| scope.diff_class.as_str()).unwrap_or("unknown");
    format!("Rust lanes skipped because diff_class={diff_class}")
}

fn selected_package_names(scope: &ScopeOutput) -> Vec<String> {
    let mut package_names: Vec<String> = scope
        .direct_crates
        .iter()
        .map(|entry| entry.name.clone())
        .chain(scope.reverse_dep_closure.iter().map(|entry| entry.name.clone()))
        .chain(scope.architecture_wideners.iter().map(|entry| entry.name.clone()))
        .collect();
    package_names.sort();
    package_names.dedup();
    package_names
}

fn package_args_from_names(package_names: &[String]) -> Vec<String> {
    package_names.iter().flat_map(|name| ["-p".to_string(), name.clone()]).collect()
}

fn render_package_args(
    mut gate: GateDefinition,
    package_args: &[String],
) -> Result<GateDefinition> {
    if !gate.command.contains("{package_args}") {
        bail!(
            "Gate '{}' has planning.role=rust_scoped but its command has no {{package_args}} placeholder",
            gate.name
        );
    }
    gate.command = gate.command.replace("{package_args}", &package_args.join(" "));
    Ok(gate)
}

fn load_package_target_index(root: &Path) -> Result<PackageTargetIndex> {
    let metadata_raw = cmd("cargo", ["metadata", "--format-version=1", "--no-deps"])
        .dir(root)
        .read()
        .context("Failed to load cargo metadata for package target planning")?;
    let metadata: serde_json::Value =
        serde_json::from_str(&metadata_raw).context("Failed to parse cargo metadata JSON")?;

    let mut index = PackageTargetIndex::default();
    let packages = metadata
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| color_eyre::eyre::eyre!("cargo metadata JSON missing packages array"))?;

    for package in packages {
        let Some(name) = package.get("name").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let has_lib =
            package.get("targets").and_then(serde_json::Value::as_array).is_some_and(|targets| {
                targets.iter().any(|target| {
                    target
                        .get("kind")
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(|kinds| kinds.iter().any(|kind| kind.as_str() == Some("lib")))
                })
            });
        if has_lib {
            index.lib_packages.insert(name.to_string());
        }
    }

    Ok(index)
}

fn package_names_for_gate(
    gate: &GateDefinition,
    package_names: &[String],
    target_index: Option<&PackageTargetIndex>,
) -> Vec<String> {
    if gate_requires_lib(gate)
        && let Some(index) = target_index
    {
        return package_names.iter().filter(|name| index.has_lib(name)).cloned().collect();
    }
    package_names.to_vec()
}

fn no_eligible_package_reason(gate: &GateDefinition) -> String {
    if gate_requires_lib(gate) {
        "no ci-scope selected packages have a lib target for this gate".to_string()
    } else {
        "no ci-scope package arguments available".to_string()
    }
}

fn gate_requires_lib(gate: &GateDefinition) -> bool {
    gate.command.split_whitespace().any(|part| part == "--lib")
}

fn package_scoped_reason(gate: &GateDefinition, package_names: &[String]) -> Option<String> {
    let packages = gate.planning.as_ref()?.packages.as_slice();
    let matched: Vec<String> = packages
        .iter()
        .filter(|package| package_names.iter().any(|name| name == *package))
        .cloned()
        .collect();
    if matched.is_empty() {
        None
    } else {
        Some(format!("package-scoped gate matched {}", matched.join(", ")))
    }
}

fn package_scoped_skip_reason(gate: &GateDefinition, scope: Option<&ScopeOutput>) -> String {
    if scope.is_some_and(is_non_rust_diff) {
        return scope_skip_reason(scope);
    }
    let packages =
        gate.planning.as_ref().map(|planning| planning.packages.join(", ")).unwrap_or_default();
    if packages.is_empty() {
        "package-scoped gate has no configured packages".to_string()
    } else {
        format!("selected packages did not include {packages}")
    }
}

/// List available gates
fn list_gates(gates: &[GateDefinition], policy: &GatePolicy) -> Result<()> {
    let mut term = Term::stdout();
    let bold = Style::new().bold();
    let dim = Style::new().dim();

    writeln!(term, "{}", bold.apply_to("Available Gates"))?;
    writeln!(term, "{}", "=".repeat(60))?;
    writeln!(term)?;

    // Group by tier
    let mut by_tier: HashMap<&str, Vec<&GateDefinition>> = HashMap::new();
    for gate in gates {
        by_tier.entry(gate.tier.as_str()).or_default().push(gate);
    }

    for tier_name in &["pr_fast", "merge_gate", "nightly", "release"] {
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

/// Run a planned set of gates and collect results.
fn run_gate_plan(
    plan: &GatePlan,
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
        let pb = ProgressBar::new(plan.selected.len() as u64);
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

    for (idx, planned_gate) in plan.selected.iter().enumerate() {
        let gate = &planned_gate.gate;
        emit_gate_begin(gate);
        if let Some(ref pb) = spinner {
            pb.set_position(idx as u64);
            pb.set_message(format!("Running {}...", gate.name));
        }

        let result = run_single_gate(gate, policy, &log_dir, config)?;
        emit_gate_end(gate, &result);

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
        if config.fail_fast && is_blocking_gate_status(&result.status) && gate.required {
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

    let blocking_failures = blocking_failure_gate_names(&results);
    let overall_status = determine_overall_status(failed, &blocking_failures);

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
    let agent_receipt = Some(build_agent_receipt(&root, &results, plan));

    Ok(Receipt {
        schema_version: "1.0.0".to_string(),
        metadata,
        gates: results,
        summary,
        agent_receipt,
        diff_config: None,
    })
}

fn emit_gate_begin(gate: &GateDefinition) {
    println!(
        "BEGIN gate={} timeout={} command={}",
        gate.name,
        gate.timeout_seconds,
        gate.command.trim()
    );
}

fn emit_gate_end(gate: &GateDefinition, result: &GateResult) {
    let exit = result.exit_code.map_or_else(|| "none".to_string(), |code| code.to_string());
    println!(
        "END gate={} status={} exit={} duration_ms={}",
        gate.name, result.status, exit, result.duration_ms
    );
}

/// Phase-1 agent-facing receipt shape contract (Issue #5020):
/// keep this as a stable, minimal JSON slice consumed by CI artifacts.
fn build_agent_receipt(root: &Path, results: &[GateResult], plan: &GatePlan) -> AgentReceipt {
    let scope_output = plan.scope.clone();
    let gate_status_by_name: HashMap<String, String> =
        results.iter().map(|result| (result.gate_name.clone(), result.status.clone())).collect();
    let selected_lanes = scope_output
        .as_ref()
        .map(|scope| {
            let standard = scope.selected_lanes.iter().map(|lane| {
                let explanation = scope.explanations.get(&lane.lane).cloned().unwrap_or_default();
                let reason = if explanation.is_empty() {
                    lane.reason.clone()
                } else {
                    format!("{} — {}", lane.reason, explanation)
                };
                AgentLane {
                    name: lane.lane.clone(),
                    reason,
                    status: gate_status_by_name
                        .get(&lane.lane)
                        .cloned()
                        .unwrap_or_else(|| "not_run".to_string()),
                }
            });
            let heavy = scope.selected_heavy_lanes.iter().map(|lane| AgentLane {
                name: lane.lane.clone(),
                reason: lane.reason.clone(),
                status: gate_status_by_name
                    .get(&lane.lane)
                    .cloned()
                    .unwrap_or_else(|| "not_run".to_string()),
            });
            standard.chain(heavy).collect()
        })
        .unwrap_or_default();
    let (failures, next_actions) = failure_guidance(results);
    let sha = cmd("git", ["rev-parse", "HEAD"])
        .dir(root)
        .read()
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let is_latest = is_latest_commit(root);

    let scope = if let Some(scope) = scope_output {
        AgentScope {
            diff_class: Some(scope.diff_class),
            direct_crates: scope.direct_crates.into_iter().map(|entry| entry.name).collect(),
            reverse_deps: scope.reverse_dep_closure.into_iter().map(|entry| entry.name).collect(),
            architecture_wideners: scope
                .architecture_wideners
                .into_iter()
                .map(|entry| entry.name)
                .collect(),
            risk_tags: scope.risk_tags,
        }
    } else {
        AgentScope::default()
    };

    AgentReceipt {
        sha,
        is_latest,
        tier: plan.tier.to_string(),
        scope,
        selected_lanes,
        failures,
        suggested_next_actions: next_actions,
        plan: Some(agent_plan_receipt(plan)),
    }
}

fn agent_plan_receipt(plan: &GatePlan) -> AgentPlanReceipt {
    AgentPlanReceipt {
        base: plan.base.clone(),
        diff_class: plan.scope.as_ref().map(|scope| scope.diff_class.clone()),
        scope_ok: plan.scope_ok,
        fallback_used: plan.fallback_used,
        fallback_reason: plan.fallback_reason.clone(),
        package_args: plan.package_args.clone(),
        selected: plan
            .selected
            .iter()
            .map(|planned| AgentPlannedGate {
                name: planned.gate.name.clone(),
                role: planned.role,
                reason: planned.reason.clone(),
            })
            .collect(),
        skipped: plan
            .skipped
            .iter()
            .map(|skipped| AgentSkippedGate {
                name: skipped.name.clone(),
                role: skipped.role,
                reason: skipped.reason.clone(),
            })
            .collect(),
    }
}

fn failure_guidance(results: &[GateResult]) -> (Vec<AgentFailure>, Vec<String>) {
    let failures: Vec<AgentFailure> = results
        .iter()
        .filter(|result| is_blocking_gate_status(&result.status) && result.required.unwrap_or(true))
        .map(|result| {
            let base_summary =
                format!("Gate '{}' ended with status '{}'", result.gate_name, result.status);
            // Augment summary with first_failure details when available
            let summary = match &result.first_failure {
                Some(ff) => {
                    let mut parts = vec![base_summary];
                    if let Some(test) = &ff.test {
                        parts.push(format!("  test:  {}", test));
                    }
                    if let Some(site) = &ff.site {
                        parts.push(format!("  site:  {}", site));
                    }
                    if let Some(msg) = &ff.message {
                        parts.push(format!("  msg:   {}", msg));
                    }
                    parts.join("\n")
                }
                None => base_summary,
            };
            AgentFailure {
                lane: result.gate_name.clone(),
                summary,
                repro: format!("{} # gate={}", result.command, result.gate_name),
            }
        })
        .collect();
    let next_actions = if failures.is_empty() {
        vec!["No blocking failures detected. Proceed with review or merge flow.".to_string()]
    } else {
        failures
            .iter()
            .map(|failure| {
                format!(
                    "Reproduce and fix gate '{}' locally, then rerun: cargo xtask gates --gate {}",
                    failure.lane, failure.lane
                )
            })
            .collect()
    };
    (failures, next_actions)
}

fn is_latest_commit(root: &Path) -> bool {
    // In detached HEAD (PR runs), @{upstream} fails with "HEAD does not point to a branch".
    // Suppress stderr so that message does not leak into CI output.
    let upstream =
        match cmd("git", ["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{upstream}"])
            .dir(root)
            .stderr_null()
            .read()
        {
            Ok(value) => value.trim().to_string(),
            Err(_) => return true,
        };
    let head = cmd("git", ["rev-parse", "HEAD"]).dir(root).read().ok();
    let upstream_sha = cmd("git", ["rev-parse", &upstream]).dir(root).read().ok();
    match (head, upstream_sha) {
        (Some(head), Some(upstream_sha)) => head.trim() == upstream_sha.trim(),
        _ => true,
    }
}

fn compute_scope_output(root: &Path, base: &str) -> Result<ScopeOutput> {
    let changed_files = cmd("git", ["diff", "--name-only", &format!("{base}...HEAD")])
        .dir(root)
        .read()
        .with_context(|| format!("Failed to read changed files for base '{base}'"))?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>();

    let metadata_raw = cmd("cargo", ["metadata", "--format-version=1", "--no-deps"])
        .dir(root)
        .read()
        .context("Failed to load cargo metadata for agent receipt scope")?;
    let metadata: serde_json::Value =
        serde_json::from_str(&metadata_raw).context("Failed to parse cargo metadata JSON")?;

    let workspace_root = root.to_string_lossy().replace('\\', "/");
    let mut scope = ci_scope::classify_files(&changed_files, &metadata, &workspace_root)?;
    scope.base = base.to_string();
    scope.head_sha = cmd("git", ["rev-parse", "HEAD"]).dir(root).read()?.trim().to_string();
    scope.changed_files = changed_files;
    Ok(scope)
}

fn select_scope_base(root: &Path) -> String {
    let env_candidates = [
        std::env::var("CI_SCOPE_BASE").ok(),
        std::env::var("GITHUB_BASE_REF").ok().map(|name| format!("origin/{name}")),
        std::env::var("GITHUB_BASE_REF").ok(),
    ];
    let mut candidates: Vec<String> = env_candidates.into_iter().flatten().collect();
    candidates.extend(
        ["origin/master", "origin/main", "origin/HEAD", "master", "main", "HEAD~1"]
            .into_iter()
            .map(str::to_string),
    );
    for candidate in candidates {
        // Suppress stderr: in shallow clones "HEAD~1" does not exist and git prints
        // "fatal: Needed a single revision" to stderr, polluting CI output.
        let exists =
            cmd("git", ["rev-parse", "--verify", &candidate]).dir(root).stderr_null().run().is_ok();
        if exists {
            return candidate;
        }
    }
    "HEAD".to_string()
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
            first_failure: None,
        });
    }

    if command == "cargo xtask fmt --check" {
        return run_internal_xtask_gate(gate, &log_path, command, start, || {
            super::fmt::run(true, None)
        });
    }

    if command == "cargo xtask fmt" {
        return run_internal_xtask_gate(gate, &log_path, command, start, || {
            super::fmt::run(false, None)
        });
    }

    if command == "just ci-publish-closure" || command == "cargo xtask publish-closure" {
        return run_internal_xtask_gate(gate, &log_path, command, start, || {
            super::publish_closure::run(None)
        });
    }

    if command == "just ci-publish-manifest-check"
        || command == "cargo xtask publish-manifest-check"
    {
        return run_internal_xtask_gate(gate, &log_path, command, start, || {
            super::publish_manifest_check::run()
        });
    }

    if command == "just ci-layer-check" || command == "cargo xtask layer-check" {
        return run_internal_xtask_gate(gate, &log_path, command, start, || {
            super::layer_check::run()
        });
    }

    if command == "just ci-published-crate-count" || command == "cargo xtask published-crate-count"
    {
        return run_internal_xtask_gate(gate, &log_path, command, start, || {
            super::count_ratchet::run()
        });
    }

    if command
        == "cargo build --release -p perl-lsp-rs --bin perl-lsp --locked && cargo xtask smoke inline-completion --binary target/release/perl-lsp"
    {
        return run_internal_xtask_gate(gate, &log_path, command, start, || {
            cmd(
                "cargo",
                ["build", "--release", "-p", "perl-lsp-rs", "--bin", "perl-lsp", "--locked"],
            )
            .run()
            .context("Failed to build release perl-lsp binary for inline-completion smoke")?;
            super::inline_completion_smoke::run(PathBuf::from("target/release/perl-lsp"))
        });
    }

    let execution = run_shell_command_with_timeout(command, &log_path, timeout_secs);
    let duration_ms = start.elapsed().as_millis() as u64;

    match execution {
        Ok(execution) => {
            let status = if execution.timed_out {
                "timeout".to_string()
            } else if execution.exit_code == 0 {
                "pass".to_string()
            } else {
                "fail".to_string()
            };

            // Extract output summary (last 10 lines or error message)
            let output_summary = extract_output_summary(&execution.stdout, 10);

            // Parse metrics if this is a test gate
            let metrics = if gate.tags.contains(&"test".to_string()) {
                parse_test_metrics(&execution.stdout)
            } else {
                None
            };

            // For failing cargo test gates, extract the first failure details
            let first_failure = if status == "fail" && is_cargo_test_command(command) {
                parse_first_failure(&execution.stdout, execution.exit_code)
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
                exit_code: Some(execution.exit_code),
                output_summary: Some(output_summary),
                log_path: Some(format!("logs/{}.log", gate.name)),
                metrics,
                artifacts: if gate.artifacts.is_empty() {
                    None
                } else {
                    Some(gate.artifacts.clone())
                },
                first_failure,
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
                first_failure: None,
            })
        }
    }
}

struct ShellExecutionResult {
    stdout: String,
    exit_code: i32,
    timed_out: bool,
}

const MAX_GATE_OUTPUT_BYTES: u64 = 4 * 1024 * 1024;

fn run_shell_command_with_timeout(
    command: &str,
    log_path: &Path,
    timeout_secs: u64,
) -> Result<ShellExecutionResult> {
    let log_file = fs::File::create(log_path)
        .with_context(|| format!("Failed to create log file: {}", log_path.display()))?;
    let log_file_err = log_file
        .try_clone()
        .with_context(|| format!("Failed to clone log file handle: {}", log_path.display()))?;

    let mut child = shell_command_process(command, timeout_secs)
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(log_file_err))
        .spawn()
        .with_context(|| format!("Failed to spawn gate command: {command}"))?;

    let start = Instant::now();
    let mut last_heartbeat = start;
    let timeout = shell_command_watchdog_timeout(timeout_secs);

    // Poll until the process exits or the deadline elapses.
    // Capture exit_code inside the loop so the timed-out branch never calls
    // wait() a second time (which would be a double-wait and returns an error
    // on Windows). Synthetic exit code 124 follows the GNU timeout(1) convention.
    let (watchdog_timed_out, exit_code) = loop {
        if let Some(status) = child.try_wait().context("Failed waiting on gate process")? {
            break (false, status.code().unwrap_or(-1));
        }
        if start.elapsed() >= timeout {
            terminate_shell_command(&mut child);
            child.wait().ok();
            break (true, 124_i32);
        }
        if last_heartbeat.elapsed() >= Duration::from_secs(30) {
            println!(
                "gate command still running elapsed_ms={} timeout_seconds={}",
                start.elapsed().as_millis(),
                timeout_secs
            );
            last_heartbeat = Instant::now();
        }
        thread::sleep(Duration::from_millis(100));
    };

    let timed_out = watchdog_timed_out || exit_code == 124;
    println!(
        "gate command exited exit_code={} timed_out={} elapsed_ms={} log_path={}",
        exit_code,
        timed_out,
        start.elapsed().as_millis(),
        log_path.display()
    );

    let stdout = read_gate_output(log_path);

    Ok(ShellExecutionResult { stdout, exit_code, timed_out })
}

fn read_gate_output(log_path: &Path) -> String {
    let metadata = match fs::metadata(log_path) {
        Ok(metadata) => metadata,
        Err(_) => return String::new(),
    };

    if metadata.len() <= MAX_GATE_OUTPUT_BYTES {
        return fs::read_to_string(log_path).unwrap_or_default();
    }

    let tail_start = metadata.len().saturating_sub(MAX_GATE_OUTPUT_BYTES);
    let mut file = match fs::File::open(log_path) {
        Ok(file) => file,
        Err(_) => return String::new(),
    };

    if file.seek(SeekFrom::Start(tail_start)).is_err() {
        return String::new();
    }

    let mut bytes = Vec::with_capacity(MAX_GATE_OUTPUT_BYTES as usize);
    if file.read_to_end(&mut bytes).is_err() {
        return String::new();
    }

    let mut tail = String::from_utf8_lossy(&bytes).into_owned();
    if tail_start > 0
        && let Some(first_newline) = tail.find('\n')
    {
        tail = tail[first_newline + 1..].to_string();
    }

    format!(
        "[gate log truncated to last {} bytes of {}]\n{}",
        MAX_GATE_OUTPUT_BYTES,
        metadata.len(),
        tail
    )
}

#[cfg(windows)]
fn shell_command_process(command: &str, _timeout_secs: u64) -> Command {
    let mut cmd = Command::new("cmd");
    cmd.args(["/C", command]);
    cmd
}

#[cfg(not(windows))]
fn shell_command_process(command: &str, timeout_secs: u64) -> Command {
    use std::os::unix::process::CommandExt;

    let mut cmd = Command::new("bash");
    let timeout_arg = format!("{timeout_secs}s");
    cmd.arg("-lc")
        .arg(
            "if command -v timeout >/dev/null 2>&1; then \
             exec timeout --signal=TERM --kill-after=60s \"$1\" bash -lc \"$2\"; \
             else exec bash -lc \"$2\"; fi",
        )
        .arg("xtask-gate-timeout")
        .arg(timeout_arg)
        .arg(command);
    cmd.process_group(0);
    cmd
}

#[cfg(windows)]
fn terminate_shell_command(child: &mut Child) {
    child.kill().ok();
}

#[cfg(not(windows))]
fn terminate_shell_command(child: &mut Child) {
    let process_group = format!("-{}", child.id());
    Command::new("kill").args(["-TERM", &process_group]).status().ok();
    thread::sleep(Duration::from_secs(1));
    Command::new("kill").args(["-KILL", &process_group]).status().ok();
    child.kill().ok();
}

#[cfg(windows)]
fn shell_command_watchdog_timeout(timeout_secs: u64) -> Duration {
    Duration::from_secs(timeout_secs)
}

#[cfg(not(windows))]
fn shell_command_watchdog_timeout(timeout_secs: u64) -> Duration {
    Duration::from_secs(timeout_secs.saturating_add(75))
}

fn run_internal_xtask_gate(
    gate: &GateDefinition,
    log_path: &std::path::Path,
    command: &str,
    start: Instant,
    f: impl FnOnce() -> Result<()>,
) -> Result<GateResult> {
    let result = f();
    let duration_ms = start.elapsed().as_millis() as u64;

    let (status, output_summary) = match result {
        Ok(()) => ("pass".to_string(), "Executed internally via xtask task dispatch".to_string()),
        Err(err) => ("fail".to_string(), format!("Internal xtask execution failed: {err:#}")),
    };

    if let Err(err) = fs::write(log_path, output_summary.as_bytes()) {
        eprintln!("Warning: Failed to write log file: {}", err);
    }

    Ok(GateResult {
        gate_name: gate.name.clone(),
        tier: gate.tier.clone(),
        status,
        required: Some(gate.required),
        duration_ms,
        command: command.to_string(),
        exit_code: None,
        output_summary: Some(output_summary),
        log_path: Some(format!("logs/{}.log", gate.name)),
        metrics: None,
        artifacts: if gate.artifacts.is_empty() { None } else { Some(gate.artifacts.clone()) },
        first_failure: None,
    })
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

    // In a detached HEAD (GitHub Actions PR runs check out by SHA), `git rev-parse
    // --abbrev-ref HEAD` returns the literal string "HEAD" rather than a branch name.
    // Prefer the CI environment variable that carries the real source branch name.
    let git_branch = std::env::var("GITHUB_HEAD_REF")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("GITHUB_REF_NAME").ok().filter(|s| !s.is_empty()))
        .unwrap_or_else(|| {
            cmd!("git", "rev-parse", "--abbrev-ref", "HEAD")
                .read()
                .unwrap_or_else(|_| "unknown".to_string())
                .trim()
                .to_string()
        });

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
    let os_version: Option<String> = None;

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
        // Build a lookup from gate name to GateResult so we can print first_failure details
        let gate_by_name: HashMap<&str, &GateResult> =
            receipt.gates.iter().map(|g| (g.gate_name.as_str(), g)).collect();
        for gate_name in failures {
            let exit_code_str = gate_by_name
                .get(gate_name.as_str())
                .and_then(|g| g.exit_code)
                .map(|c| format!(" (exit {})", c))
                .unwrap_or_default();
            writeln!(term, "  - {}{}", gate_name, exit_code_str)?;
            // Print first_failure details if available
            if let Some(ff) =
                gate_by_name.get(gate_name.as_str()).and_then(|g| g.first_failure.as_ref())
            {
                if let Some(ref test) = ff.test {
                    writeln!(term, "      test:   {}", test)?;
                }
                if let Some(ref site) = ff.site {
                    writeln!(term, "      site:   {}", site)?;
                }
                if let Some(ref msg) = ff.message {
                    writeln!(term, "      msg:    {}", msg)?;
                }
                if let Some(gate) = gate_by_name.get(gate_name.as_str()) {
                    writeln!(term, "      repro:  {}", gate.command)?;
                }
            }
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

    // Find metric changes across all tracked gate metrics.
    let mut metric_changes = Vec::new();
    for (name, current_gate) in &current_gates {
        if let (Some(_baseline_gate), Some(current_metrics), Some(baseline_metrics)) = (
            baseline_gates.get(name),
            &current_gate.metrics,
            baseline_gates.get(name).and_then(|g| g.metrics.as_ref()),
        ) {
            push_metric_change(
                &mut metric_changes,
                name,
                "tests_total",
                baseline_metrics.tests_total.map(f64::from),
                current_metrics.tests_total.map(f64::from),
            );
            push_metric_change(
                &mut metric_changes,
                name,
                "tests_passed",
                baseline_metrics.tests_passed.map(f64::from),
                current_metrics.tests_passed.map(f64::from),
            );
            push_metric_change(
                &mut metric_changes,
                name,
                "tests_failed",
                baseline_metrics.tests_failed.map(f64::from),
                current_metrics.tests_failed.map(f64::from),
            );
            push_metric_change(
                &mut metric_changes,
                name,
                "tests_skipped",
                baseline_metrics.tests_skipped.map(f64::from),
                current_metrics.tests_skipped.map(f64::from),
            );
            push_metric_change(
                &mut metric_changes,
                name,
                "tests_ignored",
                baseline_metrics.tests_ignored.map(f64::from),
                current_metrics.tests_ignored.map(f64::from),
            );
            push_metric_change(
                &mut metric_changes,
                name,
                "warnings_count",
                baseline_metrics.warnings_count.map(f64::from),
                current_metrics.warnings_count.map(f64::from),
            );
            push_metric_change(
                &mut metric_changes,
                name,
                "errors_count",
                baseline_metrics.errors_count.map(f64::from),
                current_metrics.errors_count.map(f64::from),
            );
            push_metric_change(
                &mut metric_changes,
                name,
                "coverage_percent",
                baseline_metrics.coverage_percent,
                current_metrics.coverage_percent,
            );
            push_metric_change(
                &mut metric_changes,
                name,
                "memory_peak_mb",
                baseline_metrics.memory_peak_mb,
                current_metrics.memory_peak_mb,
            );
            push_metric_change(
                &mut metric_changes,
                name,
                "files_checked",
                baseline_metrics.files_checked.map(f64::from),
                current_metrics.files_checked.map(f64::from),
            );
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

fn push_metric_change(
    metric_changes: &mut Vec<MetricChange>,
    gate_name: &str,
    metric_name: &str,
    old: Option<f64>,
    new: Option<f64>,
) {
    let (Some(old_value), Some(new_value)) = (old, new) else {
        return;
    };
    if (old_value - new_value).abs() < f64::EPSILON {
        return;
    }

    let delta_percent = if old_value.abs() < f64::EPSILON {
        if new_value.abs() < f64::EPSILON { 0.0 } else { 100.0 }
    } else {
        ((new_value - old_value) / old_value) * 100.0
    };

    metric_changes.push(MetricChange {
        gate_name: gate_name.to_string(),
        metric_name: metric_name.to_string(),
        old_value,
        new_value,
        delta_percent,
        exceeds_threshold: delta_percent.abs() > 10.0,
    });
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

fn is_blocking_gate_status(status: &str) -> bool {
    matches!(status, "fail" | "timeout" | "error")
}

fn blocking_failure_gate_names(results: &[GateResult]) -> Vec<String> {
    results
        .iter()
        .filter(|result| result.required.unwrap_or(true) && is_blocking_gate_status(&result.status))
        .map(|result| result.gate_name.clone())
        .collect()
}

fn determine_overall_status(failed: u32, blocking_failures: &[String]) -> &'static str {
    if blocking_failures.is_empty() { if failed > 0 { "partial" } else { "pass" } } else { "fail" }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap, HashSet};
    use std::fs;

    use tempfile::tempdir;

    use super::{
        DiffResult, FirstFailure, GateDefinition, GateMetrics, GatePlanningConfig,
        GatePlanningRole, GatePolicy, GateResult, GateRunnerConfig, GateTier, GlobalSettings,
        MAX_GATE_OUTPUT_BYTES, MetricChange, PackageTargetIndex, Receipt,
        blocking_failure_gate_names, build_pr_fast_plan_from_scope,
        build_pr_fast_plan_from_scope_with_targets, compare_receipts, determine_overall_status,
        extend_plan_with_non_pr_fast_static_gates, extend_plan_with_static_tiers, failure_guidance,
        is_blocking_gate_status, is_cargo_test_command, load_policy_for_inspection,
        parse_first_failure, read_gate_output, run_shell_command_with_timeout, run_single_gate,
    };
    use crate::tasks::ci_scope::{
        ArchWidener, DirectCrate, LaneDecisions, PlatformOverrides, RevDepCrate, ScopeOutput,
    };

    fn gate_result(name: &str, status: &str, required: bool) -> GateResult {
        GateResult {
            gate_name: name.to_string(),
            tier: "pr_fast".to_string(),
            status: status.to_string(),
            required: Some(required),
            duration_ms: 1,
            command: "true".to_string(),
            exit_code: Some(0),
            output_summary: None,
            log_path: None,
            metrics: None,
            artifacts: None,
            first_failure: None,
        }
    }

    fn pr_gate(name: &str, role: GatePlanningRole, command: &str) -> GateDefinition {
        GateDefinition {
            name: name.to_string(),
            tier: "pr_fast".to_string(),
            description: name.to_string(),
            required: true,
            command: command.to_string(),
            timeout_seconds: 30,
            retry_count: 0,
            budgets: None,
            quarantine: false,
            tags: Vec::new(),
            artifacts: Vec::new(),
            matrix: None,
            planning: Some(GatePlanningConfig { role, packages: Vec::new() }),
        }
    }

    fn tier_gate(name: &str, tier: &str, command: &str) -> GateDefinition {
        GateDefinition {
            tier: tier.to_string(),
            planning: None,
            ..pr_gate(name, GatePlanningRole::Static, command)
        }
    }

    fn policy_with_gates(gates: Vec<GateDefinition>) -> GatePolicy {
        GatePolicy {
            schema_version: 1,
            global: GlobalSettings {
                default_timeout_seconds: 30,
                artifact_retention_days: 0,
                default_retry_count: 0,
                environment: HashMap::new(),
                toolchain: None,
            },
            tiers: HashMap::new(),
            gates,
            flake_policy: None,
            audit: None,
        }
    }

    fn package_pr_gate(name: &str, packages: Vec<String>) -> GateDefinition {
        GateDefinition {
            planning: Some(GatePlanningConfig {
                role: GatePlanningRole::RustPackageScoped,
                packages,
            }),
            ..pr_gate(name, GatePlanningRole::RustPackageScoped, "cargo test -p perl-token")
        }
    }

    fn scope_output(
        diff_class: &str,
        direct: &[&str],
        reverse: &[&str],
        wideners: &[&str],
    ) -> ScopeOutput {
        ScopeOutput {
            schema_version: 2,
            base: "origin/master".to_string(),
            head_sha: "head".to_string(),
            changed_files: Vec::new(),
            diff_class: diff_class.to_string(),
            direct_crates: direct
                .iter()
                .map(|name| DirectCrate { name: (*name).to_string(), reason: "direct".to_string() })
                .collect(),
            reverse_dep_closure: reverse
                .iter()
                .map(|name| RevDepCrate {
                    name: (*name).to_string(),
                    reason: "reverse".to_string(),
                })
                .collect(),
            architecture_wideners: wideners
                .iter()
                .map(|name| ArchWidener { name: (*name).to_string(), rule: "widener".to_string() })
                .collect(),
            risk_tags: Vec::new(),
            platform_overrides: PlatformOverrides::default(),
            selected_lanes: Vec::new(),
            selected_heavy_lanes: Vec::new(),
            lanes: LaneDecisions::default(),
            explanations: BTreeMap::new(),
        }
    }

    fn selected_gate_names(plan: &super::GatePlan) -> Vec<String> {
        plan.selected.iter().map(|planned| planned.gate.name.clone()).collect()
    }

    fn skipped_gate_names(plan: &super::GatePlan) -> Vec<String> {
        plan.skipped.iter().map(|skipped| skipped.name.clone()).collect()
    }

    #[test]
    fn pr_fast_prose_only_keeps_always_on_and_skips_rust_lanes() -> color_eyre::eyre::Result<()> {
        let gates = vec![
            pr_gate("fmt", GatePlanningRole::AlwaysOn, "cargo xtask fmt --check"),
            pr_gate("clippy_scoped", GatePlanningRole::RustScoped, "cargo clippy {package_args}"),
            pr_gate("unit_core", GatePlanningRole::RustFallback, "cargo test -p perl-parser"),
        ];

        let plan = build_pr_fast_plan_from_scope(
            GateTier::PrFast,
            "origin/master".to_string(),
            gates,
            Some(scope_output("prose_only", &[], &[], &[])),
            true,
            false,
            None,
        )?;

        assert_eq!(selected_gate_names(&plan), vec!["fmt"]);
        assert_eq!(skipped_gate_names(&plan), vec!["clippy_scoped", "unit_core"]);
        assert!(!plan.fallback_used);
        assert!(plan.skipped.iter().all(|gate| gate.reason.contains("diff_class=prose_only")));
        Ok(())
    }

    #[test]
    fn pr_fast_code_diff_selects_scoped_rust_lanes_with_full_package_scope()
    -> color_eyre::eyre::Result<()> {
        let gates = vec![
            pr_gate("fmt", GatePlanningRole::AlwaysOn, "cargo xtask fmt --check"),
            pr_gate(
                "clippy_scoped",
                GatePlanningRole::RustScoped,
                "cargo clippy --locked {package_args} -- -D warnings",
            ),
            pr_gate(
                "unit_scoped",
                GatePlanningRole::RustScoped,
                "cargo test --locked --lib {package_args}",
            ),
            pr_gate(
                "check_tests_scoped",
                GatePlanningRole::RustScoped,
                "cargo check --locked --tests {package_args}",
            ),
            pr_gate("clippy_core", GatePlanningRole::RustFallback, "cargo clippy -p perl-parser"),
        ];

        let plan = build_pr_fast_plan_from_scope(
            GateTier::PrFast,
            "origin/master".to_string(),
            gates,
            Some(scope_output("code", &["perl-parser"], &["perl-lsp-rs"], &["perl-dap"])),
            true,
            false,
            None,
        )?;

        assert_eq!(
            selected_gate_names(&plan),
            vec!["fmt", "clippy_scoped", "unit_scoped", "check_tests_scoped"]
        );
        assert_eq!(
            plan.package_args,
            vec!["-p", "perl-dap", "-p", "perl-lsp-rs", "-p", "perl-parser"]
        );
        assert_eq!(skipped_gate_names(&plan), vec!["clippy_core"]);
        let clippy = plan
            .selected
            .iter()
            .find(|planned| planned.gate.name == "clippy_scoped")
            .ok_or_else(|| color_eyre::eyre::eyre!("missing clippy_scoped plan"))?;
        assert!(clippy.gate.command.contains("-p perl-parser"));
        assert!(clippy.gate.command.contains("-p perl-lsp-rs"));
        assert!(clippy.gate.command.contains("-p perl-dap"));
        Ok(())
    }

    #[test]
    fn pr_fast_scope_failure_preserves_always_on_and_uses_fallback() -> color_eyre::eyre::Result<()>
    {
        let gates = vec![
            pr_gate("fmt", GatePlanningRole::AlwaysOn, "cargo xtask fmt --check"),
            pr_gate("clippy_scoped", GatePlanningRole::RustScoped, "cargo clippy {package_args}"),
            pr_gate("clippy_core", GatePlanningRole::RustFallback, "cargo clippy -p perl-parser"),
            pr_gate("unit_core", GatePlanningRole::RustFallback, "cargo test -p perl-parser"),
        ];

        let plan = build_pr_fast_plan_from_scope(
            GateTier::PrFast,
            "origin/master".to_string(),
            gates,
            None,
            false,
            true,
            Some("scope failed".to_string()),
        )?;

        assert_eq!(selected_gate_names(&plan), vec!["fmt", "clippy_core", "unit_core"]);
        assert_eq!(skipped_gate_names(&plan), vec!["clippy_scoped"]);
        assert!(!plan.scope_ok);
        assert!(plan.fallback_used);
        assert_eq!(plan.fallback_reason.as_deref(), Some("scope failed"));
        Ok(())
    }

    #[test]
    fn pr_fast_docs_as_code_keeps_always_on_and_skips_rust_lanes() -> color_eyre::eyre::Result<()> {
        let gates = vec![
            pr_gate("fmt", GatePlanningRole::AlwaysOn, "cargo xtask fmt --check"),
            pr_gate("clippy_scoped", GatePlanningRole::RustScoped, "cargo clippy {package_args}"),
            pr_gate("unit_core", GatePlanningRole::RustFallback, "cargo test -p perl-parser"),
        ];

        let plan = build_pr_fast_plan_from_scope(
            GateTier::PrFast,
            "origin/master".to_string(),
            gates,
            Some(scope_output("docs_as_code", &[], &[], &[])),
            true,
            false,
            None,
        )?;

        assert_eq!(selected_gate_names(&plan), vec!["fmt"]);
        assert_eq!(skipped_gate_names(&plan), vec!["clippy_scoped", "unit_core"]);
        assert!(!plan.fallback_used);
        assert!(plan.skipped.iter().all(|gate| gate.reason.contains("diff_class=docs_as_code")));
        Ok(())
    }

    #[test]
    fn pr_fast_ci_config_keeps_always_on_and_skips_rust_lanes() -> color_eyre::eyre::Result<()> {
        let gates = vec![
            pr_gate("fmt", GatePlanningRole::AlwaysOn, "cargo xtask fmt --check"),
            pr_gate("clippy_scoped", GatePlanningRole::RustScoped, "cargo clippy {package_args}"),
            pr_gate("unit_core", GatePlanningRole::RustFallback, "cargo test -p perl-parser"),
        ];

        let plan = build_pr_fast_plan_from_scope(
            GateTier::PrFast,
            "origin/master".to_string(),
            gates,
            Some(scope_output("ci_config", &[], &[], &[])),
            true,
            false,
            None,
        )?;

        assert_eq!(selected_gate_names(&plan), vec!["fmt"]);
        assert_eq!(skipped_gate_names(&plan), vec!["clippy_scoped", "unit_core"]);
        assert!(!plan.fallback_used);
        assert!(plan.skipped.iter().all(|gate| gate.reason.contains("diff_class=ci_config")));
        Ok(())
    }

    #[test]
    fn pr_fast_code_diff_with_empty_package_set_uses_fallback() -> color_eyre::eyre::Result<()> {
        let gates = vec![
            pr_gate("fmt", GatePlanningRole::AlwaysOn, "cargo xtask fmt --check"),
            pr_gate("clippy_scoped", GatePlanningRole::RustScoped, "cargo clippy {package_args}"),
            pr_gate("clippy_core", GatePlanningRole::RustFallback, "cargo clippy -p perl-parser"),
        ];

        let plan = build_pr_fast_plan_from_scope(
            GateTier::PrFast,
            "origin/master".to_string(),
            gates,
            Some(scope_output("code", &[], &[], &[])),
            true,
            true,
            Some("ci-scope produced no package scope for a Rust-relevant diff".to_string()),
        )?;

        assert_eq!(selected_gate_names(&plan), vec!["fmt", "clippy_core"]);
        assert_eq!(skipped_gate_names(&plan), vec!["clippy_scoped"]);
        assert!(plan.fallback_used);
        Ok(())
    }

    #[test]
    fn pr_fast_package_scoped_gate_runs_only_when_package_selected() -> color_eyre::eyre::Result<()>
    {
        let gates = vec![
            pr_gate("fmt", GatePlanningRole::AlwaysOn, "cargo xtask fmt --check"),
            package_pr_gate("perl_token_leaf_contract", vec!["perl-token".to_string()]),
        ];

        let selected_plan = build_pr_fast_plan_from_scope(
            GateTier::PrFast,
            "origin/master".to_string(),
            gates.clone(),
            Some(scope_output("code", &["perl-token"], &[], &[])),
            true,
            false,
            None,
        )?;
        assert_eq!(selected_gate_names(&selected_plan), vec!["fmt", "perl_token_leaf_contract"]);

        let skipped_plan = build_pr_fast_plan_from_scope(
            GateTier::PrFast,
            "origin/master".to_string(),
            gates,
            Some(scope_output("code", &["perl-parser"], &[], &[])),
            true,
            false,
            None,
        )?;
        assert_eq!(selected_gate_names(&skipped_plan), vec!["fmt"]);
        assert_eq!(skipped_gate_names(&skipped_plan), vec!["perl_token_leaf_contract"]);
        Ok(())
    }

    #[test]
    fn pr_fast_package_scoped_gate_runs_on_scope_failure() -> color_eyre::eyre::Result<()> {
        let gates = vec![
            pr_gate("fmt", GatePlanningRole::AlwaysOn, "cargo xtask fmt --check"),
            package_pr_gate("perl_token_leaf_contract", vec!["perl-token".to_string()]),
            pr_gate("unit_core", GatePlanningRole::RustFallback, "cargo test -p perl-parser"),
        ];

        let plan = build_pr_fast_plan_from_scope(
            GateTier::PrFast,
            "origin/master".to_string(),
            gates,
            None,
            false,
            true,
            Some("scope failed".to_string()),
        )?;

        assert_eq!(
            selected_gate_names(&plan),
            vec!["fmt", "perl_token_leaf_contract", "unit_core"]
        );
        assert!(plan.fallback_used);
        Ok(())
    }

    #[test]
    fn pr_fast_lib_scoped_gate_filters_packages_without_lib_targets() -> color_eyre::eyre::Result<()>
    {
        let gates = vec![
            pr_gate(
                "unit_scoped",
                GatePlanningRole::RustScoped,
                "cargo test --locked --lib {package_args}",
            ),
            pr_gate(
                "check_tests_scoped",
                GatePlanningRole::RustScoped,
                "cargo check --locked --tests {package_args}",
            ),
        ];
        let target_index =
            PackageTargetIndex { lib_packages: HashSet::from(["perl-parser".to_string()]) };

        let plan = build_pr_fast_plan_from_scope_with_targets(
            GateTier::PrFast,
            "origin/master".to_string(),
            gates,
            Some(scope_output("code", &["perl-parser", "xtask"], &[], &[])),
            true,
            false,
            None,
            Some(&target_index),
        )?;

        let Some(unit) = plan.selected.iter().find(|planned| planned.gate.name == "unit_scoped")
        else {
            color_eyre::eyre::bail!("missing unit_scoped plan");
        };
        assert!(unit.gate.command.contains("-p perl-parser"));
        assert!(!unit.gate.command.contains("-p xtask"));

        let Some(check_tests) =
            plan.selected.iter().find(|planned| planned.gate.name == "check_tests_scoped")
        else {
            color_eyre::eyre::bail!("missing check_tests_scoped plan");
        };
        assert!(check_tests.gate.command.contains("-p perl-parser"));
        assert!(check_tests.gate.command.contains("-p xtask"));
        Ok(())
    }

    #[test]
    fn pr_fast_code_diff_package_args_include_direct_crates() -> color_eyre::eyre::Result<()> {
        let gates = vec![pr_gate(
            "clippy_scoped",
            GatePlanningRole::RustScoped,
            "cargo clippy --locked {package_args}",
        )];

        let plan = build_pr_fast_plan_from_scope(
            GateTier::PrFast,
            "origin/master".to_string(),
            gates,
            Some(scope_output("code", &["perl-parser"], &[], &[])),
            true,
            false,
            None,
        )?;

        assert_eq!(plan.package_args, vec!["-p", "perl-parser"]);
        Ok(())
    }

    #[test]
    fn pr_fast_code_diff_package_args_include_reverse_dependencies() -> color_eyre::eyre::Result<()>
    {
        let gates = vec![pr_gate(
            "clippy_scoped",
            GatePlanningRole::RustScoped,
            "cargo clippy --locked {package_args}",
        )];

        let plan = build_pr_fast_plan_from_scope(
            GateTier::PrFast,
            "origin/master".to_string(),
            gates,
            Some(scope_output("code", &[], &["perl-lsp-rs"], &[])),
            true,
            false,
            None,
        )?;

        assert_eq!(plan.package_args, vec!["-p", "perl-lsp-rs"]);
        Ok(())
    }

    #[test]
    fn pr_fast_code_diff_package_args_include_architecture_wideners() -> color_eyre::eyre::Result<()>
    {
        let gates = vec![pr_gate(
            "clippy_scoped",
            GatePlanningRole::RustScoped,
            "cargo clippy --locked {package_args}",
        )];

        let plan = build_pr_fast_plan_from_scope(
            GateTier::PrFast,
            "origin/master".to_string(),
            gates,
            Some(scope_output("code", &[], &[], &["perl-dap"])),
            true,
            false,
            None,
        )?;

        assert_eq!(plan.package_args, vec!["-p", "perl-dap"]);
        Ok(())
    }

    #[test]
    fn pr_fast_policy_planning_roles_are_complete() -> color_eyre::eyre::Result<()> {
        let root = crate::utils::project_root()?;
        let policy_path = root.join(".ci/gate-policy.yaml");
        let policy = load_policy_for_inspection(&policy_path)?;

        for gate in policy.gates.iter().filter(|gate| gate.tier == "pr_fast") {
            let Some(planning) = &gate.planning else {
                color_eyre::eyre::bail!("pr_fast gate '{}' missing planning.role", gate.name);
            };
            if planning.role == GatePlanningRole::Static {
                color_eyre::eyre::bail!(
                    "pr_fast gate '{}' must not use planning.role=static",
                    gate.name
                );
            }
            if planning.role == GatePlanningRole::RustScoped
                && !gate.command.contains("{package_args}")
            {
                color_eyre::eyre::bail!(
                    "rust_scoped gate '{}' missing {{package_args}} placeholder",
                    gate.name
                );
            }
            if planning.role == GatePlanningRole::RustPackageScoped && planning.packages.is_empty()
            {
                color_eyre::eyre::bail!(
                    "rust_package_scoped gate '{}' missing planning.packages",
                    gate.name
                );
            }
            if planning.role == GatePlanningRole::RustFallback
                && gate.command.contains("{package_args}")
            {
                color_eyre::eyre::bail!(
                    "rust_fallback gate '{}' must not contain {{package_args}}",
                    gate.name
                );
            }
        }
        Ok(())
    }

    #[test]
    fn merge_gate_static_extension_does_not_add_raw_pr_fast_templates()
    -> color_eyre::eyre::Result<()> {
        let mut plan = build_pr_fast_plan_from_scope(
            GateTier::PrFast,
            "origin/master".to_string(),
            vec![pr_gate("fmt", GatePlanningRole::AlwaysOn, "true")],
            Some(scope_output("prose_only", &[], &[], &[])),
            true,
            false,
            None,
        )?;
        let policy = policy_with_gates(vec![
            pr_gate("clippy_scoped", GatePlanningRole::RustScoped, "cargo clippy {package_args}"),
            tier_gate("clippy_full", "merge_gate", "cargo clippy --workspace"),
        ]);

        extend_plan_with_static_tiers(&mut plan, &policy, &["merge_gate"]);

        assert_eq!(selected_gate_names(&plan), vec!["fmt", "clippy_full"]);
        assert!(
            plan.selected.iter().all(|planned| !planned.gate.command.contains("{package_args}"))
        );
        Ok(())
    }

    #[test]
    fn all_static_extension_includes_release_gates() -> color_eyre::eyre::Result<()> {
        let mut plan = build_pr_fast_plan_from_scope(
            GateTier::PrFast,
            "origin/master".to_string(),
            vec![pr_gate("fmt", GatePlanningRole::AlwaysOn, "true")],
            Some(scope_output("prose_only", &[], &[], &[])),
            true,
            false,
            None,
        )?;
        let policy = policy_with_gates(vec![
            pr_gate("clippy_scoped", GatePlanningRole::RustScoped, "cargo clippy {package_args}"),
            tier_gate("clippy_full", "merge_gate", "cargo clippy --workspace"),
            tier_gate("nightly_corpus", "nightly", "cargo xtask corpus"),
            tier_gate("release_build", "release", "cargo build --release"),
        ]);

        extend_plan_with_non_pr_fast_static_gates(&mut plan, &policy);

        assert_eq!(
            selected_gate_names(&plan),
            vec!["fmt", "clippy_full", "nightly_corpus", "release_build"]
        );
        Ok(())
    }

    #[test]
    fn shell_command_timeout_marks_execution_and_writes_log() -> color_eyre::eyre::Result<()> {
        let tmp = tempdir()?;
        let log_path = tmp.path().join("timeout.log");
        // On Windows, cmd /C PowerShell quoting is unreliable for embedded double
        // quotes; use ping -n 4 (sends 4 ICMP echo requests ~1s apart, ~3s total)
        // as a portable delay that works through cmd.exe without quote issues.
        let command = if cfg!(windows) { "ping -n 4 127.0.0.1" } else { "sleep 3" };

        let execution = run_shell_command_with_timeout(command, &log_path, 1)?;

        assert!(execution.timed_out, "execution should time out");
        assert_eq!(execution.exit_code, 124, "timed out commands map to synthetic 124");
        assert!(log_path.exists(), "timeout log file should be created");
        Ok(())
    }

    #[test]
    fn shell_command_natural_exit_preserves_actual_exit_code() -> color_eyre::eyre::Result<()> {
        let tmp = tempdir()?;
        let log_path = tmp.path().join("natural_exit.log");
        // A command that exits quickly with a non-zero code.
        // On Windows `cmd /C exit 42` is reliable; on Unix `bash -lc "exit 42"`.
        let command = if cfg!(windows) { "exit 42" } else { "exit 42" };

        let execution = run_shell_command_with_timeout(command, &log_path, 30)?;

        assert!(!execution.timed_out, "process that exits naturally must not be marked timed_out");
        assert_eq!(
            execution.exit_code, 42,
            "natural exit code must be preserved (not overwritten with 124)"
        );
        Ok(())
    }

    #[test]
    fn gate_output_reader_truncates_large_logs_to_tail() -> color_eyre::eyre::Result<()> {
        let tmp = tempdir()?;
        let log_path = tmp.path().join("large.log");
        let mut contents = vec![b'a'; MAX_GATE_OUTPUT_BYTES as usize + 1024];
        contents.extend_from_slice(b"\nlast important line\n");
        fs::write(&log_path, contents)?;

        let output = read_gate_output(&log_path);

        assert!(output.starts_with("[gate log truncated"), "large log should be marked truncated");
        assert!(output.contains("last important line"), "tail should preserve useful diagnostics");
        Ok(())
    }

    #[test]
    fn required_gate_timeout_reports_receipt_fields_and_blocks_overall_status()
    -> color_eyre::eyre::Result<()> {
        let gate = GateDefinition {
            name: "synthetic_timeout_gate".to_string(),
            tier: "merge_gate".to_string(),
            description: "Synthetic timeout gate for regression coverage".to_string(),
            required: true,
            // Same rationale as shell_command_timeout_marks_execution_and_writes_log:
            // ping -n 4 is the Windows-safe delay that survives cmd.exe quoting.
            command: if cfg!(windows) {
                "ping -n 4 127.0.0.1".to_string()
            } else {
                "sleep 3".to_string()
            },
            timeout_seconds: 1,
            retry_count: 0,
            budgets: None,
            quarantine: false,
            tags: Vec::new(),
            artifacts: Vec::new(),
            matrix: None,
            planning: Some(GatePlanningConfig {
                role: GatePlanningRole::AlwaysOn,
                packages: Vec::new(),
            }),
        };
        let policy = policy_with_gates(vec![gate.clone()]);
        let tmp = tempdir()?;
        let config = GateRunnerConfig::default();

        let result = run_single_gate(&gate, &policy, tmp.path(), &config)?;

        assert_eq!(result.gate_name, "synthetic_timeout_gate");
        assert_eq!(result.status, "timeout");
        assert_eq!(gate.timeout_seconds, 1, "timeout_seconds fixture must remain explicit");
        assert!(result.duration_ms >= 1_000, "duration should include timeout window");
        assert_eq!(result.command, gate.command);
        assert_eq!(result.log_path.as_deref(), Some("logs/synthetic_timeout_gate.log"));
        assert!(result.output_summary.is_some(), "timeout should preserve output summary context");

        let blocking = blocking_failure_gate_names(std::slice::from_ref(&result));
        assert_eq!(blocking, vec!["synthetic_timeout_gate"]);
        assert_eq!(determine_overall_status(0, &blocking), "fail");

        let (failures, _) = failure_guidance(&[result]);
        assert_eq!(failures.len(), 1);
        assert!(
            failures[0].summary.contains("timeout"),
            "first_failure summary should explain timeout classification"
        );

        Ok(())
    }

    #[test]
    fn blocking_status_classification_includes_timeout_and_error() {
        assert!(is_blocking_gate_status("fail"));
        assert!(is_blocking_gate_status("timeout"));
        assert!(is_blocking_gate_status("error"));
        assert!(!is_blocking_gate_status("pass"));
        assert!(!is_blocking_gate_status("skip"));
    }

    #[test]
    fn required_timeout_and_error_are_blocking_failures() {
        let results = vec![
            gate_result("req-timeout", "timeout", true),
            gate_result("req-error", "error", true),
            gate_result("req-fail", "fail", true),
            gate_result("opt-timeout", "timeout", false),
            gate_result("opt-error", "error", false),
            gate_result("opt-fail", "fail", false),
        ];

        let blocking = blocking_failure_gate_names(&results);
        assert_eq!(blocking, vec!["req-timeout", "req-error", "req-fail"]);
    }

    #[test]
    fn overall_status_is_fail_when_required_timeout_exists_even_without_fail_count() {
        let blocking_failures = vec!["req-timeout".to_string()];
        assert_eq!(determine_overall_status(0, &blocking_failures), "fail");
    }

    #[test]
    fn failure_guidance_includes_repro_and_next_actions_for_blocking_gates() {
        let results = vec![
            gate_result("clippy", "fail", true),
            gate_result("doc", "pass", true),
            gate_result("lint", "fail", false),
        ];
        let (failures, next_actions) = failure_guidance(&results);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].lane, "clippy");
        assert_eq!(failures[0].repro, "true # gate=clippy");
        assert_eq!(
            next_actions,
            vec![
                "Reproduce and fix gate 'clippy' locally, then rerun: cargo xtask gates --gate clippy"
            ]
        );
    }

    #[test]
    fn agent_receipt_phase1_fields_roundtrip_with_correct_values() {
        // Verify that the phase-1 agent receipt shape deserializes correctly
        // and that values survive the serde round-trip unchanged.
        // Uses Option<AgentReceipt> to confirm old receipts without the field
        // still deserialize successfully (backward compat).
        let receipt: Receipt = serde_json::from_str(r#"{
            "schema_version": "1.0.0",
            "metadata": {
                "timestamp": "2026-04-23T00:00:00Z",
                "git_sha": "abc123",
                "git_sha_short": "abc123",
                "git_branch": "work",
                "git_dirty": false,
                "toolchain": {"rustc_version": "1.0.0"},
                "platform": {"os": "linux", "arch": "x86_64"},
                "environment": {"type": "local"}
            },
            "gates": [],
            "summary": {
                "total_gates": 0,
                "passed": 0,
                "failed": 0,
                "skipped": 0,
                "total_duration_ms": 10,
                "overall_status": "pass"
            },
            "agent_receipt": {
                "sha": "deadbeef1234567890abcdef1234567890abcdef",
                "is_latest": false,
                "tier": "pr_fast",
                "scope": {
                    "direct_crates": ["xtask", "perl-parser"],
                    "reverse_deps": ["perl-lsp-rs"],
                    "risk_tags": ["ci_policy", "parser_recovery"]
                },
                "selected_lanes": [
                    {"name":"clippy_scoped","reason":"direct_crate_change","status":"passed"},
                    {"name":"test_scoped","reason":"direct_crate_change","status":"not_run"}
                ],
                "failures": [{"lane":"clippy","summary":"clippy found 3 warnings","repro":"cargo clippy -p xtask"}],
                "suggested_next_actions": ["fix clippy warnings", "rerun gate"]
            }
        }"#)
        .expect("phase-1 agent receipt shape should deserialize");

        // agent_receipt must be present (Some, not None)
        let ar = receipt.agent_receipt.expect("agent_receipt should be Some when present in JSON");

        // Verify field values, not just key presence — these would fail if
        // a field were silently dropped or misnamed in the struct definition.
        assert_eq!(ar.sha, "deadbeef1234567890abcdef1234567890abcdef");
        assert!(!ar.is_latest, "is_latest should be false");
        assert_eq!(ar.tier, "pr_fast");
        assert_eq!(ar.scope.direct_crates, vec!["xtask", "perl-parser"]);
        assert_eq!(ar.scope.reverse_deps, vec!["perl-lsp-rs"]);
        assert_eq!(ar.scope.risk_tags, vec!["ci_policy", "parser_recovery"]);
        assert_eq!(ar.selected_lanes.len(), 2);
        assert_eq!(ar.selected_lanes[0].name, "clippy_scoped");
        assert_eq!(ar.selected_lanes[0].status, "passed");
        assert_eq!(ar.selected_lanes[1].status, "not_run");
        assert_eq!(ar.failures.len(), 1);
        assert_eq!(ar.failures[0].lane, "clippy");
        assert_eq!(ar.failures[0].repro, "cargo clippy -p xtask");
        assert_eq!(ar.suggested_next_actions.len(), 2);

        // Confirm backward compatibility: a receipt WITHOUT agent_receipt deserializes to None.
        let old_receipt: Receipt = serde_json::from_str(
            r#"{
            "schema_version": "1.0.0",
            "metadata": {
                "timestamp": "2026-04-23T00:00:00Z",
                "git_sha": "abc123",
                "git_sha_short": "abc123",
                "git_branch": "work",
                "git_dirty": false,
                "toolchain": {"rustc_version": "1.0.0"},
                "platform": {"os": "linux", "arch": "x86_64"},
                "environment": {"type": "local"}
            },
            "gates": [],
            "summary": {
                "total_gates": 0,
                "passed": 0,
                "failed": 0,
                "skipped": 0,
                "total_duration_ms": 10,
                "overall_status": "pass"
            }
        }"#,
        )
        .expect("receipt without agent_receipt should deserialize for backward compat");
        assert!(
            old_receipt.agent_receipt.is_none(),
            "receipt without agent_receipt field must deserialize to None"
        );
    }

    #[test]
    fn failure_guidance_with_no_gates_produces_proceed_action() {
        // Edge case: no gates ran at all (empty results slice).
        let (failures, next_actions) = failure_guidance(&[]);
        assert!(failures.is_empty(), "no failures expected when no gates ran");
        assert_eq!(next_actions.len(), 1);
        assert!(
            next_actions[0].contains("No blocking failures"),
            "expected proceed action, got: {:?}",
            next_actions[0]
        );
    }

    #[test]
    fn failure_guidance_all_required_and_failing_each_gets_action() {
        // Multiple blocking failures — each should produce its own next_action entry.
        let results = vec![
            gate_result("fmt", "fail", true),
            gate_result("clippy", "error", true),
            gate_result("tests", "timeout", true),
        ];
        let (failures, next_actions) = failure_guidance(&results);
        assert_eq!(failures.len(), 3, "all three blocking gates should appear in failures");
        assert_eq!(next_actions.len(), 3, "each failure gets one next_action");
        // Repro command must include the gate's command string
        assert!(failures[0].repro.contains("fmt"), "repro should reference the gate");
        assert!(failures[2].summary.contains("timeout"), "summary should mention the status");
    }

    fn test_receipt_with_metrics(metrics: GateMetrics) -> Receipt {
        // Deserialize from a minimal JSON skeleton so we don't have to
        // construct every required nested struct (ToolchainInfo, PlatformInfo,
        // EnvironmentInfo, AgentReceipt, …) by hand.  compare_receipts only
        // reads receipt.gates and receipt.metadata.timestamp, so the rest can
        // be placeholder values.
        let mut receipt: Receipt = serde_json::from_str(
            r#"{
            "schema_version": "1",
            "metadata": {
                "timestamp": "2026-04-23T00:00:00Z",
                "git_sha": "abc123",
                "git_sha_short": "abc123",
                "git_branch": "work",
                "git_dirty": false,
                "toolchain": {"rustc_version": "1.0.0"},
                "platform": {"os": "linux", "arch": "x86_64"},
                "environment": {"type": "local"}
            },
            "gates": [],
            "summary": {
                "total_gates": 1,
                "passed": 1,
                "failed": 0,
                "skipped": 0,
                "total_duration_ms": 10,
                "overall_status": "pass"
            },
            "agent_receipt": {
                "sha": "abc123",
                "is_latest": true,
                "tier": "merge_gate",
                "scope": {"direct_crates": [], "reverse_deps": [], "risk_tags": []},
                "selected_lanes": [],
                "failures": [],
                "suggested_next_actions": []
            }
        }"#,
        )
        .expect("minimal receipt JSON is valid");
        receipt.gates.push(GateResult {
            gate_name: "tests".to_string(),
            tier: "pr_fast".to_string(),
            status: "pass".to_string(),
            required: Some(true),
            duration_ms: 10,
            command: "cargo test".to_string(),
            exit_code: Some(0),
            output_summary: None,
            log_path: None,
            metrics: Some(metrics),
            artifacts: None,
            first_failure: None,
        });
        receipt
    }

    fn metric_change_for<'a>(diff: &'a DiffResult, name: &str) -> Option<&'a MetricChange> {
        diff.metric_changes.iter().find(|change| change.metric_name == name)
    }

    #[test]
    fn compare_receipts_reports_multiple_metric_dimensions() {
        let baseline = test_receipt_with_metrics(GateMetrics {
            tests_total: Some(100),
            tests_passed: Some(95),
            tests_failed: Some(5),
            warnings_count: Some(2),
            coverage_percent: Some(80.0),
            ..GateMetrics::default()
        });
        let current = test_receipt_with_metrics(GateMetrics {
            tests_total: Some(110),
            tests_passed: Some(108),
            tests_failed: Some(2),
            warnings_count: Some(1),
            coverage_percent: Some(82.5),
            ..GateMetrics::default()
        });

        let diff = compare_receipts(&baseline, &current).expect("compare receipts should succeed");
        assert!(
            metric_change_for(&diff, "tests_total").is_some(),
            "tests_total change should be recorded"
        );
        assert!(
            metric_change_for(&diff, "tests_passed").is_some(),
            "tests_passed change should be recorded"
        );
        assert!(
            metric_change_for(&diff, "tests_failed").is_some(),
            "tests_failed change should be recorded"
        );
        assert!(
            metric_change_for(&diff, "warnings_count").is_some(),
            "warnings_count change should be recorded"
        );
        assert!(
            metric_change_for(&diff, "coverage_percent").is_some(),
            "coverage_percent change should be recorded"
        );
    }

    #[test]
    fn compare_receipts_handles_zero_baseline_delta_without_nan() {
        let baseline = test_receipt_with_metrics(GateMetrics {
            warnings_count: Some(0),
            ..GateMetrics::default()
        });
        let current = test_receipt_with_metrics(GateMetrics {
            warnings_count: Some(3),
            ..GateMetrics::default()
        });

        let diff = compare_receipts(&baseline, &current).expect("compare receipts should succeed");
        let warning_change =
            metric_change_for(&diff, "warnings_count").expect("warnings_count metric should exist");
        assert_eq!(warning_change.delta_percent, 100.0);
        assert!(!warning_change.delta_percent.is_nan());
        assert!(!warning_change.delta_percent.is_infinite());
    }

    // ==========================================================================
    // Tests for parse_first_failure and is_cargo_test_command
    // ==========================================================================

    /// Fixture: realistic cargo test output for a failing test (Rust ≥1.73 style).
    /// Based on the evidence from issue #7031 investigation.
    const CARGO_TEST_FAILURE_NEW_STYLE: &str = r#"
running 4 tests
test refactor::refactoring::tests::validation_tests::test_cleanup_preserves_required ... ok
test refactor::refactoring::tests::validation_tests::test_cleanup_respects_retention_count ... FAILED
test refactor::refactoring::tests::validation_tests::test_basic_refactoring ... ok
test refactor::refactoring::tests::validation_tests::test_empty_input ... ok

failures:

---- refactor::refactoring::tests::validation_tests::test_cleanup_respects_retention_count stdout ----
thread 'refactor::refactoring::tests::validation_tests::test_cleanup_respects_retention_count' panicked at crates/perl-parser/src/refactor/refactoring.rs:2859:9:
assertion `left == right` failed
  left: 0
  right: 2
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

failures:
    refactor::refactoring::tests::validation_tests::test_cleanup_respects_retention_count

test result: FAILED. 3 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
"#;

    /// Fixture: Rust <1.73 style panic output (quoted message before location).
    const CARGO_TEST_FAILURE_OLD_STYLE: &str = r#"
running 2 tests
test module::tests::test_something ... ok
test module::tests::test_other ... FAILED

failures:

---- module::tests::test_other stdout ----
thread 'module::tests::test_other' panicked at 'assertion failed: x == y', src/module.rs:42:5
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

failures:
    module::tests::test_other

test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
"#;

    /// Fixture: output with no test failure markers (compile error only).
    const COMPILE_ERROR_OUTPUT: &str = r#"
error[E0308]: mismatched types
 --> src/lib.rs:10:5
  |
10 |     42
   |     ^^ expected `()`, found integer

error: aborting due to previous error
"#;

    #[test]
    fn parse_first_failure_extracts_test_name_site_and_message_new_style() {
        let ff = parse_first_failure(CARGO_TEST_FAILURE_NEW_STYLE, 101)
            .expect("should find failure in new-style output");

        assert_eq!(
            ff.test.as_deref(),
            Some(
                "refactor::refactoring::tests::validation_tests::test_cleanup_respects_retention_count"
            ),
            "test name should be the first FAILED test"
        );
        assert_eq!(
            ff.site.as_deref(),
            Some("crates/perl-parser/src/refactor/refactoring.rs:2859"),
            "site should be file:line (no column)"
        );
        // The message is the first non-empty line after `panicked at`
        assert_eq!(
            ff.message.as_deref(),
            Some("assertion `left == right` failed"),
            "message should be the line immediately after panicked at"
        );
        assert_eq!(ff.exit_code, 101);
    }

    #[test]
    fn parse_first_failure_extracts_site_old_style() {
        let ff = parse_first_failure(CARGO_TEST_FAILURE_OLD_STYLE, 101)
            .expect("should find failure in old-style output");

        assert_eq!(
            ff.test.as_deref(),
            Some("module::tests::test_other"),
            "test name should come from the FAILED line"
        );
        assert_eq!(
            ff.site.as_deref(),
            Some("src/module.rs:42"),
            "site should be extracted from old-style quoted panic location"
        );
    }

    #[test]
    fn parse_first_failure_returns_none_for_compile_error_only() {
        // No test failure markers — should return None since nothing useful to extract
        let result = parse_first_failure(COMPILE_ERROR_OUTPUT, 101);
        assert!(
            result.is_none(),
            "compile-only errors with no test failure markers should yield None"
        );
    }

    #[test]
    fn parse_first_failure_returns_none_for_empty_output() {
        let result = parse_first_failure("", 101);
        assert!(result.is_none(), "empty output should yield None");
    }

    #[test]
    fn parse_first_failure_exit_code_is_preserved() {
        let ff = parse_first_failure(CARGO_TEST_FAILURE_NEW_STYLE, 42)
            .expect("should find failure markers");
        assert_eq!(ff.exit_code, 42, "exit_code should match what was passed in");
    }

    #[test]
    fn parse_first_failure_prefers_failed_line_over_stdout_section() {
        // When both `... FAILED` and `---- ... stdout ----` are present,
        // the test name from `... FAILED` should win (it appears first).
        let ff =
            parse_first_failure(CARGO_TEST_FAILURE_NEW_STYLE, 101).expect("should find failure");
        // The `... FAILED` line should be chosen
        assert_eq!(
            ff.test.as_deref(),
            Some(
                "refactor::refactoring::tests::validation_tests::test_cleanup_respects_retention_count"
            )
        );
    }

    #[test]
    fn parse_first_failure_roundtrips_through_first_failure_struct() {
        // Verify that FirstFailure serializes and deserializes without loss.
        let ff = FirstFailure {
            test: Some("my::test::path".to_string()),
            site: Some("src/lib.rs:10".to_string()),
            message: Some("assertion failed".to_string()),
            exit_code: 101,
        };
        let json = serde_json::to_string(&ff).expect("should serialize");
        let roundtripped: FirstFailure = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(ff, roundtripped);
    }

    #[test]
    fn first_failure_skips_serializing_none_fields() {
        // None fields should be omitted from JSON (skip_serializing_if = "Option::is_none")
        let ff = FirstFailure { test: None, site: None, message: None, exit_code: 1 };
        let json = serde_json::to_string(&ff).expect("should serialize");
        assert!(!json.contains("\"test\""), "None test field should be omitted from JSON");
        assert!(!json.contains("\"site\""), "None site field should be omitted from JSON");
        assert!(!json.contains("\"message\""), "None message field should be omitted from JSON");
        assert!(json.contains("\"exit_code\""), "exit_code is always present");
    }

    #[test]
    fn is_cargo_test_command_matches_standard_forms() {
        assert!(is_cargo_test_command("cargo test"), "bare cargo test");
        assert!(is_cargo_test_command("cargo test -p perl-parser --lib"), "with flags");
        assert!(is_cargo_test_command("cargo test --workspace"), "workspace flag");
        assert!(is_cargo_test_command("/usr/local/bin/cargo test"), "absolute path cargo");
    }

    #[test]
    fn is_cargo_test_command_rejects_non_test_commands() {
        assert!(!is_cargo_test_command("cargo clippy"), "clippy is not test");
        assert!(!is_cargo_test_command("cargo build"), "build is not test");
        assert!(!is_cargo_test_command("cargo check"), "check is not test");
        assert!(!is_cargo_test_command("cargo xtask fmt --check"), "xtask fmt is not test");
        assert!(!is_cargo_test_command("true"), "bare true is not test");
        assert!(!is_cargo_test_command(""), "empty string is not test");
    }

    #[test]
    fn gate_result_first_failure_field_roundtrips_in_json() {
        // Verify that GateResult with first_failure serializes / deserializes correctly,
        // and that old receipts (without first_failure) still deserialize (backward compat).
        let result = GateResult {
            gate_name: "unit_core".to_string(),
            tier: "pr_fast".to_string(),
            status: "fail".to_string(),
            required: Some(true),
            duration_ms: 1000,
            command: "cargo test -p perl-parser --lib".to_string(),
            exit_code: Some(101),
            output_summary: None,
            log_path: None,
            metrics: None,
            artifacts: None,
            first_failure: Some(FirstFailure {
                test: Some("parser::tests::test_foo".to_string()),
                site: Some("src/lib.rs:99".to_string()),
                message: Some("assertion failed".to_string()),
                exit_code: 101,
            }),
        };
        let json = serde_json::to_string(&result).expect("should serialize");
        let roundtripped: GateResult = serde_json::from_str(&json).expect("should deserialize");
        let ff = roundtripped.first_failure.expect("first_failure should be Some after roundtrip");
        assert_eq!(ff.test.as_deref(), Some("parser::tests::test_foo"));
        assert_eq!(ff.site.as_deref(), Some("src/lib.rs:99"));
        assert_eq!(ff.exit_code, 101);
    }

    #[test]
    fn gate_result_without_first_failure_deserializes_for_backward_compat() {
        // Old receipts (before this feature) won't have `first_failure` in JSON.
        // Deserialization must succeed and produce None.
        let json = r#"{
            "gate_name": "unit_core",
            "tier": "pr_fast",
            "status": "fail",
            "duration_ms": 500,
            "command": "cargo test -p perl-parser"
        }"#;
        let result: GateResult = serde_json::from_str(json).expect("backward compat deserialize");
        assert!(result.first_failure.is_none(), "first_failure must be None when absent from JSON");
    }
}
