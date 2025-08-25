//! Xtask automation for tree-sitter-perl
//!
//! This binary provides custom automation tasks for building, testing,
//! and maintaining the tree-sitter-perl project.

use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;
use std::path::PathBuf;

mod tasks;
mod types;
use tasks::*;
use types::TestSuite;
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
    /// Build the project with various configurations
    Build {
        /// Build in release mode
        #[arg(long)]
        release: bool,

        /// Build with specific features
        #[arg(long, value_delimiter = ',')]
        features: Option<Vec<String>>,

        /// Build only the C scanner
        #[arg(long)]
        c_scanner: bool,

        /// Build only the Rust scanner
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
    Corpus {
        /// Path to corpus directory
        #[arg(long, default_value = "c/test/corpus")]
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
    Bindings {
        /// Output directory for bindings
        #[arg(long, default_value = "crates/tree-sitter-perl/src/bindings")]
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

    /// Run three-way parser comparison
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

    /// Bump version numbers across the project
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

    /// Manage feature catalog and LSP compliance
    Features {
        #[command(subcommand)]
        command: FeaturesCommand,
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
        Commands::Corpus { path, scanner, diagnose, test } => {
            corpus::run(path, scanner, diagnose, test)
        }
        Commands::Highlight { path, scanner } => highlight::run(path, scanner),
        Commands::Clean { all } => clean::run(all),
        Commands::Bindings { output } => bindings::run(output),
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
        Commands::CompareThree { verbose, format } => {
            compare_parsers::run_three_way(verbose, format.as_str())
        }
        Commands::TestLsp { create_only, test, cleanup } => {
            test_lsp::run(create_only, test, cleanup)
        }
        Commands::BumpVersion { version, yes } => bump_version::run(version, yes),
        Commands::PublishCrates { yes, dry_run } => publish::publish_crates(yes, dry_run),
        Commands::PublishVscode { yes, token } => publish::publish_vscode(yes, token),
        Commands::Features { command } => match command {
            FeaturesCommand::SyncDocs => features::sync_docs(),
            FeaturesCommand::Verify => features::verify(),
            FeaturesCommand::Report => features::report(),
        },
    }
}
