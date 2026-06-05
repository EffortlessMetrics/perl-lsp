//! Xtask automation for tree-sitter-perl
//!
//! This binary provides custom automation tasks for building, testing,
//! and maintaining the tree-sitter-perl project.

// Task-runner binary — println!/eprintln! are intentional diagnostic output.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use cli::srp::{SrpCommand, SrpMicrocratesArgs, UnwiredScanArgs};
use color_eyre::eyre::{Result, eyre};
use std::path::PathBuf;

mod allocation_tracker;
mod cli;
mod tasks;
mod types;
mod utils;
use tasks::check_test_wiring;
use tasks::dead_code::{DeadCodeConfig, DeadCodeMode};
use tasks::gate_policy::GatePolicyProfile;
use tasks::gates::{GateTier, OutputFormat as GatesOutputFormat};
use tasks::methodology_gate::MethodologyOutputFormat;
use tasks::metrics;
use tasks::targeted_checks::CheckMode;
use tasks::unwired_scan::UnwiredScanConfig;
use tasks::ux_scorecard::UxScorecardFormat;
use tasks::workflow_trigger_lint::WorkflowTriggerLintFormat;
use tasks::worktree_allocator::AgentWorktreeCommand;
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
    /// Print all available top-level xtask commands.
    #[command(name = "list-commands")]
    List,

    /// Run lean CI suite (format, clippy, tests) for constrained environments.
    /// Use `ci doctor` to check local/CI parity without running the full suite.
    Ci {
        /// Optional sub-command; omit to run the full CI suite.
        #[command(subcommand)]
        command: Option<CiSubcommand>,
    },

    /// Run format and clippy checks only (no tests)
    CheckOnly,

    /// Verify the governed Clippy lint policy ledger and workspace inheritance.
    CheckLintPolicy,

    /// Verify local Rust toolchain meets the pinned MSRV in rust-toolchain.toml.
    CheckToolchain {
        /// Show a warning when rustc satisfies the minimum MSRV but differs
        /// from the exact pinned channel string.
        #[arg(long)]
        doctor: bool,
    },

    /// Verify DevEx docs match the toolchain and command surface.
    CheckDevexDocs,

    /// Validate Real Perl Editor Trust provider/support claim tables.
    CheckProviderConfidenceMatrix,

    /// Validate Real Perl Editor Trust support claim map.
    CheckSupportClaims,

    /// Validate the active swarm goal manifest and linked docs.
    CheckActiveGoalManifest,

    /// Validate machine-readable Real Perl Editor Trust provider promotion ledger.
    CheckProviderPromotionLedger,

    /// Validate declared differential real-Perl oracle fixtures.
    CheckOracleFixtureManifest,

    /// Validate differential real-Perl oracle receipt schema.
    CheckOracleReceiptSchema,

    /// Validate semantic-token class promotion registry.
    CheckSemanticTokenClasses,

    /// Validate selected LSP 3.18 claim-boundary guardrails.
    #[command(name = "check-lsp-318-claims")]
    CheckLsp318Claims,

    /// Generate or check the selected LSP 3.18 conformance matrix.
    #[command(name = "generate-lsp-318-matrix")]
    GenerateLsp318Matrix {
        /// Check that the checked-in matrix matches generated content.
        #[arg(long)]
        check: bool,
    },

    /// Validate workspace-symbol class promotion registry.
    CheckWorkspaceSymbolClasses,

    /// Capture a GitHub PR queue snapshot for disconnected maintainership.
    Queue {
        #[command(subcommand)]
        command: QueueCommand,
    },

    /// PR-related local tooling (title check, etc.)
    Pr {
        #[command(subcommand)]
        command: PrSubcommand,
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

    /// Run local smoke checks against explicit binaries.
    Smoke {
        #[command(subcommand)]
        command: SmokeCommand,
    },

    /// Verify inline completion over stdio against a built binary.
    #[command(name = "inline-completion-smoke")]
    InlineCompletionSmoke {
        /// Path to the perl-lsp binary to execute.
        #[arg(long)]
        binary: PathBuf,
    },

    /// Emit a deterministic inline-completion quality receipt.
    #[command(name = "inline-completion-quality")]
    InlineCompletionQuality {
        /// Receipt JSON path to write.
        #[arg(long, default_value = "target/receipts/inline-completion-quality.json")]
        receipt: PathBuf,
    },

    /// Emit a semantic inline-completion UX receipt dashboard.
    #[command(name = "semantic-inline-receipts")]
    SemanticInlineReceipts {
        /// Receipt JSON path to write.
        #[arg(long, default_value = "target/receipts/semantic-inline-receipts.json")]
        receipt: PathBuf,
        /// Optional deterministic quality receipt to summarize when present.
        #[arg(long, default_value = "target/receipts/inline-completion-quality.json")]
        quality_receipt: PathBuf,
        /// Optional next-edit scaffold receipt to validate and summarize when present.
        #[arg(long, default_value = "target/receipts/semantic-inline-next-edit.json")]
        next_edit_receipt: PathBuf,
    },

    /// Emit a semantic inline-completion next-edit scaffold receipt.
    #[command(name = "semantic-inline-next-edit")]
    SemanticInlineNextEdit {
        /// Receipt JSON path to write.
        #[arg(long, default_value = "target/receipts/semantic-inline-next-edit.json")]
        receipt: PathBuf,
    },

    /// Emit a supported-editor inline-completion smoke receipt bundle.
    #[command(name = "supported-editor-inline-smoke")]
    SupportedEditorInlineSmoke {
        /// Receipt JSON path to write.
        #[arg(long, default_value = "target/receipts/supported-editor-inline-smoke.json")]
        receipt: PathBuf,
    },

    /// Regenerate public Shields endpoint JSON for README badges.
    Badges {
        /// Check committed endpoints for drift without updating badges/.
        #[arg(long)]
        check: bool,
    },

    /// Generate or check a coverage baseline receipt for the quality lane.
    #[command(name = "coverage-baseline")]
    CoverageBaseline {
        /// LCOV input path.
        #[arg(long, default_value = "target/lcov.info")]
        lcov: PathBuf,
        /// Coverage receipt JSON path.
        #[arg(long, default_value = "target/receipts/quality/coverage-baseline.json")]
        receipt: PathBuf,
        /// Codecov configuration path.
        #[arg(long, default_value = "codecov.yml")]
        codecov: PathBuf,
        /// Patch coverage percentage from Codecov for this PR.
        #[arg(long)]
        patch_coverage: Option<f64>,
        /// Compute patch coverage from executable lines changed since this git base.
        #[arg(long)]
        patch_base: Option<String>,
        /// Coverage scope recorded in the receipt.
        #[arg(long)]
        scope: Option<String>,
        /// Validate the existing receipt instead of rewriting it.
        #[arg(long)]
        check: bool,
    },

    /// Evaluate coverage and RIPR proof receipts for local and CI gates.
    #[command(name = "quality-gate")]
    QualityGate {
        /// Gate mode to evaluate.
        #[arg(long, value_enum)]
        mode: tasks::quality_gate::QualityGateMode,
        /// Temporary quality exception policy path.
        #[arg(long, default_value = "policy/quality-gate-exceptions.toml")]
        exception_policy: PathBuf,
        /// Repo-wide RIPR+ receipt JSON path.
        #[arg(long, default_value = "target/receipts/quality/ripr-plus.json")]
        ripr_receipt: PathBuf,
        /// Diff-scoped RIPR PR evidence JSON path.
        #[arg(long, default_value = "target/ripr/pr/repo-exposure.json")]
        ripr_pr_receipt: PathBuf,
        /// RIPR review-guidance receipt JSON path.
        #[arg(long, default_value = "target/ripr/review/comments.json")]
        review_receipt: PathBuf,
        /// Coverage receipt JSON path.
        #[arg(long, default_value = "target/receipts/quality/coverage-baseline.json")]
        coverage_receipt: PathBuf,
        /// Codecov configuration path.
        #[arg(long, default_value = "codecov.yml")]
        codecov: PathBuf,
        /// Patch coverage percentage from Codecov for this PR.
        #[arg(long)]
        patch_coverage: Option<f64>,
        /// Base revision used for diff-scoped RIPR receipt commands.
        #[arg(long, default_value = "origin/HEAD")]
        ripr_base: String,
        /// Head revision used for diff-scoped RIPR receipt commands.
        #[arg(long, default_value = "HEAD")]
        ripr_head: String,
        /// Quality-gate JSON receipt path.
        #[arg(long, default_value = "target/receipts/quality/quality-gate.json")]
        receipt: PathBuf,
        /// Quality-gate Markdown summary path.
        #[arg(long, default_value = "target/receipts/quality/quality-gate.md")]
        summary: PathBuf,
        /// Validate existing quality-gate outputs instead of rewriting them.
        #[arg(long)]
        check: bool,
    },

    /// Produce diff-scoped RIPR PR evidence artifacts.
    RiprPr {
        /// Root passed to RIPR. Defaults to the repository root.
        #[arg(long, default_value = ".")]
        root: String,
        /// Base revision for the PR diff.
        #[arg(long, default_value = "origin/master")]
        base: String,
        /// Head revision for the PR diff.
        #[arg(long, default_value = "HEAD")]
        head: String,
        /// Validate existing target/ripr/pr artifacts instead of regenerating.
        #[arg(long)]
        check: bool,
    },

    /// Emit a repo-wide RIPR+ baseline receipt for the quality lane.
    RiprPlus {
        /// Root passed to RIPR. Defaults to the repository root.
        #[arg(long, default_value = ".")]
        root: String,
        /// Receipt JSON path.
        #[arg(long, default_value = "target/receipts/quality/ripr-plus.json")]
        receipt: PathBuf,
        /// RIPR suppression policy path.
        #[arg(long, default_value = "policy/ripr-suppressions.toml")]
        suppressions: PathBuf,
        /// Validate the existing receipt instead of rewriting it.
        #[arg(long)]
        check: bool,
    },

    /// Produce diff-scoped RIPR review guidance artifacts without posting comments.
    RiprReviewComments {
        /// Root passed to RIPR. Defaults to the repository root.
        #[arg(long, default_value = ".")]
        root: String,
        /// Base revision for the PR diff.
        #[arg(long, default_value = "origin/master")]
        base: String,
        /// Head revision for the PR diff.
        #[arg(long, default_value = "HEAD")]
        head: String,
        /// Bound RIPR review guidance generation; timeout writes an advisory error artifact.
        #[arg(long)]
        timeout_seconds: Option<u64>,
        /// Validate existing target/ripr/review artifacts instead of regenerating.
        #[arg(long)]
        check: bool,
    },

    /// Generate the stable PR evidence summary from machine-readable artifacts.
    RiprPrSummary {
        /// Validate the generated summary instead of rewriting it.
        #[arg(long)]
        check: bool,
    },

    /// Render non-blocking GitHub warning annotations from comments[] guidance only.
    RiprAnnotations {
        /// Review guidance JSON path.
        #[arg(long, default_value = "target/ripr/review/comments.json")]
        comments: String,
        /// Output path for rendered annotation commands.
        #[arg(long, default_value = "target/ripr/review/annotations.txt")]
        out: String,
        /// Validate existing annotation output instead of regenerating.
        #[arg(long)]
        check: bool,
    },

    /// Emit mutation-routing evidence from PR evidence and labels.
    ImpactedEvidence {
        /// PR evidence JSON input.
        #[arg(long, default_value = "target/ripr/pr/repo-exposure.json")]
        pr_evidence: String,
        /// Repeatable PR label input.
        #[arg(long = "label")]
        labels: Vec<String>,
        /// Comma, semicolon, or newline separated PR labels.
        #[arg(long)]
        labels_csv: Option<String>,
        /// Validate existing impacted evidence instead of regenerating.
        #[arg(long)]
        check: bool,
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
        #[arg(long, default_value = "perllsp")]
        prefix: String,

        /// Output path for generated Homebrew formula.
        #[arg(long, default_value = "Formula/perllsp.rb")]
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

        /// Restrict formatting to one or more package names.
        ///
        /// Accepts repeated flags (`--package xtask --package perl-parser`) or
        /// a comma-delimited list (`--package xtask,perl-parser`).
        #[arg(long, short = 'p', value_delimiter = ',')]
        package: Option<Vec<String>>,
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

    /// Developer experience helpers.
    Devex {
        #[command(subcommand)]
        command: DevexCommand,
    },

    /// Audit CI workflows for PR-safety and spend-risk controls.
    CiAuditWorkflows,

    /// Lint GitHub workflow security policy invariants.
    WorkflowPolicyLint {
        /// Write a JSON receipt artifact for CI consumption.
        #[arg(long)]
        receipt: Option<PathBuf>,

        /// Lint a single workflow fixture instead of repository workflows.
        #[arg(long)]
        fixture: Option<PathBuf>,

        /// Also validate that every workflow has a `[[lane]]` entry in
        /// policy/ci-lane-whitelist.toml. Advisory (warning-level) until the
        /// whitelist has stabilized — see docs/ci/perl-lsp-rollout-plan.md PR 11.
        #[arg(long)]
        check_lane_whitelist: bool,
    },

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

    /// Compute the CI scope — changed crates, reverse-dep closure, and architectural wideners.
    ///
    /// Emits a JSON (or text) payload listing changed files, mapped crates, the
    /// reverse-dependency closure, architectural wideners applied, and the
    /// selected CI lanes with reasons. Deterministic given the same diff and
    /// `cargo metadata` output.
    ///
    /// Example: `cargo xtask ci-scope --base auto --format json`
    CiScope {
        /// Base git reference to diff against (default: auto-detect).
        #[arg(long, default_value = "auto")]
        base: String,

        /// Output format: `json` or `text` (default: json).
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Emit a markdown PR gate summary (dry-run: stdout only, no GitHub posting).
    ///
    /// Computes what CI would run for the current branch diff against `--base`,
    /// and formats it as markdown: changed crates, widened crates, gates run,
    /// gates skipped, timing estimate, and receipt links.
    ///
    /// **Claim boundary**: dry-run only. GitHub sticky-comment posting is
    /// a follow-up to issue #4825.
    ///
    /// Example: `cargo xtask ci pr-summary --base origin/master --dry-run`
    CiPrSummary {
        /// Base git reference to diff against (e.g. `origin/master`).
        #[arg(long, default_value = "origin/master")]
        base: String,

        /// Emit markdown to stdout only; do not post to GitHub.
        /// Required in this version — GitHub posting is a future follow-up.
        #[arg(long, default_value_t = true)]
        dry_run: bool,
    },

    /// Lint required workflow triggers against policy.
    WorkflowTriggerLint {
        /// Policy TOML path listing conventional required checks.
        #[arg(long)]
        policy: Option<PathBuf>,

        /// Optional receipt output path (JSON).
        #[arg(long)]
        receipt: Option<PathBuf>,

        /// Validate a single workflow fixture file instead of policy workflows.
        #[arg(long)]
        fixture: Option<PathBuf>,

        /// Output format.
        #[arg(long, value_enum, default_value = "text")]
        format: WorkflowTriggerLintFormat,
    },

    /// Run version-sync checks from `perl-ci-hygiene`.
    CheckVersionSync,

    /// Sync active release narrative docs from workspace version and publish count.
    SyncReleaseDocs {
        /// Write synced files (omit to run a dry check).
        #[arg(long)]
        write: bool,
    },

    /// Check for disallowed direct `ExitStatus::from_raw()` usage.
    CheckFromRaw,

    /// Enforce retained-state lifecycle and memory receipt invariants.
    CheckMemoryLifecyclePolicy,

    /// Warn when a diff adds retained-state owner patterns without inventory updates.
    CheckMemoryRetainedOwnerDrift {
        /// Git base ref used for diffing changed files.
        #[arg(long, default_value = "origin/master")]
        base: String,

        /// Warn instead of fail when drift appears in existing retained-owner paths.
        #[arg(long)]
        report_only: bool,
    },

    /// Render memory plateau receipt trends.
    MemoryTrends {
        #[command(subcommand)]
        command: MemoryTrendsCommand,
    },

    /// Check native formatter fixtures and emit receipts.
    NativeFormat {
        #[command(subcommand)]
        command: NativeFormatCommand,
    },

    /// Run native critic checks and emit receipts.
    NativeCritic {
        #[command(subcommand)]
        command: NativeCriticCommand,
    },

    /// Report native formatter and critic replacement status.
    NativeTooling {
        #[command(subcommand)]
        command: NativeToolingCommand,
    },

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

    /// Release automation commands.
    Release {
        #[command(subcommand)]
        command: ReleaseCommand,
    },

    /// Extract the curated release body from `docs/releases/<tag>.md`.
    ///
    /// Reads the file, strips its YAML frontmatter, and emits the body to
    /// stdout (or to `--output` if provided). Used by the `release.yml`
    /// workflow to drive GitHub Release bodies from the curated per-release
    /// notes that ship in the repo.
    ReleaseNotes {
        /// Release tag (e.g. `v0.12.4`). A bare version like `0.12.4` is
        /// accepted and normalized to `v0.12.4`.
        #[arg(long)]
        tag: String,

        /// Optional output file. When omitted, the body is written to stdout.
        #[arg(long)]
        output: Option<PathBuf>,

        /// Override the repository root used to resolve `docs/releases/`.
        /// Intended as a testing seam; the release workflow never passes this.
        #[arg(long, hide = true)]
        root: Option<PathBuf>,
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

    /// Parse one corpus file in an isolated child process for corpus-audit timeout guards.
    #[command(hide = true)]
    CorpusAuditParseOne {
        /// Path to the corpus file to parse.
        #[arg(long)]
        path: PathBuf,
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
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
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

    /// Verify transitive normal-dep closure of published crates contains only publishable deps
    PublishClosure {
        /// Check only this crate (default: all allowlisted crates)
        #[arg(long)]
        crate_name: Option<String>,
    },

    /// Ratchet gate: published-crate count must not increase above baseline.
    ///
    /// Reads the current entry count from `[workspace.metadata.publish.allow]`
    /// (via `cargo metadata --no-deps`), compares against the baseline stored in
    /// `xtask/published-crate-baseline.txt`, and fails if the count increased.
    /// When the count has decreased, the baseline is auto-tightened.
    PublishedCrateCount,

    /// Offline manifest validation: allowlist drift + LICENSE present.
    ///
    /// Checks that every entry in `[workspace.metadata.publish.allow]` is a
    /// publishable workspace member and vice versa (allowlist drift), and that
    /// every allowlisted crate has a `license` or `license-file` field set.
    /// Uses `cargo metadata --no-deps` — no network contact.
    ///
    /// Replaces the Python `--check-drift` step in `publish-dry-run.yml` and
    /// is wired into `just pr-fast` and `just ci-gate`.
    PublishManifestCheck,

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

    /// Emit parser-ratchet scaffold receipts.
    ParserRatchet {
        #[command(subcommand)]
        command: ParserRatchetCommand,
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

    /// Aggregate CI subreceipt fragments into one stable final receipt.
    AggregateReceipts {
        /// Stable final check name.
        #[arg(long)]
        check: String,
        /// Input directory containing subreceipt JSON files.
        #[arg(long)]
        inputs: PathBuf,
        /// Output path for aggregate receipt JSON.
        #[arg(long)]
        output: PathBuf,
        /// Allow required lanes to no-op without failing the final check.
        #[arg(long, default_value_t = true)]
        allow_noop: bool,
    },

    /// Compute final pass/fail outcome from an aggregate receipt.
    FinalizeCheck {
        /// Path to aggregate receipt JSON.
        #[arg(long)]
        receipt: PathBuf,
        /// Allow required lanes to no-op without failing the final check.
        #[arg(long, default_value_t = true)]
        allow_noop: bool,
        /// Treat advisory warnings/failures as fatal.
        #[arg(long, default_value_t = false)]
        fail_on_advisory: bool,
    },

    /// Emit, verify, and reconcile SHA-bound merge-readiness receipts.
    MergeReady {
        #[command(subcommand)]
        command: MergeReadyCommand,
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

    /// Manage gate receipt schema registry and validate receipt payloads.
    GateReceipts {
        #[command(subcommand)]
        command: GateReceiptsCommand,
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

    /// Check active install docs and release notes for stale install command drift.
    InstallSurfaceCheck,

    /// Validate PR intent/title/body against changed paths and closeout evidence.
    IntentDiffGate {
        /// Pull request number to inspect via `gh pr view`.
        #[arg(long)]
        pr: Option<u64>,

        /// Load PR metadata from a local JSON fixture file.
        #[arg(long)]
        fixture: Option<PathBuf>,

        /// Output receipt path (default: target/receipts/intent-diff-gate.json).
        #[arg(long)]
        receipt: Option<PathBuf>,
    },

    /// Manage feature catalog and LSP compliance
    Features {
        #[command(subcommand)]
        command: FeaturesCommand,
    },

    /// Agent lease + receipt primitives for disconnected orchestration.
    Agent {
        #[command(subcommand)]
        command: AgentCommand,
    },

    /// Classify failed CI receipts into typed fix-forward playbooks.
    FixForward {
        #[command(subcommand)]
        command: FixForwardCommand,
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

    /// SRP-oriented crate topology and wiring checks.
    Srp {
        #[command(subcommand)]
        command: SrpCommand,
    },

    /// Generate SRP microcrate inventory and split-candidate report.
    SrpMicrocrates {
        #[command(flatten)]
        args: SrpMicrocratesArgs,
    },

    /// Enforce crate layer-dependency constraints.
    LayerCheck,

    /// Scan for built-but-not-wired crates.
    UnwiredScan {
        #[command(flatten)]
        args: UnwiredScanArgs,
    },

    /// Check that test-bearing Rust files are reachable from their module tree.
    CheckTestWiring,

    /// Emit per-subsystem engineering-health metrics.
    Metrics {
        #[command(subcommand)]
        command: MetricsCommand,
    },

    /// Publish structured editor UX scorecard artifact/status from harness fixtures.
    UxScorecard {
        /// Output format for stdout.
        #[arg(long, value_enum, default_value = "human")]
        format: UxScorecardOutputFormat,
        /// Optional path to scenario measurements JSON.
        #[arg(long)]
        input: Option<PathBuf>,
        /// Optional path to emitted scorecard JSON artifact.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Optional path to generated status markdown.
        #[arg(long)]
        status_md: Option<PathBuf>,
        /// Enforce regression-only ratchet against committed baseline.
        #[arg(long)]
        ratchet_check: bool,
    },

    /// Publish/check 0.13.2 semantic scorecard artifacts from deterministic fixtures.
    SemanticScorecard {
        /// Optional path to semantic fixture manifest JSON.
        #[arg(long)]
        manifest: Option<PathBuf>,
        /// Optional path to emitted scorecard JSON artifact.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Optional path to generated status markdown.
        #[arg(long)]
        status_md: Option<PathBuf>,
        /// Verify committed artifacts are current.
        #[arg(long)]
        check: bool,
    },

    /// Publish/check deterministic semantic shadow-compare proof artifacts.
    SemanticShadowCompare {
        /// Optional path to emitted shadow-compare JSON artifact.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Optional path to generated status markdown.
        #[arg(long)]
        status_md: Option<PathBuf>,
        /// Verify committed artifacts are current.
        #[arg(long)]
        check: bool,
    },

    /// Emit structured UX regression receipt from test output
    UxRegressionReceipt {
        /// Path to test output file (e.g., /tmp/ux-test-output.txt)
        #[arg(long)]
        input: PathBuf,
        /// Optional path to write receipt JSON
        #[arg(long)]
        receipt: Option<PathBuf>,
        /// Git SHA for receipt metadata
        #[arg(long)]
        sha: Option<String>,
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

        /// Base git ref used for scope-aware PR-fast planning
        #[arg(long)]
        base: Option<String>,

        /// List available gates without running them
        #[arg(long, short)]
        list: bool,

        /// Output format (default: human)
        #[arg(long, short, value_enum, default_value = "human")]
        format: GatesOutputFormat,

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

    /// Inspect and validate effective gate policy profiles.
    GatePolicy {
        #[command(subcommand)]
        command: GatePolicyCommand,
    },

    /// Detect contradictory PR label states and emit a methodology receipt.
    MethodologyGate {
        /// Fixture JSON file (local snapshot or GitHub event payload).
        #[arg(long)]
        fixture: Option<PathBuf>,

        /// Pull request number to inspect via gh CLI.
        #[arg(long)]
        pr: Option<u64>,

        /// Path to output receipt JSON.
        #[arg(long, default_value = "target/receipts/methodology-gate.json")]
        receipt: PathBuf,

        /// Do not write receipt to disk.
        #[arg(long)]
        dry_run: bool,

        /// Enforce mode: contradictory states fail the command.
        #[arg(long)]
        enforce: bool,

        /// Output format.
        #[arg(long, value_enum, default_value = "human")]
        format: MethodologyOutputFormat,
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
        /// Base git reference for diff (default: auto-detect)
        #[arg(long, default_value = "auto")]
        base: String,

        /// Check mode: clippy, test, or all (default: all)
        #[arg(long, value_enum, default_value = "all")]
        mode: CheckMode,
    },

    /// Resolve the Cargo package name for a crate directory.
    ///
    /// Prints the package name from Cargo.toml to stdout (one line, no trailing noise).
    /// Used by the pre-push hook to convert a directory basename into the correct -p argument.
    ///
    /// Example: `cargo xtask resolve-package-name crates/perl-lsp-rs` outputs `perl-lsp-rs`
    ResolvePackageName {
        /// Crate directory path, relative to workspace root (e.g., "crates/perl-lsp-rs")
        crate_dir: String,
    },

    /// Remove stale `.claude/worktrees` entries and prune Git metadata.
    WorktreeCleanup,

    /// Validate the committed Claude swarm agent roster contract.
    ValidateSwarmAgentRoster {
        /// Repository root containing `.claude/agents/agent-roster.json`.
        #[arg(long)]
        root: Option<PathBuf>,
    },

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

    /// Validate generated-file ownership and associated receipts.
    GeneratedFiles {
        #[command(subcommand)]
        command: GeneratedFilesCommand,
    },

    /// Non-Rust file policy commands.
    NonRust {
        #[command(subcommand)]
        command: NonRustCommand,
    },

    /// Check non-Rust files against the policy allowlist and report violations.
    ///
    /// Equivalent to `non-rust check`. Default mode is `advisory` (always
    /// exits 0). Use `--mode blocking-allowlist` or `--mode blocking-strict`
    /// for enforcement. See #8566.
    ///
    /// Examples:
    ///   `cargo xtask check-file-policy`
    ///   `cargo xtask check-file-policy --mode advisory`
    ///   `cargo xtask check-file-policy --mode blocking-allowlist`
    ///   `cargo xtask check-file-policy --json target/policy/file-policy-report.json`
    CheckFilePolicy {
        /// Enforcement mode.
        #[arg(long, value_enum, default_value = "advisory")]
        mode: CheckFilePolicyCliMode,

        /// Override the default JSON receipt path
        /// (`target/policy/file-policy-report.json`).
        #[arg(long)]
        json: Option<PathBuf>,

        /// Override the default allowlist path (`policy/non-rust-allowlist.toml`).
        #[arg(long)]
        allowlist: Option<PathBuf>,

        /// Override the workspace root used for `git ls-files`. Test seam only.
        #[arg(long, hide = true)]
        root: Option<PathBuf>,
    },

    /// Check whether the current checkout is behind origin/master.
    ///
    /// Emits a JSON receipt (schema_version 1) with staleness metadata.
    /// Use --mode block to fail when stale; default is warn (exit 0 always).
    ///
    /// Example: `cargo xtask freshness-check --base origin/master --mode block`
    FreshnessCheck {
        /// Base git reference to compare HEAD against.
        #[arg(long, default_value = "origin/master")]
        base: String,

        /// Operating mode: warn (default, exit 0) or block (exit 1 when stale).
        #[arg(long, value_enum, default_value = "warn")]
        mode: FreshnessCheckMode,

        /// Write the JSON receipt to this file path instead of stdout.
        #[arg(long)]
        json: Option<PathBuf>,

        /// Skip the `git fetch` step.
        #[arg(long)]
        no_fetch: bool,

        /// Accept a stale checkout for historical/archaeology work. Requires --reason.
        #[arg(long, requires = "reason")]
        allow_historical: bool,

        /// Reason text for the historical override (required with --allow-historical).
        #[arg(long)]
        reason: Option<String>,

        /// Also check binary freshness: verify that target/debug/perl-lsp and
        /// target/release/perl-lsp are newer than the HEAD commit timestamp.
        /// Exits non-zero when a binary exists and is stale. Missing binaries
        /// are reported but do not cause a non-zero exit.
        #[arg(long)]
        binaries: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
enum CheckFilePolicyCliMode {
    Advisory,
    BlockingAllowlist,
    BlockingStrict,
}

#[derive(Subcommand)]
enum NonRustCommand {
    /// Walk `git ls-files`, classify tracked files against the allowlist,
    /// and emit `target/policy/non-rust-inventory.{md,json}` plus
    /// `docs/policy/NON_RUST_INVENTORY.md`.
    Inventory,

    /// Check non-Rust files against the allowlist and report violations.
    ///
    /// Default mode is `advisory` — always exits 0, reports findings only.
    /// Use `--mode blocking-allowlist` or `--mode blocking-strict` to enable
    /// enforcement. Strict mode is NOT promoted to CI in this PR (see #8566).
    Check {
        /// Enforcement mode.
        #[arg(long, value_enum, default_value = "advisory")]
        mode: CheckFilePolicyCliMode,

        /// Override the default JSON receipt path
        /// (`target/policy/file-policy-report.json`).
        ///
        /// Example: `--json target/policy/file-policy-report.json`
        #[arg(long)]
        json: Option<PathBuf>,

        /// Override the default allowlist path (`policy/non-rust-allowlist.toml`).
        #[arg(long)]
        allowlist: Option<PathBuf>,

        /// Override the workspace root used for `git ls-files`. Test seam only.
        #[arg(long, hide = true)]
        root: Option<PathBuf>,
    },

    /// Generate draft allowlist proposals for unclassified non-Rust files.
    ///
    /// Writes:
    ///   - `<output-dir>/non-rust-proposed-allowlist.toml` — draft entries ready for review.
    ///   - `<output-dir>/non-rust-proposal.md` — human-readable summary.
    ///
    /// NEVER modifies `policy/non-rust-allowlist.toml`. The canonical ledger is
    /// human-curated; this command only generates proposals for review.
    Propose {
        /// Output directory (default: `target/policy`).
        #[arg(long, default_value = "target/policy")]
        output_dir: PathBuf,

        /// Grouping strategy: `directory` (default) groups by top-level dir;
        /// `extension` groups by file extension.
        #[arg(long, value_enum, default_value = "directory")]
        group_by: ProposeGroupByArg,

        /// Override the workspace root used for `git ls-files`. Test seam only.
        #[arg(long, hide = true)]
        root: Option<PathBuf>,
    },

    /// Validate the non-Rust allowlist/debt TOML schema without walking git.
    ValidatePolicy {
        /// Override the default allowlist path (`policy/non-rust-allowlist.toml`).
        #[arg(long, default_value = "policy/non-rust-allowlist.toml")]
        allowlist: PathBuf,

        /// Override the default debt path (`policy/non-rust-debt.toml`).
        #[arg(long, default_value = "policy/non-rust-debt.toml")]
        debt: PathBuf,
    },

    /// Find non-Rust tooling that should be migrated into Rust-owned surfaces.
    MigrationCandidates {
        /// Output format.
        #[arg(long, value_enum, default_value = "markdown")]
        format: MigrationCandidateFormatArg,

        /// Optional output path (prints to stdout if omitted).
        #[arg(long)]
        output: Option<PathBuf>,

        /// Limit the number of candidates in the report.
        #[arg(long)]
        limit: Option<usize>,

        /// Override the workspace root used for `git ls-files`. Test seam only.
        #[arg(long, hide = true)]
        root: Option<PathBuf>,
    },
}

/// CLI-facing output format for non-Rust migration candidate reports.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum MigrationCandidateFormatArg {
    /// Human-readable Markdown.
    Markdown,
    /// Machine-readable JSON.
    Json,
}

/// CLI-facing grouping argument (mirrors `file_policy::ProposeGroupBy`).
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum ProposeGroupByArg {
    /// Group by top-level directory (default).
    Directory,
    /// Group by file extension.
    Extension,
}

#[derive(Subcommand)]
enum GeneratedFilesCommand {
    /// List generated-file ownership rules.
    List {
        /// Optional fixture JSON for deterministic tests.
        #[arg(long)]
        fixture: Option<PathBuf>,
    },
    /// Check changed generated files for matching generator receipts.
    Check {
        /// Path where generated-file receipt JSON is written.
        #[arg(long, default_value = "target/receipts/generated-files.json")]
        receipt: PathBuf,
        /// Optional fixture JSON for deterministic tests.
        #[arg(long)]
        fixture: Option<PathBuf>,
        /// Path(s) to generator receipt JSON artifacts.
        #[arg(long = "generator-receipt")]
        generator_receipt: Vec<PathBuf>,
        /// Explicit override for manual edits in this run.
        #[arg(long)]
        allow_manual_edits: bool,
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
enum ParserRatchetCommand {
    /// Produce an initial parser-ratchet scaffold receipt.
    Run {
        /// Ratchet execution profile.
        #[arg(long, value_enum)]
        profile: parser_ratchet::RatchetProfile,

        /// Explicit git revision for the base side.
        #[arg(long)]
        base: String,

        /// Explicit git revision for the head side.
        #[arg(long)]
        head: String,

        /// Output path for the receipt JSON.
        #[arg(long)]
        receipt: PathBuf,

        /// Force selection in scaffold mode.
        #[arg(long)]
        force_selected: bool,
    },
}

#[derive(Subcommand)]
enum GateReceiptsCommand {
    /// List registered receipt schemas.
    List {
        /// Output format (default: human).
        #[arg(long, value_enum, default_value = "human")]
        format: GateReceiptsFormat,
    },
    /// Validate a single receipt JSON file.
    Validate {
        /// Path to receipt JSON file.
        path: PathBuf,
        /// Output format (default: human).
        #[arg(long, value_enum, default_value = "human")]
        format: GateReceiptsFormat,
    },
    /// Validate all receipt JSON files under a directory.
    ValidateAll {
        /// Root directory containing receipt JSON files.
        dir: PathBuf,
        /// Output format (default: human).
        #[arg(long, value_enum, default_value = "human")]
        format: GateReceiptsFormat,
    },
}

#[derive(Debug, Subcommand)]
enum GatePolicyCommand {
    /// Validate policy/registry invariants for PR safety.
    Check,
    /// Show effective required/advisory gates for a profile.
    Effective {
        /// Profile to evaluate (pr/nightly/release).
        #[arg(long, value_enum, default_value = "pr")]
        profile: GatePolicyProfile,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum GateReceiptsFormat {
    Human,
    Json,
}

/// CLI-facing mode enum for freshness-check (maps to `tasks::freshness_check::FreshnessMode`).
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum FreshnessCheckMode {
    Warn,
    Block,
}

#[derive(Subcommand)]
enum MergeReadyCommand {
    /// Emit a merge-readiness receipt for a PR.
    Emit {
        /// Pull request number.
        #[arg(long)]
        pr: u64,
        /// Output path for receipt JSON.
        #[arg(long)]
        receipt: Option<PathBuf>,
    },
    /// Verify receipt freshness and verdict.
    Verify {
        /// Pull request number (advisory context).
        #[arg(long)]
        pr: Option<u64>,
        /// Verify a fixture file instead of the default receipt path.
        #[arg(long)]
        fixture: Option<PathBuf>,
    },
    /// Reconcile merge-ready label state from receipts.
    Reconcile {
        /// Apply changes (default is advisory dry-run).
        #[arg(long)]
        apply: bool,
        /// Force dry-run mode.
        #[arg(long)]
        dry_run: bool,
    },
    /// Scan all open PRs and resolve label contradictions queue-wide.
    ///
    /// Uses live CI state for ci-green/needs-ci-fix decisions, and
    /// "later-applied wins" timeline logic for other contradiction pairs.
    /// Apply mode is the default; pass --dry-run for advisory mode.
    ReconcileQueue {
        /// Apply label changes (default when neither flag given).
        #[arg(long)]
        apply: bool,
        /// Dry-run: report what would change without applying.
        #[arg(long, conflicts_with = "apply")]
        dry_run: bool,
        /// Limit to a single PR number (useful for testing).
        #[arg(long)]
        pr: Option<u64>,
        /// Output path for the queue-reconcile.json receipt.
        #[arg(long)]
        receipt: Option<PathBuf>,
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
enum ReleaseCommand {
    /// Prepare release artifacts.
    Prepare {
        /// Version to release
        version: String,

        /// Skip confirmation
        #[arg(long)]
        yes: bool,
    },
    /// Create release evidence scaffold receipt list.
    Evidence {
        /// Release version without `v` prefix (for example: 0.13.0)
        #[arg(long)]
        version: String,
        /// Output bundle directory.
        #[arg(long)]
        out: PathBuf,
    },
    /// Verify release evidence bundle and emit summary receipt.
    VerifyEvidence {
        /// Release version without `v` prefix (for example: 0.13.0)
        #[arg(long)]
        version: String,
        /// Output summary receipt path.
        #[arg(long)]
        receipt: PathBuf,
        /// Bundle directory to validate.
        #[arg(long)]
        bundle_dir: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum FixForwardCommand {
    /// Classify a failing receipt into a typed fix-forward playbook.
    Classify {
        /// Path to a CI receipt JSON.
        #[arg(long)]
        receipt: PathBuf,

        /// Output path for fix-forward receipt JSON.
        #[arg(long)]
        output: PathBuf,
    },

    /// List configured fix-forward playbooks.
    ListPlaybooks,
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
    /// Parser accuracy scorecard — denominator inventory and placeholder scoring rows.
    ParserAccuracy {
        /// Write output to target/metrics/parser_accuracy.json.
        #[arg(long)]
        json: bool,
        /// Validate the generated artifact contract without writing target output.
        #[arg(long)]
        check: bool,
        /// Export committed parser status receipts under docs/project/status/.
        #[arg(long)]
        export_status_receipts: bool,
        /// Fixture manifest path.
        #[arg(long)]
        manifest: Option<PathBuf>,
        /// Output path for --json.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Metric cadence: pr, merge_gate, nightly, or release.
        #[arg(long, default_value = "pr")]
        cadence: String,
    },
    /// HIR lowering coverage inventory and status proof.
    HirCoverage {
        /// Write JSON receipt to target/metrics/hir_coverage.json or --output.
        #[arg(long)]
        json: bool,
        /// Output path for --json.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Regenerate docs/project/status/hir_lowering.md.
        #[arg(long)]
        write_status: bool,
        /// Validate docs/project/status/hir_lowering.md is current.
        #[arg(long)]
        check: bool,
    },
    /// LSP editor-intelligence scorecard — fixture inventory and pass rates.
    LspStats {
        /// Write output to .ci/metrics/editor_intelligence.json
        #[arg(long)]
        json: bool,
        /// Directory containing ux_scenario_run receipt JSON files.
        #[arg(long)]
        receipt_dir: Option<PathBuf>,
    },
    /// [stub] Workspace index memory and timing statistics.
    WorkspaceStats,
    /// [stub] Diagnostics accuracy and latency statistics.
    DiagnosticsStats,
    /// Render memory plateau summaries and optional receipt JSON.
    Memory {
        /// Workload JSON emitted by scripts/repro_lsp_storm.py.
        #[arg(long)]
        workload_json: PathBuf,
        /// Plateau summary JSON emitted by scripts/assert_rss_plateau.py.
        #[arg(long)]
        plateau_json: PathBuf,
        /// Scenario id for the memory receipt.
        #[arg(long)]
        scenario: Option<String>,
        /// Optional output path for the generated receipt JSON.
        #[arg(long)]
        receipt: Option<PathBuf>,
        /// Git commit SHA attached to the receipt.
        #[arg(long)]
        commit: Option<String>,
        /// Receipt event: pull_request, merge_group, push, or local.
        #[arg(long, default_value = "local")]
        event: String,
        /// Render a markdown table instead of JSON to stdout.
        #[arg(long)]
        markdown: bool,
    },
    /// Release-health dashboard — debt ledger + merge-gate baseline summary.
    ReleaseHealth {
        /// Number of days of history reported in the receipt window field.
        #[arg(long, default_value_t = 30)]
        days: u64,
        /// Write output to .ci/metrics/release-health.json
        #[arg(long)]
        json: bool,
    },
    /// Check scorecard floor metrics against the committed baseline.
    ///
    /// Loads `.ci/metrics/baselines/<subsystem>.json` and compares against the
    /// current metric receipt.  Exits nonzero on any floor breach.
    RatchetCheck {
        /// Subsystem name (e.g. "parser", "engineering_health").
        subsystem: String,
        /// Path to current-metrics JSON (default: target/receipts/metrics/<subsystem>.json).
        #[arg(long)]
        current: Option<PathBuf>,
        /// Record this run in target/metrics/stable_wins/<subsystem>.json.
        #[arg(long)]
        record: bool,
    },
    /// Show which improvement metrics are stable enough to raise the floor baseline.
    PromoteBaseline {
        /// Subsystem name.
        subsystem: String,
        /// Minimum fractional improvement required (default: 1%).
        #[arg(long, default_value_t = 0.01)]
        delta_pct: f64,
    },
    /// Summarize a parser corpus sweep receipt (phase timings, slowest files,
    /// median error density, first-error buckets).
    ///
    /// Reads the JSON written by `cargo xtask parser-corpus-sweep --receipt`
    /// (or any other path via `--input`) and emits the same human-readable
    /// report that the sweep prints at end-of-run — useful for analyzing
    /// historical receipts without re-running the sweep.
    SweepStats {
        /// Path to a sweep receipt JSON. Defaults to
        /// `target/receipts/system-corpus-sweep.json`.
        #[arg(long)]
        input: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum MemoryTrendsCommand {
    /// Render memory plateau trends from receipts and baseline files.
    Render {
        /// Directory containing current memory receipts or plateau JSON files.
        #[arg(long, default_value = "target/memory")]
        input_dir: PathBuf,
        /// Additional historical receipt directories.
        #[arg(long = "history-dir")]
        history_dirs: Vec<PathBuf>,
        /// Committed baseline file to include when present.
        #[arg(long, default_value = ".ci/metrics/baselines/memory_plateau.json")]
        baseline: PathBuf,
        /// Output markdown path.
        #[arg(long, default_value = "docs/project/status/memory_plateau_trends.md")]
        output: PathBuf,
    },
}

#[derive(Subcommand)]
enum NativeFormatCommand {
    /// Run native formatter fixture checks and write JSON receipts.
    Check {
        /// Directory containing native formatter fixtures.
        #[arg(long, default_value = "crates/perl-lsp-perltidy/tests/fixtures/native_formatter")]
        fixtures: PathBuf,

        /// Directory for native formatter receipts.
        #[arg(long, default_value = "target/receipts/format")]
        receipt_dir: PathBuf,
    },
    /// Run native formatter corpus checks and write JSON/markdown receipts.
    Corpus {
        /// Files or directories containing corpus Perl sources. Defaults to examples/perl,
        /// tests/perl-corpus, and crates/perl-corpus/fixtures/parser_accuracy.
        #[arg(long = "root")]
        roots: Vec<PathBuf>,

        /// Output JSON receipt path.
        #[arg(long, default_value = "target/receipts/format/native-format-corpus.json")]
        receipt: PathBuf,

        /// Output markdown summary path.
        #[arg(long, default_value = "target/receipts/format/native-format-corpus-summary.md")]
        summary: PathBuf,
    },
    /// Classify a .perltidyrc-style profile against native formatter compatibility.
    PerltidyCompat {
        /// Path to the `.perltidyrc` profile to classify.
        #[arg(long)]
        profile: PathBuf,

        /// Output JSON receipt path.
        #[arg(long, default_value = "target/receipts/format/native-format-perltidy-compat.json")]
        receipt: PathBuf,

        /// Output markdown summary path.
        #[arg(long, default_value = "target/receipts/format/native-format-perltidy-compat.md")]
        summary: PathBuf,
    },
    /// Report the effective native formatter configuration surface.
    Config {
        /// Workspace root used to discover `.perl-lsp.toml`.
        #[arg(long, default_value = ".")]
        workspace_root: PathBuf,

        /// Output JSON receipt path.
        #[arg(long, default_value = "target/receipts/format/native-format-config.json")]
        receipt: PathBuf,

        /// Output markdown summary path.
        #[arg(long, default_value = "target/receipts/format/native-format-config.md")]
        summary: PathBuf,
    },
}

#[derive(Subcommand)]
enum NativeCriticCommand {
    /// Run native critic rules over Perl source files and write receipts.
    Check {
        /// Files or directories containing Perl sources. Defaults to examples/perl,
        /// tests/perl-corpus, and crates/perl-corpus/fixtures/parser_accuracy.
        #[arg(long = "root")]
        roots: Vec<PathBuf>,

        /// Minimum native critic severity to report.
        #[arg(long, default_value_t = 3)]
        severity: u8,

        /// Native critic profile to run: recommended or strict.
        #[arg(long, default_value = "recommended")]
        profile: String,

        /// Native rule IDs to include. Empty means all selected-profile rules.
        #[arg(long = "include")]
        include: Vec<String>,

        /// Native rule IDs to exclude.
        #[arg(long = "exclude")]
        exclude: Vec<String>,

        /// Output JSON receipt path.
        #[arg(long, default_value = "target/receipts/native-tooling/native-critic-check.json")]
        receipt: PathBuf,

        /// Output markdown summary path.
        #[arg(long, default_value = "target/receipts/native-tooling/native-critic-check.md")]
        summary: PathBuf,
    },
}

#[derive(Subcommand)]
// Native tooling status intentionally exposes many receipt path flags; boxing
// clap fields would trade a diagnostic-only enum size for noisier CLI plumbing.
#[allow(clippy::large_enum_variant)]
enum NativeToolingCommand {
    /// Write native formatter and critic status receipts.
    Status {
        /// Directory containing native formatter fixtures.
        #[arg(long, default_value = "crates/perl-lsp-perltidy/tests/fixtures/native_formatter")]
        format_fixtures: PathBuf,

        /// Native-format fixture receipt to summarize.
        #[arg(long, default_value = "target/receipts/format/native-format-fixtures.json")]
        format_receipt: PathBuf,

        /// Native-format corpus receipt to summarize.
        #[arg(long, default_value = "target/receipts/format/native-format-corpus.json")]
        format_corpus_receipt: PathBuf,

        /// Native-format perltidy compatibility receipt to summarize.
        #[arg(long, default_value = "target/receipts/format/native-format-perltidy-compat.json")]
        format_perltidy_compat_receipt: PathBuf,

        /// Native-format config receipt to summarize.
        #[arg(long, default_value = "target/receipts/format/native-format-config.json")]
        format_config_receipt: PathBuf,

        /// Native critic perlcritic compatibility receipt to summarize.
        #[arg(long, default_value = "target/receipts/native-tooling/perlcritic-compat.json")]
        critic_perlcritic_compat_receipt: PathBuf,

        /// Native critic check receipt to summarize.
        #[arg(long, default_value = "target/receipts/native-tooling/native-critic-check.json")]
        critic_check_receipt: PathBuf,

        /// Native critic false-positive fixture receipt to summarize.
        #[arg(
            long,
            default_value = "target/receipts/native-tooling/native-critic-false-positive.json"
        )]
        critic_false_positive_receipt: PathBuf,

        /// Output path for native-tooling status JSON.
        #[arg(long, default_value = "target/receipts/native-tooling/status.json")]
        receipt: PathBuf,

        /// Optional markdown status output.
        #[arg(long)]
        markdown: Option<PathBuf>,
    },

    /// Classify a .perlcriticrc-style profile against native critic compatibility.
    PerlcriticCompat {
        /// Path to the `.perlcriticrc` profile to classify.
        #[arg(long)]
        profile: PathBuf,

        /// Output JSON receipt path.
        #[arg(long, default_value = "target/receipts/native-tooling/perlcritic-compat.json")]
        receipt: PathBuf,

        /// Output markdown summary path.
        #[arg(long, default_value = "target/receipts/native-tooling/perlcritic-compat.md")]
        summary: PathBuf,
    },

    /// Verify native tooling defaults do not silently shell out.
    CheckDefaults {
        /// Repository root used for policy source checks.
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },

    /// Render native tooling default-cutover readiness from status receipts.
    Readiness {
        /// Native-tooling status receipt to evaluate.
        #[arg(long, default_value = "target/receipts/native-tooling/status.json")]
        status_receipt: PathBuf,

        /// Output path for native-tooling readiness JSON.
        #[arg(long, default_value = "target/receipts/native-tooling/readiness.json")]
        receipt: PathBuf,

        /// Optional markdown readiness output.
        #[arg(long)]
        markdown: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum CiSubcommand {
    /// Run local/CI parity diagnostic: toolchain pin, components, git state, fmt drift, Perl, binary.
    Doctor,

    /// Emit an advisory changed-file proof-pack route receipt.
    Route {
        /// Git base ref used for changed-file detection.
        #[arg(long, default_value = "origin/main")]
        base: String,

        /// Git head ref used for changed-file detection.
        #[arg(long, default_value = "HEAD")]
        head: String,

        /// Output path for the route receipt.
        #[arg(long, default_value = "target/receipts/ci-route.json")]
        receipt: PathBuf,

        /// Output path for the Markdown route summary.
        #[arg(long, default_value = "target/receipts/ci-route.md")]
        summary: PathBuf,

        /// Explicit changed file path. Repeat for tests or disconnected runs; when omitted, git diff is used.
        #[arg(long = "changed-file")]
        changed_file: Vec<String>,
    },
}

#[derive(Subcommand)]
enum PrSubcommand {
    /// Validate a PR title matches the required format and references an open issue.
    ///
    /// Mirrors the `validate-title` GitHub Actions check for local pre-push use.
    /// Pass a title string directly or omit to read from `git log -1 --pretty=%s`.
    #[command(name = "title-check")]
    TitleCheck {
        /// PR title to validate. If omitted, reads the HEAD commit subject.
        title: Option<String>,

        /// Emit a JSON receipt instead of human-readable output.
        #[arg(long)]
        json: bool,

        /// Exit 1 on warnings (e.g. closed issue) in addition to hard failures.
        #[arg(long)]
        strict: bool,

        /// Skip the GitHub issue-existence API call.
        #[arg(long)]
        no_gh: bool,
    },
}

#[derive(Subcommand)]
enum DevexCommand {
    /// Plan the cheapest correct local proof commands for the current diff.
    Plan {
        /// Git base ref used for changed-file detection.
        #[arg(long, default_value = "auto")]
        base: String,
    },

    /// Emit a JSON receipt for the current local proof plan.
    Receipt {
        /// Git base ref used for changed-file detection.
        #[arg(long, default_value = "auto")]
        base: String,

        /// Output path for the JSON receipt.
        #[arg(long, default_value = "target/devex/local-proof.json")]
        output: PathBuf,
    },

    /// Show a local PR cockpit summary for the current diff.
    Cockpit {
        /// Git base ref used for changed-file detection.
        #[arg(long, default_value = "auto")]
        base: String,

        /// Output path for the JSON receipt refreshed by the cockpit.
        #[arg(long, default_value = "target/devex/local-proof.json")]
        receipt: PathBuf,
    },

    /// Print a paste-ready PR proof packet for the current diff.
    PrBody {
        /// Git base ref used for changed-file detection.
        #[arg(long, default_value = "auto")]
        base: String,

        /// Receipt path referenced by the generated PR body.
        #[arg(long, default_value = "target/devex/local-proof.json")]
        receipt: PathBuf,
    },
}

#[derive(Subcommand)]
enum QueueCommand {
    /// Capture the open PR queue into a stable JSON snapshot document.
    Snapshot {
        /// Output file for the generated snapshot JSON.
        #[arg(long)]
        out: PathBuf,

        /// Optional fixture JSON to parse instead of live GitHub data.
        #[arg(long)]
        fixture: Option<PathBuf>,
    },

    /// Classify master queue health into GREEN/PENDING/RED modes.
    Health {
        /// Output path for queue-health receipt JSON.
        #[arg(long)]
        receipt: Option<PathBuf>,

        /// Fixture JSON input for deterministic health classification.
        #[arg(long)]
        fixture: Option<PathBuf>,
    },

    /// Project UI labels from canonical PR state receipts.
    ProjectLabels {
        /// Path to canonical queue state JSON.
        #[arg(long, default_value = "target/receipts/queue-state.json")]
        state: PathBuf,

        /// Plan label changes without applying them (default).
        #[arg(long)]
        dry_run: bool,

        /// Apply projected label changes against GitHub. Requires GH_TOKEN.
        #[arg(long)]
        apply: bool,

        /// Optional output path for the label-projection receipt JSON.
        #[arg(long)]
        receipt: Option<PathBuf>,

        /// Path to projection rules TOML.
        #[arg(long, default_value = ".ci/state/label-projection.toml")]
        config: PathBuf,
    },
}

#[derive(Subcommand)]
enum SmokeCommand {
    /// Verify textDocument/inlineCompletion over stdio against a built binary.
    #[command(name = "inline-completion")]
    InlineCompletion {
        /// Path to the perl-lsp binary to execute.
        #[arg(long)]
        binary: PathBuf,
    },
}

#[derive(Subcommand)]
enum AgentCommand {
    /// Lease lifecycle commands.
    Lease {
        #[command(subcommand)]
        command: AgentLeaseCommand,
    },
    /// Receipt commands.
    Receipt {
        #[command(subcommand)]
        command: AgentReceiptCommand,
    },
    /// Manage leased local worktrees for agent orchestration.
    Worktree {
        #[command(subcommand)]
        command: AgentWorktreeCommand,
    },
}

#[derive(Subcommand)]
enum AgentLeaseCommand {
    /// Acquire a lease from a typed task JSON.
    Acquire {
        /// Path to task JSON.
        #[arg(long)]
        task: PathBuf,
        /// Path to write lease JSON.
        #[arg(long)]
        out: PathBuf,
    },
    /// Verify lease against current snapshot state.
    Verify {
        /// Path to lease JSON.
        #[arg(long)]
        lease: PathBuf,
        /// Path to current snapshot JSON.
        #[arg(long)]
        current: PathBuf,
    },
}

#[derive(Subcommand)]
enum AgentReceiptCommand {
    /// Validate a receipt against its lease and mutation rules.
    Validate {
        /// Path to receipt JSON.
        #[arg(long)]
        receipt: PathBuf,
    },
}

#[derive(ValueEnum, Clone)]
enum PrepCratesMode {
    Core,
    All,
}

#[derive(ValueEnum, Clone)]
enum UxScorecardOutputFormat {
    Human,
    Json,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    match cli.command {
        Commands::List => {
            print_top_level_commands();
            Ok(())
        }
        Commands::Ci { command } => match command {
            None => ci::run(),
            Some(CiSubcommand::Doctor) => ci_doctor::run(),
            Some(CiSubcommand::Route { base, head, receipt, summary, changed_file }) => {
                ci_route::run(ci_route::CiRouteArgs {
                    base,
                    head,
                    receipt,
                    summary,
                    changed_files: changed_file,
                })
            }
        },
        Commands::CheckOnly => ci::check_only(),
        Commands::CheckLintPolicy => check_lint_policy::run(),
        Commands::CheckToolchain { doctor } => check_toolchain::run(doctor),
        Commands::CheckDevexDocs => devex_docs::run(),
        Commands::CheckProviderConfidenceMatrix => provider_confidence_matrix::run(),
        Commands::CheckSupportClaims => provider_confidence_matrix::run_support_claims(),
        Commands::CheckActiveGoalManifest => active_goal_manifest::run(),
        Commands::CheckProviderPromotionLedger => provider_promotion_ledger::run(),
        Commands::CheckOracleFixtureManifest => oracle_fixture_manifest::run(),
        Commands::CheckOracleReceiptSchema => oracle_receipt_schema::run(),
        Commands::CheckSemanticTokenClasses => semantic_token_classes::run(),
        Commands::CheckLsp318Claims => lsp_318_claims::run(),
        Commands::GenerateLsp318Matrix { check } => lsp_318_matrix::run(check),
        Commands::CheckWorkspaceSymbolClasses => workspace_symbol_classes::run(),
        Commands::Queue { command } => match command {
            QueueCommand::Snapshot { out, fixture } => queue_snapshot::run_snapshot(out, fixture),
            QueueCommand::Health { receipt, fixture } => {
                queue_health::run(queue_health::QueueHealthArgs { receipt, fixture })
            }
            QueueCommand::ProjectLabels { state, dry_run, apply, receipt, config } => {
                label_projector::run_project_labels(label_projector::LabelProjectorArgs {
                    state,
                    dry_run,
                    apply,
                    receipt,
                    config,
                })
            }
        },
        Commands::Pr { command } => match command {
            PrSubcommand::TitleCheck { title, json, strict, no_gh } => {
                tasks::pr::title_check::run(tasks::pr::title_check::TitleCheckConfig {
                    title,
                    json,
                    strict,
                    no_gh,
                })
            }
        },
        Commands::Build { release, features, c_scanner, rust_scanner } => {
            build::run(release, features, c_scanner, rust_scanner)
        }
        Commands::Test { release, suite, features, verbose, coverage } => {
            test::run(release, suite, features, verbose, coverage)
        }
        Commands::Smoke { command } => match command {
            SmokeCommand::InlineCompletion { binary } => inline_completion_smoke::run(binary),
        },
        Commands::InlineCompletionSmoke { binary } => inline_completion_smoke::run(binary),
        Commands::InlineCompletionQuality { receipt } => inline_completion_quality::run(receipt),
        Commands::SemanticInlineReceipts { receipt, quality_receipt, next_edit_receipt } => {
            semantic_inline_receipts::run(receipt, quality_receipt, next_edit_receipt)
        }
        Commands::SemanticInlineNextEdit { receipt } => semantic_inline_next_edit::run(receipt),
        Commands::SupportedEditorInlineSmoke { receipt } => {
            supported_editor_inline_smoke::run(receipt)
        }
        Commands::Badges { check } => badges::run(check),
        Commands::CoverageBaseline {
            lcov,
            receipt,
            codecov,
            patch_coverage,
            patch_base,
            scope,
            check,
        } => quality_baseline::run(quality_baseline::CoverageBaselineArgs {
            lcov,
            receipt,
            codecov,
            patch_coverage,
            patch_base,
            scope,
            check,
        }),
        Commands::QualityGate {
            mode,
            exception_policy,
            ripr_receipt,
            ripr_pr_receipt,
            review_receipt,
            coverage_receipt,
            codecov,
            patch_coverage,
            ripr_base,
            ripr_head,
            receipt,
            summary,
            check,
        } => quality_gate::run(quality_gate::QualityGateArgs {
            mode,
            exception_policy,
            ripr_receipt,
            ripr_pr_receipt,
            review_receipt,
            coverage_receipt,
            codecov,
            patch_coverage,
            ripr_base,
            ripr_head,
            receipt,
            summary,
            check,
        }),
        Commands::RiprPr { root, base, head, check } => {
            ripr_evidence::ripr_pr(&root, &base, &head, check)
        }
        Commands::RiprPlus { root, receipt, suppressions, check } => {
            ripr_evidence::ripr_plus(&root, &receipt, &suppressions, check)
        }
        Commands::RiprReviewComments { root, base, head, timeout_seconds, check } => {
            ripr_evidence::ripr_review_comments(&root, &base, &head, timeout_seconds, check)
        }
        Commands::RiprPrSummary { check } => ripr_evidence::ripr_pr_summary(check),
        Commands::RiprAnnotations { comments, out, check } => {
            ripr_evidence::ripr_annotations(&comments, &out, check)
        }
        Commands::ImpactedEvidence { pr_evidence, labels, labels_csv, check } => {
            ripr_evidence::impacted_evidence(&pr_evidence, &labels, labels_csv.as_deref(), check)
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
        Commands::Fmt { check, package } => fmt::run(check, package),
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
        Commands::Devex { command } => match command {
            DevexCommand::Plan { base } => devex_plan::run(devex_plan::DevexPlanConfig { base }),
            DevexCommand::Receipt { base, output } => {
                devex_plan::write_receipt(devex_plan::DevexReceiptConfig { base, output })
            }
            DevexCommand::Cockpit { base, receipt } => {
                devex_plan::cockpit(devex_plan::DevexCockpitConfig { base, receipt })
            }
            DevexCommand::PrBody { base, receipt } => {
                devex_plan::pr_body(devex_plan::DevexPrBodyConfig { base, receipt })
            }
        },
        Commands::ParseRust { source, sexp, ast, bench } => {
            parse_rust::run(source, sexp, ast, bench)
        }
        Commands::Release { command } => match command {
            ReleaseCommand::Prepare { version, yes } => release::run(version, yes),
            ReleaseCommand::Evidence { version, out } => release_evidence::scaffold(&version, &out),
            ReleaseCommand::VerifyEvidence { version, receipt, bundle_dir } => {
                let effective_bundle_dir = bundle_dir.unwrap_or_else(|| {
                    PathBuf::from(format!("target/release-evidence/v{version}"))
                });
                release_evidence::verify(&version, &effective_bundle_dir, &receipt)
            }
        },
        Commands::ReleaseNotes { tag, output, root } => release_notes::run(tag, output, root),
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
        Commands::WorkflowPolicyLint { receipt, fixture, check_lane_whitelist } => {
            workflow_policy_lint::run(workflow_policy_lint::WorkflowPolicyLintConfig {
                receipt,
                fixture,
                check_lane_whitelist,
            })
        }
        Commands::CiMeasure => ci_measure::run(),
        Commands::CiCostMonitor { days, json } => ci_metrics::run_cost_monitor(days, json),
        Commands::CiBaseline { branch, days, limit, output } => {
            ci_metrics::run_ci_baseline(branch, days, limit, output)
        }
        Commands::CiScope { base, format } => {
            ci_scope::run(ci_scope::CiScopeConfig { base, format })
        }
        Commands::CiPrSummary { base, dry_run } => {
            ci_pr_summary::run(ci_pr_summary::CiPrSummaryConfig { base, dry_run })
        }

        Commands::WorkflowTriggerLint { policy, receipt, fixture, format } => {
            workflow_trigger_lint::run(policy, receipt, fixture, format)
        }
        Commands::CheckVersionSync => check_version_sync::run(),
        Commands::SyncReleaseDocs { write } => sync_release_docs::run(write),
        Commands::CheckFromRaw => ci_policy::check_from_raw(),
        Commands::CheckMemoryLifecyclePolicy => ci_policy::check_memory_lifecycle(),
        Commands::CheckMemoryRetainedOwnerDrift { base, report_only } => {
            ci_policy::check_memory_retained_owner_drift(ci_policy::RetainedOwnerDriftConfig {
                base,
                report_only,
            })
        }
        Commands::MemoryTrends { command } => match command {
            MemoryTrendsCommand::Render { input_dir, history_dirs, baseline, output } => {
                memory_trends::render(memory_trends::MemoryTrendsConfig {
                    input_dir,
                    history_dirs,
                    baseline,
                    output,
                })
            }
        },
        Commands::NativeFormat { command } => match command {
            NativeFormatCommand::Check { fixtures, receipt_dir } => {
                native_format::check(native_format::NativeFormatCheckConfig {
                    fixtures,
                    receipt_dir,
                })
            }
            NativeFormatCommand::Corpus { roots, receipt, summary } => {
                native_format::corpus(native_format::NativeFormatCorpusConfig {
                    roots,
                    receipt,
                    summary,
                })
            }
            NativeFormatCommand::PerltidyCompat { profile, receipt, summary } => {
                native_format::perltidy_compat(native_format::NativeFormatPerltidyCompatConfig {
                    profile,
                    receipt,
                    summary,
                })
            }
            NativeFormatCommand::Config { workspace_root, receipt, summary } => {
                native_format::config(native_format::NativeFormatConfigReceiptConfig {
                    workspace_root,
                    receipt,
                    summary,
                })
            }
        },
        Commands::NativeCritic { command } => match command {
            NativeCriticCommand::Check {
                roots,
                profile,
                severity,
                include,
                exclude,
                receipt,
                summary,
            } => native_critic::check(native_critic::NativeCriticCheckConfig {
                roots,
                profile,
                severity,
                include,
                exclude,
                receipt,
                summary,
            }),
        },
        Commands::NativeTooling { command } => match command {
            NativeToolingCommand::Status {
                format_fixtures,
                format_receipt,
                format_corpus_receipt,
                format_perltidy_compat_receipt,
                format_config_receipt,
                critic_perlcritic_compat_receipt,
                critic_check_receipt,
                critic_false_positive_receipt,
                receipt,
                markdown,
            } => native_tooling::status(native_tooling::NativeToolingStatusConfig {
                format_fixtures,
                format_receipt,
                format_corpus_receipt,
                format_perltidy_compat_receipt,
                format_config_receipt,
                critic_perlcritic_compat_receipt,
                critic_check_receipt,
                critic_false_positive_receipt,
                receipt,
                markdown,
            }),
            NativeToolingCommand::PerlcriticCompat { profile, receipt, summary } => {
                native_tooling::perlcritic_compat(native_tooling::PerlcriticCompatConfig {
                    profile,
                    receipt,
                    summary,
                })
            }
            NativeToolingCommand::CheckDefaults { root } => {
                native_tooling::check_defaults(native_tooling::NativeToolingDefaultsConfig { root })
            }
            NativeToolingCommand::Readiness { status_receipt, receipt, markdown } => {
                native_tooling::readiness(native_tooling::NativeToolingReadinessConfig {
                    status_receipt,
                    receipt,
                    markdown,
                })
            }
        },
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
        Commands::CorpusAuditParseOne { path } => corpus_audit::run_parse_one(path),
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
        Commands::PublishClosure { crate_name } => publish_closure::run(crate_name),
        Commands::PublishedCrateCount => count_ratchet::run(),
        Commands::PublishManifestCheck => publish_manifest_check::run(),
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
        Commands::ParserRatchet { command } => match command {
            ParserRatchetCommand::Run { profile, base, head, receipt, force_selected } => {
                parser_ratchet::run(parser_ratchet::ParserRatchetRunConfig {
                    profile,
                    base,
                    head,
                    receipt,
                    force_selected,
                })
            }
        },
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
        Commands::AggregateReceipts { check, inputs, output, allow_noop } => {
            aggregate_receipts::run(aggregate_receipts::AggregateReceiptsConfig {
                check,
                inputs,
                output,
                allow_noop,
            })
        }
        Commands::FinalizeCheck { receipt, allow_noop, fail_on_advisory } => {
            finalize_check::run(finalize_check::FinalizeCheckConfig {
                receipt,
                allow_noop,
                fail_on_advisory,
            })
        }
        Commands::MergeReady { command } => match command {
            MergeReadyCommand::Emit { pr, receipt } => merge_ready::emit(pr, receipt),
            MergeReadyCommand::Verify { pr, fixture } => merge_ready::verify(pr, fixture),
            MergeReadyCommand::Reconcile { apply, dry_run } => {
                let run_dry = !apply || dry_run;
                merge_ready::reconcile(run_dry)
            }
            MergeReadyCommand::ReconcileQueue { apply: _, dry_run, pr, receipt } => {
                // Apply is the default. Only switch to dry-run when --dry-run is explicitly passed.
                let do_apply = !dry_run;
                queue_reconciler::reconcile_queue(do_apply, pr, receipt)
            }
        },
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
        Commands::InstallSurfaceCheck => install_surface_check::run(),
        Commands::IntentDiffGate { pr, fixture, receipt } => {
            intent_diff_gate::run(intent_diff_gate::IntentDiffGateConfig { pr, fixture, receipt })
        }
        Commands::Features { command } => match command {
            FeaturesCommand::SyncDocs => features::sync_docs(),
            FeaturesCommand::Verify => features::verify(),
            FeaturesCommand::Invariants => features::invariants(),
            FeaturesCommand::Report => features::report(),
        },
        Commands::Agent { command } => match command {
            AgentCommand::Lease { command } => match command {
                AgentLeaseCommand::Acquire { task, out } => agent_lease::acquire(&task, &out),
                AgentLeaseCommand::Verify { lease, current } => {
                    agent_lease::verify(&lease, &current)
                }
            },
            AgentCommand::Receipt { command } => match command {
                AgentReceiptCommand::Validate { receipt } => agent_receipt::validate(&receipt),
            },
            AgentCommand::Worktree { command } => worktree_allocator::run(command),
        },
        Commands::FixForward { command } => match command {
            FixForwardCommand::Classify { receipt, output } => {
                fix_forward::classify(receipt, output)
            }
            FixForwardCommand::ListPlaybooks => fix_forward::list_playbooks(),
        },
        Commands::UpdateStatus { write, check, only } => update_status::run(write, check, only),
        Commands::Srp { command } => match command {
            SrpCommand::Microcrates(args) => srp_microcrates::run(args.output),
            SrpCommand::LayerCheck => layer_check::run(),
            SrpCommand::UnwiredScan(args) => unwired_scan::run(UnwiredScanConfig {
                lsp_crate: args.lsp_crate,
                json: args.json,
                check: args.check,
            }),
            SrpCommand::CheckTestWiring => check_test_wiring::run(),
        },
        Commands::SrpMicrocrates { args } => srp_microcrates::run(args.output),
        Commands::LayerCheck => layer_check::run(),
        Commands::UnwiredScan { args } => unwired_scan::run(UnwiredScanConfig {
            lsp_crate: args.lsp_crate,
            json: args.json,
            check: args.check,
        }),
        Commands::CheckTestWiring => check_test_wiring::run(),
        Commands::Metrics { command } => match command {
            MetricsCommand::ParserStats { input, json } => metrics::parser_stats::run(input, json),
            MetricsCommand::ParserAccuracy {
                json,
                check,
                export_status_receipts,
                manifest,
                output,
                cadence,
            } => metrics::parser_accuracy::run(
                json,
                check,
                export_status_receipts,
                manifest,
                output,
                &cadence,
            ),
            MetricsCommand::HirCoverage { json, output, write_status, check } => {
                metrics::hir_coverage::run(json, output, write_status, check)
            }
            MetricsCommand::LspStats { json, receipt_dir } => {
                metrics::lsp_stats::run_with_receipt_dir(json, receipt_dir.as_deref())
            }
            MetricsCommand::WorkspaceStats => metrics::workspace_stats::run(),
            MetricsCommand::DiagnosticsStats => metrics::diagnostics_stats::run(),
            MetricsCommand::Memory {
                workload_json,
                plateau_json,
                scenario,
                receipt,
                commit,
                event,
                markdown,
            } => {
                let scenario = match scenario {
                    Some(scenario) => scenario,
                    None => metrics::memory::infer_scenario(&workload_json)
                        .map_err(|error| eyre!(error.to_string()))?,
                };
                metrics::memory::run(metrics::memory::MemoryMetricsConfig {
                    scenario,
                    workload_json,
                    plateau_json,
                    receipt,
                    commit,
                    event,
                    markdown,
                })
            }
            MetricsCommand::ReleaseHealth { days, json } => {
                metrics::release_health::run(days, json)
            }
            MetricsCommand::RatchetCheck { subsystem, current, record } => {
                let root = utils::project_root()?;
                metrics::ratchet::run_ratchet_check(&root, &subsystem, current, record)
            }
            MetricsCommand::PromoteBaseline { subsystem, delta_pct } => {
                let root = utils::project_root()?;
                metrics::ratchet::run_promote_baseline(&root, &subsystem, delta_pct)
            }
            MetricsCommand::SweepStats { input } => metrics::sweep_stats::run(input),
        },
        Commands::UxScorecard { format, input, output, status_md, ratchet_check } => {
            let format = match format {
                UxScorecardOutputFormat::Human => UxScorecardFormat::Human,
                UxScorecardOutputFormat::Json => UxScorecardFormat::Json,
            };
            ux_scorecard::run(format, input, output, status_md, ratchet_check)
        }
        Commands::SemanticScorecard { manifest, output, status_md, check } => {
            semantic_scorecard::run(manifest, output, status_md, check)
        }
        Commands::SemanticShadowCompare { output, status_md, check } => {
            semantic_shadow_compare::run(output, status_md, check)
        }
        Commands::UxRegressionReceipt { input, receipt, sha } => {
            ux_regression_receipt::run(ux_regression_receipt::UxRegressionReceiptConfig {
                input,
                receipt,
                sha,
            })
        }
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
            base,
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
            base_ref: base,
            output_format: format,
            emit_receipt: receipt,
            receipt_path,
            diff_baseline: diff,
            list_only: list,
            fail_fast,
            parallel,
            verbose,
        }),
        Commands::GatePolicy { command } => match command {
            GatePolicyCommand::Check => tasks::gate_policy::check(),
            GatePolicyCommand::Effective { profile } => tasks::gate_policy::effective(profile),
        },
        Commands::GateReceipts { command } => match command {
            GateReceiptsCommand::List { format } => {
                gate_receipts::list(convert_gate_receipts_format(format))
                    .map_err(|error| eyre!(error.to_string()))
            }
            GateReceiptsCommand::Validate { path, format } => {
                gate_receipts::validate(&path, convert_gate_receipts_format(format))
                    .map_err(|error| eyre!(error.to_string()))
            }
            GateReceiptsCommand::ValidateAll { dir, format } => {
                gate_receipts::validate_all(&dir, convert_gate_receipts_format(format))
                    .map_err(|error| eyre!(error.to_string()))
            }
        },
        Commands::MethodologyGate { fixture, pr, receipt, dry_run, enforce, format } => {
            methodology_gate::run(methodology_gate::MethodologyGateConfig {
                fixture,
                pr,
                receipt,
                dry_run,
                enforce,
                format,
            })
        }
        Commands::TargetedChecks { base, mode } => targeted_checks::run(base, mode),
        Commands::ResolvePackageName { crate_dir } => {
            // Use the current working directory as workspace root so this subcommand
            // works correctly both in the main workspace and in test synthetic workspaces.
            let root = std::env::current_dir()
                .map_err(|e| eyre!("Failed to get current working directory: {e}"))?;
            let name = tasks::targeted_checks::resolve_single_package_name(&root, &crate_dir)?;
            println!("{name}");
            Ok(())
        }
        Commands::WorktreeCleanup => worktrees::cleanup(),
        Commands::ValidateSwarmAgentRoster { root } => swarm_agent_roster::run(root),
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
        Commands::GeneratedFiles { command } => match command {
            GeneratedFilesCommand::List { fixture } => generated_files::list(fixture),
            GeneratedFilesCommand::Check {
                receipt,
                fixture,
                generator_receipt,
                allow_manual_edits,
            } => generated_files::check(receipt, fixture, generator_receipt, allow_manual_edits),
        },
        Commands::NonRust { command } => match command {
            NonRustCommand::Inventory => {
                let root = utils::project_root()?;
                tasks::file_policy::non_rust_inventory(&root)
            }
            NonRustCommand::Check { mode, json, allowlist, root: root_override } => {
                use tasks::file_policy::{CheckFilePolicyConfig, CheckFilePolicyMode};
                let root = utils::project_root()?;
                let mode = match mode {
                    CheckFilePolicyCliMode::Advisory => CheckFilePolicyMode::Advisory,
                    CheckFilePolicyCliMode::BlockingAllowlist => {
                        CheckFilePolicyMode::BlockingAllowlist
                    }
                    CheckFilePolicyCliMode::BlockingStrict => CheckFilePolicyMode::BlockingStrict,
                };
                tasks::file_policy::check_file_policy(
                    &root,
                    CheckFilePolicyConfig {
                        mode,
                        json_output: json,
                        allowlist_path: allowlist,
                        root_override,
                    },
                )
            }
            NonRustCommand::Propose { output_dir, group_by, root: root_override } => {
                use tasks::file_policy::{ProposeConfig, ProposeGroupBy};
                let root = utils::project_root()?;
                let group_by = match group_by {
                    ProposeGroupByArg::Directory => ProposeGroupBy::Directory,
                    ProposeGroupByArg::Extension => ProposeGroupBy::Extension,
                };
                tasks::file_policy::non_rust_propose(
                    &root,
                    ProposeConfig { output_dir, group_by, root_override },
                )
            }
            NonRustCommand::ValidatePolicy { allowlist, debt } => {
                use tasks::file_policy::ValidateNonRustPolicyConfig;
                tasks::file_policy::validate_non_rust_policy(ValidateNonRustPolicyConfig {
                    allowlist_path: allowlist,
                    debt_path: debt,
                })
            }
            NonRustCommand::MigrationCandidates { format, output, limit, root: root_override } => {
                use tasks::file_policy::{MigrationCandidateFormat, MigrationCandidatesConfig};
                let root = utils::project_root()?;
                let format = match format {
                    MigrationCandidateFormatArg::Markdown => MigrationCandidateFormat::Markdown,
                    MigrationCandidateFormatArg::Json => MigrationCandidateFormat::Json,
                };
                tasks::file_policy::non_rust_migration_candidates(
                    &root,
                    MigrationCandidatesConfig { format, output, limit, root_override },
                )
            }
        },
        Commands::CheckFilePolicy { mode, json, allowlist, root: root_override } => {
            use tasks::file_policy::{CheckFilePolicyConfig, CheckFilePolicyMode};
            let root = utils::project_root()?;
            let mode = match mode {
                CheckFilePolicyCliMode::Advisory => CheckFilePolicyMode::Advisory,
                CheckFilePolicyCliMode::BlockingAllowlist => CheckFilePolicyMode::BlockingAllowlist,
                CheckFilePolicyCliMode::BlockingStrict => CheckFilePolicyMode::BlockingStrict,
            };
            tasks::file_policy::check_file_policy(
                &root,
                CheckFilePolicyConfig {
                    mode,
                    json_output: json,
                    allowlist_path: allowlist,
                    root_override,
                },
            )
        }
        Commands::FreshnessCheck {
            base,
            mode,
            json,
            no_fetch,
            allow_historical,
            reason,
            binaries,
        } => {
            use tasks::freshness_check::{FreshnessCheckConfig, FreshnessMode};
            let mode = match mode {
                FreshnessCheckMode::Warn => FreshnessMode::Warn,
                FreshnessCheckMode::Block => FreshnessMode::Block,
            };
            tasks::freshness_check::run(FreshnessCheckConfig {
                base,
                mode,
                json_output: json,
                no_fetch,
                allow_historical,
                reason,
                check_binaries: binaries,
            })
        }
    }
}

fn print_top_level_commands() {
    let mut command_names = Cli::command()
        .get_subcommands()
        .map(|subcommand| subcommand.get_name().to_string())
        .collect::<Vec<_>>();
    command_names.sort_unstable();

    for command_name in command_names {
        println!("{command_name}");
    }
}

fn convert_gate_receipts_format(format: GateReceiptsFormat) -> gate_receipts::OutputFormat {
    match format {
        GateReceiptsFormat::Human => gate_receipts::OutputFormat::Human,
        GateReceiptsFormat::Json => gate_receipts::OutputFormat::Json,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

    fn parse_devex_command(args: &[&str]) -> TestResult<DevexCommand> {
        match Cli::try_parse_from(args)?.command {
            Commands::Devex { command } => Ok(command),
            _ => Err(std::io::Error::other("expected devex command").into()),
        }
    }

    #[test]
    fn devex_commands_default_to_auto_base() -> TestResult {
        let cases = [
            (["xtask", "devex", "plan"].as_slice(), "plan"),
            (["xtask", "devex", "receipt"].as_slice(), "receipt"),
            (["xtask", "devex", "cockpit"].as_slice(), "cockpit"),
            (["xtask", "devex", "pr-body"].as_slice(), "pr-body"),
        ];

        for (args, name) in cases {
            let base = match parse_devex_command(args)? {
                DevexCommand::Plan { base }
                | DevexCommand::Receipt { base, .. }
                | DevexCommand::Cockpit { base, .. }
                | DevexCommand::PrBody { base, .. } => base,
            };
            assert_eq!(base, "auto", "{name} should auto-detect the diff base by default");
        }

        Ok(())
    }

    #[test]
    fn devex_plan_respects_explicit_base() -> TestResult {
        match parse_devex_command(&["xtask", "devex", "plan", "--base", "HEAD~1"])? {
            DevexCommand::Plan { base } => assert_eq!(base, "HEAD~1"),
            _ => return Err(std::io::Error::other("expected devex plan command").into()),
        }

        Ok(())
    }
}
