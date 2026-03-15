//! Xtask automation for tree-sitter-perl
//!
//! This binary provides custom automation tasks for building, testing,
//! and maintaining the tree-sitter-perl project.

use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;
use std::path::PathBuf;

mod tasks;
mod types;
mod utils;
use tasks::gates::{GateTier, OutputFormat};
use tasks::targeted_checks::CheckMode;
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

    /// Generate bindings
    #[cfg(feature = "parser-tasks")]
    Bindings {
        /// Header file to generate bindings from
        #[arg(long, default_value = "crates/tree-sitter-perl-rs/src/tree_sitter/parser.h")]
        header: PathBuf,

        /// Output file for bindings
        #[arg(long, default_value = "crates/tree-sitter-perl-rs/src/bindings.rs")]
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

    /// Run three-way parser comparison
    #[cfg(feature = "legacy")]
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

    /// Bump version numbers across project
    BumpVersion {
        /// New version to set
        version: String,

        /// Skip confirmation
        #[arg(long)]
        yes: bool,
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

    /// Manage feature catalog and LSP compliance
    Features {
        #[command(subcommand)]
        command: FeaturesCommand,
    },

    /// Generate SRP microcrate inventory and split-candidate report
    SrpMicrocrates {
        /// Optional output path (default: docs/SRP_MICROCRATES.md)
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Validate memory profiling functionality
    ValidateMemoryProfiler,

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

    /// Generate compliance report
    Report,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    match cli.command {
        Commands::Ci => ci::run(),
        Commands::CheckOnly => ci::check_only(),
        Commands::Build { release, features, c_scanner, rust_scanner } => {
            build::run(release, features, c_scanner, rust_scanner)
        }
        Commands::Test { release, suite, features, verbose, coverage } => {
            test::run(release, suite, features, verbose, coverage)
        }
        Commands::Bench { name, save, output } => bench::run(name, save, output),
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
        #[cfg(feature = "parser-tasks")]
        Commands::Bindings { header, output } => bindings::run(header, output),
        Commands::Dev { watch, port } => dev::run(watch, port),
        Commands::ParseRust { source, sexp, ast, bench } => {
            parse_rust::run(source, sexp, ast, bench)
        }
        Commands::Release { version, yes } => release::run(version, yes),
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
        Commands::CorpusAudit { corpus_path, output, check, fresh } => {
            corpus_audit::run(corpus_audit::AuditConfig {
                corpus_path,
                output_path: output,
                timeout: std::time::Duration::from_secs(30),
                fresh,
                check,
            })
        }
        #[cfg(feature = "legacy")]
        Commands::CompareThree { verbose, format } => {
            compare_parsers::run_three_way(verbose, format.as_str())
        }
        Commands::TestLsp { create_only, test, cleanup } => {
            test_lsp::run(create_only, test, cleanup)
        }
        Commands::BumpVersion { version, yes } => bump_version::run(version, yes),
        Commands::PublishCrates { yes, dry_run } => publish::publish_crates(yes, dry_run),
        Commands::PublishVscode { yes, token } => publish::publish_vscode(yes, token),
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
                base_roots,
                corpus_roots,
                manifest_path: manifest,
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
                CpanCorpusCommand::Install { dist_list, install_dir, verbose } => {
                    if let Some(dl) = dist_list {
                        config.dist_list = dl;
                    }
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
        Commands::Features { command } => match command {
            FeaturesCommand::SyncDocs => features::sync_docs(),
            FeaturesCommand::Verify => features::verify(),
            FeaturesCommand::Report => features::report(),
        },
        Commands::SrpMicrocrates { output } => srp_microcrates::run(output),
        Commands::ValidateMemoryProfiler => compare::validate_memory_profiling(),
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
    }
}
