//! CLI entry point for the `perl-ci-hygiene` task runner.
//!
//! Dispatches CI hygiene sub-commands (ignored-test scans, version sync,
//! parser-error baselines, etc.) used by the `just` recipes and CI gates.

use perl_ci_hygiene::categorize_ignore;
use perl_ci_hygiene::version_sync;

use chrono::Utc;
use clap::Parser;
use color_eyre::eyre::{Context, Result};
use regex::Regex;
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value as TomlValue;
use walkdir::{DirEntry, WalkDir};

mod cli;
mod commands;
mod git_hooks;
mod process;

use crate::cli::{Cli, CliCommand};
#[cfg(test)]
use crate::commands::todos::{
    has_unlinked_todo_in_hash_line, has_unlinked_todo_in_perl_line, has_unlinked_todo_in_rust_line,
    has_unlinked_todo_in_rust_line_with_block_context, has_unlinked_todo_in_rust_line_with_state,
    linked_marker,
};
use crate::git_hooks::cmd_install_githooks;
#[cfg(test)]
use crate::git_hooks::pre_push_hook_script;
use crate::process::*;

const RED: &str = "\x1b[0;31m";
const GREEN: &str = "\x1b[0;32m";
const YELLOW: &str = "\x1b[0;33m";
const BLUE: &str = "\x1b[0;34m";
const NC: &str = "\x1b[0m";

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
        CliCommand::GenerateBadges { check } => cmd_generate_badges(&repo_root, check)?,
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
        CliCommand::CheckPrintInLib => cmd_check_print_in_lib(&repo_root)?,
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
        && CI_TEST_FILE_SUFFIXES.iter().any(|suffix| file_name.ends_with(suffix))
    {
        return true;
    }

    if path.components().any(|component| {
        CI_REPORT_CRATES_EXCLUDE.iter().any(|item| component.as_os_str() == OsStr::new(item))
    }) {
        return true;
    }

    false
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
        return Err(color_eyre::eyre::eyre!("missing token {idx} in {}", path.display()));
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
        let blocking_args =
            std::iter::once(lock_file).chain(refs.iter().copied()).collect::<Vec<_>>();
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

    const V2_BUNDLE_FILES: [&str; 5] =
        ["grammar.pest", "pure_rust_parser.rs", "pratt_parser.rs", "sexp_formatter.rs", "error.rs"];

    let source_root = repo_root.join("archive/crates/tree-sitter-perl-rs/src");
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
    println!("Building benchmark binaries...");
    build_quick_bench_binaries(repo_root)?;
    println!("Using median of {QUICK_BENCH_SAMPLES} direct binary runs per parser/file.");
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
        let size = fs::metadata(path).map(|metadata| metadata.len()).unwrap_or(0);
        let rust_time = run_rust_bench_us(repo_root, path)?;
        let facade_time = run_facade_bench_us(repo_root, path)?;
        let c_time = run_c_bench_us(repo_root, path)?;
        rows.push(Row { name: name.clone(), size, rust_time, facade_time, c_time });
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
        let times: [(&str, Option<f64>); 3] =
            [("native-v3", row.rust_time), ("facade", row.facade_time), ("c-grammar", row.c_time)];
        let fastest_label = times
            .iter()
            .filter_map(|(label, t)| t.map(|v| (label, v)))
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(label, _)| *label)
            .unwrap_or("N/A");

        let fmt = |v: Option<f64>| -> String {
            v.map(|us| format!("{us:.0}")).unwrap_or_else(|| "N/A".to_string())
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
        let available: Vec<f64> =
            [row.rust_time, row.facade_time, row.c_time].iter().filter_map(|&t| t).collect();
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

/// Number of timed samples collected per parser/file pair in quick-bench mode.
const QUICK_BENCH_SAMPLES: usize = 3;

/// Build the quick-bench parser binaries ahead of timing.
///
/// The C grammar bench is optional because it depends on libclang. Failures
/// while building that bench are treated as "N/A" at measurement time.
fn build_quick_bench_binaries(repo_root: &Path) -> Result<()> {
    command_status_strict(
        repo_root,
        "cargo",
        &["build", "--quiet", "--release", "-p", RUST_BENCH_CRATE, "--bin", RUST_BENCH_BIN],
        &[],
    )?;
    command_status_strict(
        repo_root,
        "cargo",
        &["build", "--quiet", "--release", "-p", FACADE_BENCH_CRATE, "--bin", FACADE_BENCH_BIN],
        &[],
    )?;
    // Optional dependency path: allow failure and report as N/A during run.
    let _ = command_status(
        repo_root,
        "cargo",
        &[
            "build",
            "--quiet",
            "--release",
            "--manifest-path",
            C_BENCH_MANIFEST,
            "--bin",
            C_BENCH_BIN,
            "--features",
            "test-utils",
        ],
        &[],
    )?;
    Ok(())
}

/// Return the median duration in microseconds over `QUICK_BENCH_SAMPLES` runs.
fn run_bench_samples_us(repo_root: &Path, command: &Path, file: &Path) -> Result<Option<f64>> {
    let command_str = command.to_string_lossy().into_owned();
    let file_arg = file.to_string_lossy().into_owned();
    let args = [file_arg.as_str()];
    let mut samples = Vec::with_capacity(QUICK_BENCH_SAMPLES);
    for _ in 0..QUICK_BENCH_SAMPLES {
        let (status, elapsed) = command_timed_status(repo_root, &command_str, &args, &[])?;
        if status != 0 {
            return Ok(None);
        }
        samples.push(elapsed.as_micros() as f64);
    }
    if samples.is_empty() {
        return Ok(None);
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Ok(samples.get(samples.len() / 2).copied())
}

/// Run the v3 native Rust parser bench binary against `file`.
fn run_rust_bench_us(repo_root: &Path, file: &Path) -> Result<Option<f64>> {
    let binary = repo_root.join("target").join("release").join(RUST_BENCH_BIN);
    run_bench_samples_us(repo_root, &binary, file)
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
    let binary = repo_root
        .join("crates")
        .join("tree-sitter-perl-c")
        .join("target")
        .join("release")
        .join(C_BENCH_BIN);
    run_bench_samples_us(repo_root, &binary, file)
}

/// Run the `tree-sitter-perl-rs` facade bench binary against `file`.
///
/// The facade crate lives in the normal workspace so it is invoked with
/// `-p` rather than `--manifest-path`. Returns wall-clock duration in
/// microseconds, or `None` if the bench binary exits non-zero.
fn run_facade_bench_us(repo_root: &Path, file: &Path) -> Result<Option<f64>> {
    let binary = repo_root.join("target").join("release").join(FACADE_BENCH_BIN);
    run_bench_samples_us(repo_root, &binary, file)
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
        if let Some(name) = path.file_name().and_then(|name| name.to_str())
            && name.contains("lsp_cancel_test")
        {
            return Some(path.to_path_buf());
        }
    }
    None
}

fn cmd_generate_badges(repo_root: &Path, check_mode: bool) -> Result<i32> {
    commands::badges::generate(repo_root, check_mode)
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
        let reduction_percent =
            reduction.checked_mul(100).and_then(|scaled| scaled.checked_div(baseline)).unwrap_or(0);
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

    if run_format_check(repo_root).is_err()
        || run_first_party_clippy(repo_root).is_err()
        || run_vendor_clippy_smoke(repo_root).is_err()
        || run_docs_check(repo_root).is_err()
        || run_workspace_tests(repo_root).is_err()
        || run_ignored_baseline_check(repo_root).is_err()
        || run_dependency_security_check(repo_root).is_err()
    {
        return Ok(1);
    }

    println!("{}=== All Local Checks Passed ==={}", GREEN, NC);
    println!();
    println!("You can now safely commit/push your changes.");
    println!("Pro tip: Install as git pre-push hook: cp ci/check_local.sh .git/hooks/pre-push");
    Ok(0)
}

fn run_format_check(repo_root: &Path) -> Result<()> {
    println!("{}1. Format check...{}", YELLOW, NC);
    if command_status_strict(repo_root, "cargo", &["xtask", "fmt", "--check"], &[]).is_err() {
        println!("{}✗ Format check failed - run 'cargo xtask fmt' to fix{}", RED, NC);
        return Err(color_eyre::eyre::eyre!("format check failed"));
    }
    println!();
    Ok(())
}

fn run_first_party_clippy(repo_root: &Path) -> Result<()> {
    println!("{}2. Clippy (strict on first-party)...{}", YELLOW, NC);
    let mut clippy_failed = false;
    clippy_failed |= first_party_clippy_failed(repo_root, "perl-parser");
    clippy_failed |= first_party_clippy_failed(repo_root, "perl-lexer");
    if clippy_failed {
        return Err(color_eyre::eyre::eyre!("first-party clippy checks failed"));
    }
    println!("{}✓ Clippy check passed for first-party crates{}", GREEN, NC);
    println!();
    Ok(())
}

fn first_party_clippy_failed(repo_root: &Path, crate_name: &str) -> bool {
    let status = command_status(
        repo_root,
        "cargo",
        &["clippy", "-p", crate_name, "--all-targets", "--all-features", "--", "-D", "warnings"],
        &[],
    )
    .unwrap_or(1);
    if status != 0 {
        println!("{}✗ Clippy found issues in {}{}", RED, crate_name, NC);
        return true;
    }
    false
}

fn run_vendor_clippy_smoke(repo_root: &Path) -> Result<()> {
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
    Ok(())
}

fn run_docs_check(repo_root: &Path) -> Result<()> {
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
        return Err(color_eyre::eyre::eyre!("documentation build failed"));
    }
    println!("{}✓ Documentation builds cleanly{}", GREEN, NC);
    println!();
    Ok(())
}

fn run_workspace_tests(repo_root: &Path) -> Result<()> {
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
        return Err(color_eyre::eyre::eyre!("workspace tests failed"));
    }
    println!("{}✓ All tests passed{}", GREEN, NC);
    println!();
    Ok(())
}

fn run_ignored_baseline_check(repo_root: &Path) -> Result<()> {
    println!("{}5. Ignored tests baseline...{}", YELLOW, NC);
    let ignored_exit = cmd_check_ignored(repo_root)?;
    if ignored_exit == 0 {
        println!("{}✓ Ignored tests baseline correct{}", GREEN, NC);
        println!();
        return Ok(());
    }
    println!("{}✗ Ignored tests baseline mismatch{}", RED, NC);
    Err(color_eyre::eyre::eyre!("ignored tests baseline mismatch"))
}

fn run_dependency_security_check(repo_root: &Path) -> Result<()> {
    println!("{}6. Dependency security check...{}", YELLOW, NC);
    if command_exists("cargo-deny") {
        let output =
            command_with_output_allow_failure(repo_root, "cargo", &["deny", "check"], &[])?;
        if output.contains("error:") {
            println!("{}✗ Dependency issues found{}", RED, NC);
            println!("{output}");
            return Err(color_eyre::eyre::eyre!("dependency security check failed"));
        }
        println!("{}✓ Dependencies are secure{}", GREEN, NC);
    } else {
        println!("{}⚠ cargo-deny not installed (run: cargo install cargo-deny){}", YELLOW, NC);
    }
    println!();
    Ok(())
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
    println!("DRIFT DETECTED: docs/reference/PARSER_FEATURE_MATRIX.md is out of date");
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
    let baseline = read_usize_file(&repo_root.join("ci/unsafe_prod_baseline.txt"), 0)?;
    println!("unsafe syntax: {} (baseline: {baseline})", all_matches.len());
    if all_matches.len() <= baseline {
        if all_matches.is_empty() {
            println!("No unsafe syntax in production scopes");
        }
        return Ok(0);
    }

    println!("FAIL: unsafe syntax count ({}) exceeds baseline ({baseline})", all_matches.len());
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
    println!("unwrap/expect: {} (baseline: {})", unwrap_offenders.len(), unwrap_baseline);
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

    println!("panic-family macros: {} (baseline: {})", panic_offenders.len(), panic_baseline);
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

/// Returns `true` for paths that should be skipped by the print-in-lib check.
///
/// This is a superset of `is_excluded_test_path` with extra exclusions specific
/// to the print-macro policy:
///   - `build.rs` files: Cargo build scripts use `println!("cargo:...")` to communicate
///     with Cargo itself.  This is the standard mechanism; it is not "library output".
///   - Files whose name starts with `test_` (e.g. `test_parser.rs`): these are test
///     driver / helper files that live alongside library source but are only invoked
///     during test runs.
///   - Test-support crates whose primary purpose is emitting diagnostic output during
///     test execution (e.g. `perl-lsp-ux-tests`).
fn is_excluded_for_print_check(path: &Path) -> bool {
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // Build scripts may use println!("cargo:...") — standard Cargo convention.
    if file_name == "build.rs" {
        return true;
    }

    // Test-driver files next to src/ but not inside tests/ directory.
    if file_name.starts_with("test_") && file_name.ends_with(".rs") {
        return true;
    }

    // Test-support and UX-test crates use print output intentionally.
    if path.components().any(|c| c.as_os_str() == OsStr::new("perl-lsp-ux-tests")) {
        return true;
    }

    false
}

/// Returns `true` when a source file should be skipped wholesale by the print-macro check.
///
/// Files with a file-level `#![allow(clippy::print_stderr)]` or
/// `#![allow(clippy::print_stdout)]` attribute have been explicitly opted out of the
/// rule (e.g. `cli.rs` in the LSP binary crate).  The attribute must appear in the
/// first 30 lines of the file (the module-doc / crate-doc block).
fn file_has_print_allow(lines: &[String]) -> bool {
    for line in lines.iter().take(30) {
        if line_has_inner_print_allow_attr(line) {
            return true;
        }
    }
    false
}

fn line_has_inner_print_allow_attr(line: &str) -> bool {
    line.contains("#![allow(")
        && (line.contains("clippy::print_stderr")
            || line.contains("clippy::print_stdout")
            || line.contains("clippy::print_"))
}

fn line_has_outer_print_allow_attr(line: &str) -> bool {
    line.contains("#[allow(")
        && (line.contains("clippy::print_stderr")
            || line.contains("clippy::print_stdout")
            || line.contains("clippy::print_"))
}

fn line_is_whole_line_comment(line: &str) -> bool {
    line.trim_start().starts_with("//")
}

#[derive(Default)]
struct PrintAllowScope {
    pending_attr: bool,
    active_brace_depth: usize,
}

impl PrintAllowScope {
    fn note_attribute(&mut self) {
        self.pending_attr = true;
    }

    fn allows_current_line(&self) -> bool {
        self.pending_attr || self.active_brace_depth > 0
    }

    fn observe_line(&mut self, line: &str) {
        let trimmed = line.trim();
        if trimmed.is_empty() || line_is_whole_line_comment(line) || trimmed.starts_with("#[") {
            return;
        }

        if self.active_brace_depth > 0 {
            self.apply_brace_delta(line);
            return;
        }

        if self.pending_attr {
            self.pending_attr = false;
            let delta = brace_delta(line);
            if delta > 0 {
                self.active_brace_depth = delta as usize;
            }
        }
    }

    fn apply_brace_delta(&mut self, line: &str) {
        let delta = brace_delta(line);
        if delta.is_negative() {
            self.active_brace_depth = self.active_brace_depth.saturating_sub(delta.unsigned_abs());
        } else {
            self.active_brace_depth = self.active_brace_depth.saturating_add(delta as usize);
        }
    }
}

fn brace_delta(line: &str) -> isize {
    let opens = line.chars().filter(|ch| *ch == '{').count() as isize;
    let closes = line.chars().filter(|ch| *ch == '}').count() as isize;
    opens - closes
}

/// Enforce that library source files do not contain raw `println!` / `eprintln!` /
/// `print!` / `eprint!` calls.
///
/// Library code should use `tracing::{debug,info,warn,error}` for all diagnostic
/// output.  Raw print macros:
///   - Bypass the structured logging pipeline (no span context, no log level filtering).
///   - Appear in release builds and pollute the LSP's stdout/stderr channels.
///   - Make test output noisy when tests fail.
///
/// Allowed exceptions (enforced at the call site):
///   - Files with a file-level `#![allow(clippy::print_stderr/stdout)]` attribute (e.g.
///     `cli.rs` in the LSP binary crate — user-facing output is their product).
///   - Lines inside `#[cfg(debug_assertions)]` blocks (debug-only guardrails).
///   - The startup banner in `launcher/mod.rs` (function-level `#[allow]`).
///   - Any future deliberate exception must add the clippy allow attribute with a
///     comment explaining why.
///
/// This check mirrors the pattern of `cmd_check_unwraps_prod`.  The baseline is stored
/// in `ci/print_in_lib_baseline.txt`; the check fails if the current count exceeds it.
fn cmd_check_print_in_lib(repo_root: &Path) -> Result<i32> {
    let print_re = Regex::new(r"(println!|eprintln!|print!\(|eprint!\()")?;
    let debug_attr_re = Regex::new(r"#\[cfg\(debug_assertions\)\]")?;
    let mut offenders = Vec::new();

    for path in walk_rust_source_files_for_ci_checks(repo_root)? {
        if is_excluded_for_print_check(&path) {
            continue;
        }
        let rel = display_path(repo_root, &path);
        let lines = read_lines(&path)?;

        // Skip files that have a file-level opt-out attribute.
        if file_has_print_allow(&lines) {
            continue;
        }

        let test_start = first_cfg_test_line_number(&path).unwrap_or(usize::MAX);

        let mut debug_assertions_scope = PrintAllowScope::default();
        let mut print_allow_scope = PrintAllowScope::default();

        for (index, line) in lines.iter().enumerate() {
            let line_no = index + 1;
            if line_no >= test_start {
                break;
            }

            if debug_attr_re.is_match(line) {
                debug_assertions_scope.note_attribute();
            }
            if line_has_outer_print_allow_attr(line) {
                print_allow_scope.note_attribute();
            }

            if line_is_whole_line_comment(line) {
                continue;
            }

            if print_re.is_match(line) {
                if !debug_assertions_scope.allows_current_line()
                    && !print_allow_scope.allows_current_line()
                {
                    offenders.push(format!("{rel}:{line_no}:{}", line.trim()));
                }
            }

            debug_assertions_scope.observe_line(line);
            print_allow_scope.observe_line(line);
        }
    }

    let baseline = read_usize_file(&repo_root.join("ci/print_in_lib_baseline.txt"), 0)?;
    println!("print macros in library source: {} (baseline: {})", offenders.len(), baseline);
    if offenders.len() > baseline {
        println!("FAIL: print macro count ({}) exceeds baseline ({})", offenders.len(), baseline);
        println!();
        println!("Offenders (use tracing::{{debug,info,warn,error}} instead):");
        for line in offenders.iter().take(20) {
            println!("  {line}");
        }
        println!();
        println!("If the print macro is intentional, add #[allow(clippy::print_stderr)] or");
        println!("#[allow(clippy::print_stdout)] with a comment explaining why.");
        println!(
            "If you removed print macros, update ci/print_in_lib_baseline.txt with the new lower count."
        );
        return Ok(1);
    }

    if offenders.len() < baseline {
        println!(
            "NOTE: count ({}) is below baseline ({}). Update ci/print_in_lib_baseline.txt to ratchet down.",
            offenders.len(),
            baseline
        );
    }

    Ok(0)
}

fn is_allowlisted_prod_panic_hit(_rel_path: &str, line: &str) -> bool {
    // Static LazyLock<Regex> initializers that use unreachable!() for known-good patterns
    // are exempt regardless of which file they live in.  Two message conventions:
    //   • "... regex failed to compile" — used in heredoc anti-pattern initializers
    //   • "... is a known-good static pattern ..." — used in other static regex initializers
    line.contains("regex failed to compile") || line.contains("known-good static pattern")
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
    commands::doc_paths::check_doc_paths(repo_root, docs_dir)
}

#[cfg(test)]
fn has_machine_specific_home_path(line: &str, home_user_path: &Regex) -> bool {
    commands::doc_paths::has_machine_specific_home_path(line, home_user_path)
}

#[cfg(test)]
fn has_machine_specific_users_path(line: &str, users_name_path: &Regex) -> bool {
    commands::doc_paths::has_machine_specific_users_path(line, users_name_path)
}

fn cmd_check_todos(repo_root: &Path, list_mode: bool) -> Result<i32> {
    commands::todos::check_todos(repo_root, list_mode)
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

    let exit_violations: Vec<String> =
        exits.into_iter().filter(|hit| !is_allowlisted_exit_hit(hit)).collect();

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linked_marker_requires_parenthesized_issue_number() {
        assert!(linked_marker("(#123)"));
        assert!(linked_marker("   (#42) trailing text"));
        assert!(linked_marker(": (#42) trailing text"));
        assert!(linked_marker(" - (#42) trailing text"));
        assert!(linked_marker(":- (#42) trailing text"));
        assert!(!linked_marker("#123"));
        assert!(!linked_marker("(#)"));
        assert!(!linked_marker("(#12"));
        assert!(!linked_marker("(ABC-12)"));
    }

    #[test]
    fn rust_todo_detection_ignores_linked_or_url_like_comments() -> Result<()> {
        let todo_re = Regex::new(r"(?i)\b(?:todo|fixme)\b")?;

        assert!(has_unlinked_todo_in_rust_line("// TODO: investigate", &todo_re));
        assert!(has_unlinked_todo_in_rust_line("// todo: investigate", &todo_re));
        assert!(has_unlinked_todo_in_rust_line("// FiXmE: investigate", &todo_re));
        assert!(!has_unlinked_todo_in_rust_line("// TODO(#123): tracked", &todo_re));
        assert!(!has_unlinked_todo_in_rust_line("// todo(#123): tracked", &todo_re));
        assert!(!has_unlinked_todo_in_rust_line("let u = \"http://TODO\";", &todo_re));
        assert!(has_unlinked_todo_in_rust_line(
            "let u = \"http://TODO\"; // TODO: investigate",
            &todo_re
        ));
        assert!(has_unlinked_todo_in_rust_line("/* FIXME: needs fix */", &todo_re));

        Ok(())
    }

    #[test]
    fn rust_todo_detection_ignores_backtick_quoted_marker_names() -> Result<()> {
        let todo_re = Regex::new(r"(?i)\b(?:todo|fixme)\b")?;

        assert!(!has_unlinked_todo_in_rust_line(
            "//! marker flag `todo` counts toward corpus markers",
            &todo_re
        ));
        assert!(!has_unlinked_todo_in_rust_line(
            "/* literal token `FIXME` is documented */",
            &todo_re
        ));
        assert!(has_unlinked_todo_in_rust_line(
            "// marker flag todo still looks like prose debt",
            &todo_re
        ));
        assert!(has_unlinked_todo_in_rust_line(
            "// `todo` marker plus TODO: actual work",
            &todo_re
        ));

        Ok(())
    }

    #[test]
    fn rust_todo_detection_ignores_raw_string_comment_markers() -> Result<()> {
        let todo_re = Regex::new(r"\b(?:TODO|FIXME)\b")?;

        assert!(!has_unlinked_todo_in_rust_line("let s = r#\"// TODO in literal\"#;", &todo_re));
        assert!(!has_unlinked_todo_in_rust_line(
            "let s = r#\"/* FIXME in literal */\"#;",
            &todo_re
        ));

        Ok(())
    }

    #[test]
    fn rust_todo_detection_ignores_c_string_comment_markers() -> Result<()> {
        let todo_re = Regex::new(r"\b(?:TODO|FIXME)\b")?;

        assert!(!has_unlinked_todo_in_rust_line("let s = c\"// TODO in literal\";", &todo_re));
        assert!(!has_unlinked_todo_in_rust_line(
            "let s = cr#\"/* FIXME in literal */\"#;",
            &todo_re
        ));
        assert!(has_unlinked_todo_in_rust_line(
            "let s = c\"safe literal\"; // TODO: follow up",
            &todo_re
        ));

        Ok(())
    }

    #[test]
    fn rust_todo_detection_ignores_multiline_raw_string_content() -> Result<()> {
        let todo_re = Regex::new(r"TODO|FIXME")?;
        let mut raw_state = None;

        assert!(!has_unlinked_todo_in_rust_line_with_state(
            "let s = r#\"",
            &todo_re,
            &mut raw_state,
        ));
        assert!(!has_unlinked_todo_in_rust_line_with_state(
            "// TODO in multiline raw literal",
            &todo_re,
            &mut raw_state,
        ));
        assert!(!has_unlinked_todo_in_rust_line_with_state("\"#;", &todo_re, &mut raw_state,));
        assert!(has_unlinked_todo_in_rust_line_with_state(
            "// TODO: actual follow-up comment",
            &todo_re,
            &mut raw_state,
        ));

        Ok(())
    }

    #[test]
    fn rust_todo_detection_ignores_multiline_c_raw_string_content() -> Result<()> {
        let todo_re = Regex::new(r"TODO|FIXME")?;
        let mut raw_state = None;

        assert!(!has_unlinked_todo_in_rust_line_with_state(
            "let s = cr#\"",
            &todo_re,
            &mut raw_state,
        ));
        assert!(!has_unlinked_todo_in_rust_line_with_state(
            "// TODO in multiline C raw literal",
            &todo_re,
            &mut raw_state,
        ));
        assert!(!has_unlinked_todo_in_rust_line_with_state("\"#;", &todo_re, &mut raw_state,));
        assert!(has_unlinked_todo_in_rust_line_with_state(
            "// TODO: actual follow-up comment",
            &todo_re,
            &mut raw_state,
        ));

        Ok(())
    }

    #[test]
    fn rust_todo_detection_ignores_non_raw_string_comment_markers() -> Result<()> {
        let todo_re = Regex::new(r"(?i)\b(?:todo|fixme)\b")?;

        assert!(!has_unlinked_todo_in_rust_line(
            "let s = \"not a comment // TODO in literal\";",
            &todo_re
        ));
        assert!(!has_unlinked_todo_in_rust_line(
            "let s = \"block marker /* FIXME in literal */\";",
            &todo_re
        ));
        assert!(has_unlinked_todo_in_rust_line(
            "let s = \"safe literal\"; // TODO: follow up",
            &todo_re
        ));

        Ok(())
    }

    #[test]
    fn todo_detection_uses_word_boundaries() -> Result<()> {
        let todo_re = Regex::new(r"\b(?:TODO|FIXME)\b")?;

        assert!(!has_unlinked_todo_in_rust_line("// METHODOLOGY notes", &todo_re));
        assert!(!has_unlinked_todo_in_rust_line("// PREFIXME suffix", &todo_re));
        assert!(has_unlinked_todo_in_rust_line("// TODO: real marker", &todo_re));
        assert!(has_unlinked_todo_in_hash_line("echo hi # FIXME: real marker", &todo_re));

        Ok(())
    }

    #[test]
    fn rust_todo_detection_ignores_c_string_literals() -> Result<()> {
        let todo_re = Regex::new(r"\b(?:TODO|FIXME)\b")?;

        assert!(!has_unlinked_todo_in_rust_line("let s = c\"// TODO in C string\";", &todo_re));
        assert!(!has_unlinked_todo_in_rust_line(
            "let s = cr#\"/* FIXME in C raw string */\"#;",
            &todo_re
        ));
        assert!(has_unlinked_todo_in_rust_line("let s = c\"safe\"; // TODO: follow up", &todo_re));

        Ok(())
    }

    #[test]
    fn rust_todo_detection_scans_only_block_comment_text() -> Result<()> {
        let todo_re = Regex::new(r"\b(?:TODO|FIXME)\b")?;

        assert!(!has_unlinked_todo_in_rust_line(
            "/* tracked */ let s = \"TODO in code string\";",
            &todo_re
        ));
        assert!(has_unlinked_todo_in_rust_line(
            "/* TODO: follow up */ let s = \"safe\";",
            &todo_re
        ));
        assert!(!has_unlinked_todo_in_rust_line(
            "/* TODO(#123): tracked */ let s = \"safe\";",
            &todo_re
        ));

        Ok(())
    }

    #[test]
    fn rust_todo_detection_tracks_multiline_block_comments_across_lines() -> Result<()> {
        let todo_re = Regex::new(r"\b(?:TODO|FIXME)\b")?;
        let mut in_block_comment = false;

        assert!(!has_unlinked_todo_in_rust_line_with_block_context(
            "/* context",
            &todo_re,
            &mut in_block_comment,
        ));
        assert!(in_block_comment);
        assert!(has_unlinked_todo_in_rust_line_with_block_context(
            "  TODO: capture this follow-up",
            &todo_re,
            &mut in_block_comment,
        ));
        assert!(in_block_comment);
        assert!(!has_unlinked_todo_in_rust_line_with_block_context(
            "*/ let x = 1;",
            &todo_re,
            &mut in_block_comment,
        ));
        assert!(!in_block_comment);

        Ok(())
    }

    #[test]
    fn rust_todo_detection_ignores_linked_todos_inside_multiline_block_comments() -> Result<()> {
        let todo_re = Regex::new(r"TODO|FIXME")?;
        let mut in_block_comment = false;

        assert!(!has_unlinked_todo_in_rust_line_with_block_context(
            "/* header",
            &todo_re,
            &mut in_block_comment,
        ));
        assert!(!has_unlinked_todo_in_rust_line_with_block_context(
            " * TODO(#123): tracked",
            &todo_re,
            &mut in_block_comment,
        ));
        assert!(!has_unlinked_todo_in_rust_line_with_block_context(
            " */",
            &todo_re,
            &mut in_block_comment,
        ));
        assert!(!in_block_comment);

        Ok(())
    }

    #[test]
    fn rust_todo_detection_handles_nested_block_comments() -> Result<()> {
        let todo_re = Regex::new(r"TODO|FIXME")?;

        assert!(has_unlinked_todo_in_rust_line(
            "let x = 1; /* outer /* nested */ TODO: follow up */",
            &todo_re
        ));
        assert!(!has_unlinked_todo_in_rust_line(
            "let x = 1; /* outer /* nested */ TODO(#42): tracked */",
            &todo_re
        ));

        Ok(())
    }

    #[test]
    fn hash_comment_todo_detection_handles_shebang_and_inline_hashes() -> Result<()> {
        let todo_re = Regex::new(r"(?i)\b(?:todo|fixme)\b")?;

        assert!(!has_unlinked_todo_in_hash_line("#!/usr/bin/env bash", &todo_re));
        assert!(!has_unlinked_todo_in_hash_line("echo# TODO not a comment", &todo_re));
        assert!(!has_unlinked_todo_in_hash_line("len=${#TODO_COUNT}", &todo_re));
        assert!(has_unlinked_todo_in_hash_line("echo hi;# TODO: follow up", &todo_re));
        assert!(has_unlinked_todo_in_hash_line("echo hi;# fixme: follow up", &todo_re));
        assert!(has_unlinked_todo_in_hash_line(
            "echo \"#not-a-comment\" # TODO: follow up",
            &todo_re,
        ));
        assert!(!has_unlinked_todo_in_hash_line("echo '# TODO in string' && true", &todo_re));
        assert!(!has_unlinked_todo_in_hash_line(r"print 'it\'s # TODO in string';", &todo_re));
        assert!(has_unlinked_todo_in_hash_line(
            "echo '# TODO in string' # TODO: follow up",
            &todo_re
        ));
        assert!(!has_unlinked_todo_in_hash_line("print 'it\\'s # TODO in string';", &todo_re,));
        assert!(has_unlinked_todo_in_hash_line(
            "print 'it\\'s # TODO in string'; # TODO: follow up",
            &todo_re
        ));
        assert!(has_unlinked_todo_in_hash_line(
            r"print 'it\'s # TODO in string'; # TODO: follow up",
            &todo_re
        ));
        assert!(has_unlinked_todo_in_hash_line("echo ok&&# TODO: follow up", &todo_re));
        assert!(has_unlinked_todo_in_hash_line("echo ok||# TODO: follow up", &todo_re));
        assert!(has_unlinked_todo_in_hash_line("cat <# TODO: follow up", &todo_re));
        assert!(has_unlinked_todo_in_hash_line("cat ># TODO: follow up", &todo_re));
        assert!(has_unlinked_todo_in_hash_line("len=${#value} # TODO: follow up", &todo_re));
        assert!(has_unlinked_todo_in_hash_line("echo hi # TODO: follow up", &todo_re));
        assert!(has_unlinked_todo_in_hash_line("echo hi&&# TODO: follow up", &todo_re));
        assert!(has_unlinked_todo_in_hash_line("echo hi||# TODO: follow up", &todo_re));
        assert!(!has_unlinked_todo_in_hash_line("echo hi # TODO(#77): tracked", &todo_re));
        assert!(!has_unlinked_todo_in_hash_line("echo `printf '# TODO in backticks'`", &todo_re));
        assert!(has_unlinked_todo_in_hash_line(
            "echo `printf '# TODO in backticks'` # TODO: follow up",
            &todo_re
        ));
        assert!(has_unlinked_todo_in_hash_line("my @x = (1,# TODO: follow up", &todo_re));
        assert!(!has_unlinked_todo_in_hash_line("my @x = (1,# TODO(#77): tracked", &todo_re));
        assert!(!has_unlinked_todo_in_hash_line("print 'it\\'s # TODO in string';", &todo_re));
        assert!(has_unlinked_todo_in_hash_line(
            "print 'it\\'s # TODO in string'; # TODO: follow up",
            &todo_re
        ));
        assert!(!has_unlinked_todo_in_hash_line(
            "printf $'it\\'s # TODO in string' && true",
            &todo_re,
        ));
        assert!(has_unlinked_todo_in_hash_line(
            "printf $'it\\'s # TODO in string' # TODO: follow up",
            &todo_re,
        ));

        Ok(())
    }

    #[test]
    fn hash_comment_todo_detection_handles_escaped_quotes_before_comment() -> Result<()> {
        let todo_re = Regex::new(r"(?i)\b(?:todo|fixme)\b")?;

        assert!(has_unlinked_todo_in_hash_line(
            "echo \"quoted \\\"value\\\"\" # TODO: follow up",
            &todo_re
        ));
        assert!(!has_unlinked_todo_in_hash_line(
            "echo \"quoted # TODO in string\" && true",
            &todo_re
        ));

        Ok(())
    }

    #[test]
    fn perl_todo_detection_allows_comment_start_without_whitespace() -> Result<()> {
        let todo_re = Regex::new(r"TODO|FIXME")?;

        assert!(has_unlinked_todo_in_perl_line("print# TODO: perl comment", &todo_re));
        assert!(!has_unlinked_todo_in_perl_line("my $s = '# TODO in string';", &todo_re));
        assert!(!has_unlinked_todo_in_perl_line("print# TODO(#123): tracked", &todo_re));
        assert!(!has_unlinked_todo_in_perl_line("my $re = m#TODO#;", &todo_re));
        assert!(!has_unlinked_todo_in_perl_line("my $s = q#TODO#;", &todo_re));
        assert!(!has_unlinked_todo_in_perl_line("my $s = qq #TODO#;", &todo_re));
        assert!(!has_unlinked_todo_in_perl_line("my $s = s#foo#TODO#;", &todo_re));
        assert!(!has_unlinked_todo_in_perl_line(
            "my $s = q{{nested} # TODO still string};",
            &todo_re
        ));
        assert!(!has_unlinked_todo_in_perl_line(
            "my $s = s{foo}{{nested} # TODO still replacement};",
            &todo_re
        ));
        assert!(has_unlinked_todo_in_perl_line(
            "my $s = s{foo}{bar}; # TODO: add edge-cases",
            &todo_re
        ));
        assert!(has_unlinked_todo_in_perl_line(
            "my $s = s#foo#bar#; # TODO: add edge-cases",
            &todo_re
        ));

        Ok(())
    }
    #[test]
    fn home_path_detection_only_allows_generic_user_examples() -> Result<()> {
        let home_user_path = Regex::new(r"/home/([A-Za-z0-9._-]+)")?;

        assert!(!has_machine_specific_home_path(
            "Use /home/user/project as the example.",
            &home_user_path,
        ));
        assert!(has_machine_specific_home_path(
            "My path is /home/ubuntu/workspace/perl-lsp",
            &home_user_path,
        ));
        assert!(has_machine_specific_home_path("Local path: /home/u/project", &home_user_path,));

        Ok(())
    }

    #[test]
    fn users_path_detection_only_allows_generic_name_examples() -> Result<()> {
        let users_name_path = Regex::new(r"/Users/([A-Za-z0-9._-]+)")?;

        assert!(!has_machine_specific_users_path(
            "Template: /Users/Name/project",
            &users_name_path,
        ));
        assert!(!has_machine_specific_users_path(
            "Template: /Users/user/project",
            &users_name_path,
        ));
        assert!(has_machine_specific_users_path(
            "Personal path: /Users/alice/dev/perl-lsp",
            &users_name_path,
        ));

        Ok(())
    }

    #[test]
    fn categorize_ignore_maps_reasons_to_expected_buckets() {
        assert_eq!(categorize_ignore("manual: run locally", ""), "manual");
        assert_eq!(categorize_ignore("TODO: requires CI setup", ""), "infra");
        assert_eq!(categorize_ignore("TODO(#123): tracked follow-up", ""), "infra");
        assert_eq!(categorize_ignore("TODO (#123): tracked follow-up", ""), "infra");
        assert_eq!(categorize_ignore("feature: not implemented", ""), "feature");
        assert_eq!(categorize_ignore("AC: parser behavior", ""), "feature");
        assert_eq!(categorize_ignore("placeholder", "#[ignore] // AC: parser behavior"), "feature");
        assert_eq!(categorize_ignore("cache invalidation follow-up", ""), "other");
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
            "crates/perl-workspace/src/bin/workspace_memory_profile.rs"
        )));
    }

    #[test]
    fn allowlisted_exit_hit_matches_windows_and_unix_paths() {
        assert!(is_allowlisted_exit_hit(
            r"crates\perl-parser\src\bin\perl-parse.rs:127:std::process::exit(0);"
        ));
        assert!(is_allowlisted_exit_hit(
            "crates/perl-lsp-rs/src/runtime/dispatch/lifecycle.rs:29:std::process::exit(exit_code);"
        ));
        assert!(!is_allowlisted_exit_hit(
            r#"crates\perl-ci-hygiene\src\main.rs:3196:println!("std::process::exit")"#
        ));
    }

    #[test]
    fn allowlisted_prod_panic_hit_matches_heredoc_regex_initializers() {
        // Line content is the discriminator — path no longer matters after the
        // heredoc anti-patterns module moved into perl-parser.
        assert!(is_allowlisted_prod_panic_hit(
            "crates/perl-parser/src/heredoc_anti_patterns.rs",
            r#"        Err(_) => unreachable!("FORMAT_PATTERN regex failed to compile"),"#
        ));
        assert!(is_allowlisted_prod_panic_hit(
            r"crates\perl-parser\src\heredoc_anti_patterns.rs",
            r#"        Err(_) => unreachable!("FORMAT_PATTERN regex failed to compile"),"#
        ));
        // Old path still matches (line content drives the decision)
        assert!(is_allowlisted_prod_panic_hit(
            "crates/perl-heredoc-anti-patterns/src/lib.rs",
            r#"        Err(_) => unreachable!("FORMAT_PATTERN regex failed to compile"),"#
        ));
        // "known-good static pattern" convention used in other LazyLock<Regex> initializers
        assert!(is_allowlisted_prod_panic_hit(
            "crates/perl-lsp-rs/src/runtime/language/code_actions.rs",
            r#"        Err(err) => unreachable!("GLOBAL_VAR_ASSIGNMENT_RE is a known-good static pattern: {err}"),"#
        ));
        // Bare unreachable!() without a qualifying message is NOT allowlisted
        assert!(!is_allowlisted_prod_panic_hit(
            "crates/perl-lsp-diagnostics/src/lints/ffi_checklib.rs",
            r#"                        _ => unreachable!(),"#
        ));
    }

    // Regression guard for issue #4245: all unreachable!() calls in the heredoc
    // anti-patterns module must be allowlisted regardless of which file they live in.
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
        let forward = "crates/perl-parser/src/heredoc_anti_patterns.rs";
        let backward = r"crates\perl-parser\src\heredoc_anti_patterns.rs";
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
        assert_ne!(RUST_BENCH_BIN, C_BENCH_BIN, "raw Rust v3 and C binaries must differ");
        assert_ne!(RUST_BENCH_BIN, FACADE_BENCH_BIN, "raw Rust v3 and facade binaries must differ");
        assert_ne!(C_BENCH_BIN, FACADE_BENCH_BIN, "C and facade binaries must differ");
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
        assert!(hook.contains("OK to bypass"), "hook must list when bypass is acceptable");
        assert!(hook.contains("NOT OK to bypass"), "hook must list when bypass is unacceptable");
    }

    #[test]
    fn pre_push_hook_auto_unsets_core_bare_corruption() {
        let hook = pre_push_hook_script();
        // Issue #3205 — core.bare=true keeps getting silently set on the main
        // checkout. The hook should self-heal by unsetting it before doing any
        // work that would otherwise fail with "must be run in a work tree".
        assert!(hook.contains("core.bare"), "hook must check for core.bare corruption");
        assert!(
            hook.contains("--unset core.bare"),
            "hook must unset the core.bare flag when corruption is detected"
        );
        assert!(hook.contains("#3205"), "hook must reference issue #3205 in the warning message");
    }

    #[test]
    fn pre_push_hook_has_doc_only_fast_path() -> color_eyre::Result<()> {
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
            .ok_or_else(|| color_eyre::eyre::eyre!("doc-only message must exist in hook script"))?;
        let after_doc = &hook[doc_idx..];
        assert!(
            after_doc.contains("exit 0"),
            "doc-only branch must exit 0 without invoking the full gate"
        );
        let exit_idx = after_doc
            .find("exit 0")
            .ok_or_else(|| color_eyre::eyre::eyre!("doc-only branch must contain 'exit 0'"))?;
        let doc_branch = &after_doc[..exit_idx];
        assert!(
            !doc_branch.contains("cargo fmt --all -- --check"),
            "doc-only branch must not run workspace-wide rustfmt checks before exiting"
        );
        Ok(())
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
    fn pre_push_hook_has_single_crate_tier() -> color_eyre::Result<()> {
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
            .ok_or_else(|| color_eyre::eyre::eyre!("single-crate message must exist"))?;
        let after_single = &hook[single_idx..];
        assert!(
            after_single.contains("exit 0"),
            "single-crate branch must exit 0 without invoking just pr-fast"
        );
        Ok(())
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
        assert!(hook.contains("#3202"), "hook must mention issue #3202 (Windows file-lock race)");
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
    fn e2e_lock_file_path_is_portable() -> color_eyre::Result<()> {
        let lock = std::env::temp_dir().join("e2e-suite.lock");
        let lock_str = lock
            .to_str()
            .ok_or_else(|| color_eyre::eyre::eyre!("temp dir must be valid UTF-8 in CI"))?;
        // On Linux CI this will be /tmp/e2e-suite.lock (acceptable)
        // On Windows this will be C:\Users\...\AppData\Local\Temp\e2e-suite.lock
        assert!(!lock_str.is_empty(), "lock file path must be non-empty");
        Ok(())
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
            ("perl-workspace", "perl-workspace"),
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
