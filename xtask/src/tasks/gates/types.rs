use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

