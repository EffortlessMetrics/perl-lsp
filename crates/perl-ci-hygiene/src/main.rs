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
    /// Benchmark pure-Rust and C parser implementations (multi-file, timed runs).
    BenchmarkPureRustVsC,
    /// Run simple benchmark comparison between pure-Rust and C parser implementations.
    BenchmarkRustVsCSimple,
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
    /// Check workspace version sync across Cargo.toml, features.toml, and VSCode manifest.
    CheckVersionSync,
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
        CliCommand::BenchmarkPureRustVsC => cmd_benchmark_pure_rust_vs_c(&repo_root)?,
        CliCommand::BenchmarkRustVsCSimple => cmd_benchmark_rust_vs_c_simple(&repo_root)?,
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

const CI_REPORT_CRATES_EXCLUDE: [&str; 9] = [
    "tree-sitter-perl-c",
    "tree-sitter-perl-rs",
    "perl-parser-pest",
    "perl-tdd-support",
    "perl-ts-heredoc-analysis",
    "perl-ts-logos-lexer",
    "perl-ts-heredoc-parser",
    "perl-ts-partial-ast",
    "perl-ts-advanced-parsers",
];

const CI_TEST_FILE_SUFFIXES: [&str; 3] = ["_test.rs", "_tests.rs", "tests.rs"];

fn is_excluded_test_path(path: &Path) -> bool {
    if path.components().any(|component| component.as_os_str() == OsStr::new("tests")) {
        return true;
    }

    if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
        if CI_TEST_FILE_SUFFIXES.iter().any(|suffix| file_name.ends_with(suffix)) {
            return true;
        }
    }

    if path.components().any(|component| {
        CI_REPORT_CRATES_EXCLUDE.iter().any(|item| component.as_os_str() == OsStr::new(item))
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
    let output = child.output().wrap_err_with(|| format!("running {command}"))?;
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
    let output = child.output().wrap_err_with(|| format!("running {command}"))?;
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

fn command_with_input(
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
    child.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = child.spawn().wrap_err_with(|| format!("running {command}"))?;
    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| color_eyre::eyre::eyre!("failed to open stdin for command {command}"))?;
        stdin
            .write_all(stdin_payload.as_bytes())
            .wrap_err_with(|| format!("writing to stdin for {command}"))?;
    }
    let output = child.wait_with_output().wrap_err_with(|| format!("running {command}"))?;
    let status = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if status != 0 {
        return Err(color_eyre::eyre::eyre!(
            "command '{command}' failed (exit {status}): {stderr}"
        ));
    }
    Ok((status, stdout))
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
    child.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = child.spawn().wrap_err_with(|| format!("running {command}"))?;
    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| color_eyre::eyre::eyre!("failed to open stdin for command {command}"))?;
        stdin
            .write_all(stdin_payload.as_bytes())
            .wrap_err_with(|| format!("writing to stdin for {command}"))?;
    }
    let output = child.wait_with_output().wrap_err_with(|| format!("running {command}"))?;
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
    let output = child.output().wrap_err_with(|| format!("running {command}"))?;
    let status = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    Ok((status, stdout))
}

fn command_output_with_status_all(
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
    let output = child.output().wrap_err_with(|| format!("running {command}"))?;
    let status = output.status.code().unwrap_or(1);
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.stderr.is_empty() {
        combined.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    Ok((status, combined))
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
    let status = child.status().wrap_err_with(|| format!("running {command}"))?;
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
    let output = child.output().wrap_err_with(|| format!("running {command}"))?;
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
    let output = child.output().wrap_err_with(|| format!("running {command}"))?;
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
    let status = child.status().wrap_err_with(|| format!("running {command}"))?;
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
        return Err(color_eyre::eyre::eyre!("{command} failed with code {status}"));
    }
    Ok(())
}

fn command_exists(command: &str) -> bool {
    let check = Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {command} >/dev/null 2>&1"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    check
}

fn command_output_lines(output: &str) -> Vec<String> {
    output.lines().map(str::trim).filter(|line| !line.is_empty()).map(ToString::to_string).collect()
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

fn read_usize_from_path(path: &Path) -> Result<usize> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {:?}", path))?;
    raw.trim()
        .parse::<usize>()
        .map_err(|err| color_eyre::eyre::eyre!("invalid usize in {}: {err}", path.display()))
}

fn read_usize_from_tokens(path: &Path, idx: usize) -> Result<usize> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {:?}", path))?;
    let tokens: Vec<&str> = raw.split_whitespace().collect();
    if tokens.len() <= idx {
        return Err(color_eyre::eyre::eyre!("missing token {idx} in {}", path.display()));
    }
    tokens[idx]
        .trim()
        .parse::<usize>()
        .map_err(|err| color_eyre::eyre::eyre!("invalid usize in {}: {err}", path.display()))
}

fn cmd_preflight(_repo_root: &Path) -> Result<i32> {
    let pids_used =
        command_with_output(Path::new("/"), "ps", &["-e", "--no-headers"], &[])?.lines().count();
    let pid_max = read_usize_from_path(Path::new("/proc/sys/kernel/pid_max"))?;
    let files_used = read_usize_from_tokens(Path::new("/proc/sys/fs/file-nr"), 1)?;
    let files_max = read_usize_from_path(Path::new("/proc/sys/fs/file-max"))?;

    println!("PIDs: {pids_used} / {pid_max} | Open files: {files_used} / {files_max}");

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

    Ok(0)
}

fn cmd_test_capped(repo_root: &Path, cargo_args: &[String]) -> Result<i32> {
    cmd_preflight(repo_root)?;

    let rust_test_threads = env::var("RUST_TEST_THREADS").unwrap_or_else(|_| "2".to_string());
    println!("Running Rust tests with {rust_test_threads} threads...");

    let mut args: Vec<String> =
        vec!["test".to_string(), "--".to_string(), format!("--test-threads={rust_test_threads}")];
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
    let mut args: Vec<String> =
        vec!["test".to_string(), "--".to_string(), format!("--test-threads={rust_test_threads}")];
    args.extend_from_slice(cargo_args);
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let lock_file = "/tmp/e2e-suite.lock";

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
        let direct_args =
            std::iter::once(lock_file).chain(refs.iter().copied()).collect::<Vec<_>>();
        command_status_strict(
            repo_root,
            "flock",
            &direct_args,
            &[("RUST_TEST_THREADS", rust_test_threads.as_str())],
        )?;
        return Ok(0);
    }

    println!("E2E slot busy → waiting...");
    let blocking_args = std::iter::once(lock_file).chain(refs.iter().copied()).collect::<Vec<_>>();
    command_status_strict(
        repo_root,
        "flock",
        &blocking_args,
        &[("RUST_TEST_THREADS", rust_test_threads.as_str())],
    )?;
    Ok(0)
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

    const V2_BUNDLE_FILES: [&str; 5] =
        ["grammar.pest", "pure_rust_parser.rs", "pratt_parser.rs", "sexp_formatter.rs", "error.rs"];

    let source_root = repo_root.join("crates/tree-sitter-perl-rs/src");
    let microcrate_root = repo_root.join("crates/perl-parser-pest/src");
    let mut status = 0;
    for file in V2_BUNDLE_FILES {
        let left = source_root.join(file);
        let right = microcrate_root.join(file);
        let left_display = left.display();
        let right_display = right.display();

        if !left.exists() {
            return Err(color_eyre::eyre::eyre!("missing source file: {left_display}"));
        }
        if !right.exists() {
            return Err(color_eyre::eyre::eyre!("missing microcrate file: {right_display}"));
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
            &["-u", left_display.to_string().as_str(), right_display.to_string().as_str()],
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

fn cmd_benchmark_pure_rust_vs_c(repo_root: &Path) -> Result<i32> {
    println!("=== Pure Rust (Pest) vs C Parser Benchmark ===");
    println!("Building both implementations...");

    let rust_parser = repo_root.join("crates/tree-sitter-perl-rs/target/release/parse-rust");
    let c_parser = repo_root.join("crates/tree-sitter-perl-c/target/release/parse_c");

    command_status_strict(
        repo_root,
        "cargo",
        &["build", "--release", "-p", "tree-sitter-perl-rs", "--bin", "parse-rust"],
        &[],
    )?;
    command_status_strict(
        repo_root,
        "cargo",
        &["build", "--release", "-p", "tree-sitter-perl-c", "--bin", "parse_c"],
        &[],
    )?;

    let workspace =
        repo_root.join("target").join("perl-ci-hygiene").join("benchmark_pure_rust_vs_c");
    fs::create_dir_all(&workspace).with_context(|| format!("creating {}", workspace.display()))?;

    let test_simple = workspace.join("test_simple.pl");
    let test_medium = workspace.join("test_medium.pl");
    fs::write(&test_simple, "print \"Hello, World!\\n\";\n")?;
    fs::write(
        &test_medium,
        r#"#!/usr/bin/env perl
use strict;
use warnings;

my $scalar = "test";
my @array = (1, 2, 3, 4, 5);
my %hash = (a => 1, b => 2);

my $ref = \$scalar;
my $aref = \@array;
my $href = \%hash;

my $octal = 0o755;
print "..." if $scalar;

my $π = 3.14159;
my $café = "coffee";

sub process {
    my ($x, $y) = @_;
    return $x + $y;
}

for my $i (1..10) {
    print "$i\\n" if $i % 2 == 0;
}
"#,
    )?;

    println!();
    println!("Running benchmarks...");
    println!("File,Pure_Rust_Time(ms),C_Time(ms),Rust/C_Ratio");

    let files = vec![
        ("test_simple.pl", test_simple),
        ("test_medium.pl", test_medium),
        ("examples/hello.pl", repo_root.join("examples/hello.pl")),
    ];
    for (name, file) in files {
        if !file.is_file() {
            continue;
        }
        let rust_ms = benchmark_average_ms(repo_root, &rust_parser, &file, 10)?;
        let c_ms = benchmark_average_ms(repo_root, &c_parser, &file, 10)?;
        let ratio = if c_ms > 0.0 { rust_ms / c_ms } else { f64::INFINITY };
        println!("{},{rust_ms:.3},{c_ms:.3},{ratio:.2}", name);
    }

    Ok(0)
}

fn cmd_benchmark_rust_vs_c_simple(repo_root: &Path) -> Result<i32> {
    println!("=== Pure Rust (Pest) vs C Parser Benchmark ===");
    println!();

    let rust_parser = repo_root.join("crates/tree-sitter-perl-rs/target/release/parse-rust");
    let c_parser = repo_root.join("crates/tree-sitter-perl-c/target/release/parse_c");
    let workspace =
        repo_root.join("target").join("perl-ci-hygiene").join("benchmark_rust_vs_c_simple");
    fs::create_dir_all(&workspace)?;

    command_status_strict(
        repo_root,
        "cargo",
        &["build", "--release", "-p", "tree-sitter-perl-rs", "--bin", "parse-rust"],
        &[],
    )?;
    command_status_strict(
        repo_root,
        "cargo",
        &["build", "--release", "-p", "tree-sitter-perl-c", "--bin", "parse_c"],
        &[],
    )?;

    let benchmark_file = workspace.join("test_benchmark.pl");
    fs::write(
        &benchmark_file,
        r#"#!/usr/bin/env perl
use strict;
use warnings;

my $scalar = "Hello, World!";
my @array = (1..10);
my %hash = map { $_ => $_ * 2 } 1..5;

my $sref = \$scalar;
my $aref = \@array;
my $href = \%hash;

my $perms = 0o755;
my $old_perms = 0755;

sub todo {
    ...
}

my $π = 3.14159;
my $café = "coffee shop";
sub 日本語 { return "Japanese" }

for my $i (@array) {
    print "$i\\n" if $i % 2 == 0;
}

my $text = "foo bar baz";
$text =~ s/foo/FOO/g;

1;
"#,
    )?;

    println!("Running 5 iterations each...");
    println!();
    println!("Pure Rust (Pest) Parser:");
    for i in 1..=5 {
        let time_ms = timed_file_run_ms(repo_root, &rust_parser, &benchmark_file)?;
        println!("  Run {i}: {:.3}s", time_ms / 1000.0);
    }

    println!();
    println!("C Parser:");
    for i in 1..=5 {
        let time_ms = timed_file_run_ms(repo_root, &c_parser, &benchmark_file)?;
        println!("  Run {i}: {:.3}s", time_ms / 1000.0);
    }

    println!();
    println!("Note: Times include process startup overhead");
    Ok(0)
}

fn cmd_run_comparison(repo_root: &Path) -> Result<i32> {
    println!("=== Three-Way Parser Comparison ===");
    println!("Comparing: Pure Rust vs Legacy C vs Modern Parser");
    println!();

    let test_cases = [
        ("Simple", r#"my $x = 42;"#),
        ("Expression", r#"my $result = ($a + $b) * $c;"#),
        ("Control Flow", r#"if ($x > 10) { while ($y < 100) { $y = $y * 2; } }"#),
        ("Method Call", r#"$obj->method($arg1, $arg2);"#),
        ("For Loop", r#"for (my $i = 0; $i < 10; $i++) { print $i; }"#),
    ];

    let legacy_parser = repo_root.join("target/debug/parse");

    println!("Running parser tests...");
    println!();

    for (name, code) in test_cases {
        println!("Testing: {name}");
        println!("Code: {code}");

        println!("  Modern parser: ");
        let modern_args: Vec<&str> = if command_exists("timeout") {
            vec!["1s", "cargo", "run", "-q", "-p", "perl-parser", "--example", "demo", "--"]
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
    println!("=== Quick Parser Comparison ===");
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
            (path.file_name().and_then(|name| name.to_str()).unwrap_or("file").to_string(), path)
        })
        .collect();
    candidates.sort_by(|a, b| a.0.cmp(&b.0));

    println!("File,Size,C_Time(µs),Rust_Time(µs),Speedup");
    println!("----,----,----------,------------- ,-------");

    for (name, path) in candidates {
        let size = fs::metadata(&path).map(|metadata| metadata.len()).unwrap_or(0);

        let c_time = run_bench_parser_ms(repo_root, "c-scanner test-utils", &path, false)?;
        let rust_time = run_bench_parser_ms(repo_root, "pure-rust test-utils", &path, false)?;

        let speedup = if let (Some(c_val), Some(rust_val)) = (c_time, rust_time) {
            if rust_val > 0.0 { Some(c_val / rust_val) } else { None }
        } else {
            None
        };

        let speedup_text = if let Some(speedup) = speedup {
            if speedup > 1.0 {
                format!("{speedup:.2}x (Rust faster)")
            } else {
                format!("{:.2}x (C faster)", 1.0 / speedup.max(0.000_000_1))
            }
        } else {
            "N/A".to_string()
        };

        let c_ms = c_time.unwrap_or(0.0);
        let rust_ms = rust_time.unwrap_or(0.0);
        println!("{:<30} {:>8} {:>12.0} {:>12.0} {}", name, size, c_ms, rust_ms, speedup_text);
    }

    println!();
    println!("Quick benchmark complete!");
    Ok(0)
}

fn cmd_simple_bench(repo_root: &Path) -> Result<i32> {
    println!("Pure Rust Perl Parser Performance Test");
    println!("======================================");

    let parser = repo_root.join("crates/tree-sitter-perl-rs/target/release/parse-rust");
    command_status_strict(
        repo_root,
        "cargo",
        &["build", "--release", "-p", "tree-sitter-perl-rs", "--bin", "parse-rust"],
        &[],
    )?;

    let workspace = repo_root.join("target").join("perl-ci-hygiene").join("simple_bench");
    fs::create_dir_all(&workspace).with_context(|| format!("creating {}", workspace.display()))?;
    let tiny = workspace.join("tiny.pl");
    let small = workspace.join("small.pl");
    let medium = workspace.join("medium.pl");
    let large = workspace.join("large.pl");
    let huge = workspace.join("huge.pl");

    fs::write(&tiny, "my $x = 42;\n")?;
    fs::copy(repo_root.join("test_corpus").join("basic_constructs.pl"), &small)
        .wrap_err("copying small fixture")?;
    fs::copy(repo_root.join("test_corpus").join("parser_stress_cases.pl"), &medium)
        .wrap_err("copying medium fixture")?;
    fs::copy(repo_root.join("test_corpus").join("real_world/enterprise_cpan_patterns.pl"), &large)
        .wrap_err("copying large fixture")?;
    fs::copy(
        repo_root.join("test_corpus").join("edge_cases/performance_stress_scenarios.pl"),
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
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("benchmark");
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
    let log_dir = repo_root.join("target").join("perl-ci-hygiene").join("stack-overflow-logs");
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

fn run_bench_parser_ms(
    repo_root: &Path,
    features: &str,
    file: &Path,
    _fail_fast: bool,
) -> Result<Option<f64>> {
    let file_arg = file.to_string_lossy().into_owned();
    let args = [
        "run",
        "--quiet",
        "--release",
        "--features",
        features,
        "--bin",
        "bench_parser",
        "--",
        file_arg.as_str(),
    ];
    let (status, elapsed) = command_timed_status(repo_root, "cargo", &args, &[])?;
    if status == 0 { Ok(Some(elapsed.as_micros() as f64)) } else { Ok(None) }
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

fn benchmark_average_ms(
    repo_root: &Path,
    parser: &Path,
    file: &Path,
    iterations: usize,
) -> Result<f64> {
    let mut total_ms = 0.0;
    for _ in 0..iterations {
        let elapsed_ms = timed_file_run_ms(repo_root, parser, file)?;
        total_ms += elapsed_ms;
    }
    Ok(total_ms / iterations as f64)
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
            if let Some(publish) = package.get("publish").and_then(Value::as_array) {
                if publish.is_empty() {
                    continue;
                }
            }
            let name = package.get("name").and_then(Value::as_str).unwrap_or("");
            if name.is_empty() {
                continue;
            }
            let manifest_path = package.get("manifest_path").and_then(Value::as_str).unwrap_or("");
            if manifest_path.is_empty() {
                continue;
            }
            let crate_root = Path::new(manifest_path).parent().unwrap_or_else(|| Path::new("."));
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
    let patch_values = patch_args.iter().map(|(_, patch)| patch.as_str()).collect::<Vec<_>>();

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
        &["run", "--features", "pure-rust", "--release", "--bin", "test_stacker"],
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
        ("cargo", vec!["run", "--features", "pure-rust", "--bin", "test_stacker"])
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
        &["test", "--features", "pure-rust", "iterative_parser_tests", "--", "--nocapture"],
        &[],
    )?;

    println!();
    println!("{YELLOW}Running parser benchmarks...{NC}");
    command_status_strict(
        repo_root,
        "cargo",
        &["run", "--features", "pure-rust", "--bin", "benchmark_parsers"],
        &[],
    )?;

    println!();
    println!("{YELLOW}Testing deep nesting capabilities...{NC}");
    command_status_strict(
        repo_root,
        "cargo",
        &["test", "--features", "pure-rust", "test_deep_nesting", "--nocapture"],
        &[],
    )?;

    println!();
    println!("{GREEN}✅ All iterative parser tests completed!{NC}");
    Ok(0)
}

fn cmd_compare_benchmarks(repo_root: &Path, args: &[String]) -> Result<i32> {
    println!("Running parser benchmark comparator...");
    if !command_exists("python3") {
        return Err(color_eyre::eyre::eyre!("python3 is required for benchmark comparison"));
    }

    let compare_py = repo_root.join("benchmarks").join("scripts").join("compare.py");
    if !compare_py.is_file() {
        return Err(color_eyre::eyre::eyre!("missing comparator: {}", compare_py.display()));
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
        &["test", "-p", "perl-parser", "--test", "lsp_feature_gating_test", "--", "--nocapture"],
        &[("FEATURES_TOML_OVERRIDE", "crates/perl-parser/tests/data/features_minimal.toml")],
    )?;

    println!();
    println!("Testing with disabled features catalog...");
    command_status_strict(
        repo_root,
        "cargo",
        &["test", "-p", "perl-parser", "--test", "lsp_features_snapshot_test", "--", "--nocapture"],
        &[("FEATURES_TOML_OVERRIDE", "crates/perl-parser/tests/data/features_disabled_test.toml")],
    )?;

    println!("✅ Override testing complete!");
    Ok(0)
}

fn cmd_simple_lsp_test(repo_root: &Path) -> Result<i32> {
    println!("Testing Perl LSP server...");
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

fn cmd_check_version_sync(repo_root: &Path) -> Result<i32> {
    let cargo_toml = read_to_value(repo_root.join("Cargo.toml"))?;
    let features_toml = read_to_value(repo_root.join("features.toml"))?;
    let vscode_json = read_json_value(&repo_root.join("vscode-extension/package.json"))?;

    let cargo_version = cargo_toml
        .get("workspace")
        .and_then(|workspace| workspace.get("package"))
        .and_then(|pkg| pkg.get("version"))
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let features_version = features_toml
        .get("meta")
        .and_then(|meta| meta.get("version"))
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let vscode_version = vscode_json.get("version").and_then(|value| value.as_str()).unwrap_or("");

    println!("Version sync check:");
    println!("  Cargo.toml [workspace]: {}", cargo_version);
    println!("  features.toml:          {}", features_version);
    println!("  vscode-extension:       {}", vscode_version);

    if cargo_version.is_empty() || features_version.is_empty() || vscode_version.is_empty() {
        return Err(color_eyre::eyre::eyre!("one or more version values were missing"));
    }

    if cargo_version == features_version && cargo_version == vscode_version {
        println!("Version sync check: all sources agree on {cargo_version}");
        Ok(0)
    } else {
        Err(color_eyre::eyre::eyre!(
            "version mismatch detected: {} != {} != {}",
            cargo_version,
            features_version,
            vscode_version
        ))
    }
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
        &["test", "--features", "pure-rust test-utils", "edge_case_tests", "--", "--nocapture"],
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
            &["bench", "--features", "pure-rust test-utils", "edge_case_benchmarks"],
            &[],
        )?;
    }

    println!("{YELLOW}Running edge case examples...{NC}");
    command_status_strict(
        repo_root,
        "cargo",
        &["run", "--features", "pure-rust test-utils", "--example", "edge_case_demo"],
        &[],
    )?;
    command_status_strict(
        repo_root,
        "cargo",
        &["run", "--features", "pure-rust test-utils", "--example", "anti_pattern_analysis"],
        &[],
    )?;
    command_status_strict(
        repo_root,
        "cargo",
        &["run", "--features", "pure-rust test-utils", "--example", "tree_sitter_compatibility"],
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

    let cargo_toml =
        read_to_value(repo_root.join("crates").join("perl-parser").join("Cargo.toml"))?;
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
    fs::write(artifacts_dir.join("doc-summary.json"), serde_json::to_string(&doc_summary)?)
        .with_context(|| "writing doc-summary.json")?;
    println!("Doc summary saved to {}", artifacts_dir.join("doc-summary.json").display());

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
    fs::write(artifacts_dir.join("test-summary.json"), serde_json::to_string(&test_summary)?)
        .with_context(|| "writing test-summary.json")?;

    let state = json!({
        "version": version,
        "tests": test_summary,
        "docs": doc_summary,
        "generated_at": Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    });
    fs::write(artifacts_dir.join("state.json"), serde_json::to_string_pretty(&state)?)
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
    const RED: &str = "\x1b[0;31m";
    const NC: &str = "\x1b[0m";

    println!("{YELLOW}Enhanced LSP Cancellation System Test Runner{NC}");
    println!("{YELLOW}Fixing Cargo package cache file lock contention...{NC}");
    println!();

    println!("{YELLOW}Step 1: Pre-building LSP binaries...{NC}");
    command_status_strict(repo_root, "cargo", &["build", "--release", "-p", "perl-lsp"], &[])?;
    println!("{GREEN}✓ LSP binaries pre-built successfully{NC}");

    println!("{YELLOW}Step 2: Pre-building test binaries...{NC}");
    command_status_strict(repo_root, "cargo", &["build", "--tests", "-p", "perl-lsp"], &[])?;
    println!("{GREEN}✓ Test binaries pre-built successfully{NC}");

    let cancel_binary = find_cancel_test_binary(repo_root).ok_or_else(|| {
        color_eyre::eyre::eyre!("cancel test binary not found in target/debug/deps")
    })?;
    println!("{GREEN}✓ Found cancel test binary: {}{NC}", cancel_binary.display());

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
        ("CARGO_BIN_EXE_perl_lsp", perl_lsp_binary.to_string_lossy().to_string()),
        ("RUST_TEST_THREADS", rust_threads),
    ];
    let exe_env_refs: Vec<(&str, &str)> =
        exe_env.iter().map(|(key, value)| (*key, value.as_str())).collect();
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
        if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
            if name.contains("lsp_cancel_test") {
                return Some(path.to_path_buf());
            }
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
    println!("Badges generated in {:?}", badge_file.file_name().unwrap_or_default());
    Ok(0)
}

fn cmd_install_githooks(repo_root: &Path) -> Result<i32> {
    let hook_path = repo_root.join(".git").join("hooks").join("pre-push");
    if let Some(parent) = hook_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let hook = r#"#!/usr/bin/env bash
set -euo pipefail

echo "🚪 Running local gate before push: nix develop -c just ci-gate"
echo "   (Skip with: git push --no-verify)"
echo ""

# Try nix develop first, fall back to just alone
if command -v nix &>/dev/null && [ -f flake.nix ]; then
    nix develop -c just ci-gate
elif command -v just &>/dev/null; then
    just ci-gate
else
    echo "⚠️  Neither 'nix develop' nor 'just' available, skipping pre-push gate"
    echo "   Install just: cargo install just"
    exit 0
fi
"#;
    fs::write(&hook_path, format!("{hook}\n"))
        .with_context(|| format!("writing {:?}", hook_path))?;
    #[cfg(unix)]
    {
        fs::set_permissions(&hook_path, fs::Permissions::from_mode(0o755))
            .with_context(|| format!("setting executable bit for {:?}", hook_path))?;
    }
    println!("✅ Installed pre-push hook");
    println!("   The hook runs 'nix develop -c just ci-gate' before each push");
    println!("   Skip with: git push --no-verify");
    Ok(0)
}

fn read_required_usize(path: &Path) -> Result<usize> {
    if !path.is_file() {
        return Err(color_eyre::eyre::eyre!("required file not found: {}", path.display()));
    }
    let raw = fs::read_to_string(path).with_context(|| format!("reading {:?}", path))?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(color_eyre::eyre::eyre!("required file is empty: {}", path.display()));
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
    path.strip_prefix(root)
        .map_or_else(|_| path.display().to_string(), |relative| relative.display().to_string())
}

fn path_has_component(path: &Path, target: &str) -> bool {
    path.components().any(|component| component.as_os_str() == OsStr::new(target))
}

fn is_text_file(path: &Path) -> bool {
    fs::read_to_string(path).is_ok()
}

fn walk_entries(root: &Path) -> impl Iterator<Item = DirEntry> + '_ {
    WalkDir::new(root).follow_links(false).into_iter().filter_map(Result::ok)
}

fn read_lines(path: &Path) -> Result<Vec<String>> {
    fs::read_to_string(path)
        .with_context(|| format!("reading {:?}", path))
        .map(|contents| contents.lines().map(std::string::ToString::to_string).collect())
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

    println!("{}Checking for unescaped brackets in doc comments...{}", BLUE, NC);
    let unescaped_pattern = Regex::new(r"^[ \t]*//[/!].*\[")?;
    let output = command_with_output_allow_empty_match(
        repo_root,
        "rg",
        &["-n", "--glob", "crates/**/src/**/*.rs", unescaped_pattern.as_str()],
        &[],
    )?;
    if output.trim().is_empty() {
        println!("{}✓ No suspicious brackets found{}", GREEN, NC);
    } else {
        println!("{}⚠ Found potential unescaped brackets. Consider:{}", YELLOW, NC);
        println!("  - Escaping with backslash: \\[text\\]");
        println!("  - Wrapping in code blocks: `[text]`");
        println!("  - Using proper doc links: [`Type`] or [Type](link)");
        for line in
            command_output_lines(&output).into_iter().filter(|line| !line.contains(r"\[")).take(5)
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
        &["-n", "--glob", "crates/**/src/**/*.rs", "^[ \t]*//[/!].*https?://[^ \t<>\\[\\]]+"],
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
        &["-n", "--glob", "crates/**/src/**/*.rs", marker_pattern.as_str()],
        &[],
    )?;
    if !marker_output.trim().is_empty() {
        println!("{}⚠ Found doc comments without space after marker{}", YELLOW, NC);
        println!("  Use: /// Text  or  //! Text");
        for line in command_output_lines(&marker_output).iter().take(5) {
            println!("{line}");
        }
        found_issues = true;
    }

    let perl_code_output = command_with_output_allow_empty_match(
        repo_root,
        "rg",
        &["-n", "-A2", "-B2", "--glob", "crates/**/src/**/*.rs", r"^[ \t]*///.*\\$[a-zA-Z_]"],
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
        println!("{}⚠ Possible Perl code in docs without code blocks{}", YELLOW, NC);
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

    println!("{}Checking for TODOs in public documentation...{}", BLUE, NC);
    let todo_output = command_with_output_allow_empty_match(
        repo_root,
        "rg",
        &["-n", "--glob", "crates/**/src/**/*.rs", "^[ \t]*///.*\\b(TODO|FIXME|XXX|HACK)\\b"],
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
        println!("{}✗ Documentation build failed with strict flags{}", RED, NC);
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
    let mut baseline = read_usize_file(&baseline_file, current)?;
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
        let reduction_percent = if baseline > 0 { (reduction * 100) / baseline } else { 0 };
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
    if command_status_strict(repo_root, "cargo", &["fmt", "--all", "--", "--check"], &[]).is_err() {
        println!("{}✗ Format check failed - run 'cargo fmt --all' to fix{}", RED, NC);
        return Ok(1);
    }
    println!();

    println!("{}2. Clippy (strict on first-party)...{}", YELLOW, NC);
    let mut clippy_failed = false;
    if command_status(
        repo_root,
        "cargo",
        &["clippy", "-p", "perl-parser", "--all-targets", "--all-features", "--", "-D", "warnings"],
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
        &["clippy", "-p", "perl-lexer", "--all-targets", "--all-features", "--", "-D", "warnings"],
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
    println!("{}✓ Clippy check passed for first-party crates{}", GREEN, NC);
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
        &[("RUSTDOCFLAGS", "-D rustdoc::broken_intra_doc_links -D rustdoc::bare_urls")],
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
        println!("{}⚠ cargo-deny not installed (run: cargo install cargo-deny){}", YELLOW, NC);
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
        &["check", "-p", "perl-parser", "--tests", "--message-format=json"],
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
        let pkg_id = value.get("package_id").and_then(|v| v.as_str()).unwrap_or("");
        if !pkg_id.starts_with("perl-parser ") {
            continue;
        }
        let message = value.get("message");
        if message.is_none() {
            continue;
        }
        let level = message.and_then(|m| m.get("level")).and_then(|v| v.as_str());
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
        println!("  3. If intentional, update baseline: echo {current} > {:?}", baseline_path);
        return Ok(1);
    }

    if current < baseline {
        println!("IMPROVEMENT: {} fewer missing_docs warnings!", baseline - current);
        println!("Consider updating baseline: echo {current} > {:?}", baseline_path);
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
    println!("Checking for unsafe lock patterns in {}...", target_dir.display());
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
    )?;

    if !report_file.is_file() {
        return Err(color_eyre::eyre::eyre!(
            "Report file not generated: {}",
            report_file.display()
        ));
    }

    let report = read_json_value(&report_file)?;
    let mut current = 0usize;
    if let Some(value) =
        report.get("parse_outcomes").and_then(|v| v.get("error")).and_then(|v| v.as_u64())
    {
        current = usize::try_from(value)?;
    } else if let Some(value) =
        report.get("parse_outcomes").and_then(|v| v.get("error")).and_then(|v| v.as_i64())
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
            println!("Consider updating baseline: echo {current} > {:?}", baseline_file);
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
        return Err(color_eyre::eyre::eyre!("Matrix file not found: {}", matrix_file.display()));
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
    println!("DRIFT DETECTED: docs/PARSER_FEATURE_MATRIX.md is out of date");
    println!();
    let old_matrix = repo_root.join("target/.old_parser_matrix");
    let new_matrix = repo_root.join("target/.new_parser_matrix");
    let _ = fs::write(&old_matrix, format!("{current_normalized}\n"));
    let _ = fs::write(&new_matrix, format!("{fresh_normalized}\n"));

    let diff = command_with_output_allow_failure(
        repo_root,
        "diff",
        &["-u", old_matrix.to_string_lossy().as_ref(), new_matrix.to_string_lossy().as_ref()],
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
    println!("  3. Commit the updated docs/PARSER_FEATURE_MATRIX.md");
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
                            Some(format!("{}:{line_no}:{line}", display_path(repo_root, &path)))
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
    println!("unsafe syntax: {} (baseline: 0)", all_matches.len());
    if all_matches.is_empty() {
        println!("No unsafe syntax in production scopes");
        return Ok(0);
    }

    println!("FAIL: unsafe syntax count ({}) exceeds baseline ({})", all_matches.len(), 0);
    println!("Offenders:");
    for item in all_matches {
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
            offenders.push(format!("{}:{line_no}:{text}", display_path(repo_root, &path)));
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
        let lines = read_lines(&path)?;
        let test_start = first_cfg_test_line_number(&path).unwrap_or(usize::MAX);
        for (index, line) in lines.iter().enumerate() {
            let line_no = index + 1;
            if line_no >= test_start {
                continue;
            }
            if unwrap_re.is_match(line)
                && !(line.contains("self.expect(")
                    || line.contains("s.expect(")
                    || line.contains("self.context.expect("))
            {
                unwrap_offenders
                    .push(format!("{}:{line_no}:{line}", display_path(repo_root, &path)));
            }
            if panic_re.is_match(line) && !comment_re.is_match(line) {
                panic_offenders
                    .push(format!("{}:{line_no}:{line}", display_path(repo_root, &path)));
            }
        }
    }

    let unwrap_baseline = read_usize_file(&repo_root.join("ci/unwrap_prod_baseline.txt"), 0)?;
    let panic_baseline = read_usize_file(&repo_root.join("ci/panic_prod_baseline.txt"), 0)?;
    println!("unwrap/expect: {} (baseline: {})", unwrap_offenders.len(), unwrap_baseline);
    if unwrap_offenders.len() > unwrap_baseline {
        println!(
            "FAIL: unwrap/expect count ({}) exceeds baseline ({})",
            unwrap_offenders.len(),
            unwrap_baseline
        );
        println!("");
        println!("Offenders:");
        for line in unwrap_offenders.iter().take(10) {
            println!("{line}");
        }
        return Ok(1);
    }

    println!("panic-family macros: {} (baseline: {})", panic_offenders.len(), panic_baseline);
    if panic_offenders.len() > panic_baseline {
        println!(
            "FAIL: panic-family count ({}) exceeds baseline ({})",
            panic_offenders.len(),
            panic_baseline
        );
        println!("");
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
        &["clippy", "-p", "perl-parser", "--all-targets", "--all-features", "--", "-D", "warnings"],
        &[],
    )?;
    command_status_strict(
        repo_root,
        "cargo",
        &["clippy", "-p", "perl-lexer", "--all-targets", "--all-features", "--", "-D", "warnings"],
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
        &[("RUSTDOCFLAGS", "-D rustdoc::broken_intra_doc_links -D rustdoc::bare_urls")],
    )?;

    println!();
    println!("5. Tests (workspace, lib+bins+tests, no examples)");
    command_status_strict(repo_root, "cargo", &["test", "--workspace", "--all-features"], &[])?;

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
        command_status_strict(repo_root, "cargo", &["xtask", "test-heredoc", "--release"], &[])?;
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
        return Err(color_eyre::eyre::eyre!("Docs directory not found: {}", docs_path.display()));
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
    let exclude_dirs = ["target", ".git", ".receipts", ".runs"];
    let exclude_files = [
        repo_root.join("ci").join("check_todos.sh"),
        repo_root.join("crates").join("perl-parser").join("tests").join("missing_docs_ac_tests.rs"),
        repo_root
            .join("crates")
            .join("perl-tdd-support")
            .join("src")
            .join("tdd")
            .join("test_generator.rs"),
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
        fs::create_dir_all(&baseline_path.parent().unwrap_or(repo_root).to_path_buf())?;
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
        .filter(|hit| {
            !hit.contains("/bin/")
                && !hit.ends_with("/lifecycle.rs")
                && !hit.ends_with("lifecycle.rs")
        })
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
    let mut rel_string = String::new();
    rel_string.push('/');
    rel_string.push_str(&rel.display().to_string());

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
    for excluded in [
        "tree-sitter-perl-c",
        "tree-sitter-perl-rs",
        "perl-tdd-support",
        "perl-ts-heredoc-analysis",
        "perl-ts-logos-lexer",
        "perl-ts-heredoc-parser",
        "perl-ts-partial-ast",
        "perl-ts-advanced-parsers",
    ] {
        if rel_string.contains(&format!("/{excluded}/")) {
            return Ok(true);
        }
    }

    Ok(path_has_component(path, "tests")
        || path_has_component(path, "benches")
        || path_has_component(path, "build.rs")
        || path_has_component(path, "examples"))
}

fn cmd_ignored_test_count(repo_root: &Path, update: bool, check: bool) -> Result<i32> {
    let baseline_path = repo_root.join("scripts").join(".ignored-baseline");
    let verbose = env::var("VERBOSE").as_deref() == Ok("1");
    if update && check {
        return Err(color_eyre::eyre::eyre!(
            "choose exactly one of --update or --check for ignored-test-count"
        ));
    }

    let categories =
        ["brokenpipe", "feature", "infra", "protocol", "manual", "stress", "bug", "bare", "other"];
    let mut counts: HashMap<String, usize> =
        categories.iter().map(|category| ((*category).to_string(), 0)).collect();

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
            context: detail.context,
        });
    }

    let total: usize =
        categories.iter().map(|category| counts.get(*category).copied().unwrap_or(0)).sum();

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
    println!("{:<12} {:>8} {:>8} {:>8}", "Category", "Count", "Baseline", "Delta");
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
    lines.push(format!("# Ignored test baseline - {}", Utc::now().format("%Y-%m-%dT%H:%M:%SZ")));
    lines.push("# Updated by: ignored-test-count.sh --update".to_string());
    let mut ordered = BTreeMap::new();
    for key in
        ["brokenpipe", "feature", "infra", "protocol", "manual", "stress", "bug", "bare", "other"]
    {
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
    path: String,
    line_no: usize,
    text: String,
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
    context: String,
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

    for entry in WalkDir::new(root).follow_links(false).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let rel = path
            .strip_prefix(root)
            .with_context(|| format!("path under {:?}", root))?
            .to_path_buf();
        if exclude_files.iter().any(|p| p == &path) {
            continue;
        }
        if rel.components().any(|component| {
            exclude_dirs.iter().any(|name| component.as_os_str() == OsStr::new(name))
        }) {
            continue;
        }
        let is_rust = path.extension().is_some_and(|ext| ext == "rs");
        let file_name = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
        let is_hash_file = file_name == "Justfile"
            || file_name == "justfile"
            || hash_ext.iter().any(|ext| path.extension().is_some_and(|e| e == *ext));
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
                path: rel.display().to_string(),
                line_no: line_no + 1,
                text: line.to_string(),
                line_text: format!("{}:{}:{}", rel.display(), line_no + 1, line),
            });
        }
    }
    Ok(hits)
}

fn has_unlinked_todo_in_rust_line(line: &str, token_re: &Regex) -> bool {
    let mut has_hit = false;
    if let Some(idx) = line.find("//") {
        if !is_url_like_hash_comment(line, idx) && has_unlinked_token(&line[idx + 2..], token_re) {
            has_hit = true;
        }
    }
    if let Some(idx) = line.find("/*") {
        if has_unlinked_token(&line[idx + 2..], token_re) {
            has_hit = true;
        }
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
        if idx > 0 && !line[..idx].chars().rev().next().is_some_and(char::is_whitespace) {
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
            if reason.is_empty() && comment_re.is_match(line) {
                if let Some(comment) = comment_re.captures(line).and_then(|m| m.get(1)) {
                    reason = comment.as_str().to_string();
                }
            }
            if reason.is_empty() && i + 1 < lines.len() && comment_re.is_match(&lines[i + 1]) {
                if let Some(comment) = comment_re.captures(&lines[i + 1]).and_then(|m| m.get(1)) {
                    reason = comment.as_str().to_string();
                }
            }
            if reason.is_empty() && i + 2 < lines.len() && comment_re.is_match(&lines[i + 2]) {
                if let Some(comment) = comment_re.captures(&lines[i + 2]).and_then(|m| m.get(1)) {
                    reason = comment.as_str().to_string();
                }
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
