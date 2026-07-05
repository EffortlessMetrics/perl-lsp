//! Shared receipt contracts for the upstream Perl core harness lane.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

pub const DISCOVERY_SCHEMA_VERSION: &str = "perl_core_harness.discovery.v1";
pub const RUN_REPORT_SCHEMA_VERSION: &str = "perl_core_harness.report.v1";
pub const COMPILE_BASELINE_SCHEMA_VERSION: &str = "perl_core_harness.compile_baseline.v1";
pub const SMOKE_SCHEMA_VERSION: &str = "perl_core_harness.smoke.v1";
pub const PREPARE_SCHEMA_VERSION: &str = "perl_core_harness.prepare.v1";
pub const GAP_MAP_SCHEMA_VERSION: &str = "perl_core_harness.gap_map.v1";
pub const RUNNER_RECORD_SCHEMA_VERSION: &str = "perl_core_harness.runner_record.v1";

/// Upstream Perl test scheduler to query.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum HarnessRunner {
    /// Bootstrap runner in upstream `t/TEST`.
    Test,
    /// TAP::Harness-backed runner in upstream `t/harness`.
    Harness,
}

impl HarnessRunner {
    pub fn script_name(self) -> &'static str {
        match self {
            Self::Test => "TEST",
            Self::Harness => "harness",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Test => "test",
            Self::Harness => "harness",
        }
    }
}

impl fmt::Display for HarnessRunner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Compiler/test mode for later run slices.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum HarnessMode {
    Parse,
    Compile,
    Execute,
}

impl HarnessMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Parse => "parse",
            Self::Compile => "compile",
            Self::Execute => "execute",
        }
    }
}

impl fmt::Display for HarnessMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Staged upstream Perl core profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum HarnessProfile {
    Base,
    Comp,
    Run,
    Core,
    Lib,
    Full,
}

impl HarnessProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::Comp => "comp",
            Self::Run => "run",
            Self::Core => "core",
            Self::Lib => "lib",
            Self::Full => "full",
        }
    }

    pub fn roots(self) -> &'static [&'static str] {
        match self {
            Self::Base => &["base"],
            Self::Comp => &["comp"],
            Self::Run => &["run"],
            Self::Core => &["base", "comp", "run", "cmd", "io", "re", "opbasic", "op"],
            Self::Lib => &["lib"],
            Self::Full => &["base", "comp", "run", "cmd", "io", "re", "opbasic", "op", "uni"],
        }
    }
}

impl fmt::Display for HarnessProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Machine-readable discovery manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscoveryReport {
    pub schema_version: String,
    pub commit: String,
    pub timestamp: String,
    pub perl_ref: String,
    pub prepared_tree: String,
    pub host_perl: String,
    pub runner: HarnessRunner,
    pub profile: HarnessProfile,
    pub tests: Vec<DiscoveredTest>,
}

/// One upstream test discovered by `--dumptests`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscoveredTest {
    pub path: String,
    pub root: String,
}

/// Machine-readable parse/compile/execute report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunReport {
    pub schema_version: String,
    pub commit: String,
    pub timestamp: String,
    pub perl_ref: String,
    pub prepared_tree: String,
    pub run_tree: String,
    pub host_perl: String,
    pub runner: HarnessRunner,
    pub mode: HarnessMode,
    pub profile: HarnessProfile,
    pub harness_status: Option<i32>,
    pub summary: RunSummary,
    pub buckets: BTreeMap<String, usize>,
    pub file_results: Vec<RunFileResult>,
    pub failures: Vec<RunFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunSummary {
    pub files_total: usize,
    pub files_passed: usize,
    pub files_failed: usize,
    pub tap_assertions_total: usize,
    pub tap_assertions_passed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunFileResult {
    pub path: String,
    pub status: RunnerStatus,
    pub assertions_passed: usize,
    pub assertions_total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunFailure {
    pub path: String,
    pub phase: String,
    pub bucket: String,
    pub first_diagnostic: String,
    pub workstream: String,
    pub lsp_impact: Vec<String>,
}

/// Checked-in baseline for a Perl core harness run report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompileBaseline {
    pub schema_version: String,
    pub report_schema_version: String,
    pub mode: HarnessMode,
    pub profile: HarnessProfile,
    pub files_total: usize,
    pub files_passed: usize,
    pub files_failed: usize,
    pub tap_assertions_total: usize,
    pub tap_assertions_passed: usize,
    pub buckets: BTreeMap<String, usize>,
    pub expected_failures: Vec<RunFailure>,
    pub file_results: Vec<RunFileResult>,
}

/// Preparation receipt for an upstream Perl source tree.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrepareReceipt {
    pub schema_version: String,
    pub requested_ref: String,
    pub resolved_ref: Option<String>,
    pub source_url: String,
    pub source_dir: String,
    pub prepared_tree: String,
    pub host_os: String,
    pub host_arch: String,
    pub configure_command: String,
    pub test_prep_command: String,
    pub status: PrepareStatus,
    pub first_error: Option<String>,
    pub started_at: String,
    pub finished_at: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrepareStatus {
    Pass,
    Fail,
}

/// Manual/advisory smoke summary for a real prepared Perl tree.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SmokeReport {
    pub schema_version: String,
    pub timestamp: String,
    pub repo_commit: String,
    pub perl_requested_ref: String,
    pub perl_resolved_ref: String,
    pub prepared_tree: String,
    pub host_perl: String,
    pub runner: HarnessRunner,
    pub profile: HarnessProfile,
    pub modes_requested: Vec<HarnessMode>,
    pub discovery_report: String,
    pub parse_report: Option<String>,
    pub compile_report: Option<String>,
    pub gap_map: String,
    pub discovery_total: usize,
    pub parse_files_total: Option<usize>,
    pub parse_files_passed: Option<usize>,
    pub parse_files_failed: Option<usize>,
    pub compile_files_total: Option<usize>,
    pub compile_files_passed: Option<usize>,
    pub compile_files_failed: Option<usize>,
    pub parse_buckets: Option<BTreeMap<String, usize>>,
    pub compile_buckets: Option<BTreeMap<String, usize>>,
    pub status: SmokeStatus,
    pub structural_failures: Vec<SmokeStructuralFailure>,
}

/// Bucketed gap map generated by real-tree smoke receipts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GapMap {
    pub schema_version: String,
    pub profile: HarnessProfile,
    pub mode: String,
    pub total_files: usize,
    pub passed_files: usize,
    pub failed_files: usize,
    pub buckets: BTreeMap<String, usize>,
    pub files_by_bucket: BTreeMap<String, Vec<String>>,
    pub first_failure_by_bucket: BTreeMap<String, RunFailure>,
    pub workstreams: BTreeMap<String, usize>,
    pub lsp_impact: BTreeMap<String, usize>,
    pub top_parse_failures: Vec<RunFailure>,
    pub top_compile_failures: Vec<RunFailure>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmokeStatus {
    Pass,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SmokeStructuralFailure {
    pub mode: Option<HarnessMode>,
    pub path: Option<String>,
    pub kind: SmokeFailureKind,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmokeFailureKind {
    MissingReport,
    ProfileMismatch,
    UnbucketedFailure,
    UnknownBucket,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BaselineComparison {
    pub violations: Vec<BaselineViolation>,
}

impl BaselineComparison {
    pub fn is_clean(&self) -> bool {
        self.violations.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BaselineViolation {
    pub kind: BaselineViolationKind,
    pub path: Option<String>,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BaselineViolationKind {
    SchemaMismatch,
    ModeMismatch,
    ProfileMismatch,
    PreviouslyPassingFileFailed,
    UnexpectedNewFailure,
    UnknownBucket,
    UnbucketedFailure,
    BucketCountIncreased,
    MissingExpectedFile,
    AssertionRegression,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunnerStatus {
    Pass,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunnerRecord {
    pub schema_version: String,
    pub mode: String,
    pub path: String,
    pub status: RunnerStatus,
    pub assertions_passed: usize,
    pub assertions_total: usize,
    pub bucket: Option<String>,
    pub first_diagnostic: Option<String>,
}

pub fn workstream_for_bucket(bucket: &str) -> &'static str {
    match bucket {
        "parse_recovery" => "parser_recovery",
        "source_decode" => "source_loading",
        "hir_lowering" => "hir",
        "compile_effect" => "compile_time_effects",
        "scope_pad" => "scope_and_pad",
        "package_stash" => "package_stash",
        "pragma_feature" => "pragma_model",
        "module_resolution" => "module_resolution",
        "runtime_value_model" => "runtime_value_model",
        "cli_switch" => "harness_cli_compat",
        "harness_prepare" => "harness_integration",
        "unknown" => "compiler_conformance",
        _ => "compiler_conformance",
    }
}

pub fn lsp_impact_for_bucket(bucket: &str) -> Vec<&'static str> {
    match bucket {
        "parse_recovery" => vec!["diagnostics", "syntax_tree", "semantic_tokens"],
        "source_decode" => vec!["workspace_index", "diagnostics"],
        "hir_lowering" => vec!["definition", "rename", "diagnostics"],
        "compile_effect" => vec!["definition", "references", "diagnostics"],
        "scope_pad" => vec!["rename", "definition", "diagnostics"],
        "package_stash" => vec!["workspace_symbols", "completion", "definition"],
        "pragma_feature" => vec!["diagnostics", "semantic_tokens"],
        "module_resolution" => vec!["definition", "hover", "completion"],
        "runtime_value_model" => vec!["compiler_conformance"],
        "cli_switch" | "harness_prepare" => vec!["compiler_conformance"],
        _ => vec!["compiler_conformance"],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_constants_are_stable() {
        assert_eq!(DISCOVERY_SCHEMA_VERSION, "perl_core_harness.discovery.v1");
        assert_eq!(RUN_REPORT_SCHEMA_VERSION, "perl_core_harness.report.v1");
        assert_eq!(COMPILE_BASELINE_SCHEMA_VERSION, "perl_core_harness.compile_baseline.v1");
        assert_eq!(SMOKE_SCHEMA_VERSION, "perl_core_harness.smoke.v1");
        assert_eq!(PREPARE_SCHEMA_VERSION, "perl_core_harness.prepare.v1");
        assert_eq!(GAP_MAP_SCHEMA_VERSION, "perl_core_harness.gap_map.v1");
        assert_eq!(RUNNER_RECORD_SCHEMA_VERSION, "perl_core_harness.runner_record.v1");
    }

    #[test]
    fn enum_display_and_roots_match_harness_contract() {
        assert_eq!(HarnessRunner::Test.script_name(), "TEST");
        assert_eq!(HarnessRunner::Test.as_str(), "test");
        assert_eq!(HarnessRunner::Harness.script_name(), "harness");
        assert_eq!(HarnessRunner::Harness.as_str(), "harness");

        assert_eq!(HarnessMode::Parse.as_str(), "parse");
        assert_eq!(HarnessMode::Compile.as_str(), "compile");
        assert_eq!(HarnessMode::Execute.as_str(), "execute");

        assert_eq!(HarnessProfile::Base.as_str(), "base");
        assert_eq!(HarnessProfile::Base.roots(), &["base"]);
        assert!(HarnessProfile::Core.roots().contains(&"comp"));
    }

    #[test]
    fn bucket_mappings_remain_behavior_oriented() {
        assert_eq!(workstream_for_bucket("source_decode"), "source_loading");
        assert_eq!(workstream_for_bucket("hir_lowering"), "hir");
        assert_eq!(workstream_for_bucket("compile_effect"), "compile_time_effects");
        assert_eq!(workstream_for_bucket("scope_pad"), "scope_and_pad");
        assert_eq!(workstream_for_bucket("package_stash"), "package_stash");
        assert_eq!(workstream_for_bucket("pragma_feature"), "pragma_model");
        assert_eq!(workstream_for_bucket("module_resolution"), "module_resolution");
        assert_eq!(workstream_for_bucket("runtime_value_model"), "runtime_value_model");
        assert_eq!(workstream_for_bucket("cli_switch"), "harness_cli_compat");
        assert_eq!(workstream_for_bucket("harness_prepare"), "harness_integration");
        assert_eq!(workstream_for_bucket("unknown_bucket"), "compiler_conformance");
        assert_eq!(lsp_impact_for_bucket("source_decode"), vec!["workspace_index", "diagnostics"]);
        assert_eq!(
            lsp_impact_for_bucket("compile_effect"),
            vec!["definition", "references", "diagnostics"]
        );
        assert_eq!(
            lsp_impact_for_bucket("module_resolution"),
            vec!["definition", "hover", "completion"]
        );
        assert_eq!(lsp_impact_for_bucket("runtime_value_model"), vec!["compiler_conformance"]);
        assert_eq!(lsp_impact_for_bucket("cli_switch"), vec!["compiler_conformance"]);
        assert_eq!(lsp_impact_for_bucket("harness_prepare"), vec!["compiler_conformance"]);
        assert_eq!(lsp_impact_for_bucket("unknown_bucket"), vec!["compiler_conformance"]);
    }

    #[test]
    fn runner_record_roundtrips() -> Result<(), Box<dyn std::error::Error>> {
        let record = RunnerRecord {
            schema_version: RUNNER_RECORD_SCHEMA_VERSION.to_string(),
            mode: "compile".to_string(),
            path: "base/ok.t".to_string(),
            status: RunnerStatus::Pass,
            assertions_passed: 1,
            assertions_total: 1,
            bucket: None,
            first_diagnostic: None,
        };

        let encoded = serde_json::to_string(&record)?;
        let decoded: RunnerRecord = serde_json::from_str(&encoded)?;
        assert_eq!(decoded, record);
        Ok(())
    }
}
