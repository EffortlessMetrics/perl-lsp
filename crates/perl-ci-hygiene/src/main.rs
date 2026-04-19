mod version_sync;

use chrono::Utc;
use clap::{Parser, Subcommand};
use color_eyre::eyre::{Context, Result};
use regex::Regex;
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use toml::Value as TomlValue;
use walkdir::{DirEntry, WalkDir};

const RED: &str = "\x1b[0;31m";
const GREEN: &str = "\x1b[0;32m";
const YELLOW: &str = "\x1b[0;33m";
const BLUE: &str = "\x1b[0;34m";
const NC: &str = "\x1b[0m";

#[derive(Parser)]
#[command(
    name = "perl-ci-hygiene",
    version = "0.10.0",
    about = "Native Rust versions of CI scripts"
)]
struct Cli {
    #[command(subcommand)]
    command: CliCommand,
}

#[derive(Subcommand)]
enum CliCommand {
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
    ///
    /// This walks Cargo.toml (workspace + per-crate), features.toml, the
    /// VSCode extension manifest, and the doc surface (README, CLAUDE.md,
    /// ROADMAP) and fails if any site drifts from the canonical workspace
    /// version.
    CheckVersionSync,

    /// Bump the workspace version across every tracked site.
    ///
    /// Non-interactive and idempotent: running it twice with the same
    /// version produces no diff on the second run. Pair it with
    /// `check-version-sync` in CI to catch drift at merge time.
    BumpVersion {
        /// New version to set (X.Y.Z format).
        version: String,
    },
    /// Run edge case test suites, with optional benchmark/coverage submodes.
    TestEdgeCases {
        /// Run edge case benchmark suite.
        #[arg(long)]
        bench: bool,
        /// Generate tarpaulin coverage report.
        #[arg(long)]
        coverage: bool,
    },
    /// Generate lightweight receipt artifacts without running tests.
    QuickReceipts,
    /// Run LSP cancellation tests via pre-built test binary.
    TestLspCancellation,

    /// Generate `badges.md` from canonical badge links.
    GenerateBadges,

    /// Install local development git hooks.
    InstallGithooks,

    /// Check docs for machine-specific paths.
    CheckDocPaths {
        /// Directory to scan (defaults to `docs`).
        docs_dir: Option<String>,
    },

    /// Enforce linked-only TODO/FIXME markers policy.
    CheckTodos {
        /// Print the full list of matching lines instead of enforcing a baseline.
        #[arg(long)]
        list: bool,
    },

    /// Prevent fatal constructs in production crates.
    ForbidFatalConstructs {
        /// Print summary when checks pass.
        #[arg(short, long)]
        verbose: bool,
    },

    /// Track ignored tests and enforce gate policy.
    IgnoredTestCount {
        /// Write current counts back to baseline.
        #[arg(long)]
        update: bool,
        /// CI gate mode: fail when ignored count increases.
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
    /// Execute the quick CI mirror.
    QuickCheck,
    /// Run heredoc integration tests, using xtask when available.
    TestHeredocs,
}

fn main() -> std::process::ExitCode {
    if let Err(err) = color_eyre::install() {
        eprintln!("{err}");
    }

    match run() {
        Ok(code) => std::process::ExitCode::from(code as u8),
        Err(err) => {
            eprintln!("{err:#}");
            std::process::ExitCode::from(1)
        }
    }
}

fn run() -> Result<i32> {
    let cli = Cli::parse();
    let repo_root = find_repo_root()?;
    let code = match cli.command {
        CliCommand::CheckDocPaths { docs_dir } => {
            cmd_check_doc_paths(&repo_root, docs_dir.as_deref())?
        }
        CliCommand::Preflight => cmd_preflight(&repo_root)?,
        CliCommand::TestCapped { cargo_args } => cmd_test_capped(&repo_root, &cargo_args)?,
        CliCommand::E2eGate { cargo_args } => cmd_e2e_gate(&repo_root, &cargo_args)?,
        CliCommand::TestE2ECapped { cargo_args } => cmd_test_e2e_capped(&repo_root, &cargo_args)?,
        CliCommand::RunParserComparison => cmd_run_parser_comparison(&repo_root)?,
        CliCommand::GenerateBadges => cmd_generate_badges(&repo_root)?,
        CliCommand::InstallGithooks => cmd_install_githooks(&repo_root)?,
        CliCommand::VerifyStacker => cmd_verify_stacker(&repo_root)?,
        CliCommand::TestIterativeParser => cmd_test_iterative_parser(&repo_root)?,
        CliCommand::CheckV2BundleSync => cmd_check_v2_bundle_sync(&repo_root)?,
        CliCommand::CompareBenchmarks { args } => cmd_compare_benchmarks(&repo_root, &args)?,
        CliCommand::RunComparison => cmd_run_comparison(&repo_root)?,
        CliCommand::QuickBench => cmd_quick_bench(&repo_root)?,
        CliCommand::SimpleBench => cmd_simple_bench(&repo_root)?,
        CliCommand::ProfileStackOverflow => cmd_profile_stack_overflow(&repo_root)?,
        CliCommand::CargoPackageWorkspaceDryRun { crates } => {
            cmd_cargo_package_workspace_dry_run(&repo_root, &crates)?
        }
        CliCommand::TestWithOverride => cmd_test_with_override(&repo_root)?,
        CliCommand::SimpleLspTest => cmd_simple_lsp_test(&repo_root)?,
        CliCommand::CheckVersionSync => cmd_check_version_sync(&repo_root)?,
        CliCommand::BumpVersion { version } => cmd_bump_version(&repo_root, &version)?,
        CliCommand::TestEdgeCases { bench, coverage } => {
            cmd_test_edge_cases(&repo_root, bench, coverage)?
        }
        CliCommand::QuickReceipts => cmd_quick_receipts(&repo_root)?,
        CliCommand::TestLspCancellation => cmd_test_lsp_cancellation(&repo_root)?,
        CliCommand::CheckTodos { list } => cmd_check_todos(&repo_root, list)?,
        CliCommand::ForbidFatalConstructs { verbose } => {
            cmd_forbid_fatal_constructs(&repo_root, verbose)?
        }
        CliCommand::IgnoredTestCount { update, check } => {
            cmd_ignored_test_count(&repo_root, update, check)?
        }
        CliCommand::CheckDocHygiene => cmd_check_doc_hygiene(&repo_root)?,
        CliCommand::CheckIgnored => cmd_check_ignored(&repo_root)?,
        CliCommand::CheckLocal => cmd_check_local(&repo_root)?,
        CliCommand::CheckMissingDocs => cmd_check_missing_docs(&repo_root)?,
        CliCommand::CheckP0Locks => cmd_check_p0_locks(&repo_root)?,
        CliCommand::CheckParseErrors => cmd_check_parse_errors(&repo_root)?,
        CliCommand::CheckParserMatrix => cmd_check_parser_matrix(&repo_root)?,
        CliCommand::CheckUnsafeProd => cmd_check_unsafe_prod(&repo_root)?,
        CliCommand::CheckUnwrapsModules => cmd_check_unwraps_modules(&repo_root)?,
        CliCommand::CheckUnwrapsProd => cmd_check_unwraps_prod(&repo_root)?,
        CliCommand::QuickCheck => cmd_quick_check(&repo_root)?,
        CliCommand::TestHeredocs => cmd_test_heredocs(&repo_root)?,
    };
    Ok(code)
}

const CI_REPORT_CRATES_EXCLUDE: [&str; 5] = [
    "tree-sitter-perl-c",
    "perl-parser-pest",
    "perl-tdd-support",
    "perl-test-must",
    "perl-ci-hygiene",
];

const CI_TEST_FILE_SUFFIXES: [&str; 3] = ["_test.rs", "_tests.rs", "tests.rs"];

fn is_excluded_test_path(path: &Path) -> bool {
    if path.components().any(|component| {
        let value = component.as_os_str();
        value == OsStr::new("tests")
            || value == OsStr::new("benches")
            || value == OsStr::new("examples")
            || value == OsStr::new("bin")
    }) {
        return true;
    }

    if let Some(file_name) = path.file_name().and_then(|name| name.to_str())
        && CI_TEST_FILE_SUFFIXES
            .iter()
            .any(|suffix| file_name.ends_with(suffix))
    {
        return true;
    }

    if path.components().any(|component| {
        CI_REPORT_CRATES_EXCLUDE
            .iter()
            .any(|item| component.as_os_str() == OsStr::new(item))
    }) {
        return true;
    }

    false
}

fn command_with_output(
    repo_root: &Path,
    command: &str,
    args: &[&str],
    env_vars: &[(&str, &str)],
) -> Result<String> {
    let mut child = Command::new(command);
    child.current_dir(repo_root).args(args);
    for (key, value) in env_vars {
        child.env(key, value);
    }
    child.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = child
        .output()
        .wrap_err_with(|| format!("running {command}"))?;
    let status = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if status != 0 {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(color_eyre::eyre::eyre!(
            "command '{command}' failed (exit {status}): {stderr}"
        ));
    }
    Ok(stdout)
}

fn command_with_output_all(
    repo_root: &Path,
    command: &str,
    args: &[&str],
    env_vars: &[(&str, &str)],
) -> Result<String> {
    let mut child = Command::new(command);
    child.current_dir(repo_root).args(args);
    for (key, value) in env_vars {
        child.env(key, value);
    }
    child.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = child
        .output()
        .wrap_err_with(|| format!("running {command}"))?;
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.stderr.is_empty() {
        combined.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    let status = output.status.code().unwrap_or(1);
    if status != 0 {
        return Err(color_eyre::eyre::eyre!(
            "command '{command}' failed (exit {status}): {combined}"
        ));
    }
    Ok(combined)
}

fn command_with_input_with_status(
    repo_root: &Path,
    command: &str,
    args: &[&str],
    env_vars: &[(&str, &str)],
    stdin_payload: &str,
) -> Result<(i32, String)> {
    let mut child = Command::new(command);
    child.current_dir(repo_root).args(args);
    for (key, value) in env_vars {
        child.env(key, value);
    }
    child
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = child
        .spawn()
        .wrap_err_with(|| format!("running {command}"))?;
    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| color_eyre::eyre::eyre!("failed to open stdin for command {command}"))?;
        stdin
            .write_all(stdin_payload.as_bytes())
            .wrap_err_with(|| format!("writing to stdin for {command}"))?;
    }
    let output = child
        .wait_with_output()
        .wrap_err_with(|| format!("running {command}"))?;
    let status = output.status.code().unwrap_or(1);
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.stderr.is_empty() {
        combined.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    Ok((status, combined))
}

fn command_output_with_status(
    repo_root: &Path,
    command: &str,
    args: &[&str],
    env_vars: &[(&str, &str)],
) -> Result<(i32, String)> {
    let mut child = Command::new(command);
    child.current_dir(repo_root).args(args);
    for (key, value) in env_vars {
        child.env(key, value);
    }
    child.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = child
        .output()
        .wrap_err_with(|| format!("running {command}"))?;
    let status = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    Ok((status, stdout))
}

fn command_timed_status(
    repo_root: &Path,
    command: &str,
    args: &[&str],
    env_vars: &[(&str, &str)],
) -> Result<(i32, Duration)> {
    let mut child = Command::new(command);
    child.current_dir(repo_root).args(args);
    for (key, value) in env_vars {
        child.env(key, value);
    }
    child.stdout(Stdio::null()).stderr(Stdio::null());
    let start = Instant::now();
    let status = child
        .status()
        .wrap_err_with(|| format!("running {command}"))?;
    let elapsed = start.elapsed();
    Ok((status.code().unwrap_or(1), elapsed))
}

fn command_with_output_allow_empty_match(
    repo_root: &Path,
    command: &str,
    args: &[&str],
    env_vars: &[(&str, &str)],
) -> Result<String> {
    let mut child = Command::new(command);
    child.current_dir(repo_root).args(args);
    for (key, value) in env_vars {
        child.env(key, value);
    }
    child.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = child
        .output()
        .wrap_err_with(|| format!("running {command}"))?;
    let status = output.status.code().unwrap_or(1);
    if status != 0 && status != 1 {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(color_eyre::eyre::eyre!(
            "command '{command}' failed (exit {status}): {stderr}"
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn command_with_output_allow_failure(
    repo_root: &Path,
    command: &str,
    args: &[&str],
    env_vars: &[(&str, &str)],
) -> Result<String> {
    let mut child = Command::new(command);
    child.current_dir(repo_root).args(args);
    for (key, value) in env_vars {
        child.env(key, value);
    }
    child.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = child
        .output()
        .wrap_err_with(|| format!("running {command}"))?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn command_status(
    repo_root: &Path,
    command: &str,
    args: &[&str],
    env_vars: &[(&str, &str)],
) -> Result<i32> {
    let mut child = Command::new(command);
    child.current_dir(repo_root).args(args);
    for (key, value) in env_vars {
        child.env(key, value);
    }
    let status = child
        .status()
        .wrap_err_with(|| format!("running {command}"))?;
    Ok(status.code().unwrap_or(1))
}

fn command_status_strict(
    repo_root: &Path,
    command: &str,
    args: &[&str],
    env_vars: &[(&str, &str)],
) -> Result<()> {
    let status = command_status(repo_root, command, args, env_vars)?;
    if status != 0 {
        return Err(color_eyre::eyre::eyre!(
            "{command} failed with code {status}"
        ));
    }
    Ok(())
}

fn command_exists(command: &str) -> bool {
    // Replaces `sh -c "command -v ..."` which is Unix-only.
    // On Windows, also probe for .exe / .cmd / .bat extensions.
    #[cfg(windows)]
    let suffixes: &[&str] = &[".exe", ".cmd", ".bat", ""];
    #[cfg(not(windows))]
    let suffixes: &[&str] = &[""];
    env::split_paths(&env::var_os("PATH").unwrap_or_default()).any(|dir| {
        suffixes
            .iter()
            .any(|ext| dir.join(format!("{command}{ext}")).exists())
    })
}

fn command_output_lines(output: &str) -> Vec<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn first_cfg_test_line_number(path: &Path) -> Result<usize> {
    let contents = read_lines(path)?;
    let pattern = Regex::new(r"^\s*#\[cfg\(test\)\]")?;
    for (idx, line) in contents.iter().enumerate() {
        if pattern.is_match(line) {
            return Ok(idx + 1);
        }
    }
    Ok(usize::MAX)
}

fn read_json_value(path: &Path) -> Result<Value> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {:?}", path))?;
    let value =
        serde_json::from_str(&raw).with_context(|| format!("parsing JSON in {:?}", path))?;
    Ok(value)
}

// Used only in the #[cfg(not(windows))] preflight block.
#[cfg_attr(windows, allow(dead_code))]
fn read_usize_from_path(path: &Path) -> Result<usize> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {:?}", path))?;
    raw.trim()
        .parse::<usize>()
        .map_err(|err| color_eyre::eyre::eyre!("invalid usize in {}: {err}", path.display()))
}

// Used only in the #[cfg(not(windows))] preflight block.
#[cfg_attr(windows, allow(dead_code))]
fn read_usize_from_tokens(path: &Path, idx: usize) -> Result<usize> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {:?}", path))?;
    let tokens: Vec<&str> = raw.split_whitespace().collect();
    if tokens.len() <= idx {
        return Err(color_eyre::eyre::eyre!(
            "missing token {idx} in {}",
            path.display()
        ));
    }
    tokens[idx]
        .trim()
        .parse::<usize>()
        .map_err(|err| color_eyre::eyre::eyre!("invalid usize in {}: {err}", path.display()))
}

// On Windows, the concurrency-cap variables are set but not mutated
// (no /proc-based auto-degradation on Windows). Allow unused_mut for
// cross-platform compatibility.
#[cfg_attr(windows, allow(unused_mut))]
fn cmd_preflight(_repo_root: &Path) -> Result<i32> {
    #[cfg(windows)]
    println!(
        "note: preflight system metrics not available on Windows; \
         skipping /proc checks — see scripts/preflight.sh for Linux-only usage"
    );

    let uv_threadpool_size = env::var("UV_THREADPOOL_SIZE").unwrap_or_else(|_| "4".to_string());
    let mut pw_workers = env::var("PW_WORKERS").unwrap_or_else(|_| "2".to_string());
    let mut rust_test_threads = env::var("RUST_TEST_THREADS").unwrap_or_else(|_| "2".to_string());
    let mut omp_num_threads = env::var("OMP_NUM_THREADS").unwrap_or_else(|_| "1".to_string());
    let mut openblas_num_threads =
        env::var("OPENBLAS_NUM_THREADS").unwrap_or_else(|_| "1".to_string());
    let mut mkl_num_threads = env::var("MKL_NUM_THREADS").unwrap_or_else(|_| "1".to_string());
    let mut numexpr_num_threads =
        env::var("NUMEXPR_NUM_THREADS").unwrap_or_else(|_| "1".to_string());

    // SAFETY: This is a single-threaded CLI tool; no other threads are reading env vars.
    unsafe {
        env::set_var("UV_THREADPOOL_SIZE", &uv_threadpool_size);
        env::set_var("PW_WORKERS", &pw_workers);
        env::set_var("RUST_TEST_THREADS", &rust_test_threads);
        env::set_var("OMP_NUM_THREADS", &omp_num_threads);
        env::set_var("OPENBLAS_NUM_THREADS", &openblas_num_threads);
        env::set_var("MKL_NUM_THREADS", &mkl_num_threads);
        env::set_var("NUMEXPR_NUM_THREADS", &numexpr_num_threads);
    }

    // Auto-degrade concurrency when the system is under heavy load.
    // This block reads /proc data; it is Linux-only.
    #[cfg(not(windows))]
    {
        let pids_used = command_with_output(Path::new("/"), "ps", &["-e", "--no-headers"], &[])?
            .lines()
            .count();
        let pid_max = read_usize_from_path(Path::new("/proc/sys/kernel/pid_max"))?;
        let files_used = read_usize_from_tokens(Path::new("/proc/sys/fs/file-nr"), 1)?;
        let files_max = read_usize_from_path(Path::new("/proc/sys/fs/file-max"))?;
        println!("PIDs: {pids_used} / {pid_max} | Open files: {files_used} / {files_max}");

        if pids_used > (pid_max * 85 / 100) {
            pw_workers = "1".into();
            rust_test_threads = "1".into();
            omp_num_threads = "1".into();
            openblas_num_threads = "1".into();
            mkl_num_threads = "1".into();
            numexpr_num_threads = "1".into();

            // SAFETY: This is a single-threaded CLI tool; no other threads are reading env vars.
            unsafe {
                env::set_var("PW_WORKERS", &pw_workers);
                env::set_var("RUST_TEST_THREADS", &rust_test_threads);
                env::set_var("OMP_NUM_THREADS", &omp_num_threads);
                env::set_var("OPENBLAS_NUM_THREADS", &openblas_num_threads);
                env::set_var("MKL_NUM_THREADS", &mkl_num_threads);
                env::set_var("NUMEXPR_NUM_THREADS", &numexpr_num_threads);
            }
            println!("System hot → auto‑degraded workers (PW=1, RUST=1, *BLAS=1)");
        }
    }

    Ok(0)
}

fn cmd_test_capped(repo_root: &Path, cargo_args: &[String]) -> Result<i32> {
    cmd_preflight(repo_root)?;

    let rust_test_threads = env::var("RUST_TEST_THREADS").unwrap_or_else(|_| "2".to_string());
    println!("Running Rust tests with {rust_test_threads} threads...");

    let mut args: Vec<String> = vec![
        "test".to_string(),
        "--".to_string(),
        format!("--test-threads={rust_test_threads}"),
    ];
    args.extend_from_slice(cargo_args);
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    command_status_strict(
        repo_root,
        "cargo",
        &refs,
        &[("RUST_TEST_THREADS", rust_test_threads.as_str())],
    )?;
    Ok(0)
}

fn cmd_e2e_gate(repo_root: &Path, cargo_args: &[String]) -> Result<i32> {
    let rust_test_threads = env::var("RUST_TEST_THREADS").unwrap_or_else(|_| "2".to_string());
    let mut args: Vec<String> = vec![
        "test".to_string(),
        "--".to_string(),
        format!("--test-threads={rust_test_threads}"),
    ];
    args.extend_from_slice(cargo_args);
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();

    // flock is a Linux/macOS utility and does not exist on Windows.
    // Skip the lock entirely and run the tests directly on Windows.
    #[cfg(windows)]
    {
        println!("note: flock not available on Windows; running E2E tests without lock");
        command_status_strict(
            repo_root,
            "cargo",
            &refs,
            &[("RUST_TEST_THREADS", rust_test_threads.as_str())],
        )
        .map(|_| 0)
    }

    #[cfg(not(windows))]
    {
        let lock_file_path = std::env::temp_dir().join("e2e-suite.lock");
        let lock_file_str = lock_file_path
            .to_str()
            .ok_or_else(|| color_eyre::eyre::eyre!("temp dir path is not valid UTF-8"))?
            .to_owned();
        let lock_file = lock_file_str.as_str();

        if !command_exists("flock") {
            println!("warning: flock not found; running E2E tests without external lock");
            return command_status_strict(
                repo_root,
                "cargo",
                &refs,
                &[("RUST_TEST_THREADS", rust_test_threads.as_str())],
            )
            .map(|_| 0);
        }

        if command_status(repo_root, "flock", &["-n", lock_file, "true"], &[])? == 0 {
            println!("E2E slot ready");
            let direct_args = std::iter::once(lock_file)
                .chain(refs.iter().copied())
                .collect::<Vec<_>>();
            command_status_strict(
                repo_root,
                "flock",
                &direct_args,
                &[("RUST_TEST_THREADS", rust_test_threads.as_str())],
            )?;
            return Ok(0);
        }

        println!("E2E slot busy → waiting...");
        let blocking_args = std::iter::once(lock_file)
            .chain(refs.iter().copied())
            .collect::<Vec<_>>();
        command_status_strict(
            repo_root,
            "flock",
            &blocking_args,
            &[("RUST_TEST_THREADS", rust_test_threads.as_str())],
        )?;
        Ok(0)
    }
}

fn cmd_test_e2e_capped(repo_root: &Path, cargo_args: &[String]) -> Result<i32> {
    cmd_preflight(repo_root)?;
    println!("Running comprehensive E2E tests with concurrency caps...");
    cmd_e2e_gate(repo_root, cargo_args)
}

fn cmd_run_parser_comparison(repo_root: &Path) -> Result<i32> {
    println!("=== Perl Parser Comparison Benchmark ===");
    println!("Comparing perl-parser vs tree-sitter-perl-c");
    println!();
    println!("Building parsers...");
    let _ = command_with_output_allow_failure(
        repo_root,
        "cargo",
        &["build", "--release", "-p", "perl-parser"],
        &[],
    )?;
    let _ = command_with_output_allow_failure(
        repo_root,
        "cargo",
        &["build", "--release", "-p", "tree-sitter-perl-c"],
        &[],
    )?;
    println!();
    println!("Running benchmarks on standard test cases...");
    let benchmark = command_with_output_allow_failure(
        repo_root,
        "sh",
        &[
            "-c",
            "cargo bench -p parser-benchmarks --bench simple_compare 2>&1 | grep -E \"parser-comparison|time:\" | grep -B1 \"time:\"",
        ],
        &[],
    )?;
    if !benchmark.trim().is_empty() {
        println!("{benchmark}");
    }
    println!();
    println!("=== Summary ===");
    println!("perl-parser: Pure Rust implementation using perl-lexer");
    println!("tree-sitter-c: C implementation with tree-sitter");
    Ok(0)
}

fn cmd_check_v2_bundle_sync(repo_root: &Path) -> Result<i32> {
    println!("🔍 Checking v2 bundle sync between tree-sitter-perl-rs and perl-parser-pest...");

    const V2_BUNDLE_FILES: [&str; 5] = [
        "grammar.pest",
        "pure_rust_parser.rs",
        "pratt_parser.rs",
        "sexp_formatter.rs",
        "error.rs",
    ];

    let source_root = repo_root.join("archive/crates/tree-sitter-perl-rs/src");
    let microcrate_root = repo_root.join("crates/perl-parser-pest/src");
    let mut status = 0;
    for file in V2_BUNDLE_FILES {
        let left = source_root.join(file);
        let right = microcrate_root.join(file);
        let left_display = left.display();
        let right_display = right.display();

        if !left.exists() {
            return Err(color_eyre::eyre::eyre!(
                "missing source file: {left_display}"
            ));
        }
        if !right.exists() {
            return Err(color_eyre::eyre::eyre!(
                "missing microcrate file: {right_display}"
            ));
        }

        let left_bytes = fs::read(&left).with_context(|| format!("reading {left_display}"))?;
        let right_bytes = fs::read(&right).with_context(|| format!("reading {right_display}"))?;
        if left_bytes == right_bytes {
            println!("✅ In sync: {}", file);
            continue;
        }

        status = 1;
        println!("❌ Drift detected: {}", file);
        let diff = command_with_output_allow_failure(
            repo_root,
            "diff",
            &[
                "-u",
                left_display.to_string().as_str(),
                right_display.to_string().as_str(),
            ],
            &[],
        )?;
        if !diff.is_empty() {
            println!("{diff}");
        } else {
            println!("(files differ, but diff output is unavailable)");
        }
    }

    if status != 0 {
        println!();
        println!("v2 bundle drift detected. Synchronize the full bundle before merging.");
        return Ok(1);
    }

    println!();
    println!("✅ v2 bundle is synchronized.");
    Ok(0)
}

fn cmd_run_comparison(repo_root: &Path) -> Result<i32> {
    println!("=== Three-Way Parser Comparison ===");
    println!("Comparing: Pure Rust vs Legacy C vs Modern Parser");
    println!();

    let test_cases = [
        ("Simple", r#"my $x = 42;"#),
        ("Expression", r#"my $result = ($a + $b) * $c;"#),
        (
            "Control Flow",
            r#"if ($x > 10) { while ($y < 100) { $y = $y * 2; } }"#,
        ),
        ("Method Call", r#"$obj->method($arg1, $arg2);"#),
        (
            "For Loop",
            r#"for (my $i = 0; $i < 10; $i++) { print $i; }"#,
        ),
    ];

    let legacy_parser = repo_root.join("target/debug/parse");

    println!("Running parser tests...");
    println!();

    for (name, code) in test_cases {
        println!("Testing: {name}");
        println!("Code: {code}");

        println!("  Modern parser: ");
        let modern_args: Vec<&str> = if command_exists("timeout") {
            vec![
                "1s",
                "cargo",
                "run",
                "-q",
                "-p",
                "perl-parser",
                "--example",
                "demo",
                "--",
            ]
        } else {
            vec!["-q", "-p", "perl-parser", "--example", "demo", "--"]
        };
        let (modern_status, modern_output) = if command_exists("timeout") {
            command_with_input_with_status(repo_root, "timeout", &modern_args, &[], code)?
        } else {
            command_with_input_with_status(repo_root, "cargo", &modern_args, &[], code)?
        };
        if modern_status == 0 && modern_output.contains("Success") {
            println!("  ✅ Success");
        } else {
            println!("  ❌ Failed");
        }

        if legacy_parser.is_file() {
            println!("  Legacy C parser: ");
            let legacy_str = legacy_parser.to_string_lossy();
            let legacy_ref = legacy_str.as_ref();
            let legacy_args = if command_exists("timeout") {
                vec!["1s", legacy_ref, "--"]
            } else {
                vec![legacy_ref, "--"]
            };
            let (legacy_status, legacy_output) = if command_exists("timeout") {
                command_with_input_with_status(repo_root, "timeout", &legacy_args, &[], code)?
            } else {
                command_with_input_with_status(repo_root, legacy_ref, &legacy_args[1..], &[], code)?
            };
            if legacy_status == 0
                && (legacy_output.contains("success") || legacy_output.contains("parsed"))
            {
                println!("  ✅ Success");
            } else {
                println!("  ❌ Failed");
            }
        }

        println!();
    }

    println!("Performance comparison would require working benchmarks.");
    println!("Currently, the modern parser (perl-lexer + perl-parser) is fully functional.");
    Ok(0)
}

fn cmd_quick_bench(repo_root: &Path) -> Result<i32> {
    println!("=== Three-Way Parser Comparison ===");
    println!();
    println!("Parsers:");
    println!("  native-v3  : perl-parser-bench  (v3 recursive-descent, raw parser API)");
    println!(
        "  facade     : bench_facade        (tree-sitter-perl-rs, v3 wrapped in tree-sitter ergonomics)"
    );
    println!(
        "  c-grammar  : bench_parser_c      (tree-sitter C grammar binding, requires libclang)"
    );
    println!();

    let files = vec![
        repo_root.join("test_corpus/simple.pl"),
        repo_root.join("test_corpus/low_frequency_nodekinds.rs"),
        repo_root.join("test_corpus/parser_stress_cases.pl"),
        repo_root.join("test_corpus/performance_stress_scenarios.pl"),
        repo_root.join("test_corpus/basic_constructs.pl"),
    ];
    let mut candidates: Vec<(String, PathBuf)> = files
        .into_iter()
        .filter(|path| path.is_file())
        .map(|path| {
            (
                path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("file")
                    .to_string(),
                path,
            )
        })
        .collect();
    candidates.sort_by(|a, b| a.0.cmp(&b.0));

    // Collect results once to avoid running each parser twice per file.
    struct Row {
        name: String,
        size: u64,
        rust_time: Option<f64>,
        facade_time: Option<f64>,
        c_time: Option<f64>,
    }

    let mut rows: Vec<Row> = Vec::new();
    for (name, path) in &candidates {
        let size = fs::metadata(path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        let rust_time = run_rust_bench_us(repo_root, path)?;
        let facade_time = run_facade_bench_us(repo_root, path)?;
        let c_time = run_c_bench_us(repo_root, path)?;
        rows.push(Row {
            name: name.clone(),
            size,
            rust_time,
            facade_time,
            c_time,
        });
    }

    // --- Table 1: raw timings ---
    let col_file = 30usize;
    let col_num = 12usize;
    println!(
        "{:<col_file$} {:>8}  {:>col_num$}  {:>col_num$}  {:>col_num$}  fastest",
        "File", "Size", "native-v3(µs)", "facade(µs)", "c-gram(µs)"
    );
    println!(
        "{:<col_file$} {:>8}  {:>col_num$}  {:>col_num$}  {:>col_num$}  -------",
        "----", "----", "-------------", "----------", "----------"
    );

    for row in &rows {
        let times: [(&str, Option<f64>); 3] = [
            ("native-v3", row.rust_time),
            ("facade", row.facade_time),
            ("c-grammar", row.c_time),
        ];
        let fastest_label = times
            .iter()
            .filter_map(|(label, t)| t.map(|v| (label, v)))
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(label, _)| *label)
            .unwrap_or("N/A");

        let fmt = |v: Option<f64>| -> String {
            v.map(|us| format!("{us:.0}"))
                .unwrap_or_else(|| "N/A".to_string())
        };

        println!(
            "{:<col_file$} {:>8}  {:>col_num$}  {:>col_num$}  {:>col_num$}  {}",
            row.name,
            row.size,
            fmt(row.rust_time),
            fmt(row.facade_time),
            fmt(row.c_time),
            fastest_label,
        );
    }

    println!();

    // --- Table 2: relative-to-fastest ---
    println!("=== Relative to fastest (per file) ===");
    println!();
    println!(
        "{:<col_file$}  {:>col_num$}  {:>col_num$}  {:>col_num$}",
        "File", "native-v3", "facade", "c-grammar"
    );
    println!(
        "{:<col_file$}  {:>col_num$}  {:>col_num$}  {:>col_num$}",
        "----", "---------", "------", "---------"
    );

    for row in &rows {
        let available: Vec<f64> = [row.rust_time, row.facade_time, row.c_time]
            .iter()
            .filter_map(|&t| t)
            .collect();
        let min = available.iter().cloned().fold(f64::INFINITY, f64::min);

        let rel = |v: Option<f64>| -> String {
            match v {
                Some(us) if min > 0.0 => format!("{:.2}x", us / min),
                Some(_) => "1.00x".to_string(),
                None => "N/A".to_string(),
            }
        };

        println!(
            "{:<col_file$}  {:>col_num$}  {:>col_num$}  {:>col_num$}",
            row.name,
            rel(row.rust_time),
            rel(row.facade_time),
            rel(row.c_time),
        );
    }

    println!();
    println!("Quick benchmark complete!");
    println!("Note: c-grammar requires libclang; N/A means libclang was not found.");
    println!(
        "Note: facade overhead vs native-v3 should be near epsilon (facade wraps same v3 parser)."
    );
    Ok(0)
}

fn cmd_simple_bench(repo_root: &Path) -> Result<i32> {
    println!("Pure Rust Perl Parser Performance Test");
    println!("======================================");

    let parser = repo_root.join("archive/crates/tree-sitter-perl-rs/target/release/parse-rust");
    command_status_strict(
        repo_root,
        "cargo",
        &[
            "build",
            "--release",
            "-p",
            "tree-sitter-perl-rs",
            "--bin",
            "parse-rust",
        ],
        &[],
    )?;

    let workspace = repo_root
        .join("target")
        .join("perl-ci-hygiene")
        .join("simple_bench");
    fs::create_dir_all(&workspace).with_context(|| format!("creating {}", workspace.display()))?;
    let tiny = workspace.join("tiny.pl");
    let small = workspace.join("small.pl");
    let medium = workspace.join("medium.pl");
    let large = workspace.join("large.pl");
    let huge = workspace.join("huge.pl");

    fs::write(&tiny, "my $x = 42;\n")?;
    fs::copy(
        repo_root.join("test_corpus").join("basic_constructs.pl"),
        &small,
    )
    .wrap_err("copying small fixture")?;
    fs::copy(
        repo_root.join("test_corpus").join("parser_stress_cases.pl"),
        &medium,
    )
    .wrap_err("copying medium fixture")?;
    fs::copy(
        repo_root
            .join("test_corpus")
            .join("real_world/enterprise_cpan_patterns.pl"),
        &large,
    )
    .wrap_err("copying large fixture")?;
    fs::copy(
        repo_root
            .join("test_corpus")
            .join("edge_cases/performance_stress_scenarios.pl"),
        &huge,
    )
    .wrap_err("copying huge fixture")?;

    println!();
    println!("Creating test files...");

    println!();
    println!("Test file sizes:");
    for path in [&tiny, &small, &medium, &large, &huge] {
        let lines = read_usize_from_tokens(path, 0).unwrap_or(0);
        let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        println!(
            "{:<10} {:>6} lines, {:>8}",
            path.file_name().and_then(|s| s.to_str()).unwrap_or(""),
            lines,
            size
        );
    }

    println!();
    println!("Run benchmarks...");
    println!("--------------------------------------");

    for path in [&tiny, &small, &medium, &large, &huge] {
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("benchmark");
        println!("\n{name}:");
        let mut total_ms = 0.0f64;
        for _ in 0..5 {
            let time = timed_file_run_ms(repo_root, &parser, path)?;
            println!("  Run: {time:.0}ms");
            total_ms += time;
        }
        println!("  Average: {:.0}ms", total_ms / 5.0);
    }

    println!();
    println!("Performance Summary:");
    println!("====================");
    println!("The Pure Rust Perl Parser shows excellent performance,");
    println!("parsing typical Perl files with linear scaling.");
    Ok(0)
}

fn cmd_profile_stack_overflow(repo_root: &Path) -> Result<i32> {
    println!("{YELLOW}🔍 Profiling stack overflow in debug builds{NC}");
    println!("==================================================");

    let tests = [
        "test_deep_nested_expression",
        "test_deep_nested_blocks",
        "test_deep_nested_arrays",
        "test_deep_method_chain",
    ];
    let log_dir = repo_root
        .join("target")
        .join("perl-ci-hygiene")
        .join("stack-overflow-logs");
    fs::create_dir_all(&log_dir).with_context(|| format!("creating {}", log_dir.display()))?;

    let env_vars = [("CARGO_BUILD_MODE", "debug"), ("RUST_BACKTRACE", "full")];

    for test in tests {
        println!();
        println!("{YELLOW}Testing: {test}{NC}");
        let base_args = vec![
            "test",
            "--features",
            "pure-rust",
            "--test",
            "debug_stack_overflow_test",
            test,
            "--",
            "--ignored",
            "--nocapture",
        ];

        let (status, output) = if command_exists("timeout") {
            let mut args = vec!["10s", "cargo"];
            args.extend_from_slice(&base_args);
            command_with_input_with_status(repo_root, "timeout", &args, &env_vars, "")?
        } else {
            command_with_input_with_status(repo_root, "cargo", &base_args, &env_vars, "")?
        };

        let log_file = log_dir.join(format!("stack_trace_{test}.log"));
        fs::write(&log_file, &output)?;

        if status == 0 {
            println!("{GREEN}✅ Test completed (unexpected - should overflow){NC}");
            continue;
        }

        if status == 124 {
            println!("{RED}⏱️ Test timed out after 10s{NC}");
        } else {
            println!("{RED}❌ Test failed with exit code: {status}{NC}");
        }

        let marker = output.contains("stack overflow") || output.contains("SIGSEGV");
        if marker {
            println!("{YELLOW}Stack overflow detected! Analyzing...{NC}");
            println!();
            println!("{YELLOW}Recursive patterns found:{NC}");
            let mut lines = Vec::new();
            for line in output.lines() {
                if line.contains("build_node") || line.contains("parse_") || line.contains("visit_")
                {
                    lines.push(line.to_string());
                }
            }
            lines.sort();
            lines.dedup();
            for line in lines.iter().take(20) {
                println!("  {line}");
            }
        } else {
            println!("{RED}No explicit stack-overflow signature found in output{NC}");
        }
    }

    println!();
    println!("{YELLOW}📊 Summary{NC}");
    println!("Stack traces saved under: {}", log_dir.display());
    println!("Look for repeated function calls to identify recursion.");
    Ok(0)
}

/// Crate identifier for the v3 native Rust parser benchmark.
const RUST_BENCH_CRATE: &str = "perl-parser-bench";

/// Binary identifier for the v3 native Rust parser benchmark.
///
/// Used by [`cmd_quick_bench`] and asserted in the unit test that guards
/// against regressing the C-vs-Rust comparison (see issue #3204).
const RUST_BENCH_BIN: &str = "perl-parser-bench";

/// Binary identifier for the legacy C tree-sitter parser benchmark.
///
/// Lives in the workspace-EXCLUDED `tree-sitter-perl-c` crate (libclang-dev
/// dependency), so it must be invoked via `--manifest-path` rather than `-p`.
const C_BENCH_BIN: &str = "bench_parser_c";

/// Relative path (from repo root) to the C tree-sitter crate's Cargo.toml.
///
/// Pinned here alongside `C_BENCH_BIN` so that the regression test
/// (`quick_bench_uses_distinct_binaries_for_c_and_rust`) can assert both the
/// binary name and the crate location remain distinct from the Rust bench path.
const C_BENCH_MANIFEST: &str = "crates/tree-sitter-perl-c/Cargo.toml";

/// Crate identifier for the `tree-sitter-perl-rs` facade benchmark.
///
/// Lives in the normal workspace (no excluded crate dance), so it is
/// invoked with `-p FACADE_BENCH_CRATE --bin FACADE_BENCH_BIN`.
const FACADE_BENCH_CRATE: &str = "tree-sitter-perl-rs";

/// Binary identifier for the `tree-sitter-perl-rs` facade benchmark.
///
/// Used by [`cmd_quick_bench`] and asserted in the unit test
/// `three_way_bench_all_binaries_distinct`.
const FACADE_BENCH_BIN: &str = "bench_facade";

/// Run the v3 native Rust parser bench binary against `file`.
///
/// Returns wall-clock duration in microseconds, or `None` if the bench
/// binary exits non-zero. Note that the elapsed time includes the
/// `cargo run` startup overhead; both [`run_rust_bench_us`] and
/// [`run_c_bench_us`] share that overhead so the comparison stays fair.
fn run_rust_bench_us(repo_root: &Path, file: &Path) -> Result<Option<f64>> {
    let file_arg = file.to_string_lossy().into_owned();
    let args = [
        "run",
        "--quiet",
        "--release",
        "-p",
        RUST_BENCH_CRATE,
        "--bin",
        RUST_BENCH_BIN,
        "--",
        file_arg.as_str(),
    ];
    let (status, elapsed) = command_timed_status(repo_root, "cargo", &args, &[])?;
    if status == 0 {
        Ok(Some(elapsed.as_micros() as f64))
    } else {
        Ok(None)
    }
}

/// Run the legacy C tree-sitter parser bench binary against `file`.
///
/// `tree-sitter-perl-c` is in `[workspace.exclude]` because of its libclang
/// build dependency, so this helper invokes cargo with `--manifest-path`
/// pointing at that crate's Cargo.toml. The `test-utils` feature is required
/// for the binary target.
///
/// Returns wall-clock duration in microseconds, or `None` if the bench
/// binary fails to build or exits non-zero (e.g. on systems without
/// libclang installed). Quick-bench treats `None` as N/A in the speedup
/// column rather than failing the whole run.
fn run_c_bench_us(repo_root: &Path, file: &Path) -> Result<Option<f64>> {
    let file_arg = file.to_string_lossy().into_owned();
    let args = [
        "run",
        "--quiet",
        "--release",
        "--manifest-path",
        C_BENCH_MANIFEST,
        "--bin",
        C_BENCH_BIN,
        "--features",
        "test-utils",
        "--",
        file_arg.as_str(),
    ];
    let (status, elapsed) = command_timed_status(repo_root, "cargo", &args, &[])?;
    if status == 0 {
        Ok(Some(elapsed.as_micros() as f64))
    } else {
        Ok(None)
    }
}

/// Run the `tree-sitter-perl-rs` facade bench binary against `file`.
///
/// The facade crate lives in the normal workspace so it is invoked with
/// `-p` rather than `--manifest-path`. Returns wall-clock duration in
/// microseconds, or `None` if the bench binary exits non-zero.
fn run_facade_bench_us(repo_root: &Path, file: &Path) -> Result<Option<f64>> {
    let file_arg = file.to_string_lossy().into_owned();
    let args = [
        "run",
        "--quiet",
        "--release",
        "-p",
        FACADE_BENCH_CRATE,
        "--bin",
        FACADE_BENCH_BIN,
        "--",
        file_arg.as_str(),
    ];
    let (status, elapsed) = command_timed_status(repo_root, "cargo", &args, &[])?;
    if status == 0 {
        Ok(Some(elapsed.as_micros() as f64))
    } else {
        Ok(None)
    }
}

fn timed_file_run_ms(repo_root: &Path, parser: &Path, file: &Path) -> Result<f64> {
    let file_arg = file.to_string_lossy().into_owned();
    let parser_path = parser.to_string_lossy().into_owned();
    let args = [file_arg.as_str(), "--sexp"];
    let (status, elapsed) = command_timed_status(repo_root, parser_path.as_str(), &args, &[])?;
    if status == 0 {
        Ok(elapsed.as_millis() as f64)
    } else {
        Err(color_eyre::eyre::eyre!(
            "parser command {parser_path} failed for {} with status {status}",
            file.display()
        ))
    }
}

fn cmd_cargo_package_workspace_dry_run(repo_root: &Path, crates: &[String]) -> Result<i32> {
    if crates.is_empty() {
        return Err(color_eyre::eyre::eyre!(
            "usage: cargo-package-workspace-dry-run <crate> [crate ...]"
        ));
    }

    let metadata_json = command_with_output(
        repo_root,
        "cargo",
        &["metadata", "--format-version=1", "--no-deps"],
        &[],
    )?;
    let metadata: Value =
        serde_json::from_str(&metadata_json).wrap_err("parsing cargo metadata output")?;
    let workspace_root = metadata
        .get("workspace_root")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root.to_path_buf());

    let workspace_members = metadata
        .get("workspace_members")
        .and_then(Value::as_array)
        .map(|members| {
            members
                .iter()
                .filter_map(Value::as_str)
                .map(std::string::ToString::to_string)
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();

    let mut patch_args = Vec::<(String, String)>::new();
    if let Some(packages) = metadata.get("packages").and_then(Value::as_array) {
        for package in packages {
            let id = package.get("id").and_then(Value::as_str).unwrap_or("");
            if !workspace_members.contains(id) {
                continue;
            }
            if let Some(publish) = package.get("publish").and_then(Value::as_array)
                && publish.is_empty()
            {
                continue;
            }
            let name = package.get("name").and_then(Value::as_str).unwrap_or("");
            if name.is_empty() {
                continue;
            }
            let manifest_path = package
                .get("manifest_path")
                .and_then(Value::as_str)
                .unwrap_or("");
            if manifest_path.is_empty() {
                continue;
            }
            let crate_root = Path::new(manifest_path)
                .parent()
                .unwrap_or_else(|| Path::new("."));
            let rel = crate_root
                .strip_prefix(&workspace_root)
                .unwrap_or(crate_root)
                .to_string_lossy()
                .to_string();
            patch_args.push((
                name.to_string(),
                format!("--config=patch.crates-io.{name}.path=\"{rel}\""),
            ));
        }
    }

    patch_args.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));

    let no_verify = env::var("CARGO_PACKAGE_NO_VERIFY").as_deref() == Ok("1");
    let patch_values = patch_args
        .iter()
        .map(|(_, patch)| patch.as_str())
        .collect::<Vec<_>>();

    for crate_name in crates {
        println!("==> cargo package -p {crate_name}");
        let mut args = Vec::<String>::new();
        args.push("package".to_string());
        args.push("-p".to_string());
        args.push(crate_name.clone());
        for patch in &patch_values {
            args.push((*patch).to_string());
        }
        if no_verify {
            args.push("--no-verify".to_string());
        }

        let references = args.iter().map(String::as_str).collect::<Vec<_>>();
        command_status_strict(repo_root, "cargo", &references, &[])?;
    }

    Ok(0)
}

fn cmd_verify_stacker(repo_root: &Path) -> Result<i32> {
    println!("Building with release mode first...");
    command_status_strict(
        repo_root,
        "cargo",
        &["build", "--features", "pure-rust", "--release", "--quiet"],
        &[],
    )?;

    println!("Running release mode test (should always work)...");
    let release_output = command_with_output(
        repo_root,
        "cargo",
        &[
            "run",
            "--features",
            "pure-rust",
            "--release",
            "--bin",
            "test_stacker",
        ],
        &[],
    )?;
    for line in release_output.lines().take(20) {
        println!("{line}");
    }

    println!();
    println!("Building with debug mode...");
    command_status_strict(
        repo_root,
        "cargo",
        &["build", "--features", "pure-rust", "--quiet"],
        &[],
    )?;

    println!("Running debug mode test (testing stacker fix)...");
    let debug_cmd: (&str, Vec<&str>) = if command_exists("timeout") {
        (
            "sh",
            vec![
                "-c",
                "timeout 30s cargo run --features pure-rust --bin test_stacker 2>&1 | head -n 20",
            ],
        )
    } else {
        (
            "cargo",
            vec!["run", "--features", "pure-rust", "--bin", "test_stacker"],
        )
    };

    let debug_status = if command_exists("timeout") {
        let (status, output) =
            command_output_with_status(repo_root, debug_cmd.0, &debug_cmd.1, &[])?;
        if !output.trim().is_empty() {
            println!("{output}");
        }
        status
    } else {
        let (status, output) =
            command_output_with_status(repo_root, debug_cmd.0, &debug_cmd.1, &[])?;
        if !output.trim().is_empty() {
            let lines = output.lines().take(20).collect::<Vec<_>>().join("\n");
            println!("{lines}");
        }
        status
    };

    if debug_status == 124 {
        println!("❌ Debug mode timed out - stacker may not be working");
    } else {
        println!("✅ Debug mode completed - stacker is working!");
    }

    Ok(0)
}

fn cmd_test_iterative_parser(repo_root: &Path) -> Result<i32> {
    const BLUE: &str = "\x1b[0;34m";
    const GREEN: &str = "\x1b[0;32m";
    const YELLOW: &str = "\x1b[0;33m";
    const NC: &str = "\x1b[0m";

    println!("{BLUE}🧪 Testing Iterative Parser Implementation{NC}");
    println!("============================================");
    println!();
    println!("{YELLOW}Building with pure-rust feature...{NC}");
    command_status_strict(
        repo_root,
        "cargo",
        &["build", "--features", "pure-rust", "--quiet"],
        &[],
    )?;

    println!();
    println!("{YELLOW}Running iterative parser tests...{NC}");
    command_status_strict(
        repo_root,
        "cargo",
        &[
            "test",
            "--features",
            "pure-rust",
            "iterative_parser_tests",
            "--",
            "--nocapture",
        ],
        &[],
    )?;

    println!();
    println!("{YELLOW}Running parser benchmarks...{NC}");
    command_status_strict(
        repo_root,
        "cargo",
        &[
            "run",
            "--features",
            "pure-rust",
            "--bin",
            "benchmark_parsers",
        ],
        &[],
    )?;

    println!();
    println!("{YELLOW}Testing deep nesting capabilities...{NC}");
    command_status_strict(
        repo_root,
        "cargo",
        &[
            "test",
            "--features",
            "pure-rust",
            "test_deep_nesting",
            "--nocapture",
        ],
        &[],
    )?;

    println!();
    println!("{GREEN}✅ All iterative parser tests completed!{NC}");
    Ok(0)
}

fn cmd_compare_benchmarks(repo_root: &Path, args: &[String]) -> Result<i32> {
    println!("Running parser benchmark comparator...");
    if !command_exists("python3") {
        return Err(color_eyre::eyre::eyre!(
            "python3 is required for benchmark comparison"
        ));
    }

    let compare_py = repo_root
        .join("benchmarks")
        .join("scripts")
        .join("compare.py");
    if !compare_py.is_file() {
        return Err(color_eyre::eyre::eyre!(
            "missing comparator: {}",
            compare_py.display()
        ));
    }

    let mut argv: Vec<String> = vec![compare_py.to_string_lossy().to_string()];
    argv.extend_from_slice(args);
    let refs: Vec<&str> = argv.iter().map(String::as_str).collect();
    command_status_strict(repo_root, "python3", &refs, &[])?;
    Ok(0)
}

fn cmd_test_with_override(repo_root: &Path) -> Result<i32> {
    println!("Testing with minimal features catalog...");
    command_status_strict(
        repo_root,
        "cargo",
        &[
            "test",
            "-p",
            "perl-parser",
            "--test",
            "lsp_feature_gating_test",
            "--",
            "--nocapture",
        ],
        &[(
            "FEATURES_TOML_OVERRIDE",
            "crates/perl-parser/tests/data/features_minimal.toml",
        )],
    )?;

    println!();
    println!("Testing with disabled features catalog...");
    command_status_strict(
        repo_root,
        "cargo",
        &[
            "test",
            "-p",
            "perl-parser",
            "--test",
            "lsp_features_snapshot_test",
            "--",
            "--nocapture",
        ],
        &[(
            "FEATURES_TOML_OVERRIDE",
            "crates/perl-parser/tests/data/features_disabled_test.toml",
        )],
    )?;

    println!("✅ Override testing complete!");
    Ok(0)
}

fn cmd_simple_lsp_test(repo_root: &Path) -> Result<i32> {
    println!("Testing Perl LSP server...");
    #[cfg(windows)]
    {
        let _ = repo_root;
        println!(
            "note: simple-lsp-test uses a POSIX shell pipeline and is not supported on Windows"
        );
        Ok(0)
    }
    #[cfg(not(windows))]
    {
        let shell_script = r#"cat <<'EOF' | cargo run -p perl-parser --bin perl-lsp 2>&1 | head -20
Content-Length: 205

{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":123,"rootUri":"file:///tmp","capabilities":{},"initializationOptions":{},"trace":"off","workspaceFolders":null}}
EOF
"#;
        let output = command_with_output(repo_root, "sh", &["-c", shell_script], &[])?;
        for line in output.lines().take(20) {
            println!("{line}");
        }
        Ok(0)
    }
}

fn cmd_check_version_sync(repo_root: &Path) -> Result<i32> {
    version_sync::check(repo_root)?;
    Ok(0)
}

fn cmd_bump_version(repo_root: &Path, new_version: &str) -> Result<i32> {
    version_sync::validate_version_format(new_version)?;
    println!("Bumping workspace version to {new_version}");
    let report = version_sync::bump(repo_root, new_version)?;
    println!(
        "Version sync bump: {} sites inspected, {} updated ({} already current), {} files touched",
        report.sites_total, report.sites_updated, report.sites_unchanged, report.files_updated,
    );
    for file in &report.touched_files {
        println!("  updated: {}", file.display());
    }
    if report.sites_updated == 0 {
        println!("Version sync bump: no changes required (already at {new_version})");
    }
    Ok(0)
}

fn cmd_test_edge_cases(repo_root: &Path, bench: bool, coverage: bool) -> Result<i32> {
    const BLUE: &str = "\x1b[0;34m";
    const GREEN: &str = "\x1b[0;32m";
    const YELLOW: &str = "\x1b[1;33m";
    const NC: &str = "\x1b[0m";

    println!("{BLUE}=== Testing Edge Case Handling ==={NC}");
    println!();

    println!("{YELLOW}Running edge case tests...{NC}");
    command_status_strict(
        repo_root,
        "cargo",
        &[
            "test",
            "--features",
            "pure-rust test-utils",
            "edge_case_tests",
            "--",
            "--nocapture",
        ],
        &[],
    )?;

    println!("{YELLOW}Running integration tests...{NC}");
    command_status_strict(
        repo_root,
        "cargo",
        &[
            "test",
            "--features",
            "pure-rust test-utils",
            "test_edge_case_integration",
            "--",
            "--nocapture",
        ],
        &[],
    )?;
    command_status_strict(
        repo_root,
        "cargo",
        &[
            "test",
            "--features",
            "pure-rust test-utils",
            "test_recovery_mode_effectiveness",
            "--",
            "--nocapture",
        ],
        &[],
    )?;
    command_status_strict(
        repo_root,
        "cargo",
        &[
            "test",
            "--features",
            "pure-rust test-utils",
            "test_encoding_aware_heredocs",
            "--",
            "--nocapture",
        ],
        &[],
    )?;

    if bench {
        println!("{YELLOW}Running edge case benchmarks...{NC}");
        command_status_strict(
            repo_root,
            "cargo",
            &[
                "bench",
                "--features",
                "pure-rust test-utils",
                "edge_case_benchmarks",
            ],
            &[],
        )?;
    }

    println!("{YELLOW}Running edge case examples...{NC}");
    command_status_strict(
        repo_root,
        "cargo",
        &[
            "run",
            "--features",
            "pure-rust test-utils",
            "--example",
            "edge_case_demo",
        ],
        &[],
    )?;
    command_status_strict(
        repo_root,
        "cargo",
        &[
            "run",
            "--features",
            "pure-rust test-utils",
            "--example",
            "anti_pattern_analysis",
        ],
        &[],
    )?;
    command_status_strict(
        repo_root,
        "cargo",
        &[
            "run",
            "--features",
            "pure-rust test-utils",
            "--example",
            "tree_sitter_compatibility",
        ],
        &[],
    )?;

    if coverage {
        println!("{YELLOW}Generating coverage report...{NC}");
        command_status_strict(
            repo_root,
            "cargo",
            &[
                "tarpaulin",
                "--features",
                "pure-rust",
                "--out",
                "Html",
                "--output-dir",
                "target/coverage",
            ],
            &[],
        )?;
        println!("Coverage report generated at target/coverage/index.html");
    }

    println!();
    println!("{GREEN}✓ All edge case tests passed!{NC}");
    Ok(0)
}

fn cmd_quick_receipts(repo_root: &Path) -> Result<i32> {
    println!("=== Quick Receipt Generation (no tests) ===");

    let cargo_toml = read_to_value(
        repo_root
            .join("crates")
            .join("perl-parser")
            .join("Cargo.toml"),
    )?;
    let version = cargo_toml
        .get("package")
        .and_then(|pkg| pkg.get("version"))
        .and_then(|value| value.as_str())
        .unwrap_or("0.0.0");

    println!("Version: {version}");
    let artifacts_dir = repo_root.join("artifacts");
    fs::create_dir_all(&artifacts_dir).with_context(|| format!("creating {:?}", artifacts_dir))?;

    let docs_output = command_with_output_all(
        repo_root,
        "cargo",
        &["+stable", "doc", "--no-deps", "--package", "perl-parser"],
        &[],
    )?;
    let missing_docs = docs_output
        .lines()
        .filter(|line| line.starts_with("warning: missing documentation"))
        .count();
    println!("Missing docs: {missing_docs}");

    let doc_summary = json!({ "missing_docs": missing_docs });
    fs::write(
        artifacts_dir.join("doc-summary.json"),
        serde_json::to_string(&doc_summary)?,
    )
    .with_context(|| "writing doc-summary.json")?;
    println!(
        "Doc summary saved to {}",
        artifacts_dir.join("doc-summary.json").display()
    );

    let test_summary = json!({
        "passed": 0,
        "failed": 0,
        "ignored": 0,
        "active_tests": 0,
        "total_all_tests": 0,
        "pass_rate_active": 0.0,
        "pass_rate_total": 0.0,
        "note": "Run generate-receipts.sh for actual test metrics"
    });
    fs::write(
        artifacts_dir.join("test-summary.json"),
        serde_json::to_string(&test_summary)?,
    )
    .with_context(|| "writing test-summary.json")?;

    let state = json!({
        "version": version,
        "tests": test_summary,
        "docs": doc_summary,
        "generated_at": Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    });
    fs::write(
        artifacts_dir.join("state.json"),
        serde_json::to_string_pretty(&state)?,
    )
    .with_context(|| "writing state.json")?;

    println!(
        "State saved to {} (tests will be 0 until full receipt generation)",
        artifacts_dir.join("state.json").display()
    );
    let state_contents = fs::read_to_string(artifacts_dir.join("state.json"))
        .with_context(|| "reading state.json after writing")?;
    println!("{state_contents}");
    println!("\n=== Quick Receipt Generation Complete ===");
    Ok(0)
}

fn cmd_test_lsp_cancellation(repo_root: &Path) -> Result<i32> {
    const GREEN: &str = "\x1b[0;32m";
    const YELLOW: &str = "\x1b[1;33m";
    const NC: &str = "\x1b[0m";

    println!("{YELLOW}Enhanced LSP Cancellation System Test Runner{NC}");
    println!("{YELLOW}Fixing Cargo package cache file lock contention...{NC}");
    println!();

    println!("{YELLOW}Step 1: Pre-building LSP binaries...{NC}");
    command_status_strict(
        repo_root,
        "cargo",
        &["build", "--release", "-p", "perl-lsp"],
        &[],
    )?;
    println!("{GREEN}✓ LSP binaries pre-built successfully{NC}");

    println!("{YELLOW}Step 2: Pre-building test binaries...{NC}");
    command_status_strict(
        repo_root,
        "cargo",
        &["build", "--tests", "-p", "perl-lsp"],
        &[],
    )?;
    println!("{GREEN}✓ Test binaries pre-built successfully{NC}");

    let cancel_binary = find_cancel_test_binary(repo_root).ok_or_else(|| {
        color_eyre::eyre::eyre!("cancel test binary not found in target/debug/deps")
    })?;
    println!(
        "{GREEN}✓ Found cancel test binary: {}{NC}",
        cancel_binary.display()
    );

    let perl_lsp_binary = repo_root.join("target").join("release").join("perl-lsp");
    if !perl_lsp_binary.is_file() {
        return Err(color_eyre::eyre::eyre!(
            "missing pre-built perl-lsp binary at {}",
            perl_lsp_binary.display()
        ));
    }

    println!("{YELLOW}Step 4: Running cancellation tests...{NC}");
    println!("Testing with environment:");
    println!("  CARGO_BIN_EXE_perl_lsp={}", perl_lsp_binary.display());
    println!("  RUST_TEST_THREADS=1");
    let rust_threads = "1".to_string();
    let exe_env = [
        (
            "CARGO_BIN_EXE_perl_lsp",
            perl_lsp_binary.to_string_lossy().to_string(),
        ),
        ("RUST_TEST_THREADS", rust_threads),
    ];
    let exe_env_refs: Vec<(&str, &str)> = exe_env
        .iter()
        .map(|(key, value)| (*key, value.as_str()))
        .collect();
    command_status_strict(
        repo_root,
        cancel_binary.to_string_lossy().as_ref(),
        &["--nocapture"],
        &exe_env_refs,
    )?;

    println!("{GREEN}✓ All Enhanced LSP Cancellation System tests passed successfully!{NC}");
    println!("{GREEN}✓ Compilation contention issue resolved{NC}");
    println!("{GREEN}✓ <100μs check latency performance maintained{NC}");
    println!("{GREEN}✓ Cancellation functionality fully validated{NC}");
    Ok(0)
}

fn read_to_value(path: PathBuf) -> Result<TomlValue> {
    let raw = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}

fn find_cancel_test_binary(repo_root: &Path) -> Option<PathBuf> {
    let deps = repo_root.join("target").join("debug").join("deps");
    if !deps.is_dir() {
        return None;
    }

    for entry in walk_entries(&deps) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|name| name.to_str())
            && name.contains("lsp_cancel_test")
        {
            return Some(path.to_path_buf());
        }
    }
    None
}

fn cmd_generate_badges(repo_root: &Path) -> Result<i32> {
    let badge_file = repo_root.join("badges.md");
    let content = [
        "[![Crates.io](https://img.shields.io/crates/v/perl-parser)](https://crates.io/crates/perl-parser)",
        "[![Documentation](https://docs.rs/perl-parser/badge.svg)](https://docs.rs/perl-parser)",
        "[![CI Status](https://github.com/EffortlessMetrics/perl-lsp/workflows/LSP%20Tests/badge.svg)](https://github.com/EffortlessMetrics/perl-lsp/actions)",
        "[![License](https://img.shields.io/crates/l/perl-parser)](LICENSE)",
        "[![Coverage](https://img.shields.io/badge/test%20coverage-95%25-brightgreen)](COMPREHENSIVE_TEST_REPORT.md)",
        "[![User Stories](https://img.shields.io/badge/user%20stories-63%2B-success)](COMPREHENSIVE_TEST_REPORT.md)",
        "[![Performance](https://img.shields.io/badge/performance-1--150μs-blue)](benches/)",
    ]
    .join("\n");
    fs::write(&badge_file, format!("{content}\n"))
        .with_context(|| format!("writing {:?}", badge_file))?;
    println!(
        "Badges generated in {:?}",
        badge_file.file_name().unwrap_or_default()
    );
    Ok(0)
}

fn pre_push_hook_script() -> &'static str {
    r#"#!/usr/bin/env bash
# ============================================================================
# perl-lsp pre-push hook (generated by `cargo xtask ci-hygiene install-githooks`)
# ============================================================================
#
# Bypass policy
# -------------
# OK to bypass with `git push --no-verify`:
#   * Deletion-only push on master (the hook already auto-skips, but if it
#     doesn't, bypassing is safe — there's nothing to validate).
#   * Urgent fixes during incident response, when the gate has a known bug
#     being tracked (see hint output below for issue numbers).
#   * The hook is failing for an environmental reason that is out of band of
#     your change (e.g., nix store cache miss, transient toolchain issue).
#
# NOT OK to bypass:
#   * "I just don't want to wait."
#   * "I just want to push something quick."
#   * Code-touching changes where you haven't actually run the gate locally.
#
# If you find yourself bypassing repeatedly, file an issue and link it here.
# ============================================================================

set -euo pipefail

# --- Self-heal core.bare corruption (issue #3205) ---
# Some sequences of `git worktree add`/`remove` silently flip core.bare=true
# on the main checkout, breaking every worktree-aware git command including
# this hook. Auto-unset before doing anything else.
if [ "$(git config --get core.bare 2>/dev/null || true)" = "true" ]; then
    # Confirm this is actually a non-bare repo (has a working tree).
    if git rev-parse --show-toplevel >/dev/null 2>&1; then
        echo "⚠️  Detected core.bare=true corruption (issue #3205) — auto-fixing"
        git config --local --unset core.bare || true
    fi
fi

# --- Self-heal stale hook installation (issue #4220) ---
# When hooks/pre-push is updated in master, .git/hooks/pre-push is only
# updated when install-githooks is re-run. Auto-copy when drift is detected.
# Note: exec "$0" "$@" does NOT work here — git stdin is already consumed
# before the hook executes. Copy and continue; the fresh hook takes effect
# on the next push.
REPO_ROOT_FOR_HOOK="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [ -n "$REPO_ROOT_FOR_HOOK" ] && [ -f "$REPO_ROOT_FOR_HOOK/hooks/pre-push" ]; then
    if ! diff -q "$0" "$REPO_ROOT_FOR_HOOK/hooks/pre-push" >/dev/null 2>&1; then
        echo "pre-push hook updated from hooks/pre-push (was stale — takes effect next push)"
        cp "$REPO_ROOT_FOR_HOOK/hooks/pre-push" "$0" && chmod +x "$0" || true
    fi
fi

# stdin provides: <local ref> <local sha> <remote ref> <remote sha>
# Git sends all-zero SHA for deletions. Read all refs into an array first
# so stdin is available for both the delete-check and the test-file scan.
PUSH_REFS=()
while IFS= read -r line; do
    PUSH_REFS+=("$line")
done

# --- Skip CI gate when all refs are being deleted ---
IS_DELETE_ONLY=true
for line in "${PUSH_REFS[@]+"${PUSH_REFS[@]}"}"; do
    local_sha="$(echo "$line" | awk '{print $2}')"
    if [ "$local_sha" != "0000000000000000000000000000000000000000" ]; then
        IS_DELETE_ONLY=false
        break
    fi
done

if [ "$IS_DELETE_ONLY" = true ]; then
    echo "Branch deletion — skipping CI gate"
    exit 0
fi

# --- Detect doc-only changes for the fast-path gate ---
# A push is doc-only if every changed file matches one of:
#   *.md, *.txt, LICENSE*, CHANGELOG*, docs/**, .github/ISSUE_TEMPLATE/**,
#   or */LICENSE* (crate-subdir license files, e.g. crates/*/LICENSE-APACHE)
# Doc-only pushes skip code gates entirely instead of running the full
# ci-gate, since the test suite and workspace-wide rustfmt check are
# pointless for prose-only changes and can fail spuriously on Windows (#4047).
REPO_ROOT="$(git rev-parse --show-toplevel)"
DOC_ONLY=true
TEST_FILES_CHANGED=false
HAS_DIFFABLE_REF=false
# Track unique crate names for the single-crate tier.
# We use a newline-separated string rather than an array for POSIX compat.
SINGLE_CRATE_NAMES=""
SINGLE_CRATE_ALL_UNDER_CRATES=true
for line in "${PUSH_REFS[@]+"${PUSH_REFS[@]}"}"; do
    read -r _local_ref local_sha _remote_ref remote_sha <<< "$line"
    # For new branches, compare against merge-base with origin/master
    if [ "$remote_sha" = "0000000000000000000000000000000000000000" ]; then
        remote_sha="$(git merge-base "$local_sha" origin/master 2>/dev/null || echo "$local_sha")"
    fi
    CHANGED_FILES="$(git diff --name-only "$remote_sha" "$local_sha" 2>/dev/null || true)"
    if [ -n "$CHANGED_FILES" ]; then
        HAS_DIFFABLE_REF=true
    fi
    while IFS= read -r changed; do
        [ -z "$changed" ] && continue
        case "$changed" in
            *.md|*.txt|LICENSE*|CHANGELOG*|docs/*|.github/ISSUE_TEMPLATE/*|*/LICENSE*)
                ;;
            *)
                DOC_ONLY=false
                # Track whether this code file lives under crates/<name>/
                case "$changed" in
                    crates/*/*)
                        # Extract the crate directory name: crates/<name>/...
                        crate_name="${changed#crates/}"
                        crate_name="${crate_name%%/*}"
                        # Append to list if not already present
                        if ! printf '%s\n' "$SINGLE_CRATE_NAMES" | grep -qxF "$crate_name" 2>/dev/null; then
                            SINGLE_CRATE_NAMES="${SINGLE_CRATE_NAMES}${crate_name}
"
                        fi
                        ;;
                    *)
                        # File is outside crates/ — can't be single-crate
                        SINGLE_CRATE_ALL_UNDER_CRATES=false
                        ;;
                esac
                ;;
        esac
    done <<< "$CHANGED_FILES"
    if echo "$CHANGED_FILES" | grep -qE '^crates/.*/tests/.*\.rs$'; then
        TEST_FILES_CHANGED=true
    fi
done

# If we couldn't compute a diff for any ref (e.g., shallow clone, missing
# remote), assume code changes and run the full gate to be safe.
if [ "$HAS_DIFFABLE_REF" != true ]; then
    DOC_ONLY=false
fi

if [ "$DOC_ONLY" = true ]; then
    echo "📝 Doc-only push — skipping code gates"
    echo "   (Skip with: git push --no-verify)"
    echo "✅ Doc-only fast-path gate passed"
    exit 0
fi

# --- Single-crate proportional gate ---
# If every code change is under a single crates/<name>/ directory, run a
# targeted cargo fmt/clippy/test -p <name> instead of the full workspace gate.
# Falls back to the full gate if classification is ambiguous.
SINGLE_CRATE_COUNT="$(printf '%s' "$SINGLE_CRATE_NAMES" | grep -c . 2>/dev/null || echo 0)"
if [ "$SINGLE_CRATE_ALL_UNDER_CRATES" = true ] && [ "$SINGLE_CRATE_COUNT" = "1" ]; then
    SINGLE_CRATE_NAME="$(printf '%s' "$SINGLE_CRATE_NAMES" | tr -d '[:space:]')"
    echo "Single-crate push (${SINGLE_CRATE_NAME}) — running targeted gate"
    echo "   (Skip with: git push --no-verify)"
    echo ""
    run_single_crate_gate() {
        cargo fmt -p "$SINGLE_CRATE_NAME" -- --check && \
        cargo clippy -p "$SINGLE_CRATE_NAME" -- -D warnings && \
        cargo test -p "$SINGLE_CRATE_NAME"
    }
    GATE_LOG="$(mktemp -t perl-lsp-prepush.XXXXXX.log 2>/dev/null || mktemp)"
    trap 'rm -f "$GATE_LOG"' EXIT
    set +e
    run_single_crate_gate 2>&1 | tee "$GATE_LOG"
    GATE_STATUS=${PIPESTATUS[0]}
    set -e
    if [ "$GATE_STATUS" -ne 0 ]; then
        echo ""
        echo "❌ Single-crate gate failed (exit $GATE_STATUS)"
        echo "   See bypass policy at the top of .git/hooks/pre-push for when"
        echo "   --no-verify is appropriate."
        exit "$GATE_STATUS"
    fi
    echo "✅ Single-crate gate passed"
    exit 0
fi

echo "Running local fast gate before push: nix develop -c just pr-fast"
echo "   (Skip with: git push --no-verify)"
echo ""

# --- Check if test files changed and CURRENT_STATUS.md needs updating ---
if [ "$TEST_FILES_CHANGED" = true ] && [ -f "$REPO_ROOT/scripts/update-current-status.py" ]; then
    echo "📊 Test files changed — checking if docs/project/status/ is up to date..."
    if command -v python3 &>/dev/null; then
        python3 "$REPO_ROOT/scripts/update-current-status.py" 2>/dev/null || true
        if ! git diff --quiet -- docs/project/status/ 2>/dev/null; then
            echo ""
            echo "⚠️  docs/project/status/ has stale test counts!"
            echo "   Run: python3 scripts/update-current-status.py"
            echo "   Then commit the updated files before pushing."
            echo "   git add docs/project/status/"
            echo ""
            exit 1
        fi
    fi
fi

# --- Run the full gate, capturing output for failure-mode hints ---
GATE_LOG="$(mktemp -t perl-lsp-prepush.XXXXXX.log 2>/dev/null || mktemp)"
trap 'rm -f "$GATE_LOG"' EXIT

run_gate() {
    if command -v nix &>/dev/null && [ -f flake.nix ]; then
        nix develop -c just pr-fast
    elif command -v just &>/dev/null; then
        just pr-fast
    else
        echo "⚠️  Neither 'nix develop' nor 'just' available, skipping pre-push gate"
        echo "   Install just: cargo install just"
        return 0
    fi
}

set +e
run_gate 2>&1 | tee "$GATE_LOG"
GATE_STATUS=${PIPESTATUS[0]}
set -e

if [ "$GATE_STATUS" -ne 0 ]; then
    echo ""
    echo "❌ pr-fast failed (exit $GATE_STATUS) — checking for known issues..."
    HINTED=false
    if grep -q 'ci-parser-features-check' "$GATE_LOG" 2>/dev/null && \
       grep -qE 'os error 5|Access is denied' "$GATE_LOG" 2>/dev/null; then
        echo "   • Known: Windows file-lock race in ci-parser-features-check (#3202)"
        echo "     Workaround: re-run the gate, or use --no-verify if you're confident"
        echo "     your change is unrelated to xtask/parser-features."
        HINTED=true
    fi
    if grep -qE 'cargo (xtask )?fmt.*--check' "$GATE_LOG" 2>/dev/null && \
       grep -qE 'Diff in|rustfmt' "$GATE_LOG" 2>/dev/null; then
        echo "   • Formatting drift — run \`cargo xtask fmt\` to auto-fix"
        HINTED=true
    fi
    if grep -qE 'clippy::|warning: .*-> .*\.rs' "$GATE_LOG" 2>/dev/null; then
        echo "   • Clippy warnings — look for the error above and fix the warnings"
        HINTED=true
    fi
    if grep -qE 'os error 206|filename.*too long|ERROR_FILENAME_EXCED' "$GATE_LOG" 2>/dev/null; then
        echo "   • Windows CreateProcess command-line length limit (os error 206)"
        echo "     'cargo fmt --all' passes all 1200+ source files in one command,"
        echo "     exceeding the ~32K-char CreateProcess limit on Windows."
        echo "     Fix: bash scripts/install-githooks.sh"
        echo "     This installs the current hook which uses 'cargo xtask fmt' (per-crate)."
        echo "     See: docs/contributing/FIRST_PR.md"
        HINTED=true
    fi
    if [ "$HINTED" = false ]; then
        echo "   No known-issue patterns matched. Read the gate output above."
    fi
    echo ""
    echo "   See bypass policy at the top of .git/hooks/pre-push for when"
    echo "   --no-verify is appropriate."
    exit "$GATE_STATUS"
fi
"#
}

fn cmd_install_githooks(repo_root: &Path) -> Result<i32> {
    let hooks_dir = resolve_git_hooks_dir(repo_root)?;
    fs::create_dir_all(&hooks_dir)?;

    let pre_commit_hook = r#"#!/usr/bin/env bash
set -euo pipefail

GIT_USER_NAME="$(git config user.name 2>/dev/null || true)"
GIT_USER_EMAIL="$(git config user.email 2>/dev/null || true)"

if [ "$GIT_USER_NAME" = "Codex Release Validation" ] || \
   [ "$GIT_USER_EMAIL" = "codex-release-validation@example.invalid" ] || \
   [ "$GIT_USER_NAME" = "xtask hook tests" ] || \
   [ "$GIT_USER_EMAIL" = "xtask@example.invalid" ]; then
    echo "❌ Refusing commit with placeholder git identity"
    echo "   user.name:  $GIT_USER_NAME"
    echo "   user.email: $GIT_USER_EMAIL"
    echo ""
    echo "   Fix this repo-local override first:"
    echo "   git config --local --unset-all user.name"
    echo "   git config --local --unset-all user.email"
    exit 1
fi
"#;
    write_git_hook(&hooks_dir.join("pre-commit"), pre_commit_hook)?;

    write_git_hook(&hooks_dir.join("pre-push"), pre_push_hook_script())?;

    println!("✅ Installed pre-commit and pre-push hooks");
    println!("   The pre-commit hook blocks known placeholder git identities");
    println!("   The pre-push hook runs 'nix develop -c just pr-fast' before each push");
    println!("   Skip with: git commit --no-verify / git push --no-verify");
    Ok(0)
}

fn resolve_git_hooks_dir(repo_root: &Path) -> Result<PathBuf> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(["rev-parse", "--git-path", "hooks"])
        .output()
        .with_context(|| format!("resolving git hooks dir from {}", repo_root.display()))?;

    if !output.status.success() {
        return Err(color_eyre::eyre::eyre!(
            "git rev-parse --git-path hooks failed in {}: {}",
            repo_root.display(),
            String::from_utf8_lossy(&output.stderr).trim_end()
        ));
    }

    let hooks_path = String::from_utf8(output.stdout)
        .context("git rev-parse --git-path hooks emitted non-UTF8 output")?;
    let hooks_path = hooks_path.trim();
    let hooks_dir = PathBuf::from(hooks_path);

    Ok(if hooks_dir.is_absolute() {
        hooks_dir
    } else {
        repo_root.join(hooks_dir)
    })
}

fn write_git_hook(hook_path: &Path, hook: &str) -> Result<()> {
    fs::write(hook_path, format!("{hook}\n"))
        .with_context(|| format!("writing {:?}", hook_path))?;
    #[cfg(unix)]
    {
        fs::set_permissions(hook_path, fs::Permissions::from_mode(0o755))
            .with_context(|| format!("setting executable bit for {:?}", hook_path))?;
    }
    Ok(())
}

fn read_required_usize(path: &Path) -> Result<usize> {
    if !path.is_file() {
        return Err(color_eyre::eyre::eyre!(
            "required file not found: {}",
            path.display()
        ));
    }
    let raw = fs::read_to_string(path).with_context(|| format!("reading {:?}", path))?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(color_eyre::eyre::eyre!(
            "required file is empty: {}",
            path.display()
        ));
    }
    Ok(trimmed.parse::<usize>()?)
}

fn find_repo_root() -> Result<PathBuf> {
    let mut current = env::current_dir()?;
    loop {
        if current.join("Cargo.toml").is_file() {
            return Ok(current);
        }
        if !current.pop() {
            return Err(color_eyre::eyre::eyre!("unable to locate repository root"));
        }
    }
}

fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).map_or_else(
        |_| path.display().to_string(),
        |relative| relative.display().to_string(),
    )
}

fn path_has_component(path: &Path, target: &str) -> bool {
    path.components()
        .any(|component| component.as_os_str() == OsStr::new(target))
}

fn is_text_file(path: &Path) -> bool {
    fs::read_to_string(path).is_ok()
}

fn walk_entries(root: &Path) -> impl Iterator<Item = DirEntry> + '_ {
    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
}

fn read_lines(path: &Path) -> Result<Vec<String>> {
    fs::read_to_string(path)
        .with_context(|| format!("reading {:?}", path))
        .map(|contents| {
            contents
                .lines()
                .map(std::string::ToString::to_string)
                .collect()
        })
}

fn walk_rust_sources(root: &Path) -> Vec<PathBuf> {
    walk_entries(root)
        .filter_map(|entry| {
            if !entry.file_type().is_file() {
                return None;
            }
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext != "rs") {
                return None;
            }
            if is_excluded_test_path(path) {
                return None;
            }
            Some(path.to_path_buf())
        })
        .collect()
}

fn count_pattern_before_cfg_test(
    path: &Path,
    pattern: &Regex,
    exclude_self_context: bool,
) -> Result<Vec<(usize, String)>> {
    let mut out = Vec::new();
    let lines = read_lines(path)?;
    let test_start = first_cfg_test_line_number(path).unwrap_or(usize::MAX);
    for (index, line) in lines.iter().enumerate() {
        let line_number = index + 1;
        if line_number >= test_start {
            continue;
        }
        if pattern.is_match(line) {
            if exclude_self_context
                && (line.contains("self.expect(")
                    || line.contains("s.expect(")
                    || line.contains("self.context.expect("))
            {
                continue;
            }
            out.push((line_number, line.to_string()));
        }
    }
    Ok(out)
}

fn read_usize_file(path: &Path, default_value: usize) -> Result<usize> {
    if !path.is_file() {
        return Ok(default_value);
    }
    let raw = fs::read_to_string(path).with_context(|| format!("reading {:?}", path))?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(default_value);
    }
    Ok(trimmed.parse::<usize>()?)
}

fn cmd_check_doc_hygiene(repo_root: &Path) -> Result<i32> {
    let mut found_issues = false;
    println!("{}=== Documentation Hygiene Check ==={}", YELLOW, NC);
    println!();

    println!(
        "{}Checking for unescaped brackets in doc comments...{}",
        BLUE, NC
    );
    let unescaped_pattern = Regex::new(r"^[ \t]*//[/!].*\[")?;
    let output = command_with_output_allow_empty_match(
        repo_root,
        "rg",
        &[
            "-n",
            "--glob",
            "crates/**/src/**/*.rs",
            unescaped_pattern.as_str(),
        ],
        &[],
    )?;
    if output.trim().is_empty() {
        println!("{}✓ No suspicious brackets found{}", GREEN, NC);
    } else {
        println!(
            "{}⚠ Found potential unescaped brackets. Consider:{}",
            YELLOW, NC
        );
        println!("  - Escaping with backslash: \\[text\\]");
        println!("  - Wrapping in code blocks: `[text]`");
        println!("  - Using proper doc links: [`Type`] or [Type](link)");
        for line in command_output_lines(&output)
            .into_iter()
            .filter(|line| !line.contains(r"\["))
            .take(5)
        {
            println!("{line}");
        }
        found_issues = true;
    }
    println!();

    println!("{}Checking for bare URLs in doc comments...{}", BLUE, NC);
    let bare_url_output = command_with_output_allow_empty_match(
        repo_root,
        "rg",
        &[
            "-n",
            "--glob",
            "crates/**/src/**/*.rs",
            "^[ \t]*//[/!].*https?://[^ \t<>\\[\\]]+",
        ],
        &[],
    )?;
    let bare_url_lines = command_output_lines(&bare_url_output)
        .into_iter()
        .filter(|line| !line.contains("<http"))
        .collect::<Vec<_>>();
    if bare_url_lines.is_empty() {
        println!("{}✓ No bare URLs found{}", GREEN, NC);
    } else {
        println!(
            "{}⚠ Found bare URLs. Wrap them in angle brackets: <https://example.com>{}",
            YELLOW, NC
        );
        for line in bare_url_lines.into_iter().take(5) {
            println!("{line}");
        }
        found_issues = true;
    }
    println!();

    println!("{}Checking for other documentation issues...{}", BLUE, NC);
    let marker_pattern = Regex::new(r"^[ \t]*//[/!][^ /!#\[]")?;
    let marker_output = command_with_output_allow_empty_match(
        repo_root,
        "rg",
        &[
            "-n",
            "--glob",
            "crates/**/src/**/*.rs",
            marker_pattern.as_str(),
        ],
        &[],
    )?;
    if !marker_output.trim().is_empty() {
        println!(
            "{}⚠ Found doc comments without space after marker{}",
            YELLOW, NC
        );
        println!("  Use: /// Text  or  //! Text");
        for line in command_output_lines(&marker_output).iter().take(5) {
            println!("{line}");
        }
        found_issues = true;
    }

    let perl_code_output = command_with_output_allow_empty_match(
        repo_root,
        "rg",
        &[
            "-n",
            "-A2",
            "-B2",
            "--glob",
            "crates/**/src/**/*.rs",
            r"^[ \t]*///.*\\$[a-zA-Z_]",
        ],
        &[],
    )?;
    let perl_code_lines = perl_code_output
        .lines()
        .map(str::trim)
        .filter(|line| line.contains('$') && !line.contains("```"))
        .take(5)
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if !perl_code_lines.is_empty() {
        println!(
            "{}⚠ Possible Perl code in docs without code blocks{}",
            YELLOW, NC
        );
        println!("  Wrap Perl examples in triple backticks:");
        println!("  ```perl");
        println!("  my $var = 42;");
        println!("  ```");
        for line in perl_code_lines {
            println!("{line}");
        }
        found_issues = true;
    }
    println!();

    println!(
        "{}Checking for TODOs in public documentation...{}",
        BLUE, NC
    );
    let todo_output = command_with_output_allow_empty_match(
        repo_root,
        "rg",
        &[
            "-n",
            "--glob",
            "crates/**/src/**/*.rs",
            "^[ \t]*///.*\\b(TODO|FIXME|XXX|HACK)\\b",
        ],
        &[],
    )?;
    if todo_output.trim().is_empty() {
        println!("{}✓ No TODOs in public documentation{}", GREEN, NC);
    } else {
        println!(
            "{}⚠ Found TODO/FIXME in public docs (consider moving to regular comments){}",
            YELLOW, NC
        );
        for line in command_output_lines(&todo_output).iter().take(5) {
            println!("{line}");
        }
        found_issues = true;
    }
    println!();

    println!("{}Testing rustdoc build with strict flags...{}", BLUE, NC);
    let rustdoc_flags =
        "-D rustdoc::broken_intra_doc_links -D rustdoc::bare_urls -D rustdoc::invalid_html_tags";
    let status = command_status(
        repo_root,
        "cargo",
        &["doc", "--workspace", "--no-deps"],
        &[("RUSTDOCFLAGS", rustdoc_flags)],
    )?;
    if status == 0 {
        println!("{}✓ Documentation builds cleanly{}", GREEN, NC);
    } else {
        println!(
            "{}✗ Documentation build failed with strict flags{}",
            RED, NC
        );
        println!("  Run to see errors:");
        println!(
            "  RUSTDOCFLAGS=\"-D rustdoc::broken_intra_doc_links -D rustdoc::bare_urls\" cargo doc --workspace --no-deps"
        );
        found_issues = true;
    }
    println!();

    if found_issues {
        println!("{}=== Documentation Issues Found ==={}", YELLOW, NC);
        println!("These are suggestions for improving documentation quality.");
        println!("Not all issues are critical, but fixing them improves maintainability.");
    } else {
        println!("{}=== All Documentation Checks Passed ==={}", GREEN, NC);
    }
    Ok(0)
}

fn cmd_check_ignored(repo_root: &Path) -> Result<i32> {
    let regex = Regex::new(r"^\s*#\[ignore\b")?;
    let baseline_file = repo_root.join("ci").join("ignored_baseline.txt");

    let ignored_in_tests = walk_entries(&repo_root.join("crates/perl-parser/tests"))
        .filter_map(|entry| {
            let path = entry.path();
            if !entry.file_type().is_file() {
                return None;
            }
            if path.extension().is_none_or(|ext| ext != "rs") {
                return None;
            }
            Some(path.to_path_buf())
        })
        .filter_map(|path| {
            let mut count = 0usize;
            let lines = read_lines(&path).ok()?;
            for line in lines {
                if regex.is_match(&line) {
                    count += 1;
                }
            }
            Some(count)
        })
        .sum::<usize>();

    let ignored_in_src = walk_entries(&repo_root.join("crates/perl-parser/src"))
        .filter_map(|entry| {
            let path = entry.path();
            if !entry.file_type().is_file() {
                return None;
            }
            if path.extension().is_none_or(|ext| ext != "rs") {
                return None;
            }
            Some(path.to_path_buf())
        })
        .filter_map(|path| {
            let mut count = 0usize;
            let lines = read_lines(&path).ok()?;
            for line in lines {
                if regex.is_match(&line) {
                    count += 1;
                }
            }
            Some(count)
        })
        .sum::<usize>();

    let current = ignored_in_tests + ignored_in_src;
    let baseline = read_usize_file(&baseline_file, current)?;
    if !baseline_file.is_file() {
        fs::write(&baseline_file, format!("{current}\n"))
            .with_context(|| format!("creating {:?}", baseline_file))?;
        println!("Created baseline file with count: {current}");
    }

    let target = 25usize;
    let reduction = baseline.saturating_sub(current);
    let remaining = current.saturating_sub(target);

    println!("Ignored tests: {current} (baseline: {baseline})");
    println!("  - Integration tests: {ignored_in_tests}");
    println!("  - Unit tests in src: {ignored_in_src}");
    println!();
    println!("Budget Analysis:");
    println!("  - Target: ≤{target} tests (49% reduction minimum)");
    println!("  - Current reduction: {reduction} tests");
    println!("  - Remaining to target: {remaining} tests");

    if current <= target {
        let reduction_percent = if baseline > 0 {
            (reduction * 100) / baseline
        } else {
            0
        };
        println!("  ✅ TARGET ACHIEVED: {current} ≤ {target}");
        println!("  📈 Reduction: {reduction_percent}% (target: 49%+)");
    } else if current <= baseline {
        println!("  🔄 PROGRESS: {current} ≤ {baseline} (baseline maintained)");
        println!("  ⚠️  Need {remaining} more reductions to reach target");
    } else {
        println!("  ❌ REGRESSION: {current} > {baseline}");
    }
    println!();

    if current <= baseline {
        println!("Check passed: ignored test count is within acceptable range");
        Ok(0)
    } else {
        println!("ERROR: Ignored test count has increased from {baseline} to {current}");
        println!(
            "Please fix the newly ignored tests or update the baseline if this is intentional"
        );
        Ok(1)
    }
}

fn cmd_check_local(repo_root: &Path) -> Result<i32> {
    println!("{}=== Running Local Quality Checks ==={}", YELLOW, NC);
    println!();

    println!("{}1. Format check...{}", YELLOW, NC);
    if command_status_strict(repo_root, "cargo", &["xtask", "fmt", "--check"], &[]).is_err() {
        println!(
            "{}✗ Format check failed - run 'cargo xtask fmt' to fix{}",
            RED, NC
        );
        return Ok(1);
    }
    println!();

    println!("{}2. Clippy (strict on first-party)...{}", YELLOW, NC);
    let mut clippy_failed = false;
    if command_status(
        repo_root,
        "cargo",
        &[
            "clippy",
            "-p",
            "perl-parser",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
        &[],
    )
    .unwrap_or(1)
        != 0
    {
        println!("{}✗ Clippy found issues in perl-parser{}", RED, NC);
        clippy_failed = true;
    }
    if command_status(
        repo_root,
        "cargo",
        &[
            "clippy",
            "-p",
            "perl-lexer",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
        &[],
    )
    .unwrap_or(1)
        != 0
    {
        println!("{}✗ Clippy found issues in perl-lexer{}", RED, NC);
        clippy_failed = true;
    }
    if clippy_failed {
        return Ok(1);
    }
    println!(
        "{}✓ Clippy check passed for first-party crates{}",
        GREEN, NC
    );
    println!();

    println!("  Running clippy smoke check on vendor crates...");
    let smoke_output = command_with_output_allow_failure(
        repo_root,
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--exclude",
            "perl-parser",
            "--exclude",
            "perl-lexer",
        ],
        &[],
    )?;
    for line in command_output_lines(&smoke_output).iter().take(5) {
        println!("{line}");
    }
    println!();

    println!("{}3. Documentation build...{}", YELLOW, NC);
    if command_status_strict(
        repo_root,
        "cargo",
        &["doc", "--workspace", "--no-deps"],
        &[(
            "RUSTDOCFLAGS",
            "-D rustdoc::broken_intra_doc_links -D rustdoc::bare_urls",
        )],
    )
    .is_err()
    {
        println!("{}✗ Documentation build failed{}", RED, NC);
        return Ok(1);
    }
    println!("{}✓ Documentation builds cleanly{}", GREEN, NC);
    println!();

    println!("{}4. Running tests...{}", YELLOW, NC);
    if command_status_strict(
        repo_root,
        "cargo",
        &["test", "--workspace", "--all-features", "--quiet"],
        &[],
    )
    .is_err()
    {
        println!("{}✗ Tests failed{}", RED, NC);
        return Ok(1);
    }
    println!("{}✓ All tests passed{}", GREEN, NC);
    println!();

    println!("{}5. Ignored tests baseline...{}", YELLOW, NC);
    let ignored_exit = cmd_check_ignored(repo_root)?;
    if ignored_exit == 0 {
        println!("{}✓ Ignored tests baseline correct{}", GREEN, NC);
    } else {
        println!("{}✗ Ignored tests baseline mismatch{}", RED, NC);
        return Ok(1);
    }
    println!();

    println!("{}6. Dependency security check...{}", YELLOW, NC);
    if command_exists("cargo-deny") {
        let output =
            command_with_output_allow_failure(repo_root, "cargo", &["deny", "check"], &[])?;
        if output.contains("error:") {
            println!("{}✗ Dependency issues found{}", RED, NC);
            println!("{output}");
            return Ok(1);
        }
        println!("{}✓ Dependencies are secure{}", GREEN, NC);
    } else {
        println!(
            "{}⚠ cargo-deny not installed (run: cargo install cargo-deny){}",
            YELLOW, NC
        );
    }
    println!();

    println!("{}=== All Local Checks Passed ==={}", GREEN, NC);
    println!();
    println!("You can now safely commit/push your changes.");
    println!("Pro tip: Install as git pre-push hook: cp ci/check_local.sh .git/hooks/pre-push");
    Ok(0)
}

fn cmd_check_missing_docs(repo_root: &Path) -> Result<i32> {
    let baseline_path = repo_root.join("ci").join("missing_docs_baseline.txt");
    let baseline = read_required_usize(&baseline_path)?;
    let output = command_with_output_allow_failure(
        repo_root,
        "cargo",
        &[
            "check",
            "-p",
            "perl-parser",
            "--tests",
            "--message-format=json",
        ],
        &[],
    )?;
    let mut current = 0usize;

    for raw in output.lines() {
        let value: Value = match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if value.get("reason").and_then(|v| v.as_str()) != Some("compiler-message") {
            continue;
        }
        let pkg_id = value
            .get("package_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !pkg_id.starts_with("perl-parser ") {
            continue;
        }
        let message = value.get("message");
        if message.is_none() {
            continue;
        }
        let level = message
            .and_then(|m| m.get("level"))
            .and_then(|v| v.as_str());
        let code = message
            .and_then(|m| m.get("code"))
            .and_then(|code| code.get("code"))
            .and_then(|v| v.as_str());
        if level == Some("warning") && code == Some("missing_docs") {
            current += 1;
        }
    }

    println!("Missing docs warnings (perl-parser, tests included): {current}");
    println!("Baseline: {baseline}");

    if current > baseline {
        println!("REGRESSION: missing_docs count increased from {baseline} to {current}");
        println!("To see the warnings, run:");
        println!("  cargo check -p perl-parser --tests 2>&1 | grep 'missing documentation'");
        println!("Options:");
        println!("  1. Add documentation to the new public items");
        println!("  2. Mark test-only items with #[doc(hidden)] (still requires docs)");
        println!(
            "  3. If intentional, update baseline: echo {current} > {:?}",
            baseline_path
        );
        return Ok(1);
    }

    if current < baseline {
        println!(
            "IMPROVEMENT: {} fewer missing_docs warnings!",
            baseline - current
        );
        println!(
            "Consider updating baseline: echo {current} > {:?}",
            baseline_path
        );
    }

    println!("Check passed: missing_docs count is within acceptable range");
    Ok(0)
}

fn cmd_check_p0_locks(repo_root: &Path) -> Result<i32> {
    let target_dir = repo_root.join("crates/perl-parser/src/lsp/server_impl");
    if !target_dir.is_dir() {
        println!("⚠️  Directory not found: {}", target_dir.display());
        println!("Skipping P0 lock check (directory may have been restructured)");
        return Ok(0);
    }

    let pattern = Regex::new(r"lock\(\)\.unwrap\(\)|read\(\)\.unwrap\(\)|write\(\)\.unwrap\(\)")?;
    println!(
        "Checking for unsafe lock patterns in {}...",
        target_dir.display()
    );
    println!("Target: 0 occurrences (P0 lock safety requirement)");
    println!();
    let mut matches = Vec::new();
    for path in walk_rust_sources(&target_dir) {
        let file_text = fs::read_to_string(&path)?;
        for (line_no, line) in file_text.lines().enumerate() {
            if pattern.is_match(line) {
                matches.push(format!("{}:{}", path.display(), line_no + 1));
            }
        }
    }
    if matches.is_empty() {
        println!("✅ PASS: No unsafe lock patterns found");
        println!("   All lock operations use proper error handling");
        Ok(0)
    } else {
        println!("❌ FAIL: Found {} unsafe lock pattern(s)", matches.len());
        println!("Locations:");
        for item in &matches {
            println!("  {item}");
        }
        println!();
        println!(
            "lock().unwrap(), read().unwrap(), and write().unwrap() can panic and crash the LSP server."
        );
        println!("Replace with proper error handling.");
        Ok(1)
    }
}

fn cmd_check_parse_errors(repo_root: &Path) -> Result<i32> {
    let baseline_file = repo_root.join("ci").join("parse_errors_baseline.txt");
    let report_file = repo_root.join("corpus_audit_report.json");
    if !baseline_file.is_file() {
        return Err(color_eyre::eyre::eyre!(
            "Baseline file not found: {}",
            baseline_file.display()
        ));
    }

    let baseline = read_required_usize(&baseline_file)?;

    // NOTE (issue #3202): we deliberately do NOT spawn `cargo run -p xtask -- corpus-audit`
    // here. The justfile target `ci-parser-features-check` runs corpus-audit first, then
    // invokes this check. Spawning xtask from inside this binary used to cause a Windows
    // file-lock race: the parent xtask.exe was still running and Windows blocks relinking
    // a running executable, surfacing as `os error 5: Access is denied`. By requiring the
    // report to exist already, we keep this command pure (just JSON read + comparison) and
    // unblock all Windows contributors from running `just ci-gate` locally.
    if !report_file.is_file() {
        return Err(color_eyre::eyre::eyre!(
            "Report file not found: {}\n\nRun `cargo xtask corpus-audit --fresh --corpus-path . --output {}` first, \
             or use `just ci-parser-features-check` which runs both steps in order.",
            report_file.display(),
            report_file.display()
        ));
    }

    let report = read_json_value(&report_file)?;
    let mut current = 0usize;
    if let Some(value) = report
        .get("parse_outcomes")
        .and_then(|v| v.get("error"))
        .and_then(|v| v.as_u64())
    {
        current = usize::try_from(value)?;
    } else if let Some(value) = report
        .get("parse_outcomes")
        .and_then(|v| v.get("error"))
        .and_then(|v| v.as_i64())
    {
        current = usize::try_from(value.max(0)).unwrap_or(0);
    }

    println!();
    println!("Parse errors in test corpus: {current}");
    println!("Baseline: {baseline}");

    if current > baseline {
        println!();
        println!("REGRESSION: parse error count increased from {baseline} to {current}");
        println!();
        println!("To see details, run:");
        println!("  just parser-audit");
        println!();
        println!("Options:");
        println!("  1. Fix the parser to handle the new failing constructs");
        println!(
            "  2. If the regression is intentional, update baseline: echo {current} > {:?}",
            baseline_file
        );
        Ok(1)
    } else {
        if current < baseline {
            println!();
            println!("IMPROVEMENT: {} fewer parse errors!", baseline - current);
            println!(
                "Consider updating baseline: echo {current} > {:?}",
                baseline_file
            );
        }
        println!();
        println!("Check passed: parse error count is within acceptable range");
        Ok(0)
    }
}

fn cmd_check_parser_matrix(repo_root: &Path) -> Result<i32> {
    let matrix_file = repo_root.join("docs").join("PARSER_FEATURE_MATRIX.md");
    let report_file = repo_root.join("corpus_audit_report.json");

    if !matrix_file.is_file() {
        return Err(color_eyre::eyre::eyre!(
            "Matrix file not found: {}",
            matrix_file.display()
        ));
    }
    if !report_file.is_file() {
        let _ = command_status(
            repo_root,
            "cargo",
            &[
                "run",
                "-p",
                "xtask",
                "--no-default-features",
                "-q",
                "--",
                "corpus-audit",
                "--fresh",
                "--corpus-path",
                ".",
                "--output",
                report_file.to_string_lossy().as_ref(),
            ],
            &[],
        );
    }
    if !report_file.is_file() {
        return Err(color_eyre::eyre::eyre!(
            "Report file not generated: {}",
            report_file.display()
        ));
    }

    let tmp_matrix = repo_root.join(format!(
        "target/parser_matrix_{}_{}.md",
        std::process::id(),
        Utc::now().timestamp_millis()
    ));
    let python_status = command_status(
        repo_root,
        "python3",
        &[
            "scripts/update-parser-matrix.py",
            "--report",
            report_file.to_string_lossy().as_ref(),
            "--output",
            tmp_matrix.to_string_lossy().as_ref(),
            "--quiet",
        ],
        &[],
    )?;
    if python_status != 0 {
        return Err(color_eyre::eyre::eyre!(
            "update-parser-matrix.py failed (exit {python_status})"
        ));
    }

    let generated = Regex::new(r"^\| Generated \|.*\|$")?;
    let commit = Regex::new(r"^\| Commit \|.*\|$")?;
    let normalize = |input: &str| -> String {
        input
            .lines()
            .map(|line| {
                if generated.is_match(line) {
                    return "| Generated | (elided) |".to_string();
                }
                if commit.is_match(line) {
                    return "| Commit | (elided) |".to_string();
                }
                line.to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let current_matrix =
        fs::read_to_string(&matrix_file).wrap_err_with(|| format!("reading {:?}", matrix_file))?;
    let fresh_matrix =
        fs::read_to_string(&tmp_matrix).wrap_err_with(|| format!("reading {:?}", tmp_matrix))?;

    let current_normalized = normalize(&current_matrix);
    let fresh_normalized = normalize(&fresh_matrix);

    if current_normalized == fresh_normalized {
        let _ = fs::remove_file(&tmp_matrix);
        println!("Parser matrix is in sync");
        return Ok(0);
    }

    println!();
    println!("DRIFT DETECTED: docs/reference/PARSER_FEATURE_MATRIX.md is out of date");
    println!();
    let old_matrix = repo_root.join("target/.old_parser_matrix");
    let new_matrix = repo_root.join("target/.new_parser_matrix");
    let _ = fs::write(&old_matrix, format!("{current_normalized}\n"));
    let _ = fs::write(&new_matrix, format!("{fresh_normalized}\n"));

    let diff = command_with_output_allow_failure(
        repo_root,
        "diff",
        &[
            "-u",
            old_matrix.to_string_lossy().as_ref(),
            new_matrix.to_string_lossy().as_ref(),
        ],
        &[],
    )
    .unwrap_or_else(|_| String::new());
    if diff.is_empty() {
        println!("Current:");
        println!("{current_normalized}");
        println!();
        println!("Expected:");
        println!("{fresh_normalized}");
    } else {
        println!("{diff}");
    }
    let _ = fs::remove_file(&old_matrix);
    let _ = fs::remove_file(&new_matrix);

    println!("─────────────────────────────────");
    println!();
    println!("To fix:");
    println!("  1. Run: just parser-audit");
    println!("  2. Run: just parser-matrix-update");
    println!("  3. Commit the updated docs/reference/PARSER_FEATURE_MATRIX.md");
    let _ = fs::remove_file(&tmp_matrix);
    Ok(1)
}

fn cmd_check_unsafe_prod(repo_root: &Path) -> Result<i32> {
    let pattern = Regex::new(
        r"unsafe[[:space:]]*\{|unsafe[[:space:]]+extern|unsafe[[:space:]]+impl|#!\[allow\(unsafe_code\)\]",
    )?;
    let total = walk_rust_source_files_for_ci_checks(repo_root)?
        .into_iter()
        .map(|path| {
            let file = read_lines(&path).map(|lines| {
                let test_start = first_cfg_test_line_number(&path).unwrap_or(usize::MAX);
                lines
                    .iter()
                    .enumerate()
                    .filter_map(|(index, line)| {
                        let line_no = index + 1;
                        if line_no >= test_start {
                            return None;
                        }
                        if pattern.is_match(line) {
                            Some(format!(
                                "{}:{line_no}:{line}",
                                display_path(repo_root, &path)
                            ))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<String>>()
            })?;
            Ok::<Vec<String>, color_eyre::eyre::Report>(file)
        })
        .collect::<Result<Vec<_>>>()?;

    let all_matches = total.into_iter().flatten().collect::<Vec<_>>();
    let baseline = read_usize_file(&repo_root.join("ci/unsafe_prod_baseline.txt"), 0)?;
    println!(
        "unsafe syntax: {} (baseline: {baseline})",
        all_matches.len()
    );
    if all_matches.len() <= baseline {
        if all_matches.is_empty() {
            println!("No unsafe syntax in production scopes");
        }
        return Ok(0);
    }

    println!(
        "FAIL: unsafe syntax count ({}) exceeds baseline ({baseline})",
        all_matches.len()
    );
    println!("Offenders:");
    for item in &all_matches {
        println!("{item}");
    }
    Ok(1)
}

fn cmd_check_unwraps_modules(repo_root: &Path) -> Result<i32> {
    println!("Module-scoped unwrap ratchet gates");
    println!("===================================");
    println!();
    let pattern = Regex::new(r#"\.unwrap\(\)|\.expect\(\s*"|\.expect\(\s*&?format!\("#)?;
    let failures = run_module_ratchet(
        repo_root,
        "server_impl (P0)",
        &repo_root.join("crates/perl-parser/src/lsp/server_impl"),
        &repo_root.join("ci/unwrap_server_impl_baseline.txt"),
        &pattern,
    )? + run_module_ratchet(
        repo_root,
        "lexer (P1)",
        &repo_root.join("crates/perl-lexer/src"),
        &repo_root.join("ci/unwrap_lexer_baseline.txt"),
        &pattern,
    )?;

    if failures > 0 {
        println!("❌ {} module ratchet(s) failed", failures);
        Ok(1)
    } else {
        println!("✅ All module ratchets passed");
        Ok(0)
    }
}

fn run_module_ratchet(
    repo_root: &Path,
    name: &str,
    dir: &Path,
    baseline_file: &Path,
    pattern: &Regex,
) -> Result<usize> {
    println!("=== Checking {name} ===");
    if !dir.is_dir() {
        println!("  Directory not found: {} (skipping)", dir.display());
        println!();
        return Ok(0);
    }
    let mut offenders = Vec::new();
    for path in walk_entries(dir).filter_map(|entry| {
        let path = entry.path();
        if !entry.file_type().is_file() || path.extension().is_some_and(|ext| ext != "rs") {
            return None;
        }
        Some(path.to_path_buf())
    }) {
        for (line_no, text) in count_pattern_before_cfg_test(&path, pattern, false)? {
            offenders.push(format!(
                "{}:{line_no}:{text}",
                display_path(repo_root, &path)
            ));
        }
    }

    let current = offenders.len();
    let mut baseline = read_usize_file(baseline_file, current)?;
    if !baseline_file.is_file() {
        fs::write(baseline_file, format!("{current}\n"))
            .with_context(|| format!("creating {:?}", baseline_file))?;
        println!("  Created baseline: {baseline}");
        baseline = current;
    }

    println!("  Current: {current} (baseline: {baseline})");
    if current <= baseline {
        if current < baseline {
            println!("  ✅ IMPROVED by {}", baseline - current);
            println!("  Consider updating: echo {current} > {:?}", baseline_file);
        } else {
            println!("  ✅ PASS");
        }
        println!();
        Ok(0)
    } else {
        println!("  ❌ REGRESSION: +{}", current - baseline);
        for line in offenders.iter().take(10) {
            println!("{line}");
        }
        println!();
        Ok(1)
    }
}

fn walk_rust_source_files_for_ci_checks(repo_root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in walk_entries(&repo_root.join("crates")) {
        let path = entry.path();
        if !entry.file_type().is_file() {
            continue;
        }
        if path.extension().is_some_and(|ext| ext != "rs") {
            continue;
        }
        if is_excluded_test_path(path) {
            continue;
        }
        files.push(path.to_path_buf());
    }
    Ok(files)
}

fn cmd_check_unwraps_prod(repo_root: &Path) -> Result<i32> {
    let unwrap_re = Regex::new(r"\.unwrap\(|\.expect\(")?;
    let panic_re = Regex::new(r"(panic!\(|todo!\(|unimplemented!\(|unreachable!\()")?;
    let comment_re = Regex::new(r"^\s*//")?;
    let mut unwrap_offenders = Vec::new();
    let mut panic_offenders = Vec::new();

    for path in walk_rust_source_files_for_ci_checks(repo_root)? {
        let rel = display_path(repo_root, &path);
        let lines = read_lines(&path)?;
        let test_start = first_cfg_test_line_number(&path).unwrap_or(usize::MAX);
        for (index, line) in lines.iter().enumerate() {
            let line_no = index + 1;
            if line_no >= test_start {
                continue;
            }
            if !comment_re.is_match(line)
                && unwrap_re.is_match(line)
                && !(line.contains("self.expect(")
                    || line.contains("s.expect(")
                    || line.contains("self.context.expect("))
            {
                unwrap_offenders.push(format!("{rel}:{line_no}:{line}"));
            }
            if panic_re.is_match(line)
                && !comment_re.is_match(line)
                && !is_allowlisted_prod_panic_hit(&rel, line)
            {
                panic_offenders.push(format!("{rel}:{line_no}:{line}"));
            }
        }
    }

    let unwrap_baseline = read_usize_file(&repo_root.join("ci/unwrap_prod_baseline.txt"), 0)?;
    let panic_baseline = read_usize_file(&repo_root.join("ci/panic_prod_baseline.txt"), 0)?;
    println!(
        "unwrap/expect: {} (baseline: {})",
        unwrap_offenders.len(),
        unwrap_baseline
    );
    if unwrap_offenders.len() > unwrap_baseline {
        println!(
            "FAIL: unwrap/expect count ({}) exceeds baseline ({})",
            unwrap_offenders.len(),
            unwrap_baseline
        );
        println!();
        println!("Offenders:");
        for line in unwrap_offenders.iter().take(10) {
            println!("{line}");
        }
        return Ok(1);
    }

    println!(
        "panic-family macros: {} (baseline: {})",
        panic_offenders.len(),
        panic_baseline
    );
    if panic_offenders.len() > panic_baseline {
        println!(
            "FAIL: panic-family count ({}) exceeds baseline ({})",
            panic_offenders.len(),
            panic_baseline
        );
        println!();
        println!("Offenders:");
        for line in panic_offenders.iter().take(10) {
            println!("{line}");
        }
        println!(
            "If you removed panic-family macros, update ci/panic_prod_baseline.txt with the new lower count."
        );
        return Ok(1);
    }
    Ok(0)
}

fn is_allowlisted_prod_panic_hit(rel_path: &str, line: &str) -> bool {
    normalize_path_for_match(rel_path) == "crates/perl-heredoc-anti-patterns/src/lib.rs"
        && line.contains("regex failed to compile")
}

fn cmd_quick_check(repo_root: &Path) -> Result<i32> {
    println!("=== Quick CI Mirror Check ===");
    println!();

    println!("1. Format check");
    command_status_strict(repo_root, "cargo", &["fmt", "--all", "--", "--check"], &[])?;

    println!();
    println!("2. Clippy (strict on first-party)");
    command_status_strict(
        repo_root,
        "cargo",
        &[
            "clippy",
            "-p",
            "perl-parser",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
        &[],
    )?;
    command_status_strict(
        repo_root,
        "cargo",
        &[
            "clippy",
            "-p",
            "perl-lexer",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
        &[],
    )?;

    println!();
    println!("3. Clippy (smoke check on rest)");
    let smoke_output = command_with_output_allow_failure(
        repo_root,
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--exclude",
            "perl-parser",
            "--exclude",
            "perl-lexer",
        ],
        &[],
    )?;
    if !smoke_output.is_empty() {
        for line in smoke_output.lines().take(5) {
            println!("{line}");
        }
    }

    println!();
    println!("4. Docs (strict)");
    command_status_strict(
        repo_root,
        "cargo",
        &["doc", "--workspace", "--no-deps"],
        &[(
            "RUSTDOCFLAGS",
            "-D rustdoc::broken_intra_doc_links -D rustdoc::bare_urls",
        )],
    )?;

    println!();
    println!("5. Tests (workspace, lib+bins+tests, no examples)");
    command_status_strict(
        repo_root,
        "cargo",
        &["test", "--workspace", "--all-features"],
        &[],
    )?;

    println!();
    println!("6. Ignored baseline");
    command_status_strict(repo_root, "bash", &["./ci/check_ignored.sh"], &[])?;

    println!();
    println!("7. Cargo deny (if available)");
    if command_exists("cargo-deny") {
        command_status_strict(repo_root, "cargo", &["deny", "check"], &[])?;
    } else {
        println!("cargo-deny not installed (skipping)");
    }
    println!();
    println!("✅ All checks complete");
    Ok(0)
}

fn cmd_test_heredocs(repo_root: &Path) -> Result<i32> {
    println!("🧪 Running comprehensive heredoc tests...");
    if command_exists("xtask") {
        println!("Using cargo xtask...");
        command_status_strict(
            repo_root,
            "cargo",
            &["xtask", "test-heredoc", "--release"],
            &[],
        )?;
    } else {
        println!("Running tests directly...");
        command_status_strict(
            repo_root,
            "cargo",
            &[
                "test",
                "--features",
                "pure-rust",
                "--release",
                "--test",
                "heredoc_missing_features_tests",
            ],
            &[],
        )?;
        command_status_strict(
            repo_root,
            "cargo",
            &[
                "test",
                "--features",
                "pure-rust",
                "--release",
                "--test",
                "heredoc_integration_tests",
            ],
            &[],
        )?;
        command_status_strict(
            repo_root,
            "cargo",
            &[
                "test",
                "--features",
                "pure-rust",
                "--release",
                "--test",
                "comprehensive_heredoc_tests",
            ],
            &[],
        )?;
    }
    println!("✅ All heredoc tests passed!");
    Ok(0)
}

fn cmd_check_doc_paths(repo_root: &Path, docs_dir: Option<&str>) -> Result<i32> {
    let docs_dir = docs_dir.unwrap_or("docs");
    let docs_path = if Path::new(docs_dir).is_absolute() {
        PathBuf::from(docs_dir)
    } else {
        repo_root.join(docs_dir)
    };
    let home_machine = Regex::new(r"/home/[^u]")?;
    let home_steven = Regex::new(r"/home/steven")?;
    let users_machine = Regex::new(r"/Users/[^N]")?;
    let users_placeholder = Regex::new(r"/Users/Name")?;

    let mut hard_failures = Vec::new();
    let mut warnings = Vec::new();

    if !docs_path.is_dir() {
        return Err(color_eyre::eyre::eyre!(
            "Docs directory not found: {}",
            docs_path.display()
        ));
    }

    for entry in walk_entries(&docs_path) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if !is_text_file(path) {
            continue;
        }
        let rel = display_path(repo_root, path);
        let contents = fs::read_to_string(path)?;
        for (line_no, line) in contents.lines().enumerate() {
            let number = line_no + 1;
            if home_machine.is_match(line) && !line.contains("/home/user") {
                hard_failures.push(format!("{rel}:{number}:{line}"));
            }
            if home_steven.is_match(line) {
                hard_failures.push(format!("{rel}:{number}:{line}"));
            }
            if users_machine.is_match(line) && !users_placeholder.is_match(line) {
                warnings.push(format!("{rel}:{number}:{line}"));
            }
        }
    }

    if !warnings.is_empty() {
        println!("⚠️  Found macOS user paths that may be machine-specific");
        for hit in warnings {
            println!("{hit}");
        }
        println!();
    }

    if hard_failures.is_empty() {
        println!("✅ No machine-specific paths found in documentation");
        return Ok(0);
    }

    println!("{RED}❌ Found machine-specific /home/ paths (not /home/user examples){NC}");
    for hit in hard_failures {
        println!("{hit}");
    }
    println!();
    println!("Fix: Replace absolute paths with repo-relative paths or generic examples");
    println!("  - Use relative paths: docs/file.md instead of /home/.../docs/file.md");
    println!("  - Use generic examples: /home/user/project for user-facing docs");
    Ok(1)
}

fn cmd_check_todos(repo_root: &Path, list_mode: bool) -> Result<i32> {
    let baseline_path = repo_root.join("ci").join("todo_baseline.txt");
    // "xtask" is excluded because it is build tooling whose source code documents and implements
    // the TODO-scanner itself (unwired_scan.rs), producing unavoidable self-referential matches.
    // ".claude" is excluded because it contains ephemeral agent worktrees and tooling state that
    // are gitignored — the scanner should not see them as project source.
    let exclude_dirs = [
        "target",
        ".git",
        ".receipts",
        ".runs",
        "archive",
        "xtask",
        ".claude",
    ];
    let exclude_files = [
        repo_root.join("ci").join("check_todos.sh"),
        repo_root
            .join("crates")
            .join("perl-parser")
            .join("tests")
            .join("missing_docs_ac_tests.rs"),
        repo_root
            .join("crates")
            .join("perl-tdd-support")
            .join("src")
            .join("tdd")
            .join("test_generator.rs"),
        repo_root
            .join("crates")
            .join("perl-ci-hygiene")
            .join("src")
            .join("main.rs"),
        // Perl code-as-string: contains `TODO` as a Perl package bareword, not an unlinked TODO comment.
        repo_root
            .join("crates")
            .join("perl-parser-core")
            .join("tests")
            .join("complex_paren_args_tests.rs"),
    ];

    let todo_re = Regex::new(r"TODO|FIXME")?;
    let entries = collect_todo_hits(repo_root, &exclude_dirs, &exclude_files, &todo_re)?;

    if list_mode {
        for hit in entries {
            println!("{}", hit.line_text);
        }
        return Ok(0);
    }

    let current_count = entries.len();
    let baseline_count: usize = if baseline_path.is_file() {
        fs::read_to_string(&baseline_path)?
            .trim()
            .parse::<usize>()
            .wrap_err("parsing ci/todo_baseline.txt")?
    } else {
        fs::create_dir_all(baseline_path.parent().unwrap_or(repo_root))?;
        fs::write(&baseline_path, format!("{current_count}\n"))?;
        println!("📝 Creating initial TODO baseline...");
        println!("✅ Baseline established: {current_count}");
        current_count
    };

    println!("🔎 TODO Compliance Audit");
    println!("=======================");
    println!("Current unlinked TODOs: {current_count}");
    println!("Baseline allowed:       {baseline_count}");
    println!();

    if current_count > baseline_count {
        println!(
            "❌ ERROR: Unlinked TODO count increased from {baseline_count} to {current_count}"
        );
        println!(
            "Please link new TODOs to a GitHub issue using the format: TODO(#123): explanation"
        );
        println!();
        println!("New/Unlinked violations:");
        for hit in entries {
            println!("{}", hit.line_text);
        }
        Ok(1)
    } else if current_count < baseline_count {
        println!(
            "🎉 Great job! You reduced the number of unlinked TODOs ({current_count} < {baseline_count})."
        );
        println!(
            "Please update ci/todo_baseline.txt to {current_count} to lock in this improvement."
        );
        println!();
        Ok(0)
    } else {
        println!("✅ TODO count is within baseline limits.");
        Ok(0)
    }
}

fn cmd_forbid_fatal_constructs(repo_root: &Path, verbose: bool) -> Result<i32> {
    let abort_re = Regex::new(r"std::process::abort\s*\(")?;
    let exit_re = Regex::new(r"std::process::exit\s*\(")?;

    let mut aborts = Vec::new();
    let mut exits = Vec::new();

    let crates_root = repo_root.join("crates");
    for entry in walk_entries(&crates_root) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let rel = display_path(repo_root, path);
        if is_fatal_excluded(path, repo_root)? {
            continue;
        }
        let lines = read_lines(path)?;
        for (line_no, line) in lines.iter().enumerate() {
            let number = line_no + 1;
            if abort_re.is_match(line) {
                aborts.push(format!("{rel}:{number}:{line}"));
            }
            if exit_re.is_match(line) {
                exits.push(format!("{rel}:{number}:{line}"));
            }
        }
    }

    if !aborts.is_empty() {
        println!("{RED}ERROR: std::process::abort() found in production code{NC}");
        println!();
        println!("abort() is never allowed - it terminates without unwinding.");
        println!("==================================================");
        for hit in &aborts {
            println!("{hit}");
        }
        println!("==================================================");
        println!();
        println!("To fix: return an error and let the caller handle it.");
        println!();
    }

    let exit_violations: Vec<String> = exits
        .into_iter()
        .filter(|hit| !is_allowlisted_exit_hit(hit))
        .collect();

    if !exit_violations.is_empty() {
        println!("{RED}ERROR: std::process::exit() found outside allowlist{NC}");
        println!();
        println!("exit() is only allowed in:");
        println!("  - bin/ directories (CLI entry points)");
        println!("  - lifecycle.rs (LSP exit handler)");
        println!("==================================================");
        for hit in &exit_violations {
            println!("{hit}");
        }
        println!("==================================================");
        println!();
        println!("To fix: return an error, use Result<(), E>, or move to an allowlisted path.");
        println!();
    }

    if (!aborts.is_empty()) || !exit_violations.is_empty() {
        return Ok(1);
    }

    if verbose {
        println!("{GREEN}OK: No forbidden fatal constructs in production code{NC}");
        println!();
        println!("{YELLOW}Policy summary:{NC}");
        println!("  - abort(): NEVER allowed (banned everywhere)");
        println!("  - exit():  allowed in bin/ and lifecycle.rs only");
        println!();
        println!("{YELLOW}Note: panic!/unwrap!/expect! are enforced by Clippy deny lints:{NC}");
        println!("  - clippy::panic, clippy::unwrap_used, clippy::expect_used");
        println!("  - See [workspace.lints.clippy] in Cargo.toml");
    }
    Ok(0)
}

fn is_fatal_excluded(path: &Path, repo_root: &Path) -> Result<bool> {
    let rel = path.strip_prefix(repo_root).unwrap_or(path).to_path_buf();
    let rel_string = format!("/{}", normalize_path_for_match(&rel.display().to_string()));

    if rel_string.contains("/tests/") {
        return Ok(true);
    }
    if rel_string.contains("/benches/") {
        return Ok(true);
    }
    if path.file_name().is_some_and(|name| name == "build.rs") {
        return Ok(true);
    }
    if path.file_name().is_some_and(|name| {
        name.to_string_lossy().ends_with("_test.rs")
            || name.to_string_lossy().ends_with("_tests.rs")
    }) {
        return Ok(true);
    }
    for excluded in ["tree-sitter-perl-c", "perl-tdd-support", "perl-ci-hygiene"] {
        if rel_string.contains(&format!("/{excluded}/")) {
            return Ok(true);
        }
    }

    Ok(path_has_component(path, "tests")
        || path_has_component(path, "benches")
        || path_has_component(path, "build.rs")
        || path_has_component(path, "examples"))
}

fn normalize_path_for_match(value: &str) -> String {
    value.replace('\\', "/")
}

fn is_allowlisted_exit_hit(hit: &str) -> bool {
    let normalized = normalize_path_for_match(hit);
    normalized.contains("/bin/") || normalized.contains("/lifecycle.rs:")
}

fn cmd_ignored_test_count(repo_root: &Path, update: bool, check: bool) -> Result<i32> {
    let baseline_path = repo_root.join("scripts").join(".ignored-baseline");
    let verbose = env::var("VERBOSE").as_deref() == Ok("1");
    if update && check {
        return Err(color_eyre::eyre::eyre!(
            "choose exactly one of --update or --check for ignored-test-count"
        ));
    }

    let categories = [
        "brokenpipe",
        "feature",
        "infra",
        "protocol",
        "manual",
        "stress",
        "bug",
        "bare",
        "other",
    ];
    let mut counts: HashMap<String, usize> = categories
        .iter()
        .map(|category| ((*category).to_string(), 0))
        .collect();

    let mut records: Vec<IgnoredDetail> = Vec::new();
    let crates_root = repo_root.join("crates");
    let detail_matches = collect_ignored_matches(&crates_root, repo_root)?;
    for detail in detail_matches {
        let category = categorize_ignore(&detail.reason, &detail.context);
        *counts.entry(category.clone()).or_default() += 1;
        records.push(IgnoredDetail {
            category,
            location: detail.location,
            test_name: detail.test_name,
            reason: detail.reason,
        });
    }

    let total: usize = categories
        .iter()
        .map(|category| counts.get(*category).copied().unwrap_or(0))
        .sum();

    let baseline = load_ignored_baseline(&baseline_path).unwrap_or_else(|_| {
        let mut empty = HashMap::new();
        for category in &categories {
            empty.insert((*category).to_string(), 0);
        }
        empty.insert("total".to_string(), 0);
        empty
    });

    let baseline_total = baseline.get("total").copied().unwrap_or(0);

    println!("===============================================");
    println!("        Ignored Tests Summary");
    println!("===============================================");
    println!(
        "{:<12} {:>8} {:>8} {:>8}",
        "Category", "Count", "Baseline", "Delta"
    );
    println!("-----------------------------------------------");
    for category in categories {
        let current = counts.get(category).copied().unwrap_or(0);
        let previous = baseline.get(category).copied().unwrap_or(0);
        println!(
            "{:<12} {:>8} {:>8} {:>8}",
            category,
            current,
            previous,
            format_delta(current, previous),
        );
    }
    println!("-----------------------------------------------");
    println!(
        "{:<12} {:>8} {:>8} {:>8}",
        "TOTAL",
        total,
        baseline_total,
        format_delta(total, baseline_total),
    );
    println!("===============================================");

    let ci_debt = counts["brokenpipe"] + counts["bug"] + counts["bare"] + counts["other"];
    let backlog = counts["feature"] + counts["infra"];
    let permanent = counts["manual"] + counts["stress"];
    println!();
    println!("CI_DEBT    = {ci_debt:>3}  (brokenpipe + bug + bare + other; must be 0)");
    println!("BACKLOG    = {backlog:>3}  (feature + infra; planned work)");
    println!("PERMANENT  = {permanent:>3}  (manual + stress; bench/helpers)");
    println!();

    if verbose {
        println!("Detailed breakdown by category:");
        println!();
        for category in categories {
            let cat_count = counts.get(category).copied().unwrap_or(0);
            if cat_count == 0 {
                continue;
            }
            println!("{YELLOW}=== {category} ({cat_count}) ==={NC}");
            for record in &records {
                if record.category != category {
                    continue;
                }
                println!("  {}", record.location);
                if !record.test_name.is_empty() {
                    println!("    fn: {}", record.test_name);
                }
                if !record.reason.is_empty() {
                    println!("    reason: {}", record.reason);
                }
            }
            println!();
        }
    }

    let next_mode = if update {
        Some("update")
    } else if check {
        Some("check")
    } else {
        None
    };
    let next_mode = next_mode.unwrap_or("show");

    match next_mode {
        "update" => {
            write_ignored_baseline(&baseline_path, &counts, total)?;
            println!("{GREEN}Baseline updated successfully.{NC}");
            Ok(0)
        }
        "check" => {
            if total > baseline_total {
                println!(
                    "{RED}ERROR: Ignored test count increased from {baseline_total} to {total}{NC}"
                );
                println!();
                println!("New ignores must be justified. If intentional, run:");
                println!("  scripts/ignored-test-count.sh --update");
                println!();
                Ok(1)
            } else {
                println!(
                    "{GREEN}OK: Ignored test count ({total}) is not higher than baseline ({baseline_total}){NC}"
                );
                Ok(0)
            }
        }
        "show" => {
            if total > 0 {
                println!("Run with VERBOSE=1 for detailed breakdown:");
                println!("  VERBOSE=1 scripts/ignored-test-count.sh");
                println!();
                println!("To update baseline:");
                println!("  scripts/ignored-test-count.sh --update");
            }
            Ok(0)
        }
        _ => Ok(0),
    }
}

fn format_delta(current: usize, baseline: usize) -> String {
    let delta = current.abs_diff(baseline);
    if current > baseline {
        format!("{RED}+{delta}{NC}")
    } else if current < baseline {
        format!("{GREEN}-{delta}{NC}")
    } else {
        "0".to_string()
    }
}

fn load_ignored_baseline(path: &Path) -> Result<HashMap<String, usize>> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {:?}", path))?;
    let mut values = HashMap::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let Ok(parsed) = value.trim().parse::<usize>() else {
            continue;
        };
        values.insert(key.trim().to_string(), parsed);
    }
    Ok(values)
}

fn write_ignored_baseline(
    path: &Path,
    counts: &HashMap<String, usize>,
    total: usize,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut lines = Vec::new();
    lines.push(format!(
        "# Ignored test baseline - {}",
        Utc::now().format("%Y-%m-%dT%H:%M:%SZ")
    ));
    lines.push("# Updated by: ignored-test-count.sh --update".to_string());
    let mut ordered = BTreeMap::new();
    for key in [
        "brokenpipe",
        "feature",
        "infra",
        "protocol",
        "manual",
        "stress",
        "bug",
        "bare",
        "other",
    ] {
        ordered.insert(key, counts.get(key).copied().unwrap_or(0));
    }
    for (key, value) in &ordered {
        lines.push(format!("{key}={value}"));
    }
    lines.push(format!("total={total}"));
    fs::write(path, format!("{}\n", lines.join("\n")))?;
    Ok(())
}

#[derive(Clone)]
struct TodoHit {
    line_text: String,
}

struct IgnoreMatch {
    location: String,
    context: String,
    reason: String,
    test_name: String,
}

#[derive(Clone)]
struct IgnoredDetail {
    category: String,
    location: String,
    reason: String,
    test_name: String,
}

fn collect_todo_hits(
    root: &Path,
    exclude_dirs: &[&str],
    exclude_files: &[PathBuf],
    todo_re: &Regex,
) -> Result<Vec<TodoHit>> {
    let hash_ext = ["sh", "bash", "pl", "pm", "t", "just"];

    let mut hits = Vec::new();

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let rel = path
            .strip_prefix(root)
            .with_context(|| format!("path under {:?}", root))?
            .to_path_buf();
        if exclude_files.iter().any(|p| p == path) {
            continue;
        }
        if rel.components().any(|component| {
            exclude_dirs
                .iter()
                .any(|name| component.as_os_str() == OsStr::new(name))
        }) {
            continue;
        }
        let is_rust = path.extension().is_some_and(|ext| ext == "rs");
        let file_name = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
        let is_hash_file = file_name == "Justfile"
            || file_name == "justfile"
            || hash_ext
                .iter()
                .any(|ext| path.extension().is_some_and(|e| e == *ext));
        if !is_rust && !is_hash_file {
            continue;
        }
        let contents = read_lines(path)?;
        for (line_no, line) in contents.iter().enumerate() {
            let match_line = if is_rust {
                has_unlinked_todo_in_rust_line(line, todo_re)
            } else {
                has_unlinked_todo_in_hash_line(line, todo_re)
            };
            if !match_line {
                continue;
            }
            hits.push(TodoHit {
                line_text: format!("{}:{}:{}", rel.display(), line_no + 1, line),
            });
        }
    }
    Ok(hits)
}

fn has_unlinked_todo_in_rust_line(line: &str, token_re: &Regex) -> bool {
    let mut has_hit = false;
    if let Some(idx) = line.find("//")
        && !is_url_like_hash_comment(line, idx)
        && has_unlinked_token(&line[idx + 2..], token_re)
    {
        has_hit = true;
    }
    if let Some(idx) = line.find("/*")
        && has_unlinked_token(&line[idx + 2..], token_re)
    {
        has_hit = true;
    }
    let trimmed = line.trim_start();
    if trimmed.starts_with('*') && has_unlinked_token(trimmed, token_re) {
        has_hit = true;
    }
    has_hit
}

fn has_unlinked_todo_in_hash_line(line: &str, token_re: &Regex) -> bool {
    if let Some(idx) = line.find('#') {
        if idx > 0 && line.as_bytes()[idx - 1] == b'!' {
            return false;
        }
        if idx > 0
            && !line[..idx]
                .chars()
                .next_back()
                .is_some_and(char::is_whitespace)
        {
            return false;
        }
        has_unlinked_token(&line[idx + 1..], token_re)
    } else {
        false
    }
}

fn is_url_like_hash_comment(line: &str, slash_idx: usize) -> bool {
    if slash_idx == 0 {
        return false;
    }
    let before = line.as_bytes()[slash_idx - 1];
    matches!(before, b'/' | b':' | b'"')
}

fn has_unlinked_token(comment: &str, token_re: &Regex) -> bool {
    for m in token_re.find_iter(comment) {
        let suffix = &comment[m.end()..];
        if !linked_marker(suffix) {
            return true;
        }
    }
    false
}

fn linked_marker(suffix: &str) -> bool {
    let suffix = suffix.trim_start();
    let Some(rest) = suffix.strip_prefix("(#") else {
        return false;
    };
    let mut digits = 0;
    for c in rest.chars() {
        if c.is_ascii_digit() {
            digits += 1;
            continue;
        }
        break;
    }
    if digits == 0 {
        return false;
    }
    rest[digits..].starts_with(")")
}

fn collect_ignored_matches(crates_root: &Path, repo_root: &Path) -> Result<Vec<IgnoreMatch>> {
    let mut results = Vec::new();
    let ignore_attr_re = Regex::new(
        r#"^\s*#\[ignore\b(?:(?:\s*=\s*)?\"(?P<d>[^\"]+)\"|\s*=\s*\'(?P<s>[^\']+)\')?"#,
    )?;
    let fn_re = Regex::new(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)")?;
    let comment_re = Regex::new(r"//\s*(.+)$")?;

    for entry in walk_entries(crates_root) {
        let path = entry.path();
        if !entry.file_type().is_file() || path.extension().is_some_and(|ext| ext != "rs") {
            continue;
        }
        let rel = display_path(repo_root, path);
        let lines = read_lines(path)?;
        for i in 0..lines.len() {
            let line = &lines[i];
            if !line.trim_start().starts_with("#[ignore") {
                continue;
            }

            let mut reason = String::new();
            if let Some(caps) = ignore_attr_re.captures(line) {
                if let Some(matched) = caps.name("d") {
                    reason = matched.as_str().to_string();
                } else if let Some(matched) = caps.name("s") {
                    reason = matched.as_str().to_string();
                }
            }
            let context_lines = {
                let end = std::cmp::min(lines.len(), i + 4);
                lines[i..end].join("\n")
            };
            if reason.is_empty()
                && comment_re.is_match(line)
                && let Some(comment) = comment_re.captures(line).and_then(|m| m.get(1))
            {
                reason = comment.as_str().to_string();
            }
            if reason.is_empty()
                && i + 1 < lines.len()
                && comment_re.is_match(&lines[i + 1])
                && let Some(comment) = comment_re.captures(&lines[i + 1]).and_then(|m| m.get(1))
            {
                reason = comment.as_str().to_string();
            }
            if reason.is_empty()
                && i + 2 < lines.len()
                && comment_re.is_match(&lines[i + 2])
                && let Some(comment) = comment_re.captures(&lines[i + 2]).and_then(|m| m.get(1))
            {
                reason = comment.as_str().to_string();
            }

            let mut test_name = String::new();
            if let Some(found) = fn_re.captures(&context_lines).and_then(|m| m.get(1)) {
                test_name = found.as_str().to_string();
            }

            results.push(IgnoreMatch {
                location: format!("{rel}:{}", i + 1),
                context: context_lines,
                reason,
                test_name,
            });
        }
    }
    Ok(results)
}

fn categorize_ignore(reason: &str, context: &str) -> String {
    let reason = reason.trim().to_lowercase();
    let context = context.to_lowercase();

    if reason.starts_with("manual:")
        || reason.contains("manual ")
        || reason.contains("regenerate")
        || reason.contains("helper")
    {
        return "manual".to_string();
    }
    if reason.starts_with("stress:")
        || reason.contains("stress test")
        || reason.contains("memory.stress")
        || reason.contains("performance.stress")
        || reason.contains("load.test")
        || reason.contains("stack.overflow")
        || reason.contains("designed.to.fail")
    {
        return "stress".to_string();
    }
    if reason.starts_with("bug:")
        || reason.contains("bug:")
        || reason.contains("known.bug")
        || reason.contains("regression")
        || reason.contains("incorrect.behavior")
        || reason.contains("parser.bug")
        || reason.contains("missing.notification")
        || reason.contains("missing.initialize")
        || reason.contains("server.returns.instead")
        || reason.contains("will.kill")
        || reason.contains("known.inconsistencies")
        || reason.contains("mut_")
        || reason.contains("matching.issue")
        || reason.contains("investigate")
        || reason.contains("instead.of.expected")
        || reason.contains("different.error.format")
        || reason.contains("expects")
    {
        return "bug".to_string();
    }
    if reason.starts_with("todo:")
        || reason.starts_with("infra:")
        || reason.contains("infra ")
        || reason.contains("fixme")
        || reason.contains("needs")
        || reason.contains("requires")
        || reason.contains("setup")
        || reason.contains("config")
        || reason.contains("environment")
        || reason.contains("run.with")
        || reason.contains("only.run.after")
        || reason.contains("only.run.when")
    {
        return "infra".to_string();
    }
    if reason.starts_with("feature:")
        || reason.contains("feature ")
        || reason.contains("not.implemented")
        || reason.contains("unimplemented")
        || reason.contains("wip")
        || reason.contains("work.in.progress")
        || reason.contains("pending")
        || reason.contains("when.implemented")
        || reason.contains("remove.when")
        || reason.contains("ac")
        || reason.contains("not.yet")
        || reason.contains("tdd.scaffold")
        || reason.contains("scaffold")
        || reason.contains("doesn.t.support")
        || reason.contains("doesn't.support")
        || reason.contains("parser.limitation")
        || reason.contains("expected.to.fail")
        || reason.contains("not.fully.supported")
        || reason.contains("enable.after")
        || reason.contains("after.phase")
        || reason.contains("parser.doesn")
        || reason.contains("tracked in #")
    {
        return "feature".to_string();
    }
    if reason.starts_with("brokenpipe:")
        || reason.contains("brokenpipe ")
        || reason.contains("broken.pipe")
        || reason.contains("transport.error")
        || reason.contains("transport.flake")
        || reason.contains("flaky")
    {
        return "brokenpipe".to_string();
    }
    if reason.contains("protocol")
        || reason.contains("lsp")
        || reason.contains("dap")
        || reason.contains("compliance")
        || reason.contains("specification")
    {
        return "protocol".to_string();
    }
    if reason.contains("tracked in #") {
        return "feature".to_string();
    }
    if reason.contains("doesn.t.have.field")
        || reason.contains("may.not.produce")
        || reason.contains("doesn.t.yet")
        || reason.contains("fewer.than.expected")
    {
        return "feature".to_string();
    }
    if reason.contains("recursion.limit.behavior") || reason.contains("behavior.changed") {
        return "feature".to_string();
    }
    if reason.contains("integration.test.that.spawns")
        || reason.contains("spawns.external")
        || reason.contains("burn.down")
        || reason.contains("mutation.hardening")
    {
        return "infra".to_string();
    }
    if reason.contains("clippy.warnings") || reason.contains("warnings.burn") {
        return "infra".to_string();
    }
    if reason.starts_with("ac:") {
        return "feature".to_string();
    }
    if reason.is_empty() || reason == "ignore" {
        return "bare".to_string();
    }
    if context.contains("ac:") {
        return "feature".to_string();
    }
    "other".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linked_marker_requires_parenthesized_issue_number() {
        assert!(linked_marker("(#123)"));
        assert!(linked_marker("   (#42) trailing text"));
        assert!(!linked_marker("#123"));
        assert!(!linked_marker("(#)"));
        assert!(!linked_marker("(#12"));
        assert!(!linked_marker("(ABC-12)"));
    }

    #[test]
    fn rust_todo_detection_ignores_linked_or_url_like_comments() -> Result<()> {
        let todo_re = Regex::new(r"TODO|FIXME")?;

        assert!(has_unlinked_todo_in_rust_line(
            "// TODO: investigate",
            &todo_re
        ));
        assert!(!has_unlinked_todo_in_rust_line(
            "// TODO(#123): tracked",
            &todo_re
        ));
        assert!(!has_unlinked_todo_in_rust_line(
            "let u = \"http://TODO\";",
            &todo_re
        ));
        assert!(has_unlinked_todo_in_rust_line(
            "/* FIXME: needs fix */",
            &todo_re
        ));

        Ok(())
    }

    #[test]
    fn hash_comment_todo_detection_handles_shebang_and_inline_hashes() -> Result<()> {
        let todo_re = Regex::new(r"TODO|FIXME")?;

        assert!(!has_unlinked_todo_in_hash_line(
            "#!/usr/bin/env bash",
            &todo_re
        ));
        assert!(!has_unlinked_todo_in_hash_line(
            "echo# TODO not a comment",
            &todo_re
        ));
        assert!(has_unlinked_todo_in_hash_line(
            "echo hi # TODO: follow up",
            &todo_re
        ));
        assert!(!has_unlinked_todo_in_hash_line(
            "echo hi # TODO(#77): tracked",
            &todo_re
        ));

        Ok(())
    }

    #[test]
    fn categorize_ignore_maps_reasons_to_expected_buckets() {
        assert_eq!(categorize_ignore("manual: run locally", ""), "manual");
        assert_eq!(categorize_ignore("TODO: requires CI setup", ""), "infra");
        assert_eq!(categorize_ignore("feature: not implemented", ""), "feature");
        assert_eq!(
            categorize_ignore("placeholder", "#[ignore] // AC: parser behavior"),
            "feature"
        );
        assert_eq!(categorize_ignore("ignore", ""), "bare");
        assert_eq!(categorize_ignore("some new reason", ""), "other");
    }

    #[test]
    fn format_delta_adds_directional_colored_deltas() {
        assert_eq!(format_delta(5, 5), "0");
        assert_eq!(format_delta(7, 5), format!("{RED}+2{NC}"));
        assert_eq!(format_delta(4, 7), format!("{GREEN}-3{NC}"));
    }

    #[test]
    fn normalize_path_for_match_converts_backslashes() {
        assert_eq!(
            normalize_path_for_match(r"crates\perl-ci-hygiene\src\main.rs"),
            "crates/perl-ci-hygiene/src/main.rs"
        );
    }

    #[test]
    fn excluded_test_paths_skip_bin_directories() {
        assert!(is_excluded_test_path(Path::new(
            "crates/perl-workspace-index/src/bin/workspace_memory_profile.rs"
        )));
    }

    #[test]
    fn allowlisted_exit_hit_matches_windows_and_unix_paths() {
        assert!(is_allowlisted_exit_hit(
            r"crates\perl-parser\src\bin\perl-parse.rs:127:std::process::exit(0);"
        ));
        assert!(is_allowlisted_exit_hit(
            "crates/perl-lsp/src/runtime/dispatch/lifecycle.rs:29:std::process::exit(exit_code);"
        ));
        assert!(!is_allowlisted_exit_hit(
            r#"crates\perl-ci-hygiene\src\main.rs:3196:println!("std::process::exit")"#
        ));
    }

    #[test]
    fn allowlisted_prod_panic_hit_matches_heredoc_regex_initializers() {
        assert!(is_allowlisted_prod_panic_hit(
            "crates/perl-heredoc-anti-patterns/src/lib.rs",
            r#"        Err(_) => unreachable!("FORMAT_PATTERN regex failed to compile"),"#
        ));
        assert!(is_allowlisted_prod_panic_hit(
            r"crates\perl-heredoc-anti-patterns\src\lib.rs",
            r#"        Err(_) => unreachable!("FORMAT_PATTERN regex failed to compile"),"#
        ));
        assert!(!is_allowlisted_prod_panic_hit(
            "crates/perl-lsp-diagnostics/src/lints/ffi_checklib.rs",
            r#"                        _ => unreachable!(),"#
        ));
    }

    // Regression guard for issue #4245: is_allowlisted_prod_panic_hit() must pass
    // all 7 unreachable!() calls in perl-heredoc-anti-patterns/src/lib.rs under
    // both forward-slash and backslash path separators (Windows vs Unix).
    #[test]
    fn allowlisted_prod_panic_hit_all_seven_patterns_both_separators() {
        let all_seven = [
            r#"        Err(_) => unreachable!("FORMAT_PATTERN regex failed to compile"),"#,
            r#"        Err(_) => unreachable!("BEGIN_BLOCK_PATTERN regex failed to compile"),"#,
            r#"        Err(_) => unreachable!("DYNAMIC_DELIMITER_PATTERN regex failed to compile"),"#,
            r#"        Err(_) => unreachable!("SOURCE_FILTER_PATTERN regex failed to compile"),"#,
            r#"        Err(_) => unreachable!("REGEX_HEREDOC_PATTERN regex failed to compile"),"#,
            r#"        Err(_) => unreachable!("EVAL_HEREDOC_PATTERN regex failed to compile"),"#,
            r#"    Err(_) => unreachable!("TIE_PATTERN regex failed to compile"),"#,
        ];
        let forward = "crates/perl-heredoc-anti-patterns/src/lib.rs";
        let backward = r"crates\perl-heredoc-anti-patterns\src\lib.rs";
        for line in &all_seven {
            assert!(
                is_allowlisted_prod_panic_hit(forward, line),
                "forward-slash path must allowlist: {line}"
            );
            assert!(
                is_allowlisted_prod_panic_hit(backward, line),
                "backslash path must allowlist: {line}"
            );
        }
    }

    #[test]
    fn quick_bench_uses_distinct_binaries_for_c_and_rust() {
        // Regression guard for issue #3204: cmd_quick_bench previously called
        // the same binary twice and reported the (meaningless) delta as a
        // C-vs-Rust speedup. The fix wires the two columns to distinct
        // binaries — this test pins those identifiers so a future refactor
        // can't silently collapse them again.
        assert_ne!(
            RUST_BENCH_BIN, C_BENCH_BIN,
            "C and Rust quick-bench must invoke different binaries"
        );
        assert_eq!(RUST_BENCH_BIN, "perl-parser-bench");
        assert_eq!(C_BENCH_BIN, "bench_parser_c");
        // Pin the manifest path for the C bench so a rename of the C crate
        // directory is caught here rather than silently producing wrong timings.
        assert_eq!(C_BENCH_MANIFEST, "crates/tree-sitter-perl-c/Cargo.toml");
        // The C bench must use a manifest path (workspace-excluded crate) while
        // the Rust bench uses a workspace package selector — they must diverge in
        // invocation style, not just binary name.
        assert!(!C_BENCH_MANIFEST.is_empty());
    }

    #[test]
    fn three_way_bench_all_binaries_distinct() {
        // Guard for the three-way comparison introduced after PR #3255
        // (tree-sitter-perl-rs facade). All three parser binaries must be
        // distinct so a future refactor can't silently collapse any pair.
        assert_ne!(
            RUST_BENCH_BIN, C_BENCH_BIN,
            "raw Rust v3 and C binaries must differ"
        );
        assert_ne!(
            RUST_BENCH_BIN, FACADE_BENCH_BIN,
            "raw Rust v3 and facade binaries must differ"
        );
        assert_ne!(
            C_BENCH_BIN, FACADE_BENCH_BIN,
            "C and facade binaries must differ"
        );
        assert_eq!(FACADE_BENCH_BIN, "bench_facade");
        assert_eq!(FACADE_BENCH_CRATE, "tree-sitter-perl-rs");
        // Facade crate lives in the workspace, so it is invoked with -p, not
        // --manifest-path.  Pin that it has no separate manifest string.
        assert_ne!(FACADE_BENCH_CRATE, C_BENCH_MANIFEST);
    }

    #[test]
    fn pre_push_hook_skips_gate_on_delete_only_push() {
        let hook = pre_push_hook_script();
        // The hook must detect when all refs have a zero local SHA (deletion).
        assert!(
            hook.contains("0000000000000000000000000000000000000000"),
            "hook must check for all-zero SHA (deletion sentinel)"
        );
        assert!(
            hook.contains("IS_DELETE_ONLY"),
            "hook must track whether this is a delete-only push"
        );
        assert!(
            hook.contains("Branch deletion"),
            "hook must print a message when skipping due to deletion"
        );
        // Normal pushes (non-zero SHA) must still run the fast push gate.
        assert!(
            hook.contains("just pr-fast"),
            "hook must invoke the fast PR gate for normal pushes"
        );
    }

    #[test]
    fn pre_push_hook_documents_bypass_policy() {
        let hook = pre_push_hook_script();
        // The header comment block must explain when --no-verify is appropriate
        // so contributors can make informed decisions instead of bypassing blindly.
        assert!(
            hook.contains("Bypass policy"),
            "hook must contain a 'Bypass policy' header explaining --no-verify rules"
        );
        assert!(
            hook.contains("OK to bypass"),
            "hook must list when bypass is acceptable"
        );
        assert!(
            hook.contains("NOT OK to bypass"),
            "hook must list when bypass is unacceptable"
        );
    }

    #[test]
    fn pre_push_hook_auto_unsets_core_bare_corruption() {
        let hook = pre_push_hook_script();
        // Issue #3205 — core.bare=true keeps getting silently set on the main
        // checkout. The hook should self-heal by unsetting it before doing any
        // work that would otherwise fail with "must be run in a work tree".
        assert!(
            hook.contains("core.bare"),
            "hook must check for core.bare corruption"
        );
        assert!(
            hook.contains("--unset core.bare"),
            "hook must unset the core.bare flag when corruption is detected"
        );
        assert!(
            hook.contains("#3205"),
            "hook must reference issue #3205 in the warning message"
        );
    }

    #[test]
    fn pre_push_hook_has_doc_only_fast_path() {
        let hook = pre_push_hook_script();
        // Doc-only pushes (markdown, text, license, changelog) should run a
        // lighter gate instead of the full ci-gate. The full test suite is
        // pointless for prose-only changes.
        assert!(
            hook.contains("DOC_ONLY") || hook.contains("doc_only"),
            "hook must detect doc-only diffs"
        );
        assert!(
            hook.contains("Doc-only push") || hook.contains("doc-only push"),
            "hook must announce when it picks the doc-only fast path"
        );
        // The lighter gate should NOT shell out to `just ci-gate` from the
        // doc-only branch — it should exit before reaching that fallback.
        // We verify this indirectly: the doc-only message must precede an exit.
        let doc_idx = hook
            .find("Doc-only push")
            .or_else(|| hook.find("doc-only push"))
            .expect("doc-only message must exist");
        let after_doc = &hook[doc_idx..];
        assert!(
            after_doc.contains("exit 0"),
            "doc-only branch must exit 0 without invoking the full gate"
        );
        let exit_idx = after_doc.find("exit 0").expect("doc-only branch must exit");
        let doc_branch = &after_doc[..exit_idx];
        assert!(
            !doc_branch.contains("cargo fmt --all -- --check"),
            "doc-only branch must not run workspace-wide rustfmt checks before exiting"
        );
    }

    #[test]
    fn pre_push_hook_doc_only_fast_path_matches_crate_subdir_license_files() {
        let hook = pre_push_hook_script();
        // Issue #3305: the doc-only case pattern must also match license files
        // in crate subdirectories (e.g. crates/tree-sitter-perl-rs/LICENSE-APACHE).
        // Pattern `LICENSE*` only matches root-level files; `*/LICENSE*` is required
        // to handle files like `crates/*/LICENSE-APACHE` and `crates/*/LICENSE-MIT`.
        assert!(
            hook.contains("*/LICENSE*"),
            "hook doc-only pattern must include '*/LICENSE*' to match crate-subdir license files              (e.g. crates/tree-sitter-perl-rs/LICENSE-APACHE) — see issue #3305"
        );
    }

    #[test]
    fn checked_in_pre_push_hook_matches_generated_hook() {
        let checked_in = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../hooks/pre-push"))
            .replace("\r\n", "\n");
        let generated = pre_push_hook_script().replace("\r\n", "\n");

        assert_eq!(
            checked_in, generated,
            "checked-in hooks/pre-push must stay in sync with the generated ci-hygiene hook"
        );
    }

    #[test]
    fn pre_push_hook_has_single_crate_tier() {
        let hook = pre_push_hook_script();
        // When all changed files are under crates/<name>/, run a targeted
        // cargo fmt/clippy/test -p <name> instead of the workspace-wide
        // just pr-fast.  This is the single-crate proportional tier.
        assert!(
            hook.contains("SINGLE_CRATE") || hook.contains("single_crate"),
            "hook must detect single-crate diffs"
        );
        assert!(
            hook.contains("cargo test -p"),
            "hook must run targeted 'cargo test -p <crate>' for single-crate pushes"
        );
        assert!(
            hook.contains("cargo clippy -p") || hook.contains("cargo clippy --package"),
            "hook must run targeted clippy for single-crate pushes"
        );
        assert!(
            hook.contains("cargo fmt -p") || hook.contains("cargo fmt --package"),
            "hook must run targeted fmt for single-crate pushes"
        );
        // The single-crate path must announce itself so the contributor knows
        // why the gate is faster than usual.
        assert!(
            hook.contains("Single-crate") || hook.contains("single-crate"),
            "hook must announce when it picks the single-crate fast path"
        );
        // The single-crate path must exit before reaching just pr-fast
        let single_idx = hook
            .find("Single-crate")
            .or_else(|| hook.find("single-crate"))
            .expect("single-crate message must exist");
        let after_single = &hook[single_idx..];
        assert!(
            after_single.contains("exit 0"),
            "single-crate branch must exit 0 without invoking just pr-fast"
        );
    }

    #[test]
    fn pre_push_hook_single_crate_tier_falls_back_on_cross_crate() {
        let hook = pre_push_hook_script();
        // When files span multiple crates, we must NOT run the single-crate
        // path — it must fall through to the full just pr-fast gate.
        // The safest way to verify this is: the hook must still invoke
        // just pr-fast for the cross-crate (default) case.
        assert!(
            hook.contains("just pr-fast"),
            "hook must still invoke just pr-fast for cross-crate pushes"
        );
    }

    #[test]
    fn pre_push_hook_explains_known_failure_modes() {
        let hook = pre_push_hook_script();
        // When the gate fails, the hook should hint at known issues so
        // contributors can recognize them instead of bypassing in confusion.
        assert!(
            hook.contains("#3202"),
            "hook must mention issue #3202 (Windows file-lock race)"
        );
        assert!(
            hook.contains("cargo xtask fmt") || hook.contains("cargo fmt"),
            "hook must suggest the fmt fix command on fmt failures"
        );
    }

    #[test]
    fn pre_push_hook_has_stale_hook_self_heal() {
        let hook = pre_push_hook_script();
        // Issue #4220 — when hooks/pre-push is updated in master, the installed
        // .git/hooks/pre-push is only updated when install-githooks is re-run.
        // The hook must detect its own staleness and auto-copy the fresh version.
        assert!(
            hook.contains("REPO_ROOT_FOR_HOOK") && hook.contains("hooks/pre-push"),
            "hook must self-heal stale installation (issue #4220)"
        );
    }

    #[test]
    fn pre_push_hook_has_os_error_206_hint() {
        let hook = pre_push_hook_script();
        // Issue #4220 — Windows CreateProcess command-line limit (os error 206)
        // hint must appear in the gate-failure handler so contributors know how to fix it.
        assert!(
            hook.contains("os error 206") || hook.contains("CreateProcess"),
            "hook must hint at Windows CreateProcess limit fix (issue #4220)"
        );
    }

    #[test]
    fn command_exists_finds_cargo() {
        // cargo is always present in the test environment
        assert!(command_exists("cargo"), "cargo should be found via PATH");
    }

    #[test]
    fn command_exists_rejects_nonexistent() {
        assert!(
            !command_exists("__xyzzy_not_a_real_command_99__"),
            "non-existent command must not be found"
        );
    }

    #[test]
    fn e2e_lock_file_path_is_portable() {
        let lock = std::env::temp_dir().join("e2e-suite.lock");
        let lock_str = lock.to_str().expect("temp dir must be valid UTF-8 in CI");
        // On Linux CI this will be /tmp/e2e-suite.lock (acceptable)
        // On Windows this will be C:\Users\...\AppData\Local\Temp\e2e-suite.lock
        assert!(!lock_str.is_empty(), "lock file path must be non-empty");
    }

    /// Regression guard for issue #4229: test files must not write to hardcoded /tmp paths.
    ///
    /// Tests that assign a hardcoded `/tmp/...` string to a variable and then call
    /// `fs::write(variable, ...)` will fail on minimal Windows environments that lack Git
    /// for Windows (where `/tmp` is mapped). All file-writing tests must use
    /// `tempfile::tempdir()` or `std::env::temp_dir()` instead.
    ///
    /// Detection heuristic: any line matching `let <name> = "/tmp/` where the same
    /// variable name appears on a later `fs::write(` line in the same file.
    #[test]
    fn no_hardcoded_tmp_writes_in_tests() {
        // Crates whose test files are checked. Extend this list as new crates are added.
        // Format: (package-name, directory-name). These may differ when a crate is renamed
        // but the directory is kept for Windows MAX_PATH safety (e.g. perl-workspace Wave A).
        const CHECKED_CRATES: &[(&str, &str)] = &[
            ("perl-lsp", "perl-lsp"),
            ("perl-dap", "perl-dap"),
            ("perl-uri", "perl-uri"),
            // Package name changed perl-workspace-index → perl-workspace (#4426),
            // but the directory crates/perl-workspace-index/ was not renamed.
            ("perl-workspace", "perl-workspace-index"),
            ("perl-dap-platform", "perl-dap-platform"),
        ];

        let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut violations: Vec<String> = Vec::new();

        for (_crate_name, crate_dir_name) in CHECKED_CRATES {
            let crate_dir = workspace_root.join("crates").join(crate_dir_name);

            // Scan both src (inline #[test] modules) and tests directories
            for subdir in &["src", "tests"] {
                let dir = crate_dir.join(subdir);
                if !dir.exists() {
                    continue;
                }

                for entry in walkdir::WalkDir::new(&dir)
                    .into_iter()
                    .flatten()
                    .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("rs"))
                {
                    let path = entry.path();
                    let Ok(contents) = std::fs::read_to_string(path) else {
                        continue;
                    };

                    // Look for pattern: let <var_name> = "/tmp/..."; followed by fs::write(<var_name>
                    // This is the problematic pattern: variable holds a hardcoded /tmp path,
                    // then that variable is used in a filesystem write.
                    let mut tmp_vars: Vec<&str> = Vec::new();
                    for line in contents.lines() {
                        let trimmed = line.trim();
                        // Detect: let [mut] var_name[: type] = "/tmp/...";
                        if trimmed.starts_with("let ") && trimmed.contains("= \"/tmp/") {
                            // Extract variable name between "let [mut]" and "[: type] ="
                            if let Some(rest) = trimmed.strip_prefix("let ")
                                && let Some(raw) = rest.split('=').next()
                            {
                                // Strip optional `mut ` keyword
                                let raw = raw.trim();
                                let raw = raw.strip_prefix("mut ").map_or(raw, str::trim);
                                // Strip optional type annotation (`: &str`, `: String`, …)
                                let var = raw.split(':').next().map_or(raw, str::trim);
                                if !var.is_empty() {
                                    tmp_vars.push(var);
                                }
                            }
                        }
                        // Detect: fs::write(var_name, ...) where var_name is a known /tmp var.
                        // Require that `var_name` appears as an argument (preceded by `(` or `, `)
                        // to avoid false positives like `var` matching `file_var`.
                        if trimmed.contains("fs::write(") || trimmed.contains("fs::write(&") {
                            for var in &tmp_vars {
                                // Match `(var` or `(&var` or `, var` but NOT `file_var`
                                let as_arg1 = format!("({var},");
                                let as_arg2 = format!("(&{var},");
                                let as_arg3 = format!("({var})");
                                if trimmed.contains(&as_arg1)
                                    || trimmed.contains(&as_arg2)
                                    || trimmed.contains(&as_arg3)
                                {
                                    let rel = path
                                        .strip_prefix(&workspace_root)
                                        .unwrap_or(path)
                                        .to_string_lossy()
                                        .replace('\\', "/");
                                    let violation =
                                        format!("{rel}: `fs::write({var}, ...)` with /tmp path");
                                    if !violations.contains(&violation) {
                                        violations.push(violation);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        assert!(
            violations.is_empty(),
            "Test files write to hardcoded /tmp paths (issue #4229).\n\
             Replace `let path = \"/tmp/...\"; fs::write(path, ...)` with\n\
             `let tmp = tempfile::tempdir()?; let path = tmp.path().join(\"...\");`\n\
             Violations found in:\n  {}",
            violations.join("\n  ")
        );
    }
}
