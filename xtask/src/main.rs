//! Xtask automation for tree-sitter-perl
//!
//! This binary provides custom automation tasks for building, testing,
//! and maintaining the tree-sitter-perl project.

use clap::{Parser, Subcommand, ValueEnum};
use color_eyre::eyre::Result;
use std::path::PathBuf;

mod tasks;
mod types;
mod utils;
use tasks::check_test_wiring;
use tasks::dead_code::{DeadCodeConfig, DeadCodeMode};
use tasks::gates::{GateTier, OutputFormat};
use tasks::metrics;
use tasks::targeted_checks::CheckMode;
use tasks::unwired_scan::UnwiredScanConfig;
use tasks::*;
use types::TestSuite;
#[cfg(any(feature = "legacy", feature = "parser-tasks"))]
use types::*;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Custom tasks for tree-sitter-perl")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run lean CI suite (format, clippy, tests) for constrained environments
    Ci,

    /// Run format and clippy checks only (no tests)
    CheckOnly,

    /// Verify local Rust toolchain meets the pinned MSRV in rust-toolchain.toml.
    CheckToolchain {
        /// Show a warning when rustc satisfies the minimum MSRV but differs
        /// from the exact pinned channel string.
        #[arg(long)]
        doctor: bool,
    },

    /// Build project with various configurations
    Build {
        /// Build in release mode
        #[arg(long)]
        release: bool,

        /// Build with specific features
        #[arg(long, value_delimiter = ',')]
        features: Option<Vec<String>>,

        /// Build only C scanner
        #[arg(long)]
        c_scanner: bool,

        /// Build only Rust scanner
        #[arg(long)]
        rust_scanner: bool,
    },

    /// Run tests with various configurations
    Test {
        /// Run tests in release mode
        #[arg(long)]
        release: bool,

        /// Run specific test suite
        #[arg(long, value_enum)]
        suite: Option<TestSuite>,

        /// Run tests with specific features
        #[arg(long, value_delimiter = ',')]
        features: Option<Vec<String>>,

        /// Run tests with verbose output
        #[arg(long)]
        verbose: bool,

        /// Run tests with coverage
        #[arg(long)]
        coverage: bool,
    },

    /// Run benchmarks
    Bench {
        /// Run specific benchmark
        #[arg(long)]
        name: Option<String>,

        /// Save benchmark results
        #[arg(long)]
        save: bool,

        /// Output file for results
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Run C vs Rust benchmark comparison
    Compare {
        /// Run only C implementation benchmarks
        #[arg(long)]
        c_only: bool,

        /// Run only Rust implementation benchmarks
        #[arg(long)]
        rust_only: bool,

        /// Run scanner comparison only
        #[arg(long)]
        scanner_only: bool,

        /// Validate existing results only
        #[arg(long)]
        validate_only: bool,

        /// Output directory for results
        #[arg(long, default_value = "benchmark_results")]
        output_dir: PathBuf,

        /// Check performance gates
        #[arg(long)]
        check_gates: bool,

        /// Generate detailed report
        #[arg(long)]
        report: bool,
    },

    /// Run the benchmark script wrapper (`benchmarks/scripts/run-benchmarks.sh`).
    BenchRun {
        /// Write benchmark results to a JSON file.
        #[arg(long)]
        output: Option<PathBuf>,

        /// Run quick smoke benchmarks with reduced sample size.
        #[arg(long)]
        quick: bool,

        /// Restrict benchmarks to a specific category.
        #[arg(long)]
        category: Option<String>,
    },

    /// Compare benchmark output receipts (`benchmarks/scripts/compare.sh`).
    BenchCompare {
        /// Enable strict mode (exit non-zero on regression).
        #[arg(long)]
        fail_on_regression: bool,
    },

    /// Format benchmark JSON via `benchmarks/scripts/format-results.py`.
    BenchFormat {
        /// Emit a receipt summary for CI.
        #[arg(long)]
        receipt: bool,

        /// Emit markdown summary.
        #[arg(long)]
        markdown: bool,
    },

    /// Extract and normalize Criterion benchmark outputs (`target/criterion/.../estimates.json`).
    BenchExtract {
        /// Root path that contains `target/criterion`.
        #[arg(long)]
        base_path: Option<PathBuf>,

        /// Output JSON path.
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Run benchmark alert checks (`benchmarks/scripts/alert.py`).
    BenchAlert {
        /// Output markdown alerts.
        #[arg(long)]
        format: Option<String>,

        /// Run checks and fail on warning conditions.
        #[arg(long)]
        check: bool,
    },

    /// Run the local benchmark alert regression test suite.
    BenchAlertTest,

    /// Generate Homebrew formula and VS Code asset map from checksums JSON.
    InjectShaAssets {
        /// Version tag used by release artifacts (e.g. v0.8.3).
        #[arg(long)]
        version: String,

        /// GitHub organization owning the release repository.
        #[arg(long)]
        owner: String,

        /// GitHub repository name for releases.
        #[arg(long)]
        repo: String,

        /// Artifact prefix for release filenames.
        #[arg(long)]
        prefix: String,

        /// Path to checksums JSON from cargo-dist.
        #[arg(long)]
        checksums: PathBuf,

        /// Optional output path for generated Homebrew formula.
        #[arg(long)]
        brew_out: Option<PathBuf>,

        /// Optional output path for generated VS Code extension asset map.
        #[arg(long)]
        asset_map_out: Option<PathBuf>,
    },

    /// Generate Homebrew formula from a release SHA256SUMS file.
    UpdateHomebrew {
        /// Release version tag used by release artifacts (e.g. v0.8.3).
        #[arg(long)]
        version: String,

        /// GitHub organization owning the release repository.
        #[arg(long, default_value = "EffortlessMetrics")]
        owner: String,

        /// GitHub repository name for releases.
        #[arg(long, default_value = "perl-lsp")]
        repo: String,

        /// Artifact prefix for release filenames.
        #[arg(long, default_value = "perl-lsp")]
        prefix: String,

        /// Output path for generated Homebrew formula.
        #[arg(long, default_value = "homebrew/perl-lsp.rb")]
        output: PathBuf,
    },

    /// Generate documentation
    Doc {
        /// Open docs in browser
        #[arg(long)]
        open: bool,

        /// Build docs for all features
        #[arg(long)]
        all_features: bool,
    },

    /// Run code quality checks
    Check {
        /// Run clippy
        #[arg(long)]
        clippy: bool,

        /// Run formatting check
        #[arg(long)]
        fmt: bool,

        /// Run all checks
        #[arg(long)]
        all: bool,
    },

    /// Format code
    Fmt {
        /// Check formatting without making changes
        #[arg(long)]
        check: bool,
    },

    /// Run corpus tests
    #[cfg(feature = "legacy")]
    Corpus {
        /// Path to corpus directory
        #[arg(long, default_value = "tree-sitter-perl/test/corpus")]
        path: PathBuf,

        /// Run with specific scanner
        #[arg(long, value_enum)]
        scanner: Option<ScannerType>,

        /// Run diagnostic analysis on first failing test
        #[arg(long)]
        diagnose: bool,

        /// Test current parser behavior with simple expressions
        #[arg(long)]
        test: bool,
    },

    /// Run highlight tests
    #[cfg(feature = "parser-tasks")]
    Highlight {
        /// Path to highlight test directory
        #[arg(long, default_value = "c/test/highlight")]
        path: PathBuf,

        /// Run with specific scanner
        #[arg(long, value_enum)]
        scanner: Option<ScannerType>,
    },

    /// Clean build artifacts
    Clean {
        /// Clean all artifacts including target
        #[arg(long)]
        all: bool,
    },

    /// Detect dead code, unused dependencies, and unused imports
    ///
    /// Combines cargo-machete/cargo-udeps with clippy dead_code lints.
    /// Supports check (against baseline), baseline generation, and JSON report modes.
    DeadCode {
        /// Mode: check (default), baseline, or report
        #[arg(value_enum, default_value = "check")]
        mode: DeadCodeMode,

        /// Strict mode: fail on any regression above baseline
        #[arg(long)]
        strict: bool,
    },

    /// Run a developer environment smoke check.
    DevexDoctor,

    /// Audit CI workflows for PR-safety and spend-risk controls.
    CiAuditWorkflows,

    /// Measure CI lane runtimes and emit timing artifacts.
    CiMeasure,

    /// Analyze GitHub Actions costs over a recent period.
    CiCostMonitor {
        /// Number of days to analyze.
        #[arg(long, default_value_t = 30)]
        days: u64,

        /// Emit machine-readable output.
        #[arg(long)]
        json: bool,
    },

    /// Measure CI baseline from recent workflow runs.
    CiBaseline {
        /// Branch to analyze.
        #[arg(short, long, default_value = "master")]
        branch: String,

        /// Number of days to analyze.
        #[arg(short, long, default_value_t = 30)]
        days: u64,

        /// Max runs to fetch.
        #[arg(short, long, default_value_t = 200)]
        limit: usize,

        /// Output directory for ci_baseline artifacts.
        #[arg(short, long, default_value = ".ci")]
        output: PathBuf,
    },

    /// Run version-sync checks from `perl-ci-hygiene`.
    CheckVersionSync,

    /// Check for disallowed direct `ExitStatus::from_raw()` usage.
    CheckFromRaw,

    /// Run production security hardening checks.
    SecurityHardening,

    /// Run production performance hardening checks.
    PerformanceHardening,

    /// Validate production hardening gate posture and SLOs.
    ProductionGatesValidation,

    /// Harvest forensics data for a merged PR.
    ForensicsHarvest {
        /// PR number or identifier.
        pr: String,
    },

    /// Analyze temporal behavior for a merged PR.
    ForensicsTemporal {
        /// PR number or identifier.
        pr: String,
    },

    /// Run quick static telemetry for a merged PR.
    ForensicsTelemetryQuick {
        /// PR number or identifier.
        pr: String,
    },

    /// Run full static telemetry for a merged PR.
    ForensicsTelemetryFull {
        /// PR number or identifier.
        pr: String,
    },

    /// Generate a full forensics dossier for a merged PR.
    ForensicsDossier {
        /// PR number or identifier.
        pr: String,
    },

    /// Render a forensics dossier for a merged PR.
    ForensicsRender {
        /// PR number or identifier.
        pr: String,

        /// Output format for the rendered dossier (`full` or `summary`).
        #[arg(default_value = "full")]
        format: String,
    },

    /// Verify publication claims from `docs/project/PUBLICATION_FACTS_LEDGER.md`.
    VerifyPublicationFacts {
        /// Forward extra args to the checker (`--strict`, `--json`).
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Ensure issue labels are present and correctly configured in GitHub.
    GhLabels,

    /// Show open issues missing required taxonomy labels from GitHub.
    GhTriage {
        /// Maximum number of issues to list.
        #[arg(default_value = "500")]
        limit: usize,
    },

    /// Backfill prefixed labels on GitHub issues (dry run by default).
    GhBackfillPrefixedLabels {
        /// Apply label updates instead of dry run.
        #[arg(long)]
        apply: bool,
    },

    /// Generate bindings
    #[cfg(feature = "parser-tasks")]
    Bindings {
        /// Header file to generate bindings from
        #[arg(long, default_value = "archive/crates/tree-sitter-perl-rs/src/tree_sitter/parser.h")]
        header: PathBuf,

        /// Output file for bindings
        #[arg(long, default_value = "archive/crates/tree-sitter-perl-rs/src/bindings.rs")]
        output: PathBuf,
    },

    /// Run development server
    Dev {
        /// Watch for changes
        #[arg(long)]
        watch: bool,

        /// Port for development server
        #[arg(long, default_value = "8080")]
        port: u16,
    },

    /// Run pure Rust parser
    ParseRust {
        /// Source file to parse
        source: PathBuf,

        /// Output S-expression
        #[arg(long)]
        sexp: bool,

        /// Output AST debug format
        #[arg(long)]
        ast: bool,

        /// Benchmark parsing time
        #[arg(long)]
        bench: bool,
    },

    /// Prepare release
    Release {
        /// Version to release
        version: String,

        /// Skip confirmation
        #[arg(long)]
        yes: bool,
    },

    /// Trigger PR-driven release orchestration workflow
    ReleaseTurnkey {
        /// Release version (preferred: use `--version`; positional is also accepted).
        #[arg(long)]
        version: Option<String>,

        /// Release version as positional argument.
        #[arg(value_name = "VERSION")]
        positional_version: Option<String>,

        /// Trigger prerelease mode for workflows.
        #[arg(long)]
        prerelease: bool,

        /// Validate commands only; do not trigger workflows.
        #[arg(long)]
        dry_run: bool,

        /// Skip crates.io publish workflow.
        #[arg(long)]
        skip_crates: bool,

        /// Skip VSCode extension publish workflow.
        #[arg(long)]
        skip_extension: bool,

        /// Skip Docker image publish workflow.
        #[arg(long)]
        skip_docker: bool,

        /// Base branch for release orchestration.
        #[arg(long)]
        base_branch: Option<String>,

        /// Do not auto-merge the version bump PR.
        #[arg(long)]
        no_auto_merge: bool,

        /// Do not wait for the version bump PR merge.
        #[arg(long)]
        no_wait_pr_merge: bool,

        /// Do not wait for release workflows to finish.
        #[arg(long)]
        no_wait_release: bool,

        /// Workflow wait timeout in seconds.
        #[arg(long)]
        workflow_timeout: Option<u64>,
    },

    /// Run crates.io launch-preparation checks.
    PrepCratesIoLaunch {
        /// Launch mode: `core` for launch-critical crates, `all` for all publishable crates.
        #[arg(long, value_enum, default_value = "core")]
        mode: PrepCratesMode,
    },

    /// Run heredoc-specific tests
    TestHeredoc {
        /// Run tests in release mode
        #[arg(long)]
        release: bool,

        /// Run tests with verbose output
        #[arg(long)]
        verbose: bool,
    },

    /// Test edge case handling functionality
    TestEdgeCases {
        /// Run benchmarks
        #[arg(long)]
        bench: bool,

        /// Generate coverage report
        #[arg(long)]
        coverage: bool,

        /// Run specific edge case test
        #[arg(long)]
        test: Option<String>,
    },

    /// Run corpus audit for coverage analysis
    CorpusAudit {
        /// Path to corpus directory
        #[arg(long, default_value = ".")]
        corpus_path: PathBuf,

        /// Output path for audit report
        #[arg(long, default_value = "corpus_audit_report.json")]
        output: PathBuf,

        /// Check mode for CI (fails if issues found)
        #[arg(long)]
        check: bool,

        /// Fresh mode (regenerate report even if it exists)
        #[arg(long)]
        fresh: bool,
    },

    /// Generate parser feature matrix from a parser-audit report.
    ParserMatrix {
        /// Path to parser audit report JSON.
        #[arg(long, default_value = "corpus_audit_report.json")]
        report: PathBuf,

        /// Output path for generated matrix documentation.
        #[arg(long, default_value = "docs/reference/PARSER_FEATURE_MATRIX.md")]
        output: PathBuf,
    },

    /// Run three-way parser comparison
    #[cfg(feature = "parser-tasks")]
    CompareThree {
        /// Show detailed output
        #[arg(long)]
        verbose: bool,

        /// Output format (table, json, markdown)
        #[arg(long, default_value = "table")]
        format: String,
    },

    /// Test LSP features with demo scripts
    TestLsp {
        /// Create test files only (don't run tests)
        #[arg(long)]
        create_only: bool,

        /// Run specific test
        #[arg(long)]
        test: Option<String>,

        /// Clean up test files after running
        #[arg(long)]
        cleanup: bool,
    },

    /// Bump the workspace version across every tracked site.
    ///
    /// Non-interactive and idempotent. Delegates to `perl-ci-hygiene
    /// bump-version`, which owns the canonical site list shared with the
    /// `check-version-sync` CI gate.
    BumpVersion {
        /// New version to set (X.Y.Z format).
        version: String,
    },

    /// Publish crates to crates.io
    PublishCrates {
        /// Skip confirmation
        #[arg(long)]
        yes: bool,

        /// Dry run (don't actually publish)
        #[arg(long)]
        dry_run: bool,
    },

    /// Dispatch the "Publish to crates.io" workflow for a release
    PublishRelease {
        /// Release version (for example 0.x.y)
        version: String,

        /// Dry run (don't actually publish)
        #[arg(long)]
        dry_run: bool,

        /// Target git ref (defaults to v<version>)
        #[arg(long = "ref")]
        git_ref: Option<String>,
    },

    /// Run a full release smoke test via installed binaries
    SmokeTestRelease {
        /// Release version to smoke-test (for example 0.x.y)
        version: String,
    },

    /// Run forbidden-fatal construct checks from `perl-ci-hygiene`.
    ForbidFatalConstructs {
        /// Forwarded arguments for `forbid-fatal-constructs`.
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Run arbitrary `perl-ci-hygiene` subcommands.
    CiHygiene {
        /// Subcommand name for `perl-ci-hygiene`.
        command: String,

        /// Arguments to pass to the subcommand.
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Publish a review receipt bundle in `review/receipts/YYYY-MM-DD/`.
    PublishReceipts {
        /// Optional date override in `YYYY-MM-DD` format.
        date: Option<String>,
    },

    /// Publish VSCode extension to marketplace
    PublishVscode {
        /// Skip confirmation
        #[arg(long)]
        yes: bool,

        /// PAT token for authentication
        #[arg(long)]
        token: Option<String>,
    },

    /// Sweep system Perl corpus for parser error rates
    ParserCorpusSweep {
        /// Comma-separated corpus root directories
        #[arg(long, value_delimiter = ',', conflicts_with = "manifest")]
        roots: Option<Vec<PathBuf>>,

        /// Manifest file listing module names to resolve via perl
        #[arg(long, conflicts_with = "roots")]
        manifest: Option<PathBuf>,

        /// Write JSON report to file
        #[arg(long)]
        output: Option<PathBuf>,

        /// Compare against baseline JSON file
        #[arg(long)]
        baseline: Option<PathBuf>,

        /// Return nonzero if regression detected
        #[arg(long)]
        enforce: bool,

        /// Include per-file details in output
        #[arg(long)]
        verbose: bool,

        /// Write receipt JSON to target/receipts/corpus-sweep.json
        #[arg(long)]
        receipt: bool,
    },

    /// Manage CPAN top-1000 corpus acquisition, sweep, and ratchet
    CpanCorpus {
        #[command(subcommand)]
        command: CpanCorpusCommand,
    },

    /// Generate canonical receipts (test summary, doc metrics, consolidated state)
    ///
    /// Runs workspace tests and doc builds, parses output, and produces
    /// JSON artifacts in the artifacts/ directory. Replaces scripts/generate-receipts.sh.
    Receipts {
        /// Only generate test receipts (skip doc build)
        #[arg(long)]
        tests_only: bool,

        /// Only generate doc receipts (skip test run)
        #[arg(long)]
        docs_only: bool,

        /// Output directory for artifacts (default: artifacts/)
        #[arg(long)]
        output_dir: Option<PathBuf>,

        /// Number of test threads (default: 2)
        #[arg(long, default_value = "2")]
        test_threads: u32,
    },

    /// Track ignored tests and enforce gate policy
    IgnoredTests {
        /// Write current counts back to baseline
        #[arg(long)]
        update: bool,
        /// CI gate mode: fail when ignored count increases
        #[arg(long)]
        check: bool,
        /// Print detailed per-category breakdown
        #[arg(long, short)]
        verbose: bool,
    },

    /// Show technical debt report from debt ledger
    ///
    /// Reads `.ci/debt-ledger.yaml` and reports on quarantined tests,
    /// known issues, and technical debt items with budget tracking.
    DebtReport {
        /// CI gate mode: exit 1 if over budget or expired quarantines
        #[arg(long)]
        check: bool,

        /// Output JSON format for receipt integration
        #[arg(long)]
        json: bool,

        /// Output a compact markdown summary table.
        #[arg(long)]
        summary: bool,

        /// Show only expired quarantines
        #[arg(long)]
        expired: bool,

        /// Path to debt ledger (default: .ci/debt-ledger.yaml)
        #[arg(long)]
        ledger: Option<PathBuf>,
    },

    /// Check invariants in features.toml
    DocClaims,

    /// Manage feature catalog and LSP compliance
    Features {
        #[command(subcommand)]
        command: FeaturesCommand,
    },

    /// Update derived metrics in docs/project/status/ subsystem files.
    ///
    /// Computes workspace test counts, ignored test counts, feature catalog
    /// metrics from features.toml, corpus statistics, and missing-docs
    /// warnings, then patches the markdown files between fenced markers.
    ///
    /// Subsystem files: docs/project/status/{lsp,tests,parser,quality}.md
    UpdateStatus {
        /// Write updates back to docs/project/status/
        #[arg(long)]
        write: bool,

        /// Check whether docs are up-to-date (CI gate); exit non-zero if stale
        #[arg(long)]
        check: bool,

        /// Only regenerate one subsystem (lsp, tests, parser, quality).
        /// When omitted, all four subsystems are regenerated.
        #[arg(long, value_enum)]
        only: Option<update_status::StatusSubsystem>,
    },

    /// Generate SRP microcrate inventory and split-candidate report
    SrpMicrocrates {
        /// Optional output path (default: docs/SRP_MICROCRATES.md)
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Scan for built-but-not-wired crates: those with tests but zero import by perl-lsp
    ///
    /// Finds crates that have `#[test]` annotations but are not listed as direct
    /// dependencies of `perl-lsp`. Also surfaces TODO/FIXME wiring comments.
    /// Use `--check` to make CI fail when unwired crates are found.
    UnwiredScan {
        /// Emit JSON to stdout instead of human-readable output
        #[arg(long)]
        json: bool,

        /// Exit 1 if any unwired crates are found (CI gate mode)
        #[arg(long)]
        check: bool,

        /// Name of the root LSP crate to check (default: perl-lsp-rs)
        #[arg(long, default_value = "perl-lsp-rs")]
        lsp_crate: String,
    },

    /// Check that test-bearing Rust files are reachable from their module tree.
    CheckTestWiring,

    /// Emit per-subsystem engineering-health metrics.
    Metrics {
        #[command(subcommand)]
        command: MetricsCommand,
    },

    /// Validate memory profiling functionality
    ValidateMemoryProfiler,

    /// Run end-to-end validation sweep
    ///
    /// Tests core crates in release mode, runs a large workspace smoke
    /// test against the LSP server, checks benchmark compilation, and
    /// produces an optional JSON report.
    E2eValidate {
        /// Number of Perl files to generate for the workspace smoke test
        #[arg(long, default_value = "200")]
        workspace_size: usize,

        /// Write a JSON report to this path
        #[arg(long)]
        report: Option<PathBuf>,

        /// Skip the large-workspace smoke test
        #[arg(long)]
        skip_workspace: bool,

        /// Skip the benchmark compilation check
        #[arg(long)]
        skip_bench: bool,

        /// Show verbose output from test runs
        #[arg(long, short)]
        verbose: bool,
    },

    /// Run CI gates with receipt generation
    ///
    /// Executes gates defined in .ci/gate-policy.yaml and generates
    /// machine-readable receipts for tracking and comparison.
    Gates {
        /// Gate tier to run (default: merge-gate)
        #[arg(long, short, value_enum, default_value = "merge-gate")]
        tier: GateTier,

        /// Run a specific gate by name
        #[arg(long, short)]
        gate: Option<String>,

        /// List available gates without running them
        #[arg(long, short)]
        list: bool,

        /// Output format (default: human)
        #[arg(long, short, value_enum, default_value = "human")]
        format: OutputFormat,

        /// Emit receipt JSON (also writes to target/receipts/receipt.json)
        #[arg(long, short)]
        receipt: bool,

        /// Path to write receipt (default: target/receipts/receipt.json)
        #[arg(long)]
        receipt_path: Option<PathBuf>,

        /// Compare against a baseline receipt JSON
        #[arg(long, short)]
        diff: Option<PathBuf>,

        /// Stop on first failure (fail-fast mode)
        #[arg(long)]
        fail_fast: bool,

        /// Run gates in parallel where safe (experimental)
        #[arg(long)]
        parallel: bool,

        /// Verbose output (include quarantined gates)
        #[arg(long, short)]
        verbose: bool,
    },

    /// Verify hook scripts are executable.
    HookCheck,

    /// Verify hook registry references are present and executable.
    HookRegistryCheck,

    /// Run hook behavior tests and output summaries.
    HookTests,

    /// Run targeted clippy/test checks for crates changed since a base ref
    ///
    /// Detects which crates have changed since the given base git ref
    /// and runs clippy and/or tests only for those crates. This gives
    /// fast feedback during active development.
    TargetedChecks {
        /// Base git reference for diff (default: origin/master)
        #[arg(long, default_value = "origin/master")]
        base: String,

        /// Check mode: clippy, test, or all (default: all)
        #[arg(long, value_enum, default_value = "all")]
        mode: CheckMode,
    },

    /// Remove stale `.claude/worktrees` entries and prune Git metadata.
    WorktreeCleanup,

    /// Show summary statistics from swarm-metrics.jsonl.
    SwarmSummary {
        /// Path to operations directory (defaults to `.ops-perl-lsp`).
        #[arg(default_value = ".ops-perl-lsp")]
        ops_dir: PathBuf,

        /// Summarize only entries at or after the given window, e.g. `24h`, `7d`, `30m`, or `all`.
        #[arg(long)]
        since: Option<String>,

        /// Maximum number of rows to show in each summary section.
        #[arg(long, default_value_t = 10)]
        limit: usize,

        /// Output format for the swarm summary.
        #[arg(long, value_enum, default_value = "human")]
        format: swarm_summary::SwarmSummaryOutputFormat,
    },

    /// Populate mdBook source directory from `docs/`.
    PopulateBook,

    /// Validate workspace exclusion strategy and dependency invariants.
    ValidateWorkspaceExclusions,

    /// Generate a build-timing receipt JSON with workspace duration metrics.
    BuildTimingReceipt {
        /// Measure clean build with `cargo build --workspace --locked`.
        #[arg(long)]
        clean: bool,

        /// Measure incremental rebuild using incremental crate touch.
        #[arg(long)]
        incremental: bool,

        /// Measure test build with `cargo test --workspace --lib --locked`.
        #[arg(long)]
        tests: bool,

        /// Output file for the generated receipt.
        #[arg(long)]
        output: Option<PathBuf>,

        /// Write the baseline artifact (`artifacts/build-timing-baseline.json`).
        #[arg(long)]
        baseline: bool,
    },

    /// Compare two build-timing receipts and print a markdown report.
    CompareBuildTiming {
        /// Baseline receipt JSON path.
        baseline: PathBuf,
        /// Current receipt JSON path.
        current: PathBuf,
    },
}

#[derive(Subcommand)]
enum CpanCorpusCommand {
    /// Fetch top N distributions from MetaCPAN by reverse dependency count
    FetchList {
        /// Number of distributions to fetch (default: 1000)
        #[arg(long, default_value = "1000")]
        top_n: usize,

        /// Output path for distribution list
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Install distributions from the list via cpanm
    Install {
        /// Path to distribution list file
        #[arg(long)]
        dist_list: Option<PathBuf>,

        /// Local install directory
        #[arg(long)]
        install_dir: Option<PathBuf>,

        /// Verbose output
        #[arg(long)]
        verbose: bool,

        /// Force a full wipe of the install directory before installing.
        /// Default is an incremental install that keeps `lib/perl5` between
        /// runs and lets cpanm skip already-installed modules.
        #[arg(long)]
        reset: bool,
    },

    /// Run parser corpus sweep against installed CPAN modules
    Sweep {
        /// Write JSON report to file
        #[arg(long)]
        output: Option<PathBuf>,

        /// Return nonzero if regression detected
        #[arg(long)]
        enforce: bool,

        /// Verbose output
        #[arg(long)]
        verbose: bool,

        /// Local install directory containing CPAN modules
        #[arg(long)]
        install_dir: Option<PathBuf>,
    },

    /// Auto-append newly-clean modules to the CPAN manifest
    Ratchet {
        /// Verbose output
        #[arg(long)]
        verbose: bool,

        /// Local install directory containing CPAN modules
        #[arg(long)]
        install_dir: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum FeaturesCommand {
    /// Sync documentation from features.toml
    SyncDocs,

    /// Verify features match capabilities
    Verify,

    /// Run feature catalog invariant checks
    Invariants,

    /// Generate compliance report
    Report,
}

#[derive(Subcommand)]
enum MetricsCommand {
    /// Emit parser phase timings and benchmark summary.
    ParserStats {
        /// Path to benchmark JSON (default: most recent in benchmarks/results/)
        #[arg(long)]
        input: Option<PathBuf>,
        /// Write output to .ci/metrics/parser.json
        #[arg(long)]
        json: bool,
    },
    /// [stub] LSP request latency statistics.
    LspStats,
    /// [stub] Workspace index memory and timing statistics.
    WorkspaceStats,
    /// [stub] Diagnostics accuracy and latency statistics.
    DiagnosticsStats,
    /// [stub] Hierarchical memory breakdown across LSP subsystems.
    Memory,
    /// [stub] Release-health dashboard.
    ReleaseHealth {
        /// Number of days of history to analyze
        #[arg(long, default_value_t = 30)]
        days: u64,
        /// Write output to .ci/metrics/release-health.json
        #[arg(long)]
        json: bool,
    },
}

#[derive(ValueEnum, Clone)]
enum PrepCratesMode {
    Core,
    All,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    match cli.command {
        Commands::Ci => ci::run(),
        Commands::CheckOnly => ci::check_only(),
        Commands::CheckToolchain { doctor } => check_toolchain::run(doctor),
        Commands::Build { release, features, c_scanner, rust_scanner } => {
            build::run(release, features, c_scanner, rust_scanner)
        }
        Commands::Test { release, suite, features, verbose, coverage } => {
            test::run(release, suite, features, verbose, coverage)
        }
        Commands::Bench { name, save, output } => bench::run(name, save, output),
        Commands::BenchRun { output, quick, category } => {
            benchmarks::run_benchmarks(output, quick, category)
        }
        Commands::BenchCompare { fail_on_regression } => {
            benchmarks::compare_benchmarks(fail_on_regression)
        }
        Commands::BenchFormat { receipt, markdown } => {
            benchmarks::format_benchmarks(receipt, markdown)
        }
        Commands::BenchExtract { base_path, output } => {
            benchmarks::extract_criterion(base_path, output)
        }
        Commands::BenchAlert { format, check } => benchmarks::alert_benchmarks(format, check),
        Commands::BenchAlertTest => benchmarks::test_alert_system(),
        Commands::InjectShaAssets {
            version,
            owner,
            repo,
            prefix,
            checksums,
            brew_out,
            asset_map_out,
        } => inject_sha_assets::run(inject_sha_assets::InjectShaAssetsConfig {
            version,
            owner,
            repo,
            prefix,
            checksums,
            brew_out,
            asset_map_out,
        }),
        Commands::UpdateHomebrew { version, owner, repo, prefix, output } => {
            update_homebrew::run(update_homebrew::UpdateHomebrewConfig {
                version,
                owner,
                repo,
                prefix,
                output,
            })
        }
        Commands::Compare {
            c_only,
            rust_only,
            scanner_only,
            validate_only,
            output_dir,
            check_gates,
            report,
        } => compare::run(
            c_only,
            rust_only,
            scanner_only,
            validate_only,
            output_dir,
            check_gates,
            report,
        ),
        Commands::Doc { open, all_features } => doc::run(open, all_features),
        Commands::Check { clippy, fmt, all } => check::run(clippy, fmt, all),
        Commands::Fmt { check } => fmt::run(check),
        #[cfg(feature = "legacy")]
        Commands::Corpus { path, scanner, diagnose, test } => {
            corpus::run(path, scanner, diagnose, test)
        }
        #[cfg(feature = "parser-tasks")]
        Commands::Highlight { path, scanner } => highlight::run(path, scanner),
        Commands::Clean { all } => clean::run(all),
        Commands::DeadCode { mode, strict } => dead_code::run(DeadCodeConfig { mode, strict }),
        #[cfg(feature = "parser-tasks")]
        Commands::Bindings { header, output } => bindings::run(header, output),
        Commands::Dev { watch, port } => dev::run(watch, port),
        Commands::DevexDoctor => devex_doctor::run(),
        Commands::ParseRust { source, sexp, ast, bench } => {
            parse_rust::run(source, sexp, ast, bench)
        }
        Commands::Release { version, yes } => release::run(version, yes),
        Commands::ReleaseTurnkey {
            version,
            positional_version,
            prerelease,
            dry_run,
            skip_crates,
            skip_extension,
            skip_docker,
            base_branch,
            no_auto_merge,
            no_wait_pr_merge,
            no_wait_release,
            workflow_timeout,
        } => release_turnkey::run(release_turnkey::ReleaseTurnkeyConfig {
            version,
            positional_version,
            prerelease,
            dry_run,
            skip_crates,
            skip_extension,
            skip_docker,
            base_branch,
            no_auto_merge,
            no_wait_pr_merge,
            no_wait_release,
            workflow_timeout,
        }),
        Commands::PrepCratesIoLaunch { mode } => {
            prep_crates_io_launch::run(matches!(mode, PrepCratesMode::All))
        }
        Commands::TestHeredoc { release, verbose } => {
            // Run heredoc tests using the test module with heredoc suite
            test::run(
                release,
                Some(TestSuite::Heredoc),
                Some(vec!["pure-rust".to_string()]),
                verbose,
                false,
            )
        }
        Commands::TestEdgeCases { bench, coverage, test } => edge_cases::run(bench, coverage, test),
        Commands::CiAuditWorkflows => ci_audit_workflows::run(),
        Commands::CiMeasure => ci_measure::run(),
        Commands::CiCostMonitor { days, json } => ci_metrics::run_cost_monitor(days, json),
        Commands::CiBaseline { branch, days, limit, output } => {
            ci_metrics::run_ci_baseline(branch, days, limit, output)
        }
        Commands::CheckVersionSync => check_version_sync::run(),
        Commands::CheckFromRaw => ci_policy::check_from_raw(),
        Commands::SecurityHardening => hardening::security_hardening(),
        Commands::PerformanceHardening => hardening::performance_hardening(),
        Commands::ProductionGatesValidation => hardening::production_gates_validation(),
        Commands::ForensicsHarvest { pr } => forensics::run_harvest(&pr),
        Commands::ForensicsTemporal { pr } => forensics::run_temporal(&pr),
        Commands::ForensicsTelemetryQuick { pr } => forensics::run_telemetry_quick(&pr),
        Commands::ForensicsTelemetryFull { pr } => forensics::run_telemetry_full(&pr),
        Commands::ForensicsDossier { pr } => forensics::run_dossier(&pr),
        Commands::ForensicsRender { pr, format } => forensics::run_render(&pr, &format),
        Commands::VerifyPublicationFacts { args } => publication_facts::run(args),
        Commands::GhLabels => github::run_labels(),
        Commands::GhTriage { limit } => github::run_issues_needing_triage(limit),
        Commands::GhBackfillPrefixedLabels { apply } => github::run_backfill_prefixed_labels(apply),
        Commands::CorpusAudit { corpus_path, output, check, fresh } => {
            corpus_audit::run(corpus_audit::AuditConfig {
                corpus_path,
                output_path: output,
                timeout: std::time::Duration::from_secs(30),
                fresh,
                check,
            })
        }
        Commands::ParserMatrix { report, output } => parser_matrix::run_with_paths(report, output),
        #[cfg(feature = "parser-tasks")]
        Commands::CompareThree { verbose, format } => {
            compare_parsers::run_three_way(verbose, format.as_str())
        }
        Commands::TestLsp { create_only, test, cleanup } => {
            test_lsp::run(create_only, test, cleanup)
        }
        Commands::BumpVersion { version } => bump_version::run(version),
        Commands::PublishCrates { yes, dry_run } => publish::publish_crates(yes, dry_run),
        Commands::PublishRelease { version, dry_run, git_ref } => {
            publish::publish_release(version, dry_run, git_ref)
        }
        Commands::HookCheck => hook_checks::run_hook_check(),
        Commands::HookRegistryCheck => hook_checks::run_hook_registry_check(),
        Commands::HookTests => hook_checks::run_hook_tests(),
        Commands::ForbidFatalConstructs { args } => forbid_fatal_constructs::run(args),
        Commands::CiHygiene { command, args } => ci_hygiene::run(command, args),
        Commands::PublishVscode { yes, token } => publish::publish_vscode(yes, token),
        Commands::SmokeTestRelease { version } => publish::smoke_test_release(version),
        Commands::PublishReceipts { date } => publish_receipts::run(date),
        Commands::ParserCorpusSweep {
            roots,
            manifest,
            output,
            baseline,
            enforce,
            verbose,
            receipt,
        } => {
            let base_roots = roots.unwrap_or_else(parser_corpus_sweep::default_base_roots);
            let corpus_roots = parser_corpus_sweep::resolve_corpus_roots(&base_roots);
            parser_corpus_sweep::run(parser_corpus_sweep::SweepConfig {
                corpus_profile: None,
                base_roots,
                corpus_roots,
                manifest_path: manifest,
                manifest_perl5lib: Vec::new(),
                output_path: output,
                baseline_path: baseline,
                enforce,
                verbose,
                receipt,
            })
        }
        Commands::CpanCorpus { command } => {
            let mut config = cpan_corpus::CpanCorpusConfig::default();
            match command {
                CpanCorpusCommand::FetchList { top_n, output } => {
                    config.top_n = top_n;
                    if let Some(out) = output {
                        config.dist_list = out;
                    }
                    cpan_corpus::fetch_list(&config)
                }
                CpanCorpusCommand::Install { dist_list, install_dir, verbose, reset } => {
                    if let Some(dl) = dist_list {
                        config.dist_list = dl;
                    }
                    config.force_reset = reset;
                    if let Some(id) = install_dir {
                        config.install_dir = id;
                    }
                    config.verbose = verbose;
                    cpan_corpus::install(&config)
                }
                CpanCorpusCommand::Sweep { output, enforce, verbose, install_dir } => {
                    if let Some(id) = install_dir {
                        config.install_dir = id;
                    }
                    config.verbose = verbose;
                    cpan_corpus::sweep(&config, output, enforce)
                }
                CpanCorpusCommand::Ratchet { verbose, install_dir } => {
                    if let Some(id) = install_dir {
                        config.install_dir = id;
                    }
                    config.verbose = verbose;
                    cpan_corpus::ratchet(&config)
                }
            }
        }
        Commands::Receipts { tests_only, docs_only, output_dir, test_threads } => {
            receipts::run(receipts::ReceiptsConfig {
                tests_only,
                docs_only,
                output_dir,
                test_threads,
            })
        }
        Commands::IgnoredTests { update, check, verbose } => {
            ignored_tests::run(update, check, verbose)
        }
        Commands::DebtReport { check, json, summary, expired, ledger } => {
            debt_report::run(debt_report::DebtReportConfig {
                check,
                json,
                summary,
                expired,
                ledger,
            })
        }
        Commands::DocClaims => doc_claims::run(),
        Commands::Features { command } => match command {
            FeaturesCommand::SyncDocs => features::sync_docs(),
            FeaturesCommand::Verify => features::verify(),
            FeaturesCommand::Invariants => features::invariants(),
            FeaturesCommand::Report => features::report(),
        },
        Commands::UpdateStatus { write, check, only } => update_status::run(write, check, only),
        Commands::SrpMicrocrates { output } => srp_microcrates::run(output),
        Commands::UnwiredScan { json, check, lsp_crate } => {
            unwired_scan::run(UnwiredScanConfig { lsp_crate, json, check })
        }
        Commands::CheckTestWiring => check_test_wiring::run(),
        Commands::Metrics { command } => match command {
            MetricsCommand::ParserStats { input, json } => metrics::parser_stats::run(input, json),
            MetricsCommand::LspStats => metrics::lsp_stats::run(),
            MetricsCommand::WorkspaceStats => metrics::workspace_stats::run(),
            MetricsCommand::DiagnosticsStats => metrics::diagnostics_stats::run(),
            MetricsCommand::Memory => metrics::memory::run(),
            MetricsCommand::ReleaseHealth { days, json } => {
                metrics::release_health::run(days, json)
            }
        },
        Commands::ValidateMemoryProfiler => compare::validate_memory_profiling(),
        Commands::E2eValidate { workspace_size, report, skip_workspace, skip_bench, verbose } => {
            e2e_validate::run(e2e_validate::E2eConfig {
                workspace_size,
                report_path: report,
                skip_workspace,
                skip_bench,
                verbose,
            })
        }
        Commands::Gates {
            tier,
            gate,
            list,
            format,
            receipt,
            receipt_path,
            diff,
            fail_fast,
            parallel,
            verbose,
        } => gates::run(gates::GateRunnerConfig {
            tier,
            gate_filter: gate,
            output_format: format,
            emit_receipt: receipt,
            receipt_path,
            diff_baseline: diff,
            list_only: list,
            fail_fast,
            parallel,
            verbose,
        }),
        Commands::TargetedChecks { base, mode } => targeted_checks::run(base, mode),
        Commands::WorktreeCleanup => worktrees::cleanup(),
        Commands::SwarmSummary { ops_dir, since, limit, format } => {
            swarm_summary::run(swarm_summary::SwarmSummaryConfig { ops_dir, since, limit, format })
        }
        Commands::PopulateBook => populate_book::run(),
        Commands::ValidateWorkspaceExclusions => validate_workspace_exclusions::run(),
        Commands::BuildTimingReceipt { clean, incremental, tests, output, baseline } => {
            build_timing::run_receipt(clean, incremental, tests, output, baseline)
        }
        Commands::CompareBuildTiming { baseline, current } => {
            build_timing::run_compare(baseline, current)
        }
    }
}
