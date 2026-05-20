use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "perl-ci-hygiene",
    version = "0.10.0",
    about = "Native Rust versions of CI scripts"
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: CliCommand,
}

#[derive(Subcommand)]
pub(crate) enum CliCommand {
    /// Benchmark perl-parser against tree-sitter-perl-c for standard cases.
    RunParserComparison,
    /// Print and apply environment caps for local safety checks.
    Preflight,
    /// Run cargo test with concurrency caps for Rust tasks.
    TestCapped {
        #[arg(trailing_var_arg = true)]
        cargo_args: Vec<String>,
    },
    /// Run E2E test subset with a shared lock to cap parallel invocations.
    E2eGate {
        #[arg(trailing_var_arg = true)]
        cargo_args: Vec<String>,
    },
    /// Run preflight checks then E2E lock-gated cargo test.
    TestE2ECapped {
        #[arg(trailing_var_arg = true)]
        cargo_args: Vec<String>,
    },
    /// Verify stacker behavior in release/debug modes.
    VerifyStacker,
    /// Run iterative parser validation and related tests/benchmarks.
    TestIterativeParser,
    /// Compare bundled parser artifacts between v2 parser modules.
    CheckV2BundleSync,
    /// Compare benchmark outputs with the Python benchmark comparator.
    CompareBenchmarks {
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Compare modern, C legacy, and parser outputs across sample snippets.
    RunComparison,
    /// Run quick parser benchmarks across preselected fixture files.
    QuickBench,
    /// Run pure-Rust parser benchmark across generated fixture sizes.
    SimpleBench,
    /// Profile stack-overflow behavior in debug-mode parser tests.
    ProfileStackOverflow,
    /// Build cargo package --dry-run for workspace crates with dynamic local patch config.
    CargoPackageWorkspaceDryRun {
        #[arg(trailing_var_arg = true)]
        crates: Vec<String>,
    },
    /// Run perl-parser tests with feature-catalog override fixtures.
    TestWithOverride,
    /// Emit a single initialize request against perl-lsp stdin.
    SimpleLspTest,
    /// Check workspace version sync across every tracked site.
    CheckVersionSync,
    /// Bump the workspace version across every tracked site.
    BumpVersion { version: String },
    /// Run edge case test suites, with optional benchmark/coverage submodes.
    TestEdgeCases {
        #[arg(long)]
        bench: bool,
        #[arg(long)]
        coverage: bool,
    },
    /// Generate lightweight receipt artifacts without running tests.
    QuickReceipts,
    /// Run LSP cancellation tests via pre-built test binary.
    TestLspCancellation,
    /// Generate badges from publication facts and update README files.
    GenerateBadges {
        #[arg(long)]
        check: bool,
    },
    /// Install local development git hooks.
    InstallGithooks,
    /// Check docs for machine-specific paths.
    CheckDocPaths { docs_dir: Option<String> },
    /// Enforce linked-only task-marker policy.
    CheckTodos {
        #[arg(long)]
        list: bool,
    },
    /// Prevent fatal constructs in production crates.
    ForbidFatalConstructs {
        #[arg(short, long)]
        verbose: bool,
    },
    /// Track ignored tests and enforce gate policy.
    IgnoredTestCount {
        #[arg(long)]
        update: bool,
        #[arg(long)]
        check: bool,
    },
    /// Scan docs for documentation hygiene problems.
    CheckDocHygiene,
    /// Enforce ignored test cap and trend baseline.
    CheckIgnored,
    /// Run local development quality checks mirroring CI.
    CheckLocal,
    /// Count missing_docs warnings and enforce baseline ratchet.
    CheckMissingDocs,
    /// Enforce no lock().unwrap() and similar panic-prone calls.
    CheckP0Locks,
    /// Enforce parse-error baseline against corpus audit report.
    CheckParseErrors,
    /// Ensure parser feature matrix stays in sync with latest audit report.
    CheckParserMatrix,
    /// Enforce production unsafe syntax budget.
    CheckUnsafeProd,
    /// Enforce module-scoped unwrap budgets.
    CheckUnwrapsModules,
    /// Enforce production unwrap/panic-family budgets.
    CheckUnwrapsProd,
    /// Enforce no raw print macros in library source (println!/eprintln! belong in tracing).
    CheckPrintInLib,
    /// Execute the quick CI mirror.
    QuickCheck,
    /// Run heredoc integration tests, using xtask when available.
    TestHeredocs,
}
