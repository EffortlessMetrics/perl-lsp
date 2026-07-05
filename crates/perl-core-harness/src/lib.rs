//! Upstream Perl core harness integration scaffold.
//!
//! The scaffold can discover tests from a prepared Perl source tree and run the
//! staged profile through a `t/perl` compatibility wrapper in parse and compile
//! modes. Execute mode is limited to explicit execute-one selections.

use chrono::Utc;
use color_eyre::eyre::{Context, Result, bail};
use perl_core_harness_types::{
    BaselineComparison, BaselineViolation, BaselineViolationKind, COMPILE_BASELINE_SCHEMA_VERSION,
    CompileBaseline, DISCOVERY_SCHEMA_VERSION, DiscoveredTest, DiscoveryReport,
    GAP_MAP_SCHEMA_VERSION, GapMap, PREPARE_SCHEMA_VERSION, PrepareReceipt, PrepareStatus,
    RUN_REPORT_SCHEMA_VERSION, RunFailure, RunFileResult, RunReport, RunSummary, RunnerRecord,
    RunnerStatus, SMOKE_SCHEMA_VERSION, SmokeFailureKind, SmokeReport, SmokeStatus,
    SmokeStructuralFailure, lsp_impact_for_bucket, workstream_for_bucket,
};
pub use perl_core_harness_types::{HarnessMode, HarnessProfile, HarnessRunner};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

const PERL_SOURCE_URL: &str = "https://github.com/Perl/perl5";
static RUN_COPY_COUNTER: AtomicU64 = AtomicU64::new(0);

fn project_root() -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|crates_dir| crates_dir.parent())
        .map(Path::to_path_buf)
        .ok_or_else(|| color_eyre::eyre::eyre!("perl-core-harness should live under crates/"))
}

fn profile_runner_args(
    profile: HarnessProfile,
    t_dir: &Path,
    runner: HarnessRunner,
) -> Result<Vec<String>> {
    match runner {
        HarnessRunner::Test => explicit_test_runner_args(t_dir, profile.roots()),
        HarnessRunner::Harness => {
            Ok(profile.roots().iter().map(|root| format!("{root}/*.t")).collect())
        }
    }
}

fn explicit_test_runner_args(t_dir: &Path, roots: &[&str]) -> Result<Vec<String>> {
    let mut args = Vec::new();
    for root in roots {
        collect_test_files(t_dir, &t_dir.join(root), &mut args)?;
    }
    args.sort();
    args.dedup();
    Ok(args)
}

fn collect_test_files(t_dir: &Path, dir: &Path, args: &mut Vec<String>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    let entries = fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("reading entry in {}", dir.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("reading file type for {}", path.display()))?;
        if file_type.is_dir() {
            collect_test_files(t_dir, &path, args)?;
            continue;
        }
        if !file_type.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("t") {
            continue;
        }
        let relative = path
            .strip_prefix(t_dir)
            .with_context(|| format!("normalizing test path {}", path.display()))?;
        args.push(relative.display().to_string().replace('\\', "/"));
    }
    Ok(())
}

fn normalize_selected_tests(profile: HarnessProfile, tests: &[String]) -> Result<Vec<String>> {
    let allowed_roots = profile.roots().iter().copied().collect::<BTreeSet<_>>();
    let mut normalized = Vec::new();
    for test in tests {
        let path = normalize_test_path(test)
            .ok_or_else(|| color_eyre::eyre::eyre!("invalid Perl core test path: {test}"))?;
        if path.contains("..") || path.starts_with('/') || !path.ends_with(".t") {
            bail!("invalid Perl core test path: {test}");
        }
        let Some((root, _rest)) = path.split_once('/') else {
            bail!("selected Perl core test must include a profile root: {test}");
        };
        if !allowed_roots.contains(root) {
            bail!(
                "selected Perl core test {path} is outside profile {} roots {:?}",
                profile,
                profile.roots()
            );
        }
        normalized.push(path);
    }
    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

fn validate_execute_selection(mode: HarnessMode, selected_tests: &[String]) -> Result<()> {
    if mode != HarnessMode::Execute {
        return Ok(());
    }
    if selected_tests.len() != 1 || selected_tests.first().map(String::as_str) != Some("base/if.t")
    {
        bail!("perl-core-harness run --mode execute currently requires exactly --test base/if.t");
    }
    Ok(())
}

fn filter_discovered_tests(
    tests: Vec<DiscoveredTest>,
    selected_tests: &[String],
) -> Result<Vec<DiscoveredTest>> {
    if selected_tests.is_empty() {
        return Ok(tests);
    }

    let selected = selected_tests.iter().cloned().collect::<BTreeSet<_>>();
    let filtered =
        tests.into_iter().filter(|test| selected.contains(&test.path)).collect::<Vec<_>>();
    let found = filtered.iter().map(|test| test.path.clone()).collect::<BTreeSet<_>>();
    if let Some(missing) = selected.iter().find(|path| !found.contains(*path)) {
        bail!("selected Perl core test {missing} was not discovered by upstream harness");
    }
    Ok(filtered)
}

/// Configuration for `perl-core-harness discover`.
#[derive(Debug, Clone)]
pub struct DiscoverConfig {
    pub perl_tree: PathBuf,
    pub host_perl: PathBuf,
    pub runner: HarnessRunner,
    pub profile: HarnessProfile,
    pub output: Option<PathBuf>,
}

/// Configuration for `perl-core-harness prepare`.
#[derive(Debug, Clone)]
pub struct PrepareConfig {
    pub perl_ref: String,
    pub output_dir: Option<PathBuf>,
}

/// Configuration for `perl-core-harness run`.
#[derive(Debug, Clone)]
pub struct RunConfig {
    pub perl_tree: PathBuf,
    pub host_perl: PathBuf,
    pub runner: HarnessRunner,
    pub mode: HarnessMode,
    pub profile: HarnessProfile,
    pub tests: Vec<String>,
    pub output: Option<PathBuf>,
    pub runner_binary: Option<PathBuf>,
}

/// Configuration for `perl-core-harness baseline`.
#[derive(Debug, Clone)]
pub struct BaselineConfig {
    pub mode: HarnessMode,
    pub profile: HarnessProfile,
    pub report: Option<PathBuf>,
    pub baseline: Option<PathBuf>,
    pub accept: bool,
}

/// Configuration for `perl-core-harness smoke`.
#[derive(Debug, Clone)]
pub struct SmokeConfig {
    pub perl_tree: PathBuf,
    pub host_perl: PathBuf,
    pub runner: HarnessRunner,
    pub profile: HarnessProfile,
    pub modes: Vec<HarnessMode>,
    pub output_dir: Option<PathBuf>,
    pub runner_binary: Option<PathBuf>,
    pub perl_ref: Option<String>,
}

/// Discover test files from a prepared Perl tree and write a JSON manifest.
pub fn discover(config: DiscoverConfig) -> Result<()> {
    let perl_tree = canonicalize_existing_dir(&config.perl_tree, "prepared Perl tree")?;
    let t_dir = perl_tree.join("t");
    let script = validate_runner_script(&t_dir, config.runner)?;
    let output_path = config.output.unwrap_or_else(|| default_discovery_path(config.profile));

    let output = invoke_dumptests(
        &config.host_perl,
        &t_dir,
        &script,
        &profile_runner_args(config.profile, &t_dir, config.runner)?,
    )
    .with_context(|| {
        format!("discovering Perl core tests via {} {}", config.runner, config.profile)
    })?;

    let tests = parse_dumptests_output(&output.stdout)?;
    let report = DiscoveryReport {
        schema_version: DISCOVERY_SCHEMA_VERSION.to_string(),
        commit: current_commit(),
        timestamp: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        perl_ref: perl_tree_ref(&perl_tree),
        prepared_tree: perl_tree.display().to_string(),
        host_perl: config.host_perl.display().to_string(),
        runner: config.runner,
        profile: config.profile,
        tests,
    };

    write_discovery_report(&output_path, &report)?;
    tracing::info!(
        "perl-core-harness: discovered {} tests for profile {} via {}",
        report.tests.len(),
        report.profile,
        report.runner
    );
    tracing::info!("wrote {}", output_path.display());
    Ok(())
}

/// Prepare an upstream Perl source tree for advisory smoke runs.
pub fn prepare(config: PrepareConfig) -> Result<()> {
    validate_prepare_ref(&config.perl_ref)?;
    let started_at = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let output_dir =
        config.output_dir.clone().unwrap_or_else(|| default_prepare_output_dir(&config.perl_ref));
    let source_dir = output_dir.join("source").join("perl5");
    let prepared_tree = output_dir.join("perl5");
    let receipt_path = default_prepare_receipt_path(&config.perl_ref);
    let configure_command = "sh Configure -des -Dusedevel".to_string();
    let test_prep_command = "make test_prep".to_string();

    let result = prepare_inner(
        &config,
        &output_dir,
        &source_dir,
        &prepared_tree,
        &configure_command,
        &test_prep_command,
    );
    let (status, resolved_ref, first_error) = match result {
        Ok(resolved_ref) => (PrepareStatus::Pass, Some(resolved_ref), None),
        Err(err) => (PrepareStatus::Fail, None, Some(err.to_string())),
    };
    let receipt = PrepareReceipt {
        schema_version: PREPARE_SCHEMA_VERSION.to_string(),
        requested_ref: config.perl_ref,
        resolved_ref,
        source_url: PERL_SOURCE_URL.to_string(),
        source_dir: source_dir.display().to_string(),
        prepared_tree: prepared_tree.display().to_string(),
        host_os: std::env::consts::OS.to_string(),
        host_arch: std::env::consts::ARCH.to_string(),
        configure_command,
        test_prep_command,
        status,
        first_error,
        started_at,
        finished_at: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    };
    write_prepare_receipt(&receipt_path, &receipt)?;

    tracing::info!("perl-core-harness: prepare {:?} for {}", receipt.status, receipt.requested_ref);
    tracing::info!("wrote {}", receipt_path.display());

    if receipt.status == PrepareStatus::Fail {
        let detail = receipt.first_error.as_deref().unwrap_or("unknown prepare failure");
        bail!("perl-core-harness prepare failed: {detail}");
    }
    Ok(())
}

/// Run upstream Perl core tests through the compatibility runner.
pub fn run_mode(config: RunConfig) -> Result<()> {
    let selected_tests = normalize_selected_tests(config.profile, &config.tests)?;
    validate_execute_selection(config.mode, &selected_tests)?;

    let perl_tree = canonicalize_existing_dir(&config.perl_tree, "prepared Perl tree")?;
    let run_tree = prepare_run_copy(&perl_tree, config.runner, config.mode, config.profile)?;
    let t_dir = run_tree.join("t");
    let script = validate_runner_script(&t_dir, config.runner)?;
    install_t_perl_wrapper(&run_tree)?;
    let dumptests_args = if selected_tests.is_empty() {
        profile_runner_args(config.profile, &t_dir, config.runner)?
    } else {
        selected_tests.clone()
    };
    let dumptests_output = invoke_dumptests(&config.host_perl, &t_dir, &script, &dumptests_args)?;
    let discovered = filter_discovered_tests(
        parse_dumptests_output(&dumptests_output.stdout)?,
        &selected_tests,
    )?;

    let runner_binary = resolve_runner_binary(config.runner_binary.as_deref())?;
    let context_path = run_tree.join("target").join("perl-lsp-runner-records.jsonl");
    if context_path.exists() {
        let context = format!("removing stale context {}", context_path.display());
        fs::remove_file(&context_path).context(context)?;
    }

    let output = invoke_harness_run(
        &config.host_perl,
        &t_dir,
        &script,
        &dumptests_args,
        &runner_binary,
        &context_path,
        config.mode,
    )
    .with_context(|| format!("running Perl core tests via {} {}", config.runner, config.profile))?;

    let mut records = read_runner_records_or_empty(&context_path)?;
    let used_direct_runner = invoke_runner_for_missing_records(
        &t_dir,
        &discovered,
        &records,
        &runner_binary,
        &context_path,
        config.mode,
    )?;
    if used_direct_runner {
        records = read_runner_records_or_empty(&context_path)?;
    }
    let report = build_run_report(BuildRunReportInput {
        config: &config,
        perl_tree: &perl_tree,
        run_tree: &run_tree,
        discovered: &discovered,
        records: &records,
        harness_status: output.status.code(),
    });
    let output_path =
        config.output.unwrap_or_else(|| default_run_report_path(config.mode, config.profile));
    write_run_report(&output_path, &report)?;

    tracing::info!(
        "perl-core-harness: {} {}/{} files passed via {}",
        report.mode,
        report.summary.files_passed,
        report.summary.files_total,
        report.runner
    );
    tracing::info!("wrote {}", output_path.display());

    if report.summary.files_failed > 0 {
        bail!(
            "perl-core-harness {} {} failed for {} of {} files; see {}",
            report.mode,
            report.profile,
            report.summary.files_failed,
            report.summary.files_total,
            output_path.display()
        );
    }
    if !output.status.success() && !used_direct_runner {
        bail!(
            "upstream harness exited with status {} despite no recorded file failures\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

/// Stub for future report rendering.
pub fn report() -> Result<()> {
    bail!("perl-core-harness report is not implemented until run receipts exist")
}

/// Check or update a checked-in Perl core harness baseline.
pub fn baseline(config: BaselineConfig) -> Result<()> {
    let report_path =
        config.report.unwrap_or_else(|| default_run_report_path(config.mode, config.profile));
    let baseline_path =
        config.baseline.unwrap_or_else(|| default_baseline_path(config.mode, config.profile));
    let report = read_run_report(&report_path)?;

    if config.accept {
        let baseline = baseline_from_report(&report)?;
        write_compile_baseline(&baseline_path, &baseline)?;
        tracing::info!(
            "perl-core-harness: accepted {} {} baseline",
            baseline.mode,
            baseline.profile
        );
        tracing::info!("wrote {}", baseline_path.display());
        return Ok(());
    }

    let baseline = read_compile_baseline(&baseline_path)?;
    let comparison = compare_baseline(&baseline, &report);
    if !comparison.is_clean() {
        let details = comparison
            .violations
            .iter()
            .map(|violation| {
                let path = violation.path.as_deref().unwrap_or("-");
                format!("{:?} {path}: {}", violation.kind, violation.message)
            })
            .collect::<Vec<_>>()
            .join("\n");
        bail!(
            "perl-core-harness baseline check failed with {} violation(s):\n{}",
            comparison.violations.len(),
            details
        );
    }

    tracing::info!(
        "perl-core-harness: baseline check passed for {} {}",
        report.mode,
        report.profile
    );
    Ok(())
}

/// Run a manual real-tree discovery + parse/compile smoke and write receipts.
pub fn smoke(config: SmokeConfig) -> Result<()> {
    let modes = normalized_smoke_modes(&config.modes)?;

    let output_dir = config.output_dir.clone().unwrap_or_else(|| default_smoke_dir(config.profile));
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("creating smoke output directory {}", output_dir.display()))?;
    let discovery_path = output_dir.join("discovery.json");
    let parse_path = output_dir.join("parse.json");
    let compile_path = output_dir.join("compile.json");
    let gap_map_path = output_dir.join("gap-map.json");
    let smoke_path = output_dir.join("smoke.json");

    discover(DiscoverConfig {
        perl_tree: config.perl_tree.clone(),
        host_perl: config.host_perl.clone(),
        runner: config.runner,
        profile: config.profile,
        output: Some(discovery_path.clone()),
    })?;
    let discovery = read_discovery_report(&discovery_path)?;

    let mut parse_report = None;
    let mut compile_report = None;
    for mode in &modes {
        let report_path = match mode {
            HarnessMode::Parse => parse_path.clone(),
            HarnessMode::Compile => compile_path.clone(),
            HarnessMode::Execute => {
                bail!("perl-core-harness smoke does not support execute mode");
            }
        };
        let run_result = run_mode(RunConfig {
            perl_tree: config.perl_tree.clone(),
            host_perl: config.host_perl.clone(),
            runner: config.runner,
            mode: *mode,
            profile: config.profile,
            tests: Vec::new(),
            output: Some(report_path.clone()),
            runner_binary: config.runner_binary.clone(),
        });
        if let Err(err) = &run_result {
            if !report_path.is_file() {
                bail!(
                    "{} smoke did not write {} after runner failure: {}",
                    mode,
                    report_path.display(),
                    err
                );
            }
            tracing::info!(
                "perl-core-harness: {mode} smoke preserved bucketed failure report: {err}"
            );
        }

        let report = read_run_report(&report_path)?;
        match mode {
            HarnessMode::Parse => parse_report = Some(report),
            HarnessMode::Compile => compile_report = Some(report),
            HarnessMode::Execute => unreachable!("execute mode is rejected above"),
        }
    }

    let gap_map =
        build_gap_map(config.profile, &modes, parse_report.as_ref(), compile_report.as_ref());
    write_gap_map(&gap_map_path, &gap_map)?;

    let smoke_report = build_smoke_report(BuildSmokeReportInput {
        config: &config,
        modes: &modes,
        discovery: &discovery,
        discovery_path: &discovery_path,
        parse_path: parse_report.as_ref().map(|_| parse_path.as_path()),
        parse_report: parse_report.as_ref(),
        compile_path: compile_report.as_ref().map(|_| compile_path.as_path()),
        compile_report: compile_report.as_ref(),
        gap_map_path: &gap_map_path,
    });
    write_smoke_report(&smoke_path, &smoke_report)?;

    tracing::info!(
        "perl-core-harness: smoke {} for {} via {}",
        match smoke_report.status {
            SmokeStatus::Pass => "passed",
            SmokeStatus::Fail => "failed",
        },
        smoke_report.profile,
        smoke_report.runner
    );
    tracing::info!("wrote {}", smoke_path.display());

    if smoke_report.status == SmokeStatus::Fail {
        let details = smoke_report
            .structural_failures
            .iter()
            .map(|failure| {
                let mode = failure.mode.map(|mode| mode.to_string()).unwrap_or_else(|| "-".into());
                let path = failure.path.as_deref().unwrap_or("-");
                format!("{:?} {mode} {path}: {}", failure.kind, failure.message)
            })
            .collect::<Vec<_>>()
            .join("\n");
        bail!(
            "perl-core-harness smoke failed receipt integrity with {} structural failure(s):\n{}",
            smoke_report.structural_failures.len(),
            details
        );
    }

    Ok(())
}

fn validate_prepare_ref(perl_ref: &str) -> Result<()> {
    if perl_ref.trim().is_empty() {
        bail!("perl-core-harness prepare --ref must not be empty");
    }
    if perl_ref.contains("..") || perl_ref.contains('\\') {
        bail!("perl-core-harness prepare --ref contains unsupported path-like syntax");
    }
    Ok(())
}

fn prepare_inner(
    config: &PrepareConfig,
    output_dir: &Path,
    source_dir: &Path,
    prepared_tree: &Path,
    configure_command: &str,
    test_prep_command: &str,
) -> Result<String> {
    if !cfg!(target_os = "linux") {
        bail!(
            "upstream Perl prepare is Linux-only in this slice; current host is {}",
            std::env::consts::OS
        );
    }
    ensure_host_tool("git")?;
    ensure_host_tool("perl")?;
    ensure_host_tool("make")?;
    ensure_host_tool("sh")?;

    fs::create_dir_all(output_dir)
        .with_context(|| format!("creating prepare output directory {}", output_dir.display()))?;
    if source_dir.join(".git").is_dir() {
        run_command(
            Command::new("git").arg("-C").arg(source_dir).args(["fetch", "--tags", "origin"]),
        )?;
    } else {
        if let Some(parent) = source_dir.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating source parent {}", parent.display()))?;
        }
        run_command(
            Command::new("git")
                .args(["clone", "--filter=blob:none", PERL_SOURCE_URL])
                .arg(source_dir),
        )?;
    }
    run_command(Command::new("git").arg("-C").arg(source_dir).args([
        "checkout",
        "--detach",
        &config.perl_ref,
    ]))?;
    let resolved_ref =
        command_stdout(Command::new("git").arg("-C").arg(source_dir).args(["rev-parse", "HEAD"]))?;

    if prepared_tree.exists() {
        fs::remove_dir_all(prepared_tree)
            .with_context(|| format!("removing prior prepared tree {}", prepared_tree.display()))?;
    }
    copy_dir_all_filtered(source_dir, prepared_tree, &|path| {
        path.file_name().is_some_and(|name| name == ".git")
    })?;

    run_command(Command::new("sh").current_dir(prepared_tree).args([
        "Configure",
        "-des",
        "-Dusedevel",
    ]))?;
    run_command(Command::new("make").current_dir(prepared_tree).arg("test_prep"))?;
    validate_prepared_tree(prepared_tree)?;

    tracing::info!(
        "perl-core-harness: ran `{configure_command}` and `{test_prep_command}` in {}",
        prepared_tree.display()
    );
    Ok(resolved_ref)
}

fn ensure_host_tool(tool: &str) -> Result<()> {
    let output = Command::new("sh")
        .args(["-c", &format!("command -v {tool}")])
        .output()
        .with_context(|| format!("checking for host tool {tool}"))?;
    if !output.status.success() {
        bail!("required host tool is missing from PATH: {tool}");
    }
    Ok(())
}

fn command_stdout(command: &mut Command) -> Result<String> {
    let display = format!("{command:?}");
    let output = command.output().with_context(|| format!("spawning {display}"))?;
    if !output.status.success() {
        bail!(
            "command failed: {display}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let text = String::from_utf8(output.stdout).context("decoding command stdout")?;
    Ok(text.trim().to_string())
}

fn run_command(command: &mut Command) -> Result<()> {
    let display = format!("{command:?}");
    let output = command.output().with_context(|| format!("spawning {display}"))?;
    if !output.status.success() {
        bail!(
            "command failed: {display}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn validate_prepared_tree(prepared_tree: &Path) -> Result<()> {
    let t_dir = prepared_tree.join("t");
    for required in [t_dir.join("TEST"), t_dir.join("harness")] {
        if !required.is_file() {
            bail!("prepared Perl tree is missing required file {}", required.display());
        }
    }
    let base_dir = t_dir.join("base");
    let has_base_tests = base_dir
        .read_dir()
        .with_context(|| format!("reading {}", base_dir.display()))?
        .filter_map(|entry| entry.ok())
        .any(|entry| entry.path().extension().is_some_and(|extension| extension == "t"));
    if !has_base_tests {
        bail!("prepared Perl tree has no t/base/*.t tests: {}", base_dir.display());
    }
    let has_test_perl = ["perl", "perl.exe"].iter().any(|name| t_dir.join(name).is_file());
    if !has_test_perl {
        bail!("prepared Perl tree is missing t/perl or t/perl.exe in {}", t_dir.display());
    }
    Ok(())
}

fn canonicalize_existing_dir(path: &Path, label: &str) -> Result<PathBuf> {
    if !path.is_dir() {
        bail!("{label} does not exist or is not a directory: {}", path.display());
    }
    path.canonicalize().with_context(|| format!("canonicalizing {label}: {}", path.display()))
}

fn validate_runner_script(t_dir: &Path, runner: HarnessRunner) -> Result<PathBuf> {
    if !t_dir.is_dir() {
        bail!("prepared Perl tree is missing t/ directory: {}", t_dir.display());
    }
    let script = t_dir.join(runner.script_name());
    if !script.is_file() {
        bail!(
            "prepared Perl tree is missing t/{} for {} runner: {}",
            runner.script_name(),
            runner,
            script.display()
        );
    }
    Ok(script)
}

fn invoke_dumptests(
    host_perl: &Path,
    t_dir: &Path,
    script: &Path,
    profile_args: &[String],
) -> Result<Output> {
    let script_name = script
        .file_name()
        .ok_or_else(|| color_eyre::eyre::eyre!("runner script has no file name"))?;
    let mut command = Command::new(host_perl);
    command.current_dir(t_dir);
    command.arg(script_name);
    command.arg("--dumptests");
    for arg in profile_args {
        command.arg(arg);
    }
    command.env("LC_ALL", "C");
    command.env_remove("PERL5LIB");
    command.env_remove("PERLLIB");
    command.env_remove("PERL5OPT");
    command.env_remove("PERL_UNICODE");
    command.env_remove("PERL_LOCAL_LIB_ROOT");
    command.env_remove("PERL_MB_OPT");

    let output =
        command.output().with_context(|| format!("spawning host Perl: {}", host_perl.display()))?;
    if !output.status.success() {
        bail!(
            "upstream harness --dumptests failed with status {}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(output)
}

fn invoke_harness_run(
    host_perl: &Path,
    t_dir: &Path,
    script: &Path,
    profile_args: &[String],
    runner_binary: &Path,
    context_path: &Path,
    mode: HarnessMode,
) -> Result<Output> {
    let script_name = script
        .file_name()
        .ok_or_else(|| color_eyre::eyre::eyre!("runner script has no file name"))?;
    let tap_dir = project_root()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("target")
        .join("perl-core")
        .join("tap")
        .join(mode.as_str());
    fs::create_dir_all(&tap_dir)
        .with_context(|| format!("creating TAP dump directory {}", tap_dir.display()))?;

    let mut command = Command::new(host_perl);
    command.current_dir(t_dir);
    command.arg(script_name);
    for arg in profile_args {
        command.arg(arg);
    }
    command.env("LC_ALL", "C");
    command.env("TEST_JOBS", "1");
    command.env("PERL_TEST_HARNESS_DUMP_TAP", tap_dir);
    command.env("PERL_LSP_HARNESS_MODE", mode.as_str());
    command.env("PERL_LSP_HARNESS_CONTEXT", context_path);
    command.env("PERL_LSP_CORE_TEST_RUNNER", runner_binary);
    sanitize_perl_env(&mut command);

    command.output().with_context(|| format!("spawning host Perl: {}", host_perl.display()))
}

fn invoke_runner_for_missing_records(
    t_dir: &Path,
    discovered: &[DiscoveredTest],
    records: &[RunnerRecord],
    runner_binary: &Path,
    context_path: &Path,
    mode: HarnessMode,
) -> Result<bool> {
    let recorded = records
        .iter()
        .filter_map(|record| normalize_test_path(&record.path))
        .collect::<BTreeSet<_>>();
    let missing =
        discovered.iter().filter(|test| !recorded.contains(&test.path)).collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(false);
    }

    if let Some(parent) = context_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating runner context directory {}", parent.display()))?;
    }

    for test in &missing {
        let output = invoke_direct_runner(t_dir, runner_binary, context_path, mode, &test.path)?;
        if !context_path.is_file() {
            bail!(
                "direct runner did not write context for {}\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
                test.path,
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    let refreshed = read_runner_records_or_empty(context_path)?;
    let recorded_after = refreshed
        .iter()
        .filter_map(|record| normalize_test_path(&record.path))
        .collect::<BTreeSet<_>>();
    if let Some(test) = missing.iter().find(|test| !recorded_after.contains(&test.path)) {
        bail!("direct runner did not write a record for {}", test.path);
    }

    Ok(true)
}

fn invoke_direct_runner(
    t_dir: &Path,
    runner_binary: &Path,
    context_path: &Path,
    mode: HarnessMode,
    test_path: &str,
) -> Result<Output> {
    let mut command = Command::new(runner_binary);
    command.current_dir(t_dir);
    command.arg(test_path);
    command.env("LC_ALL", "C");
    command.env("PERL_LSP_HARNESS_MODE", mode.as_str());
    command.env("PERL_LSP_HARNESS_CONTEXT", context_path);
    sanitize_perl_env(&mut command);

    command
        .output()
        .with_context(|| format!("spawning perl-core-test-runner: {}", runner_binary.display()))
}

fn sanitize_perl_env(command: &mut Command) {
    command.env_remove("PERL5LIB");
    command.env_remove("PERLLIB");
    command.env_remove("PERL5OPT");
    command.env_remove("PERL_UNICODE");
    command.env_remove("PERL_LOCAL_LIB_ROOT");
    command.env_remove("PERL_MB_OPT");
}

fn parse_dumptests_output(stdout: &[u8]) -> Result<Vec<DiscoveredTest>> {
    let text = String::from_utf8(stdout.to_vec()).context("decoding --dumptests output")?;
    let mut tests = Vec::new();
    for line in text.lines() {
        let Some(path) = normalize_test_path(line) else {
            continue;
        };
        let root = path
            .split('/')
            .next()
            .filter(|part| !part.is_empty())
            .ok_or_else(|| color_eyre::eyre::eyre!("test path has no root: {path}"))?
            .to_string();
        tests.push(DiscoveredTest { path, root });
    }
    tests.sort_by(|left, right| left.path.cmp(&right.path));
    tests.dedup_by(|left, right| left.path == right.path);
    if tests.is_empty() {
        bail!("upstream harness --dumptests returned no .t files");
    }
    Ok(tests)
}

fn normalize_test_path(line: &str) -> Option<String> {
    let trimmed = line.trim().trim_matches('"').trim_matches('\'');
    if trimmed.is_empty() || !trimmed.ends_with(".t") {
        return None;
    }
    let normalized = trimmed.replace('\\', "/");
    let normalized = normalized.strip_prefix("./").unwrap_or(&normalized);
    let normalized = normalized.strip_prefix("t/").unwrap_or(normalized);
    Some(normalized.to_string())
}

fn default_discovery_path(profile: HarnessProfile) -> PathBuf {
    let root = project_root().unwrap_or_else(|_| PathBuf::from("."));
    root.join("target").join("perl-core").join("discovery").join(format!("{profile}.json"))
}

fn default_run_report_path(mode: HarnessMode, profile: HarnessProfile) -> PathBuf {
    let root = project_root().unwrap_or_else(|_| PathBuf::from("."));
    root.join("target").join("perl-core").join("reports").join(format!("{profile}-{mode}.json"))
}

fn default_baseline_path(mode: HarnessMode, profile: HarnessProfile) -> PathBuf {
    let root = project_root().unwrap_or_else(|_| PathBuf::from("."));
    root.join(".ci").join("perl-core-harness").join(format!("{profile}-{mode}-baseline.json"))
}

fn default_prepare_output_dir(perl_ref: &str) -> PathBuf {
    let root = project_root().unwrap_or_else(|_| PathBuf::from("."));
    root.join("target").join("perl-core").join("upstream").join(safe_path_component(perl_ref))
}

fn default_prepare_receipt_path(perl_ref: &str) -> PathBuf {
    let root = project_root().unwrap_or_else(|_| PathBuf::from("."));
    root.join("target")
        .join("perl-core")
        .join("prepare")
        .join(safe_path_component(perl_ref))
        .join("prepare.json")
}

fn default_smoke_dir(profile: HarnessProfile) -> PathBuf {
    let root = project_root().unwrap_or_else(|_| PathBuf::from("."));
    root.join("target").join("perl-core").join("smoke").join(profile.as_str())
}

fn safe_path_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '_' { ch } else { '-' }
        })
        .collect()
}

fn write_discovery_report(path: &Path, report: &DiscoveryReport) -> Result<()> {
    if let Some(parent) = path.parent() {
        let context = format!("creating output directory {}", parent.display());
        fs::create_dir_all(parent).context(context)?;
    }
    let json = serde_json::to_string_pretty(report).context("serializing discovery report")?;
    fs::write(path, format!("{json}\n"))
        .with_context(|| format!("writing discovery report {}", path.display()))
}

fn read_discovery_report(path: &Path) -> Result<DiscoveryReport> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("reading discovery report {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("decoding discovery report {}", path.display()))
}

fn write_run_report(path: &Path, report: &RunReport) -> Result<()> {
    if let Some(parent) = path.parent() {
        let context = format!("creating output directory {}", parent.display());
        fs::create_dir_all(parent).context(context)?;
    }
    let json = serde_json::to_string_pretty(report).context("serializing run report")?;
    fs::write(path, format!("{json}\n"))
        .with_context(|| format!("writing run report {}", path.display()))
}

fn read_run_report(path: &Path) -> Result<RunReport> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("reading run report {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("decoding run report {}", path.display()))
}

fn read_compile_baseline(path: &Path) -> Result<CompileBaseline> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("reading baseline {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("decoding baseline {}", path.display()))
}

fn write_compile_baseline(path: &Path, baseline: &CompileBaseline) -> Result<()> {
    if let Some(parent) = path.parent() {
        let context = format!("creating baseline directory {}", parent.display());
        fs::create_dir_all(parent).context(context)?;
    }
    let json = serde_json::to_string_pretty(baseline).context("serializing compile baseline")?;
    fs::write(path, format!("{json}\n"))
        .with_context(|| format!("writing baseline {}", path.display()))
}

fn write_prepare_receipt(path: &Path, receipt: &PrepareReceipt) -> Result<()> {
    if let Some(parent) = path.parent() {
        let context = format!("creating prepare receipt directory {}", parent.display());
        fs::create_dir_all(parent).context(context)?;
    }
    let json = serde_json::to_string_pretty(receipt).context("serializing prepare receipt")?;
    fs::write(path, format!("{json}\n"))
        .with_context(|| format!("writing prepare receipt {}", path.display()))
}

fn write_smoke_report(path: &Path, report: &SmokeReport) -> Result<()> {
    if let Some(parent) = path.parent() {
        let context = format!("creating smoke report directory {}", parent.display());
        fs::create_dir_all(parent).context(context)?;
    }
    let json = serde_json::to_string_pretty(report).context("serializing smoke report")?;
    fs::write(path, format!("{json}\n"))
        .with_context(|| format!("writing smoke report {}", path.display()))
}

fn write_gap_map(path: &Path, gap_map: &GapMap) -> Result<()> {
    if let Some(parent) = path.parent() {
        let context = format!("creating gap-map directory {}", parent.display());
        fs::create_dir_all(parent).context(context)?;
    }
    let json = serde_json::to_string_pretty(gap_map).context("serializing gap map")?;
    fs::write(path, format!("{json}\n"))
        .with_context(|| format!("writing gap map {}", path.display()))
}

fn baseline_from_report(report: &RunReport) -> Result<CompileBaseline> {
    let mut baseline = CompileBaseline {
        schema_version: COMPILE_BASELINE_SCHEMA_VERSION.to_string(),
        report_schema_version: report.schema_version.clone(),
        mode: report.mode,
        profile: report.profile,
        files_total: report.summary.files_total,
        files_passed: report.summary.files_passed,
        files_failed: report.summary.files_failed,
        tap_assertions_total: report.summary.tap_assertions_total,
        tap_assertions_passed: report.summary.tap_assertions_passed,
        buckets: report.buckets.clone(),
        expected_failures: report.failures.clone(),
        file_results: report.file_results.clone(),
    };
    sort_baseline(&mut baseline);
    let validation = validate_report_bucket_shape(report);
    if !validation.is_empty() {
        let details = validation
            .iter()
            .map(|violation| {
                let path = violation.path.as_deref().unwrap_or("-");
                format!("{:?} {path}: {}", violation.kind, violation.message)
            })
            .collect::<Vec<_>>()
            .join("\n");
        bail!("cannot accept baseline with invalid failure buckets:\n{details}");
    }
    Ok(baseline)
}

fn sort_baseline(baseline: &mut CompileBaseline) {
    baseline.file_results.sort_by(|left, right| left.path.cmp(&right.path));
    baseline.expected_failures.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.bucket.cmp(&right.bucket))
            .then_with(|| left.phase.cmp(&right.phase))
    });
}

fn compare_baseline(baseline: &CompileBaseline, report: &RunReport) -> BaselineComparison {
    let mut violations = Vec::new();

    if baseline.schema_version != COMPILE_BASELINE_SCHEMA_VERSION {
        violations.push(violation(
            BaselineViolationKind::SchemaMismatch,
            None,
            format!(
                "baseline schema {} does not match {}",
                baseline.schema_version, COMPILE_BASELINE_SCHEMA_VERSION
            ),
        ));
    }
    if baseline.report_schema_version != RUN_REPORT_SCHEMA_VERSION {
        violations.push(violation(
            BaselineViolationKind::SchemaMismatch,
            None,
            format!(
                "baseline report schema {} does not match {}",
                baseline.report_schema_version, RUN_REPORT_SCHEMA_VERSION
            ),
        ));
    }
    if report.schema_version != baseline.report_schema_version {
        violations.push(violation(
            BaselineViolationKind::SchemaMismatch,
            None,
            format!(
                "report schema {} does not match baseline report schema {}",
                report.schema_version, baseline.report_schema_version
            ),
        ));
    }
    if baseline.mode != report.mode {
        violations.push(violation(
            BaselineViolationKind::ModeMismatch,
            None,
            format!("baseline mode {} does not match report mode {}", baseline.mode, report.mode),
        ));
    }
    if baseline.profile != report.profile {
        violations.push(violation(
            BaselineViolationKind::ProfileMismatch,
            None,
            format!(
                "baseline profile {} does not match report profile {}",
                baseline.profile, report.profile
            ),
        ));
    }

    violations.extend(validate_report_bucket_shape(report));
    violations.extend(compare_file_results(baseline, report));
    violations.extend(compare_failure_buckets(baseline, report));
    violations.extend(compare_summary_assertions(baseline, report));

    BaselineComparison { violations }
}

fn compare_file_results(baseline: &CompileBaseline, report: &RunReport) -> Vec<BaselineViolation> {
    let baseline_results = baseline
        .file_results
        .iter()
        .map(|result| (result.path.as_str(), result))
        .collect::<BTreeMap<_, _>>();
    let report_results = report
        .file_results
        .iter()
        .map(|result| (result.path.as_str(), result))
        .collect::<BTreeMap<_, _>>();
    let expected_failure_paths = baseline
        .expected_failures
        .iter()
        .map(|failure| failure.path.as_str())
        .collect::<BTreeSet<_>>();
    let report_failure_paths =
        report.failures.iter().map(|failure| failure.path.as_str()).collect::<BTreeSet<_>>();
    let mut violations = Vec::new();

    for (path, baseline_result) in &baseline_results {
        let Some(report_result) = report_results.get(path) else {
            violations.push(violation(
                BaselineViolationKind::MissingExpectedFile,
                Some((*path).to_string()),
                "baseline file is missing from current report",
            ));
            continue;
        };

        if baseline_result.status == RunnerStatus::Pass
            && report_result.status == RunnerStatus::Fail
        {
            violations.push(violation(
                BaselineViolationKind::PreviouslyPassingFileFailed,
                Some((*path).to_string()),
                "file passed in baseline but fails in current report",
            ));
        }
        if report_result.assertions_passed < baseline_result.assertions_passed {
            violations.push(violation(
                BaselineViolationKind::AssertionRegression,
                Some((*path).to_string()),
                format!(
                    "assertions passed regressed from {} to {}",
                    baseline_result.assertions_passed, report_result.assertions_passed
                ),
            ));
        }
        if report_result.assertions_total < baseline_result.assertions_total {
            violations.push(violation(
                BaselineViolationKind::AssertionRegression,
                Some((*path).to_string()),
                format!(
                    "assertions total regressed from {} to {}",
                    baseline_result.assertions_total, report_result.assertions_total
                ),
            ));
        }
    }

    for report_failure in &report_failure_paths {
        if !baseline_results.contains_key(report_failure)
            || !expected_failure_paths.contains(report_failure)
        {
            violations.push(violation(
                BaselineViolationKind::UnexpectedNewFailure,
                Some((*report_failure).to_string()),
                "current report contains a failure not accepted by the baseline",
            ));
        }
    }

    violations
}

fn compare_failure_buckets(
    baseline: &CompileBaseline,
    report: &RunReport,
) -> Vec<BaselineViolation> {
    let mut violations = Vec::new();
    for (bucket, current_count) in &report.buckets {
        let baseline_count = baseline.buckets.get(bucket).copied().unwrap_or(0);
        if *current_count > baseline_count {
            violations.push(violation(
                BaselineViolationKind::BucketCountIncreased,
                None,
                format!("bucket {bucket} increased from {baseline_count} to {current_count}"),
            ));
        }
    }
    violations
}

fn compare_summary_assertions(
    baseline: &CompileBaseline,
    report: &RunReport,
) -> Vec<BaselineViolation> {
    let mut violations = Vec::new();
    if report.summary.tap_assertions_passed < baseline.tap_assertions_passed {
        violations.push(violation(
            BaselineViolationKind::AssertionRegression,
            None,
            format!(
                "passed assertions regressed from {} to {}",
                baseline.tap_assertions_passed, report.summary.tap_assertions_passed
            ),
        ));
    }
    if report.summary.tap_assertions_total < baseline.tap_assertions_total {
        violations.push(violation(
            BaselineViolationKind::AssertionRegression,
            None,
            format!(
                "total assertions regressed from {} to {}",
                baseline.tap_assertions_total, report.summary.tap_assertions_total
            ),
        ));
    }
    violations
}

fn validate_report_bucket_shape(report: &RunReport) -> Vec<BaselineViolation> {
    let mut violations = Vec::new();
    let failure_paths =
        report.failures.iter().map(|failure| failure.path.as_str()).collect::<BTreeSet<_>>();
    for failure in &report.failures {
        if failure.bucket.trim().is_empty() {
            violations.push(violation(
                BaselineViolationKind::UnbucketedFailure,
                Some(failure.path.clone()),
                "failure has an empty bucket",
            ));
        } else if failure.bucket == "unknown" {
            violations.push(violation(
                BaselineViolationKind::UnknownBucket,
                Some(failure.path.clone()),
                "failure is bucketed as unknown",
            ));
        }
    }
    for result in &report.file_results {
        if result.status == RunnerStatus::Fail && !failure_paths.contains(result.path.as_str()) {
            violations.push(violation(
                BaselineViolationKind::UnbucketedFailure,
                Some(result.path.clone()),
                "failing file has no failure bucket record",
            ));
        }
    }
    violations
}

fn violation(
    kind: BaselineViolationKind,
    path: Option<String>,
    message: impl Into<String>,
) -> BaselineViolation {
    BaselineViolation { kind, path, message: message.into() }
}

struct BuildSmokeReportInput<'a> {
    config: &'a SmokeConfig,
    modes: &'a [HarnessMode],
    discovery: &'a DiscoveryReport,
    discovery_path: &'a Path,
    parse_path: Option<&'a Path>,
    parse_report: Option<&'a RunReport>,
    compile_path: Option<&'a Path>,
    compile_report: Option<&'a RunReport>,
    gap_map_path: &'a Path,
}

fn build_smoke_report(input: BuildSmokeReportInput<'_>) -> SmokeReport {
    let mut structural_failures = Vec::new();
    let modes_requested = input.modes.to_vec();

    if modes_requested.contains(&HarnessMode::Parse) {
        collect_smoke_report_failures(
            HarnessMode::Parse,
            input.config.profile,
            input.parse_report,
            &mut structural_failures,
        );
    }
    if modes_requested.contains(&HarnessMode::Compile) {
        collect_smoke_report_failures(
            HarnessMode::Compile,
            input.config.profile,
            input.compile_report,
            &mut structural_failures,
        );
    }

    let status = if structural_failures.is_empty() { SmokeStatus::Pass } else { SmokeStatus::Fail };

    SmokeReport {
        schema_version: SMOKE_SCHEMA_VERSION.to_string(),
        timestamp: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        repo_commit: current_commit(),
        perl_requested_ref: input
            .config
            .perl_ref
            .clone()
            .unwrap_or_else(|| input.discovery.perl_ref.clone()),
        perl_resolved_ref: input.discovery.perl_ref.clone(),
        prepared_tree: input.discovery.prepared_tree.clone(),
        host_perl: input.discovery.host_perl.clone(),
        runner: input.config.runner,
        profile: input.config.profile,
        modes_requested,
        discovery_report: path_to_string(input.discovery_path),
        parse_report: input.parse_path.map(path_to_string),
        compile_report: input.compile_path.map(path_to_string),
        gap_map: path_to_string(input.gap_map_path),
        discovery_total: input.discovery.tests.len(),
        parse_files_total: input.parse_report.map(|report| report.summary.files_total),
        parse_files_passed: input.parse_report.map(|report| report.summary.files_passed),
        parse_files_failed: input.parse_report.map(|report| report.summary.files_failed),
        compile_files_total: input.compile_report.map(|report| report.summary.files_total),
        compile_files_passed: input.compile_report.map(|report| report.summary.files_passed),
        compile_files_failed: input.compile_report.map(|report| report.summary.files_failed),
        parse_buckets: input.parse_report.map(|report| report.buckets.clone()),
        compile_buckets: input.compile_report.map(|report| report.buckets.clone()),
        status,
        structural_failures,
    }
}

fn normalized_smoke_modes(modes: &[HarnessMode]) -> Result<Vec<HarnessMode>> {
    let selected = if modes.is_empty() {
        vec![HarnessMode::Parse, HarnessMode::Compile]
    } else {
        modes.to_vec()
    };
    let mut normalized = Vec::new();
    for mode in selected {
        if mode == HarnessMode::Execute {
            bail!("perl-core-harness smoke does not support execute mode");
        }
        if !normalized.contains(&mode) {
            normalized.push(mode);
        }
    }
    if normalized.is_empty() {
        bail!("perl-core-harness smoke needs at least one mode");
    }
    Ok(normalized)
}

fn build_gap_map(
    profile: HarnessProfile,
    modes: &[HarnessMode],
    parse_report: Option<&RunReport>,
    compile_report: Option<&RunReport>,
) -> GapMap {
    let mut total_files = 0usize;
    let mut passed_files = 0usize;
    let mut failed_files = 0usize;
    let mut buckets = BTreeMap::new();
    let mut files_by_bucket: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut first_failure_by_bucket = BTreeMap::new();
    let mut workstreams = BTreeMap::new();
    let mut lsp_impact = BTreeMap::new();

    for report in [parse_report, compile_report].into_iter().flatten() {
        total_files = total_files.saturating_add(report.summary.files_total);
        passed_files = passed_files.saturating_add(report.summary.files_passed);
        failed_files = failed_files.saturating_add(report.summary.files_failed);
        for (bucket, count) in &report.buckets {
            *buckets.entry(bucket.clone()).or_insert(0) += count;
        }
        for failure in &report.failures {
            files_by_bucket.entry(failure.bucket.clone()).or_default().push(failure.path.clone());
            first_failure_by_bucket
                .entry(failure.bucket.clone())
                .or_insert_with(|| failure.clone());
            *workstreams.entry(failure.workstream.clone()).or_insert(0) += 1;
            for impact in &failure.lsp_impact {
                *lsp_impact.entry(impact.clone()).or_insert(0) += 1;
            }
        }
    }
    for paths in files_by_bucket.values_mut() {
        paths.sort();
        paths.dedup();
    }

    GapMap {
        schema_version: GAP_MAP_SCHEMA_VERSION.to_string(),
        profile,
        mode: modes.iter().map(|mode| mode.as_str()).collect::<Vec<_>>().join(","),
        total_files,
        passed_files,
        failed_files,
        buckets,
        files_by_bucket,
        first_failure_by_bucket,
        workstreams,
        lsp_impact,
        top_parse_failures: top_failures(parse_report),
        top_compile_failures: top_failures(compile_report),
    }
}

fn top_failures(report: Option<&RunReport>) -> Vec<RunFailure> {
    report.map(|report| report.failures.iter().take(10).cloned().collect()).unwrap_or_default()
}

fn collect_smoke_report_failures(
    expected_mode: HarnessMode,
    expected_profile: HarnessProfile,
    report: Option<&RunReport>,
    failures: &mut Vec<SmokeStructuralFailure>,
) {
    let Some(report) = report else {
        failures.push(SmokeStructuralFailure {
            mode: Some(expected_mode),
            path: None,
            kind: SmokeFailureKind::MissingReport,
            message: format!("{expected_mode} report was not written"),
        });
        return;
    };

    if report.mode != expected_mode {
        failures.push(SmokeStructuralFailure {
            mode: Some(expected_mode),
            path: None,
            kind: SmokeFailureKind::ProfileMismatch,
            message: format!(
                "report mode {} does not match requested {expected_mode}",
                report.mode
            ),
        });
    }
    if report.profile != expected_profile {
        failures.push(SmokeStructuralFailure {
            mode: Some(expected_mode),
            path: None,
            kind: SmokeFailureKind::ProfileMismatch,
            message: format!(
                "report profile {} does not match requested {expected_profile}",
                report.profile
            ),
        });
    }

    for violation in validate_report_bucket_shape(report) {
        let kind = match violation.kind {
            BaselineViolationKind::UnknownBucket => SmokeFailureKind::UnknownBucket,
            BaselineViolationKind::UnbucketedFailure => SmokeFailureKind::UnbucketedFailure,
            _ => continue,
        };
        failures.push(SmokeStructuralFailure {
            mode: Some(expected_mode),
            path: violation.path,
            kind,
            message: violation.message,
        });
    }
}

fn path_to_string(path: &Path) -> String {
    path.display().to_string()
}

fn prepare_run_copy(
    perl_tree: &Path,
    runner: HarnessRunner,
    mode: HarnessMode,
    profile: HarnessProfile,
) -> Result<PathBuf> {
    let nonce = RUN_COPY_COUNTER.fetch_add(1, Ordering::Relaxed);
    let run_tree = project_root()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("target")
        .join("perl-core")
        .join("runs")
        .join(format!("{runner}-{mode}-{profile}-{}-{nonce}", std::process::id()));
    if run_tree.exists() {
        fs::remove_dir_all(&run_tree)
            .with_context(|| format!("removing prior run tree {}", run_tree.display()))?;
    }
    copy_dir_all(perl_tree, &run_tree)?;
    Ok(run_tree)
}

fn copy_dir_all(source: &Path, destination: &Path) -> Result<()> {
    copy_dir_all_filtered(source, destination, &|_| false)
}

fn copy_dir_all_filtered(
    source: &Path,
    destination: &Path,
    skip: &dyn Fn(&Path) -> bool,
) -> Result<()> {
    let create_context = format!("creating directory {}", destination.display());
    fs::create_dir_all(destination).context(create_context)?;
    let read_context = format!("reading {}", source.display());
    for entry in fs::read_dir(source).context(read_context)? {
        let entry_context = format!("reading entry in {}", source.display());
        let entry = entry.context(entry_context)?;
        let entry_path = entry.path();
        if skip(&entry_path) {
            continue;
        }
        let type_context = format!("reading file type for {}", entry_path.display());
        let ty = entry.file_type().context(type_context)?;
        let child_destination = destination.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all_filtered(&entry_path, &child_destination, skip)?;
        } else if ty.is_file() {
            let copy_context =
                format!("copying {} to {}", entry_path.display(), child_destination.display());
            fs::copy(entry_path, &child_destination).context(copy_context)?;
        }
    }
    Ok(())
}

fn install_t_perl_wrapper(run_tree: &Path) -> Result<()> {
    let source = project_root()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("scripts")
        .join("perl-core")
        .join("t-perl-wrapper.sh");
    let destination = run_tree.join("t").join("perl");
    let context =
        format!("installing t/perl wrapper {} -> {}", source.display(), destination.display());
    fs::copy(&source, &destination).context(context)?;
    set_executable(&destination)?;
    Ok(())
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)
        .with_context(|| format!("reading permissions for {}", path.display()))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
        .with_context(|| format!("setting executable bit on {}", path.display()))
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<()> {
    Ok(())
}

fn resolve_runner_binary(configured: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = configured {
        if !path.is_file() {
            bail!("runner binary does not exist: {}", path.display());
        }
        let context = format!("canonicalizing {}", path.display());
        return path.canonicalize().context(context);
    }

    let root = project_root().unwrap_or_else(|_| PathBuf::from("."));
    let binary_name =
        if cfg!(windows) { "perl-core-test-runner.exe" } else { "perl-core-test-runner" };
    let binary = root.join("target").join("agent").join(binary_name);
    if binary.is_file() {
        let context = format!("canonicalizing {}", binary.display());
        return binary.canonicalize().context(context);
    }

    let status = Command::new("cargo")
        .current_dir(&root)
        .args(["build", "-p", "perl-core-test-runner", "--profile", "agent", "--locked"])
        .status()
        .context("building perl-core-test-runner")?;
    if !status.success() {
        bail!("cargo build -p perl-core-test-runner failed with status {status}");
    }
    if !binary.is_file() {
        bail!("runner build succeeded but binary was not found at {}", binary.display());
    }
    let context = format!("canonicalizing {}", binary.display());
    binary.canonicalize().context(context)
}

fn read_runner_records(path: &Path) -> Result<Vec<RunnerRecord>> {
    if !path.is_file() {
        bail!("runner context was not written: {}", path.display());
    }
    let raw = fs::read_to_string(path)
        .with_context(|| format!("reading runner context {}", path.display()))?;
    let mut records = Vec::new();
    for (index, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let record = serde_json::from_str(trimmed).with_context(|| {
            format!("decoding runner record {} in {}", index + 1, path.display())
        })?;
        records.push(record);
    }
    if records.is_empty() {
        bail!("runner context contained no records: {}", path.display());
    }
    Ok(records)
}

fn read_runner_records_or_empty(path: &Path) -> Result<Vec<RunnerRecord>> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(path)
        .with_context(|| format!("reading runner context {}", path.display()))?;
    if raw.lines().all(|line| line.trim().is_empty()) {
        return Ok(Vec::new());
    }
    read_runner_records(path)
}

struct BuildRunReportInput<'a> {
    config: &'a RunConfig,
    perl_tree: &'a Path,
    run_tree: &'a Path,
    discovered: &'a [DiscoveredTest],
    records: &'a [RunnerRecord],
    harness_status: Option<i32>,
}

fn build_run_report(input: BuildRunReportInput<'_>) -> RunReport {
    let records_by_path = input
        .records
        .iter()
        .filter_map(|record| normalize_test_path(&record.path).map(|path| (path, record)))
        .collect::<BTreeMap<_, _>>();
    let mut file_results = Vec::new();
    let mut failures = Vec::new();
    let mut buckets = BTreeMap::new();
    let mut assertions_total = 0usize;
    let mut assertions_passed = 0usize;

    for test in input.discovered {
        match records_by_path.get(&test.path) {
            Some(record) => {
                assertions_total = assertions_total.saturating_add(record.assertions_total);
                assertions_passed = assertions_passed.saturating_add(record.assertions_passed);
                file_results.push(RunFileResult {
                    path: test.path.clone(),
                    status: record.status,
                    assertions_passed: record.assertions_passed,
                    assertions_total: record.assertions_total,
                });
                if record.status == RunnerStatus::Fail {
                    let bucket = record.bucket.clone().unwrap_or_else(|| "unknown".to_string());
                    *buckets.entry(bucket.clone()).or_insert(0) += 1;
                    failures.push(failure_for_record(&test.path, &bucket, record));
                }
            }
            None => {
                assertions_total = assertions_total.saturating_add(1);
                file_results.push(RunFileResult {
                    path: test.path.clone(),
                    status: RunnerStatus::Fail,
                    assertions_passed: 0,
                    assertions_total: 1,
                });
                let bucket = "harness_prepare".to_string();
                *buckets.entry(bucket.clone()).or_insert(0) += 1;
                failures.push(RunFailure {
                    path: test.path.clone(),
                    phase: input.config.mode.as_str().to_string(),
                    bucket,
                    first_diagnostic: "test was discovered but produced no runner record"
                        .to_string(),
                    workstream: "harness_integration".to_string(),
                    lsp_impact: vec!["compiler_conformance".to_string()],
                });
            }
        }
    }

    let files_failed =
        file_results.iter().filter(|result| result.status == RunnerStatus::Fail).count();
    let files_total = file_results.len();
    let files_passed = files_total.saturating_sub(files_failed);

    RunReport {
        schema_version: RUN_REPORT_SCHEMA_VERSION.to_string(),
        commit: current_commit(),
        timestamp: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        perl_ref: perl_tree_ref(input.perl_tree),
        prepared_tree: input.perl_tree.display().to_string(),
        run_tree: input.run_tree.display().to_string(),
        host_perl: input.config.host_perl.display().to_string(),
        runner: input.config.runner,
        mode: input.config.mode,
        profile: input.config.profile,
        harness_status: input.harness_status,
        summary: RunSummary {
            files_total,
            files_passed,
            files_failed,
            tap_assertions_total: assertions_total,
            tap_assertions_passed: assertions_passed,
        },
        buckets,
        file_results,
        failures,
    }
}

fn failure_for_record(path: &str, bucket: &str, record: &RunnerRecord) -> RunFailure {
    RunFailure {
        path: path.to_string(),
        phase: record.mode.clone(),
        bucket: bucket.to_string(),
        first_diagnostic: record
            .first_diagnostic
            .clone()
            .unwrap_or_else(|| "runner reported failure without diagnostic".to_string()),
        workstream: workstream_for_bucket(bucket).to_string(),
        lsp_impact: lsp_impact_for_bucket(bucket).into_iter().map(ToString::to_string).collect(),
    }
}

fn current_commit() -> String {
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn perl_tree_ref(perl_tree: &Path) -> String {
    let Ok(top_level) = Command::new("git")
        .arg("-C")
        .arg(perl_tree)
        .args(["rev-parse", "--show-toplevel"])
        .output()
    else {
        return "unknown".to_string();
    };
    if !top_level.status.success() {
        return "unknown".to_string();
    }
    let Ok(top_level_text) = String::from_utf8(top_level.stdout) else {
        return "unknown".to_string();
    };
    let top_level_path = PathBuf::from(top_level_text.trim());
    let Ok(top_level_path) = top_level_path.canonicalize() else {
        return "unknown".to_string();
    };
    let Ok(perl_tree_path) = perl_tree.canonicalize() else {
        return "unknown".to_string();
    };
    if top_level_path != perl_tree_path {
        return "unknown".to_string();
    }

    Command::new("git")
        .arg("-C")
        .arg(perl_tree)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult<T = ()> = Result<T>;

    #[test]
    fn parses_dumptests_paths_and_ignores_noise() -> TestResult {
        let output = b"base/if.t\n# note from harness\n./base/lex.t\n t/base/term.t \n";

        let tests = parse_dumptests_output(output)?;

        assert_eq!(
            tests,
            vec![
                DiscoveredTest { path: "base/if.t".into(), root: "base".into() },
                DiscoveredTest { path: "base/lex.t".into(), root: "base".into() },
                DiscoveredTest { path: "base/term.t".into(), root: "base".into() },
            ]
        );
        Ok(())
    }

    #[test]
    fn parses_windows_style_paths() -> TestResult {
        let tests = parse_dumptests_output(b"base\\if.t\n")?;

        assert_eq!(tests[0].path, "base/if.t");
        assert_eq!(tests[0].root, "base");
        Ok(())
    }

    #[test]
    fn deduplicates_normalized_dumptests_paths() -> TestResult {
        let output = b"base/if.t\n./base/if.t\nt/base/if.t\nbase\\if.t\nbase/lex.t\n";

        let tests = parse_dumptests_output(output)?;

        assert_eq!(
            tests,
            vec![
                DiscoveredTest { path: "base/if.t".into(), root: "base".into() },
                DiscoveredTest { path: "base/lex.t".into(), root: "base".into() },
            ]
        );
        Ok(())
    }

    #[test]
    fn rejects_empty_dumptests_output() -> TestResult {
        let Err(err) = parse_dumptests_output(b"no tests here\n") else {
            bail!("empty output should fail");
        };

        assert!(err.to_string().contains("no .t files"));
        Ok(())
    }

    #[test]
    fn runner_names_match_upstream_scripts_and_receipt_values() {
        assert_eq!(HarnessRunner::Test.script_name(), "TEST");
        assert_eq!(HarnessRunner::Test.as_str(), "test");
        assert_eq!(HarnessRunner::Harness.script_name(), "harness");
        assert_eq!(HarnessRunner::Harness.as_str(), "harness");
    }

    #[test]
    fn mode_and_profile_names_match_cli_and_receipt_values() {
        let modes = [
            (HarnessMode::Parse, "parse"),
            (HarnessMode::Compile, "compile"),
            (HarnessMode::Execute, "execute"),
        ];
        for (mode, expected) in modes {
            assert_eq!(mode.as_str(), expected);
            assert_eq!(mode.to_string(), expected);
        }

        let profiles = [
            (HarnessProfile::Base, "base"),
            (HarnessProfile::Comp, "comp"),
            (HarnessProfile::Run, "run"),
            (HarnessProfile::Core, "core"),
            (HarnessProfile::Lib, "lib"),
            (HarnessProfile::Full, "full"),
        ];
        for (profile, expected) in profiles {
            assert_eq!(profile.as_str(), expected);
            assert_eq!(profile.to_string(), expected);
            assert!(!profile.roots().is_empty(), "{expected} profile should have roots");
        }
    }

    #[test]
    fn profile_base_expands_test_runner_args_to_explicit_files() -> TestResult {
        let temp = tempfile::tempdir()?;
        let t_dir = temp.path().join("t");
        fs::create_dir_all(t_dir.join("base").join("nested"))?;
        fs::write(t_dir.join("base").join("ok.t"), "1;\n")?;
        fs::write(t_dir.join("base").join("nested").join("deep.t"), "1;\n")?;
        fs::write(t_dir.join("base").join("README"), "not a test\n")?;

        let args = profile_runner_args(HarnessProfile::Base, &t_dir, HarnessRunner::Test)?;

        assert_eq!(args, vec!["base/nested/deep.t", "base/ok.t"]);
        Ok(())
    }

    #[test]
    fn profile_base_uses_glob_for_tap_harness_runner() -> TestResult {
        let temp = tempfile::tempdir()?;
        let args = profile_runner_args(
            HarnessProfile::Base,
            &temp.path().join("t"),
            HarnessRunner::Harness,
        )?;

        assert_eq!(args, vec!["base/*.t"]);
        Ok(())
    }

    #[test]
    fn execute_mode_requires_explicit_if_test_selection() -> TestResult {
        let config = RunConfig {
            perl_tree: PathBuf::from("unused"),
            host_perl: PathBuf::from("perl"),
            runner: HarnessRunner::Test,
            mode: HarnessMode::Execute,
            profile: HarnessProfile::Base,
            tests: Vec::new(),
            output: None,
            runner_binary: None,
        };

        let Err(err) = run_mode(config) else {
            bail!("execute mode without a selected test should fail");
        };

        assert!(err.to_string().contains("requires exactly --test base/if.t"));
        Ok(())
    }

    #[test]
    fn execute_mode_rejects_non_allowlisted_test_selection() -> TestResult {
        let config = RunConfig {
            perl_tree: PathBuf::from("unused"),
            host_perl: PathBuf::from("perl"),
            runner: HarnessRunner::Test,
            mode: HarnessMode::Execute,
            profile: HarnessProfile::Base,
            tests: vec!["base/while.t".into()],
            output: None,
            runner_binary: None,
        };

        let Err(err) = run_mode(config) else {
            bail!("execute mode should reject non-allowlisted tests");
        };

        assert!(err.to_string().contains("requires exactly --test base/if.t"));
        Ok(())
    }

    #[test]
    fn run_report_buckets_runner_records_and_missing_files() -> TestResult {
        let temp = tempfile::tempdir()?;
        let config = RunConfig {
            perl_tree: temp.path().join("perl"),
            host_perl: PathBuf::from("perl"),
            runner: HarnessRunner::Test,
            mode: HarnessMode::Parse,
            profile: HarnessProfile::Base,
            tests: Vec::new(),
            output: None,
            runner_binary: Some(PathBuf::from("runner")),
        };
        let discovered = vec![
            DiscoveredTest { path: "base/ok.t".into(), root: "base".into() },
            DiscoveredTest { path: "base/bad.t".into(), root: "base".into() },
            DiscoveredTest { path: "base/missing.t".into(), root: "base".into() },
        ];
        let records = vec![
            RunnerRecord {
                schema_version: "perl_core_harness.runner_record.v1".into(),
                mode: "parse".into(),
                path: "base/ok.t".into(),
                status: RunnerStatus::Pass,
                assertions_passed: 1,
                assertions_total: 1,
                bucket: None,
                first_diagnostic: None,
            },
            RunnerRecord {
                schema_version: "perl_core_harness.runner_record.v1".into(),
                mode: "parse".into(),
                path: "base/bad.t".into(),
                status: RunnerStatus::Fail,
                assertions_passed: 0,
                assertions_total: 1,
                bucket: Some("parse_recovery".into()),
                first_diagnostic: Some("expected expression".into()),
            },
        ];
        let run_tree = temp.path().join("run");

        let report = build_run_report(BuildRunReportInput {
            config: &config,
            perl_tree: temp.path(),
            run_tree: &run_tree,
            discovered: &discovered,
            records: &records,
            harness_status: Some(1),
        });

        assert_eq!(report.summary.files_total, 3);
        assert_eq!(report.summary.files_passed, 1);
        assert_eq!(report.summary.files_failed, 2);
        assert_eq!(report.buckets.get("parse_recovery"), Some(&1));
        assert_eq!(report.buckets.get("harness_prepare"), Some(&1));
        assert_eq!(report.failures.len(), 2);
        assert!(report.failures.iter().any(|failure| failure.path == "base/bad.t"));
        assert!(report.failures.iter().any(|failure| failure.path == "base/missing.t"));
        Ok(())
    }

    #[test]
    fn discovery_report_schema_roundtrips() -> TestResult {
        let report = DiscoveryReport {
            schema_version: DISCOVERY_SCHEMA_VERSION.into(),
            commit: "abc".into(),
            timestamp: "2026-07-02T00:00:00Z".into(),
            perl_ref: "perl-ref".into(),
            prepared_tree: "/tmp/perl".into(),
            host_perl: "perl".into(),
            runner: HarnessRunner::Test,
            profile: HarnessProfile::Base,
            tests: vec![DiscoveredTest { path: "base/if.t".into(), root: "base".into() }],
        };

        let json = serde_json::to_string(&report)?;
        let back: DiscoveryReport = serde_json::from_str(&json)?;

        assert_eq!(back, report);
        Ok(())
    }

    #[test]
    fn compile_run_report_schema_roundtrips() -> TestResult {
        let report = RunReport {
            schema_version: RUN_REPORT_SCHEMA_VERSION.into(),
            commit: "abc".into(),
            timestamp: "2026-07-02T00:00:00Z".into(),
            perl_ref: "perl-ref".into(),
            prepared_tree: "/tmp/perl".into(),
            run_tree: "/tmp/run".into(),
            host_perl: "perl".into(),
            runner: HarnessRunner::Test,
            mode: HarnessMode::Compile,
            profile: HarnessProfile::Base,
            harness_status: Some(0),
            summary: RunSummary {
                files_total: 1,
                files_passed: 1,
                files_failed: 0,
                tap_assertions_total: 1,
                tap_assertions_passed: 1,
            },
            buckets: BTreeMap::new(),
            file_results: vec![RunFileResult {
                path: "base/ok.t".into(),
                status: RunnerStatus::Pass,
                assertions_passed: 1,
                assertions_total: 1,
            }],
            failures: Vec::new(),
        };

        let json = serde_json::to_string(&report)?;
        let back: RunReport = serde_json::from_str(&json)?;

        assert_eq!(back, report);
        Ok(())
    }

    #[test]
    fn smoke_report_schema_roundtrips() -> TestResult {
        let discovery = sample_discovery_report();
        let parse = sample_parse_report();
        let compile = sample_compile_report();
        let smoke = build_smoke_report(BuildSmokeReportInput {
            config: &sample_smoke_config(vec![HarnessMode::Parse, HarnessMode::Compile]),
            discovery: &discovery,
            modes: &[HarnessMode::Parse, HarnessMode::Compile],
            discovery_path: Path::new("target/perl-core/smoke/base/discovery.json"),
            parse_path: Some(Path::new("target/perl-core/smoke/base/parse.json")),
            parse_report: Some(&parse),
            compile_path: Some(Path::new("target/perl-core/smoke/base/compile.json")),
            compile_report: Some(&compile),
            gap_map_path: Path::new("target/perl-core/smoke/base/gap-map.json"),
        });

        let json = serde_json::to_string(&smoke)?;
        let back: SmokeReport = serde_json::from_str(&json)?;

        assert_eq!(back, smoke);
        assert_eq!(back.schema_version, SMOKE_SCHEMA_VERSION);
        assert_eq!(back.status, SmokeStatus::Pass);
        assert_eq!(back.discovery_total, 2);
        assert_eq!(back.parse_files_total, Some(2));
        assert_eq!(back.compile_files_total, Some(2));
        Ok(())
    }

    #[test]
    fn smoke_summary_records_bucketed_parse_failures_without_structural_failure() -> TestResult {
        let discovery = sample_discovery_report();
        let mut parse = sample_parse_report();
        mark_file_failed(&mut parse, "base/ok.t", "parse_recovery");

        let smoke = build_smoke_report(BuildSmokeReportInput {
            config: &sample_smoke_config(vec![HarnessMode::Parse]),
            discovery: &discovery,
            modes: &[HarnessMode::Parse],
            discovery_path: Path::new("discovery.json"),
            parse_path: Some(Path::new("parse.json")),
            parse_report: Some(&parse),
            compile_path: None,
            compile_report: None,
            gap_map_path: Path::new("gap-map.json"),
        });

        assert_eq!(smoke.status, SmokeStatus::Pass);
        assert_eq!(smoke.parse_files_failed, Some(1));
        assert_eq!(
            smoke.parse_buckets.as_ref().and_then(|buckets| buckets.get("parse_recovery")),
            Some(&1)
        );
        assert!(smoke.structural_failures.is_empty());
        Ok(())
    }

    #[test]
    fn smoke_summary_records_bucketed_compile_failures_without_structural_failure() -> TestResult {
        let discovery = sample_discovery_report();
        let mut compile = sample_compile_report();
        mark_file_failed(&mut compile, "base/lex.t", "compile_effect");

        let smoke = build_smoke_report(BuildSmokeReportInput {
            config: &sample_smoke_config(vec![HarnessMode::Compile]),
            discovery: &discovery,
            modes: &[HarnessMode::Compile],
            discovery_path: Path::new("discovery.json"),
            parse_path: None,
            parse_report: None,
            compile_path: Some(Path::new("compile.json")),
            compile_report: Some(&compile),
            gap_map_path: Path::new("gap-map.json"),
        });

        assert_eq!(smoke.status, SmokeStatus::Pass);
        assert_eq!(smoke.compile_files_failed, Some(1));
        assert_eq!(
            smoke.compile_buckets.as_ref().and_then(|buckets| buckets.get("compile_effect")),
            Some(&1)
        );
        assert!(smoke.structural_failures.is_empty());
        Ok(())
    }

    #[test]
    fn smoke_summary_fails_on_unknown_bucket() -> TestResult {
        let discovery = sample_discovery_report();
        let mut compile = sample_compile_report();
        mark_file_failed(&mut compile, "base/lex.t", "unknown");

        let smoke = build_smoke_report(BuildSmokeReportInput {
            config: &sample_smoke_config(vec![HarnessMode::Compile]),
            discovery: &discovery,
            modes: &[HarnessMode::Compile],
            discovery_path: Path::new("discovery.json"),
            parse_path: None,
            parse_report: None,
            compile_path: Some(Path::new("compile.json")),
            compile_report: Some(&compile),
            gap_map_path: Path::new("gap-map.json"),
        });

        assert_eq!(smoke.status, SmokeStatus::Fail);
        assert!(smoke.structural_failures.iter().any(|failure| {
            failure.kind == SmokeFailureKind::UnknownBucket
                && failure.mode == Some(HarnessMode::Compile)
                && failure.path.as_deref() == Some("base/lex.t")
        }));
        Ok(())
    }

    #[test]
    fn smoke_summary_fails_on_unbucketed_failure() -> TestResult {
        let discovery = sample_discovery_report();
        let mut compile = sample_compile_report();
        let Some(result) =
            compile.file_results.iter_mut().find(|result| result.path == "base/lex.t")
        else {
            bail!("sample report missing base/lex.t");
        };
        result.status = RunnerStatus::Fail;
        result.assertions_passed = 0;
        compile.summary.files_passed = 1;
        compile.summary.files_failed = 1;
        compile.summary.tap_assertions_passed = 1;

        let smoke = build_smoke_report(BuildSmokeReportInput {
            config: &sample_smoke_config(vec![HarnessMode::Compile]),
            discovery: &discovery,
            modes: &[HarnessMode::Compile],
            discovery_path: Path::new("discovery.json"),
            parse_path: None,
            parse_report: None,
            compile_path: Some(Path::new("compile.json")),
            compile_report: Some(&compile),
            gap_map_path: Path::new("gap-map.json"),
        });

        assert_eq!(smoke.status, SmokeStatus::Fail);
        assert!(smoke.structural_failures.iter().any(|failure| {
            failure.kind == SmokeFailureKind::UnbucketedFailure
                && failure.path.as_deref() == Some("base/lex.t")
        }));
        Ok(())
    }

    #[test]
    fn smoke_summary_fails_when_requested_report_is_missing() -> TestResult {
        let discovery = sample_discovery_report();
        let smoke = build_smoke_report(BuildSmokeReportInput {
            config: &sample_smoke_config(vec![HarnessMode::Compile]),
            discovery: &discovery,
            modes: &[HarnessMode::Compile],
            discovery_path: Path::new("discovery.json"),
            parse_path: None,
            parse_report: None,
            compile_path: None,
            compile_report: None,
            gap_map_path: Path::new("gap-map.json"),
        });

        assert_eq!(smoke.status, SmokeStatus::Fail);
        assert!(smoke.structural_failures.iter().any(|failure| {
            failure.kind == SmokeFailureKind::MissingReport
                && failure.mode == Some(HarnessMode::Compile)
        }));
        Ok(())
    }

    #[test]
    fn compile_baseline_schema_roundtrips() -> TestResult {
        let baseline = baseline_from_report(&sample_compile_report())?;

        let json = serde_json::to_string(&baseline)?;
        let back: CompileBaseline = serde_json::from_str(&json)?;

        assert_eq!(back, baseline);
        Ok(())
    }

    #[test]
    fn identical_compile_report_passes_baseline() -> TestResult {
        let report = sample_compile_report();
        let baseline = baseline_from_report(&report)?;

        let comparison = compare_baseline(&baseline, &report);

        assert!(comparison.is_clean(), "identical report should pass: {comparison:?}");
        Ok(())
    }

    #[test]
    fn compile_baseline_file_results_are_order_independent() -> TestResult {
        let mut report = sample_compile_report();
        let baseline = baseline_from_report(&report)?;
        report.file_results.reverse();

        let comparison = compare_baseline(&baseline, &report);

        assert!(comparison.is_clean(), "reordered file results should pass: {comparison:?}");
        Ok(())
    }

    #[test]
    fn compile_baseline_fails_when_previously_passing_file_fails() -> TestResult {
        let mut report = sample_compile_report();
        let baseline = baseline_from_report(&report)?;
        mark_file_failed(&mut report, "base/ok.t", "parse_recovery");

        let comparison = compare_baseline(&baseline, &report);

        assert_violation(&comparison, BaselineViolationKind::PreviouslyPassingFileFailed);
        Ok(())
    }

    #[test]
    fn compile_baseline_fails_on_unexpected_new_failure() -> TestResult {
        let mut report = sample_compile_report();
        let baseline = baseline_from_report(&report)?;
        report.file_results.push(RunFileResult {
            path: "base/new.t".into(),
            status: RunnerStatus::Fail,
            assertions_passed: 0,
            assertions_total: 1,
        });
        report.failures.push(sample_failure("base/new.t", "parse_recovery"));
        report.buckets.insert("parse_recovery".into(), 1);
        report.summary.files_total = 3;
        report.summary.files_failed = 1;
        report.summary.files_passed = 2;
        report.summary.tap_assertions_total = 3;
        report.summary.tap_assertions_passed = 2;

        let comparison = compare_baseline(&baseline, &report);

        assert_violation(&comparison, BaselineViolationKind::UnexpectedNewFailure);
        Ok(())
    }

    #[test]
    fn compile_baseline_fails_on_unknown_bucket() -> TestResult {
        let mut report = sample_compile_report();
        let baseline = baseline_from_report(&report)?;
        mark_file_failed(&mut report, "base/ok.t", "unknown");

        let comparison = compare_baseline(&baseline, &report);

        assert_violation(&comparison, BaselineViolationKind::UnknownBucket);
        Ok(())
    }

    #[test]
    fn compile_baseline_fails_on_unbucketed_failure() -> TestResult {
        let mut report = sample_compile_report();
        let baseline = baseline_from_report(&report)?;
        let Some(result) = report.file_results.iter_mut().find(|result| result.path == "base/ok.t")
        else {
            bail!("sample report missing base/ok.t");
        };
        result.status = RunnerStatus::Fail;
        result.assertions_passed = 0;
        report.summary.files_passed = 1;
        report.summary.files_failed = 1;
        report.summary.tap_assertions_passed = 1;

        let comparison = compare_baseline(&baseline, &report);

        assert_violation(&comparison, BaselineViolationKind::UnbucketedFailure);
        Ok(())
    }

    #[test]
    fn compile_baseline_fails_when_bucket_count_increases() -> TestResult {
        let mut baseline_report = sample_compile_report();
        mark_file_failed(&mut baseline_report, "base/ok.t", "parse_recovery");
        let baseline = baseline_from_report(&baseline_report)?;
        let mut current_report = baseline_report.clone();
        current_report.file_results.push(RunFileResult {
            path: "base/new.t".into(),
            status: RunnerStatus::Fail,
            assertions_passed: 0,
            assertions_total: 1,
        });
        current_report.failures.push(sample_failure("base/new.t", "parse_recovery"));
        current_report.buckets.insert("parse_recovery".into(), 2);
        current_report.summary.files_total = 3;
        current_report.summary.files_passed = 1;
        current_report.summary.files_failed = 2;
        current_report.summary.tap_assertions_total = 3;
        current_report.summary.tap_assertions_passed = 1;

        let comparison = compare_baseline(&baseline, &current_report);

        assert_violation(&comparison, BaselineViolationKind::BucketCountIncreased);
        Ok(())
    }

    #[test]
    fn compile_baseline_fails_on_assertion_regression() -> TestResult {
        let mut report = sample_compile_report();
        let baseline = baseline_from_report(&report)?;
        let Some(result) = report.file_results.iter_mut().find(|result| result.path == "base/ok.t")
        else {
            bail!("sample report missing base/ok.t");
        };
        result.assertions_passed = 0;
        report.summary.tap_assertions_passed = 1;

        let comparison = compare_baseline(&baseline, &report);

        assert_violation(&comparison, BaselineViolationKind::AssertionRegression);
        Ok(())
    }

    #[test]
    fn compile_baseline_fails_on_mode_and_profile_mismatch() -> TestResult {
        let mut report = sample_compile_report();
        let mut baseline = baseline_from_report(&report)?;
        baseline.mode = HarnessMode::Parse;
        baseline.profile = HarnessProfile::Comp;
        report.mode = HarnessMode::Compile;
        report.profile = HarnessProfile::Base;

        let comparison = compare_baseline(&baseline, &report);

        assert_violation(&comparison, BaselineViolationKind::ModeMismatch);
        assert_violation(&comparison, BaselineViolationKind::ProfileMismatch);
        Ok(())
    }

    #[test]
    fn compile_baseline_fails_when_expected_file_is_missing() -> TestResult {
        let mut report = sample_compile_report();
        let baseline = baseline_from_report(&report)?;
        report.file_results.retain(|result| result.path != "base/ok.t");
        report.summary.files_total = 1;
        report.summary.files_passed = 1;
        report.summary.tap_assertions_total = 1;
        report.summary.tap_assertions_passed = 1;

        let comparison = compare_baseline(&baseline, &report);

        assert_violation(&comparison, BaselineViolationKind::MissingExpectedFile);
        Ok(())
    }

    #[test]
    fn compile_baseline_accept_writes_deterministic_sorted_json() -> TestResult {
        let temp = tempfile::tempdir()?;
        let report_path = temp.path().join("report.json");
        let baseline_path = temp.path().join("baseline.json");
        let mut report = sample_compile_report();
        report.file_results.reverse();
        write_run_report(&report_path, &report)?;

        baseline(BaselineConfig {
            mode: HarnessMode::Compile,
            profile: HarnessProfile::Base,
            report: Some(report_path),
            baseline: Some(baseline_path.clone()),
            accept: true,
        })?;

        let raw = fs::read_to_string(&baseline_path)?;
        let accepted: CompileBaseline = serde_json::from_str(&raw)?;
        let paths =
            accepted.file_results.iter().map(|result| result.path.as_str()).collect::<Vec<_>>();
        assert_eq!(paths, vec!["base/lex.t", "base/ok.t"]);
        assert!(raw.ends_with('\n'));
        Ok(())
    }

    #[test]
    fn generated_two_file_compile_report_passes_checked_in_baseline() -> TestResult {
        let root = project_root()?;
        let baseline = read_compile_baseline(
            &root.join(".ci").join("perl-core-harness").join("base-compile-baseline.json"),
        )?;
        let report = sample_compile_report();

        let comparison = compare_baseline(&baseline, &report);

        assert!(
            comparison.is_clean(),
            "checked-in baseline should match fixture report: {comparison:?}"
        );
        Ok(())
    }

    #[test]
    fn non_git_perl_tree_has_unknown_ref() -> TestResult {
        let dir = tempfile::tempdir()?;

        assert_eq!(perl_tree_ref(dir.path()), "unknown");
        Ok(())
    }

    #[test]
    fn default_paths_match_receipt_layout() {
        assert!(
            default_discovery_path(HarnessProfile::Base)
                .ends_with("target/perl-core/discovery/base.json")
        );
        assert!(
            default_run_report_path(HarnessMode::Parse, HarnessProfile::Base)
                .ends_with("target/perl-core/reports/base-parse.json")
        );
        assert!(default_smoke_dir(HarnessProfile::Comp).ends_with("target/perl-core/smoke/comp"));
    }

    #[test]
    fn validate_runner_script_rejects_missing_tree_layouts() -> TestResult {
        let temp = tempfile::tempdir()?;
        let missing_t_dir = temp.path().join("missing-t");
        let Err(err) = validate_runner_script(&missing_t_dir, HarnessRunner::Test) else {
            bail!("missing t directory should fail");
        };
        assert!(err.to_string().contains("missing t/ directory"));

        let t_dir = temp.path().join("t");
        fs::create_dir_all(&t_dir)?;
        let Err(err) = validate_runner_script(&t_dir, HarnessRunner::Test) else {
            bail!("missing TEST script should fail");
        };
        assert!(err.to_string().contains("missing t/TEST"));
        Ok(())
    }

    #[test]
    fn configured_runner_binary_must_exist() -> TestResult {
        let temp = tempfile::tempdir()?;
        let missing = temp.path().join("missing-runner");

        let Err(err) = resolve_runner_binary(Some(&missing)) else {
            bail!("missing configured runner should fail");
        };

        assert!(err.to_string().contains("runner binary does not exist"));
        Ok(())
    }

    #[test]
    fn configured_runner_binary_returns_canonical_path() -> TestResult {
        let temp = tempfile::tempdir()?;
        let runner = temp.path().join("runner");
        fs::write(&runner, "")?;

        let resolved = resolve_runner_binary(Some(&runner))?;

        assert_eq!(resolved, runner.canonicalize()?);
        Ok(())
    }

    #[test]
    fn default_runner_binary_uses_existing_agent_binary_without_building() -> TestResult {
        let root = project_root()?;
        let binary_name =
            if cfg!(windows) { "perl-core-test-runner.exe" } else { "perl-core-test-runner" };
        let binary = root.join("target").join("agent").join(binary_name);
        let created = if binary.is_file() {
            false
        } else {
            if let Some(parent) = binary.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&binary, "")?;
            true
        };

        let expected = binary.canonicalize()?;
        let resolved = resolve_runner_binary(None);
        if created {
            fs::remove_file(&binary)?;
        }
        let resolved = resolved?;

        assert_eq!(resolved, expected);
        Ok(())
    }

    #[test]
    fn runner_context_reader_rejects_missing_empty_and_invalid_records() -> TestResult {
        let temp = tempfile::tempdir()?;
        let context = temp.path().join("records.jsonl");

        let Err(err) = read_runner_records(&context) else {
            bail!("missing runner context should fail");
        };
        assert!(err.to_string().contains("runner context was not written"));

        fs::write(&context, "\n")?;
        let Err(err) = read_runner_records(&context) else {
            bail!("empty runner context should fail");
        };
        assert!(err.to_string().contains("runner context contained no records"));

        fs::write(&context, "{not-json}\n")?;
        let Err(err) = read_runner_records(&context) else {
            bail!("invalid runner context should fail");
        };
        assert!(err.to_string().contains("decoding runner record 1"));
        Ok(())
    }

    #[test]
    fn bucket_metadata_maps_failure_classes() {
        assert_eq!(workstream_for_bucket("source_decode"), "source_loading");
        assert_eq!(workstream_for_bucket("hir_lowering"), "hir");
        assert_eq!(workstream_for_bucket("compile_effect"), "compile_time_effects");
        assert_eq!(workstream_for_bucket("scope_pad"), "scope_and_pad");
        assert_eq!(workstream_for_bucket("package_stash"), "package_stash");
        assert_eq!(workstream_for_bucket("pragma_feature"), "pragma_model");
        assert_eq!(workstream_for_bucket("module_resolution"), "module_resolution");
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
        assert_eq!(lsp_impact_for_bucket("cli_switch"), vec!["compiler_conformance"]);
        assert_eq!(lsp_impact_for_bucket("harness_prepare"), vec!["compiler_conformance"]);
        assert_eq!(lsp_impact_for_bucket("unknown_bucket"), vec!["compiler_conformance"]);
    }

    #[test]
    fn report_writers_create_parent_directories() -> TestResult {
        let temp = tempfile::tempdir()?;
        let discovery_path = temp.path().join("nested").join("discovery").join("base.json");
        let run_path = temp.path().join("nested").join("reports").join("base-parse.json");

        let discovery = DiscoveryReport {
            schema_version: DISCOVERY_SCHEMA_VERSION.into(),
            commit: "abc".into(),
            timestamp: "2026-07-02T00:00:00Z".into(),
            perl_ref: "unknown".into(),
            prepared_tree: "/tmp/perl".into(),
            host_perl: "perl".into(),
            runner: HarnessRunner::Test,
            profile: HarnessProfile::Base,
            tests: vec![DiscoveredTest { path: "base/if.t".into(), root: "base".into() }],
        };
        write_discovery_report(&discovery_path, &discovery)?;
        assert!(discovery_path.is_file());

        let config = RunConfig {
            perl_tree: temp.path().join("prepared"),
            host_perl: PathBuf::from("perl"),
            runner: HarnessRunner::Test,
            mode: HarnessMode::Parse,
            profile: HarnessProfile::Base,
            tests: Vec::new(),
            output: Some(run_path.clone()),
            runner_binary: Some(PathBuf::from("runner")),
        };
        let discovered = vec![DiscoveredTest { path: "base/if.t".into(), root: "base".into() }];
        let records = vec![RunnerRecord {
            schema_version: "perl_core_harness.runner_record.v1".into(),
            mode: "parse".into(),
            path: "base/if.t".into(),
            status: RunnerStatus::Pass,
            assertions_passed: 1,
            assertions_total: 1,
            bucket: None,
            first_diagnostic: None,
        }];
        let run_tree = temp.path().join("run");
        let report = build_run_report(BuildRunReportInput {
            config: &config,
            perl_tree: temp.path(),
            run_tree: &run_tree,
            discovered: &discovered,
            records: &records,
            harness_status: Some(0),
        });
        write_run_report(&run_path, &report)?;
        assert!(run_path.is_file());
        Ok(())
    }

    fn sample_compile_report() -> RunReport {
        RunReport {
            schema_version: RUN_REPORT_SCHEMA_VERSION.into(),
            commit: "abc".into(),
            timestamp: "2026-07-02T00:00:00Z".into(),
            perl_ref: "unknown".into(),
            prepared_tree: "/tmp/perl".into(),
            run_tree: "/tmp/run".into(),
            host_perl: "perl".into(),
            runner: HarnessRunner::Test,
            mode: HarnessMode::Compile,
            profile: HarnessProfile::Base,
            harness_status: Some(0),
            summary: RunSummary {
                files_total: 2,
                files_passed: 2,
                files_failed: 0,
                tap_assertions_total: 2,
                tap_assertions_passed: 2,
            },
            buckets: BTreeMap::new(),
            file_results: vec![
                RunFileResult {
                    path: "base/lex.t".into(),
                    status: RunnerStatus::Pass,
                    assertions_passed: 1,
                    assertions_total: 1,
                },
                RunFileResult {
                    path: "base/ok.t".into(),
                    status: RunnerStatus::Pass,
                    assertions_passed: 1,
                    assertions_total: 1,
                },
            ],
            failures: Vec::new(),
        }
    }

    fn sample_parse_report() -> RunReport {
        let mut report = sample_compile_report();
        report.mode = HarnessMode::Parse;
        report
    }

    fn sample_discovery_report() -> DiscoveryReport {
        DiscoveryReport {
            schema_version: DISCOVERY_SCHEMA_VERSION.into(),
            commit: "abc".into(),
            timestamp: "2026-07-02T00:00:00Z".into(),
            perl_ref: "perl-ref".into(),
            prepared_tree: "/tmp/perl".into(),
            host_perl: "perl".into(),
            runner: HarnessRunner::Test,
            profile: HarnessProfile::Base,
            tests: vec![
                DiscoveredTest { path: "base/lex.t".into(), root: "base".into() },
                DiscoveredTest { path: "base/ok.t".into(), root: "base".into() },
            ],
        }
    }

    fn sample_smoke_config(modes: Vec<HarnessMode>) -> SmokeConfig {
        SmokeConfig {
            perl_tree: PathBuf::from("/tmp/perl"),
            host_perl: PathBuf::from("perl"),
            runner: HarnessRunner::Test,
            profile: HarnessProfile::Base,
            modes,
            output_dir: Some(PathBuf::from("target/perl-core/smoke")),
            runner_binary: Some(PathBuf::from("runner")),
            perl_ref: Some("perl-ref".into()),
        }
    }

    fn mark_file_failed(report: &mut RunReport, path: &str, bucket: &str) {
        if let Some(result) = report.file_results.iter_mut().find(|result| result.path == path) {
            result.status = RunnerStatus::Fail;
            result.assertions_passed = 0;
        }
        report.failures.push(sample_failure(path, bucket));
        report.buckets.insert(bucket.to_string(), 1);
        report.summary.files_passed = report.summary.files_passed.saturating_sub(1);
        report.summary.files_failed = report.summary.files_failed.saturating_add(1);
        report.summary.tap_assertions_passed =
            report.summary.tap_assertions_passed.saturating_sub(1);
    }

    fn sample_failure(path: &str, bucket: &str) -> RunFailure {
        RunFailure {
            path: path.to_string(),
            phase: "compile".into(),
            bucket: bucket.into(),
            first_diagnostic: "sample failure".into(),
            workstream: workstream_for_bucket(bucket).into(),
            lsp_impact: lsp_impact_for_bucket(bucket)
                .into_iter()
                .map(ToString::to_string)
                .collect(),
        }
    }

    fn assert_violation(comparison: &BaselineComparison, kind: BaselineViolationKind) {
        assert!(
            comparison.violations.iter().any(|violation| violation.kind == kind),
            "expected {kind:?} in {comparison:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn discover_invokes_dumptests_and_writes_manifest() -> TestResult {
        let temp = tempfile::tempdir()?;
        let perl_tree = write_fake_perl_tree(temp.path())?;
        let output = temp.path().join("discovery.json");

        discover(DiscoverConfig {
            perl_tree: perl_tree.clone(),
            host_perl: PathBuf::from("/bin/sh"),
            runner: HarnessRunner::Test,
            profile: HarnessProfile::Base,
            output: Some(output.clone()),
        })?;

        let raw = fs::read_to_string(output)?;
        let report: DiscoveryReport = serde_json::from_str(&raw)?;
        assert_eq!(report.schema_version, DISCOVERY_SCHEMA_VERSION);
        assert_eq!(report.runner, HarnessRunner::Test);
        assert_eq!(report.profile, HarnessProfile::Base);
        assert_eq!(report.prepared_tree, perl_tree.canonicalize()?.display().to_string());
        assert_eq!(report.host_perl, "/bin/sh");
        assert_eq!(
            report.tests,
            vec![DiscoveredTest { path: "base/ok.t".into(), root: "base".into() }]
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn run_mode_invokes_wrapper_and_writes_parse_report() -> TestResult {
        let temp = tempfile::tempdir()?;
        let perl_tree = write_fake_perl_tree(temp.path())?;
        let runner = write_fake_runner(temp.path(), RunnerStatus::Pass)?;
        let output = temp.path().join("parse-report.json");

        run_mode(RunConfig {
            perl_tree: perl_tree.clone(),
            host_perl: PathBuf::from("/bin/sh"),
            runner: HarnessRunner::Test,
            mode: HarnessMode::Parse,
            profile: HarnessProfile::Base,
            tests: Vec::new(),
            output: Some(output.clone()),
            runner_binary: Some(runner),
        })?;

        let raw = fs::read_to_string(output)?;
        let report: RunReport = serde_json::from_str(&raw)?;
        assert_eq!(report.summary.files_total, 1);
        assert_eq!(report.summary.files_passed, 1);
        assert_eq!(report.summary.files_failed, 0);
        assert_eq!(report.file_results[0].path, "base/ok.t");
        assert_eq!(report.file_results[0].status, RunnerStatus::Pass);
        assert!(!perl_tree.join("t").join("perl").exists(), "source Perl tree must not be mutated");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn run_mode_generated_fixture_smoke_runs_two_base_tests() -> TestResult {
        let temp = tempfile::tempdir()?;
        let perl_tree = write_fake_perl_tree_with_two_base_tests(temp.path())?;
        let runner = write_fake_runner(temp.path(), RunnerStatus::Pass)?;
        let output = temp.path().join("parse-report.json");

        run_mode(RunConfig {
            perl_tree: perl_tree.clone(),
            host_perl: PathBuf::from("/bin/sh"),
            runner: HarnessRunner::Test,
            mode: HarnessMode::Parse,
            profile: HarnessProfile::Base,
            tests: Vec::new(),
            output: Some(output.clone()),
            runner_binary: Some(runner),
        })?;

        let raw = fs::read_to_string(output)?;
        let report: RunReport = serde_json::from_str(&raw)?;
        assert_eq!(report.summary.files_total, 2);
        assert_eq!(report.summary.files_passed, 2);
        assert_eq!(report.summary.files_failed, 0);
        let mut paths =
            report.file_results.iter().map(|result| result.path.as_str()).collect::<Vec<_>>();
        paths.sort_unstable();
        assert_eq!(paths, vec!["base/lex.t", "base/ok.t"]);
        assert!(report.file_results.iter().all(|result| result.status == RunnerStatus::Pass));
        assert!(!perl_tree.join("t").join("perl").exists(), "source Perl tree must not be mutated");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn run_mode_installs_wrapper_before_run_copy_dumptests() -> TestResult {
        let temp = tempfile::tempdir()?;
        let perl_tree = write_fake_perl_tree_requiring_t_perl_for_dumptests(temp.path())?;
        let runner = write_fake_runner(temp.path(), RunnerStatus::Pass)?;
        let output = temp.path().join("parse-report.json");

        run_mode(RunConfig {
            perl_tree: perl_tree.clone(),
            host_perl: PathBuf::from("/bin/sh"),
            runner: HarnessRunner::Test,
            mode: HarnessMode::Parse,
            profile: HarnessProfile::Base,
            tests: Vec::new(),
            output: Some(output.clone()),
            runner_binary: Some(runner),
        })?;

        let raw = fs::read_to_string(output)?;
        let report: RunReport = serde_json::from_str(&raw)?;
        assert_eq!(report.summary.files_total, 1);
        assert_eq!(report.summary.files_passed, 1);
        assert!(!perl_tree.join("t").join("perl").exists(), "source Perl tree must not be mutated");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn run_mode_generated_fixture_smoke_runs_two_compile_tests() -> TestResult {
        let temp = tempfile::tempdir()?;
        let perl_tree = write_fake_perl_tree_with_two_base_tests(temp.path())?;
        let runner = write_fake_runner(temp.path(), RunnerStatus::Pass)?;
        let output = temp.path().join("compile-report.json");

        run_mode(RunConfig {
            perl_tree: perl_tree.clone(),
            host_perl: PathBuf::from("/bin/sh"),
            runner: HarnessRunner::Test,
            mode: HarnessMode::Compile,
            profile: HarnessProfile::Base,
            tests: Vec::new(),
            output: Some(output.clone()),
            runner_binary: Some(runner),
        })?;

        let raw = fs::read_to_string(output)?;
        let report: RunReport = serde_json::from_str(&raw)?;
        assert_eq!(report.mode, HarnessMode::Compile);
        assert_eq!(report.summary.files_total, 2);
        assert_eq!(report.summary.files_passed, 2);
        assert_eq!(report.summary.files_failed, 0);
        let mut paths =
            report.file_results.iter().map(|result| result.path.as_str()).collect::<Vec<_>>();
        paths.sort_unstable();
        assert_eq!(paths, vec!["base/lex.t", "base/ok.t"]);
        assert!(report.file_results.iter().all(|result| result.status == RunnerStatus::Pass));
        assert!(!perl_tree.join("t").join("perl").exists(), "source Perl tree must not be mutated");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn run_mode_execute_runs_selected_base_if_test() -> TestResult {
        let temp = tempfile::tempdir()?;
        let perl_tree = write_fake_perl_tree_with_base_if_test(temp.path())?;
        let runner = write_fake_execute_runner(temp.path())?;
        let output = temp.path().join("execute-report.json");

        run_mode(RunConfig {
            perl_tree: perl_tree.clone(),
            host_perl: PathBuf::from("/bin/sh"),
            runner: HarnessRunner::Test,
            mode: HarnessMode::Execute,
            profile: HarnessProfile::Base,
            tests: vec!["base/if.t".into()],
            output: Some(output.clone()),
            runner_binary: Some(runner),
        })?;

        let raw = fs::read_to_string(output)?;
        let report: RunReport = serde_json::from_str(&raw)?;
        assert_eq!(report.mode, HarnessMode::Execute);
        assert_eq!(report.summary.files_total, 1);
        assert_eq!(report.summary.files_passed, 1);
        assert_eq!(report.summary.files_failed, 0);
        assert_eq!(report.summary.tap_assertions_total, 2);
        assert_eq!(report.summary.tap_assertions_passed, 2);
        assert_eq!(report.file_results[0].path, "base/if.t");
        assert_eq!(report.file_results[0].assertions_total, 2);
        assert!(!perl_tree.join("t").join("perl").exists(), "source Perl tree must not be mutated");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn smoke_writes_discovery_parse_compile_and_summary_reports() -> TestResult {
        let temp = tempfile::tempdir()?;
        let perl_tree = write_fake_perl_tree_with_two_base_tests(temp.path())?;
        let runner = write_fake_runner(temp.path(), RunnerStatus::Pass)?;
        let output_dir = temp.path().join("smoke");

        smoke(SmokeConfig {
            perl_tree: perl_tree.clone(),
            host_perl: PathBuf::from("/bin/sh"),
            runner: HarnessRunner::Test,
            profile: HarnessProfile::Base,
            modes: vec![HarnessMode::Parse, HarnessMode::Compile],
            output_dir: Some(output_dir.clone()),
            runner_binary: Some(runner),
            perl_ref: Some("fake-ref".into()),
        })?;

        for file in ["discovery.json", "parse.json", "compile.json", "gap-map.json", "smoke.json"] {
            assert!(output_dir.join(file).is_file(), "{file} should be written");
        }
        let raw = fs::read_to_string(output_dir.join("smoke.json"))?;
        let report: SmokeReport = serde_json::from_str(&raw)?;
        assert_eq!(report.schema_version, SMOKE_SCHEMA_VERSION);
        assert_eq!(report.status, SmokeStatus::Pass);
        assert_eq!(report.discovery_total, 2);
        assert_eq!(report.parse_files_total, Some(2));
        assert_eq!(report.parse_files_passed, Some(2));
        assert_eq!(report.compile_files_total, Some(2));
        assert_eq!(report.compile_files_passed, Some(2));
        assert!(report.structural_failures.is_empty());
        assert!(!perl_tree.join("t").join("perl").exists(), "source Perl tree must not be mutated");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn smoke_writes_comp_profile_receipts() -> TestResult {
        let temp = tempfile::tempdir()?;
        let perl_tree = write_fake_perl_tree_with_two_comp_tests(temp.path())?;
        let runner = write_fake_runner(temp.path(), RunnerStatus::Pass)?;
        let output_dir = temp.path().join("smoke-comp");

        smoke(SmokeConfig {
            perl_tree: perl_tree.clone(),
            host_perl: PathBuf::from("/bin/sh"),
            runner: HarnessRunner::Test,
            profile: HarnessProfile::Comp,
            modes: vec![HarnessMode::Parse, HarnessMode::Compile],
            output_dir: Some(output_dir.clone()),
            runner_binary: Some(runner),
            perl_ref: Some("fake-ref".into()),
        })?;

        for file in ["discovery.json", "parse.json", "compile.json", "gap-map.json", "smoke.json"] {
            assert!(output_dir.join(file).is_file(), "{file} should be written");
        }

        let discovery: DiscoveryReport =
            serde_json::from_str(&fs::read_to_string(output_dir.join("discovery.json"))?)?;
        assert_eq!(discovery.profile, HarnessProfile::Comp);
        let mut discovered =
            discovery.tests.iter().map(|test| test.path.as_str()).collect::<Vec<_>>();
        discovered.sort_unstable();
        assert_eq!(discovered, vec!["comp/require.t", "comp/use.t"]);

        let smoke_report: SmokeReport =
            serde_json::from_str(&fs::read_to_string(output_dir.join("smoke.json"))?)?;
        assert_eq!(smoke_report.profile, HarnessProfile::Comp);
        assert_eq!(smoke_report.status, SmokeStatus::Pass);
        assert_eq!(smoke_report.discovery_total, 2);
        assert_eq!(smoke_report.parse_files_passed, Some(2));
        assert_eq!(smoke_report.compile_files_passed, Some(2));
        assert!(smoke_report.structural_failures.is_empty());
        assert!(!perl_tree.join("t").join("perl").exists(), "source Perl tree must not be mutated");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn smoke_writes_run_profile_receipts() -> TestResult {
        let temp = tempfile::tempdir()?;
        let perl_tree = write_fake_perl_tree_with_two_run_tests(temp.path())?;
        let runner = write_fake_runner(temp.path(), RunnerStatus::Pass)?;
        let output_dir = temp.path().join("smoke-run");

        smoke(SmokeConfig {
            perl_tree: perl_tree.clone(),
            host_perl: PathBuf::from("/bin/sh"),
            runner: HarnessRunner::Test,
            profile: HarnessProfile::Run,
            modes: vec![HarnessMode::Parse, HarnessMode::Compile],
            output_dir: Some(output_dir.clone()),
            runner_binary: Some(runner),
            perl_ref: Some("fake-ref".into()),
        })?;

        for file in ["discovery.json", "parse.json", "compile.json", "gap-map.json", "smoke.json"] {
            assert!(output_dir.join(file).is_file(), "{file} should be written");
        }

        let discovery: DiscoveryReport =
            serde_json::from_str(&fs::read_to_string(output_dir.join("discovery.json"))?)?;
        assert_eq!(discovery.profile, HarnessProfile::Run);
        let mut discovered =
            discovery.tests.iter().map(|test| test.path.as_str()).collect::<Vec<_>>();
        discovered.sort_unstable();
        assert_eq!(discovered, vec!["run/import.t", "run/switches.t"]);

        let smoke_report: SmokeReport =
            serde_json::from_str(&fs::read_to_string(output_dir.join("smoke.json"))?)?;
        assert_eq!(smoke_report.profile, HarnessProfile::Run);
        assert_eq!(smoke_report.status, SmokeStatus::Pass);
        assert_eq!(smoke_report.discovery_total, 2);
        assert_eq!(smoke_report.parse_files_passed, Some(2));
        assert_eq!(smoke_report.compile_files_passed, Some(2));
        assert!(smoke_report.structural_failures.is_empty());
        assert!(!perl_tree.join("t").join("perl").exists(), "source Perl tree must not be mutated");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn smoke_preserves_bucketed_parse_failures_as_gap_receipts() -> TestResult {
        let temp = tempfile::tempdir()?;
        let perl_tree = write_fake_perl_tree(temp.path())?;
        let runner = write_fake_runner(temp.path(), RunnerStatus::Fail)?;
        let output_dir = temp.path().join("smoke");

        smoke(SmokeConfig {
            perl_tree,
            host_perl: PathBuf::from("/bin/sh"),
            runner: HarnessRunner::Test,
            profile: HarnessProfile::Base,
            modes: vec![HarnessMode::Parse],
            output_dir: Some(output_dir.clone()),
            runner_binary: Some(runner),
            perl_ref: Some("fake-ref".into()),
        })?;

        let raw = fs::read_to_string(output_dir.join("smoke.json"))?;
        let report: SmokeReport = serde_json::from_str(&raw)?;
        assert_eq!(report.status, SmokeStatus::Pass);
        assert_eq!(report.parse_files_failed, Some(1));
        assert_eq!(
            report.parse_buckets.as_ref().and_then(|buckets| buckets.get("parse_recovery")),
            Some(&1)
        );
        assert!(output_dir.join("parse.json").is_file());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn smoke_preserves_bucketed_compile_failures_as_gap_receipts() -> TestResult {
        let temp = tempfile::tempdir()?;
        let perl_tree = write_fake_perl_tree(temp.path())?;
        let runner =
            write_fake_runner_with_bucket(temp.path(), RunnerStatus::Fail, Some("compile_effect"))?;
        let output_dir = temp.path().join("smoke");

        smoke(SmokeConfig {
            perl_tree,
            host_perl: PathBuf::from("/bin/sh"),
            runner: HarnessRunner::Test,
            profile: HarnessProfile::Base,
            modes: vec![HarnessMode::Compile],
            output_dir: Some(output_dir.clone()),
            runner_binary: Some(runner),
            perl_ref: Some("fake-ref".into()),
        })?;

        let raw = fs::read_to_string(output_dir.join("smoke.json"))?;
        let report: SmokeReport = serde_json::from_str(&raw)?;
        assert_eq!(report.status, SmokeStatus::Pass);
        assert_eq!(report.compile_files_failed, Some(1));
        assert_eq!(
            report.compile_buckets.as_ref().and_then(|buckets| buckets.get("compile_effect")),
            Some(&1)
        );
        assert!(output_dir.join("compile.json").is_file());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn smoke_fails_when_runner_emits_unknown_bucket() -> TestResult {
        let temp = tempfile::tempdir()?;
        let perl_tree = write_fake_perl_tree(temp.path())?;
        let runner =
            write_fake_runner_with_bucket(temp.path(), RunnerStatus::Fail, Some("unknown"))?;
        let output_dir = temp.path().join("smoke");

        let Err(err) = smoke(SmokeConfig {
            perl_tree,
            host_perl: PathBuf::from("/bin/sh"),
            runner: HarnessRunner::Test,
            profile: HarnessProfile::Base,
            modes: vec![HarnessMode::Compile],
            output_dir: Some(output_dir.clone()),
            runner_binary: Some(runner),
            perl_ref: Some("fake-ref".into()),
        }) else {
            bail!("unknown bucket should fail smoke receipt integrity");
        };

        assert!(err.to_string().contains("receipt integrity"));
        let raw = fs::read_to_string(output_dir.join("smoke.json"))?;
        let report: SmokeReport = serde_json::from_str(&raw)?;
        assert_eq!(report.status, SmokeStatus::Fail);
        assert!(
            report
                .structural_failures
                .iter()
                .any(|failure| failure.kind == SmokeFailureKind::UnknownBucket)
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn smoke_fails_when_runner_omits_bucket() -> TestResult {
        let temp = tempfile::tempdir()?;
        let perl_tree = write_fake_perl_tree(temp.path())?;
        let runner = write_fake_runner_with_bucket(temp.path(), RunnerStatus::Fail, None)?;
        let output_dir = temp.path().join("smoke");

        let Err(err) = smoke(SmokeConfig {
            perl_tree,
            host_perl: PathBuf::from("/bin/sh"),
            runner: HarnessRunner::Test,
            profile: HarnessProfile::Base,
            modes: vec![HarnessMode::Compile],
            output_dir: Some(output_dir.clone()),
            runner_binary: Some(runner),
            perl_ref: Some("fake-ref".into()),
        }) else {
            bail!("missing bucket should fail smoke receipt integrity");
        };

        assert!(err.to_string().contains("receipt integrity"));
        let raw = fs::read_to_string(output_dir.join("smoke.json"))?;
        let report: SmokeReport = serde_json::from_str(&raw)?;
        assert_eq!(report.status, SmokeStatus::Fail);
        assert!(
            report
                .structural_failures
                .iter()
                .any(|failure| failure.kind == SmokeFailureKind::UnknownBucket)
        );
        Ok(())
    }

    #[test]
    fn smoke_rejects_missing_prepared_tree() -> TestResult {
        let temp = tempfile::tempdir()?;

        let Err(err) = smoke(SmokeConfig {
            perl_tree: temp.path().join("missing"),
            host_perl: PathBuf::from("perl"),
            runner: HarnessRunner::Test,
            profile: HarnessProfile::Base,
            modes: vec![HarnessMode::Parse, HarnessMode::Compile],
            output_dir: Some(temp.path().join("smoke")),
            runner_binary: None,
            perl_ref: Some("fake-ref".into()),
        }) else {
            bail!("missing prepared tree should fail");
        };

        assert!(err.to_string().contains("prepared Perl tree"));
        Ok(())
    }

    #[test]
    fn smoke_rejects_missing_runner_script() -> TestResult {
        let temp = tempfile::tempdir()?;
        let perl_tree = temp.path().join("prepared-perl");
        fs::create_dir_all(perl_tree.join("t").join("base"))?;

        let Err(err) = smoke(SmokeConfig {
            perl_tree,
            host_perl: PathBuf::from("perl"),
            runner: HarnessRunner::Test,
            profile: HarnessProfile::Base,
            modes: vec![HarnessMode::Parse, HarnessMode::Compile],
            output_dir: Some(temp.path().join("smoke")),
            runner_binary: None,
            perl_ref: Some("fake-ref".into()),
        }) else {
            bail!("missing t/TEST should fail");
        };

        assert!(err.to_string().contains("missing t/TEST"));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn run_mode_buckets_runner_failure_records() -> TestResult {
        let temp = tempfile::tempdir()?;
        let perl_tree = write_fake_perl_tree(temp.path())?;
        let runner = write_fake_runner(temp.path(), RunnerStatus::Fail)?;
        let output = temp.path().join("parse-report.json");

        let Err(err) = run_mode(RunConfig {
            perl_tree,
            host_perl: PathBuf::from("/bin/sh"),
            runner: HarnessRunner::Test,
            mode: HarnessMode::Parse,
            profile: HarnessProfile::Base,
            tests: Vec::new(),
            output: Some(output.clone()),
            runner_binary: Some(runner),
        }) else {
            bail!("failing runner record should fail the harness run");
        };

        assert!(err.to_string().contains("failed for 1 of 1 files"));
        let raw = fs::read_to_string(output)?;
        let report: RunReport = serde_json::from_str(&raw)?;
        assert_eq!(report.summary.files_total, 1);
        assert_eq!(report.summary.files_passed, 0);
        assert_eq!(report.summary.files_failed, 1);
        assert_eq!(report.buckets.get("parse_recovery"), Some(&1));
        assert_eq!(report.failures[0].workstream, "parser_recovery");
        assert_eq!(
            report.failures[0].lsp_impact,
            vec!["diagnostics", "syntax_tree", "semantic_tokens"]
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn run_mode_rejects_nonzero_harness_status_without_file_failures() -> TestResult {
        let temp = tempfile::tempdir()?;
        let perl_tree = write_fake_perl_tree_with_run_body(
            temp.path(),
            r#"./perl base/ok.t
exit 7
"#,
        )?;
        let runner = write_fake_runner(temp.path(), RunnerStatus::Pass)?;
        let output = temp.path().join("parse-report.json");

        let Err(err) = run_mode(RunConfig {
            perl_tree,
            host_perl: PathBuf::from("/bin/sh"),
            runner: HarnessRunner::Test,
            mode: HarnessMode::Parse,
            profile: HarnessProfile::Base,
            tests: Vec::new(),
            output: Some(output.clone()),
            runner_binary: Some(runner),
        }) else {
            bail!("nonzero harness status should fail even when runner records pass");
        };

        assert!(err.to_string().contains("upstream harness exited with status"));
        let raw = fs::read_to_string(output)?;
        let report: RunReport = serde_json::from_str(&raw)?;
        assert_eq!(report.summary.files_passed, 1);
        assert_eq!(report.summary.files_failed, 0);
        assert_eq!(report.harness_status, Some(7));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn run_mode_invokes_runner_directly_when_harness_writes_no_records() -> TestResult {
        let temp = tempfile::tempdir()?;
        let perl_tree = write_fake_perl_tree_with_run_body(
            temp.path(),
            r#"# Deliberately do not invoke ./perl; real harness integration bugs should fall
# back to direct runner invocation for harness-selected files.
exit 7
"#,
        )?;
        let runner = write_fake_runner(temp.path(), RunnerStatus::Pass)?;
        let output = temp.path().join("parse-report.json");

        run_mode(RunConfig {
            perl_tree,
            host_perl: PathBuf::from("/bin/sh"),
            runner: HarnessRunner::Test,
            mode: HarnessMode::Parse,
            profile: HarnessProfile::Base,
            tests: Vec::new(),
            output: Some(output.clone()),
            runner_binary: Some(runner),
        })?;

        let raw = fs::read_to_string(output)?;
        let report: RunReport = serde_json::from_str(&raw)?;
        assert_eq!(report.summary.files_total, 1);
        assert_eq!(report.summary.files_passed, 1);
        assert_eq!(report.summary.files_failed, 0);
        assert!(report.buckets.is_empty());
        assert!(report.failures.is_empty());
        assert_eq!(report.harness_status, Some(7));
        Ok(())
    }

    #[cfg(unix)]
    fn write_fake_perl_tree_requiring_t_perl_for_dumptests(root: &Path) -> TestResult<PathBuf> {
        let perl_tree = root.join("prepared-perl-requires-t-perl");
        let t_dir = perl_tree.join("t");
        fs::create_dir_all(t_dir.join("base"))?;
        fs::write(t_dir.join("base").join("ok.t"), "1;\n")?;
        let script = r#"#!/bin/sh
set -eu
if [ "${1:-}" = "--dumptests" ]; then
  if [ ! -f ./perl ]; then
    echo 'You need to run "make test_prep" first to set things up.' >&2
    exit 2
  fi
  echo "base/ok.t"
  exit 0
fi
./perl base/ok.t
"#;
        fs::write(t_dir.join("TEST"), script)?;
        Ok(perl_tree)
    }

    #[cfg(unix)]
    fn write_fake_perl_tree(root: &Path) -> TestResult<PathBuf> {
        write_fake_perl_tree_with_run_body(
            root,
            r#"./perl base/ok.t
"#,
        )
    }

    #[cfg(unix)]
    fn write_fake_perl_tree_with_two_base_tests(root: &Path) -> TestResult<PathBuf> {
        let perl_tree = root.join("prepared-perl-two-base-tests");
        let t_dir = perl_tree.join("t");
        fs::create_dir_all(t_dir.join("base"))?;
        fs::write(t_dir.join("base").join("ok.t"), "1;\n")?;
        fs::write(t_dir.join("base").join("lex.t"), "1;\n")?;
        let script = r#"#!/bin/sh
set -eu
if [ "${1:-}" = "--dumptests" ]; then
  echo "base/ok.t"
  echo "base/lex.t"
  exit 0
fi
./perl base/ok.t
./perl base/lex.t
"#;
        fs::write(t_dir.join("TEST"), script)?;
        Ok(perl_tree)
    }

    #[cfg(unix)]
    fn write_fake_perl_tree_with_base_if_test(root: &Path) -> TestResult<PathBuf> {
        let perl_tree = root.join("prepared-perl-base-if");
        let t_dir = perl_tree.join("t");
        fs::create_dir_all(t_dir.join("base"))?;
        fs::write(t_dir.join("base").join("if.t"), "1;\n")?;
        let script = r#"#!/bin/sh
set -eu
if [ "${1:-}" = "--dumptests" ]; then
  echo "base/if.t"
  exit 0
fi
./perl base/if.t
"#;
        fs::write(t_dir.join("TEST"), script)?;
        Ok(perl_tree)
    }

    #[cfg(unix)]
    fn write_fake_perl_tree_with_two_comp_tests(root: &Path) -> TestResult<PathBuf> {
        let perl_tree = root.join("prepared-perl-two-comp-tests");
        let t_dir = perl_tree.join("t");
        fs::create_dir_all(t_dir.join("comp"))?;
        fs::write(t_dir.join("comp").join("require.t"), "1;\n")?;
        fs::write(t_dir.join("comp").join("use.t"), "1;\n")?;
        let script = r#"#!/bin/sh
set -eu
if [ "${1:-}" = "--dumptests" ]; then
  echo "comp/require.t"
  echo "comp/use.t"
  exit 0
fi
./perl comp/require.t
./perl comp/use.t
"#;
        fs::write(t_dir.join("TEST"), script)?;
        Ok(perl_tree)
    }

    #[cfg(unix)]
    fn write_fake_perl_tree_with_two_run_tests(root: &Path) -> TestResult<PathBuf> {
        let perl_tree = root.join("prepared-perl-two-run-tests");
        let t_dir = perl_tree.join("t");
        fs::create_dir_all(t_dir.join("run"))?;
        fs::write(t_dir.join("run").join("import.t"), "1;\n")?;
        fs::write(t_dir.join("run").join("switches.t"), "1;\n")?;
        let script = r#"#!/bin/sh
set -eu
if [ "${1:-}" = "--dumptests" ]; then
  echo "run/import.t"
  echo "run/switches.t"
  exit 0
fi
./perl run/import.t
./perl run/switches.t
"#;
        fs::write(t_dir.join("TEST"), script)?;
        Ok(perl_tree)
    }

    #[cfg(unix)]
    fn write_fake_perl_tree_with_run_body(root: &Path, run_body: &str) -> TestResult<PathBuf> {
        let perl_tree = root.join("prepared-perl");
        let t_dir = perl_tree.join("t");
        fs::create_dir_all(t_dir.join("base"))?;
        fs::write(t_dir.join("base").join("ok.t"), "1;\n")?;
        let stale_context_dir = perl_tree.join("target");
        fs::create_dir_all(&stale_context_dir)?;
        fs::write(
            stale_context_dir.join("perl-lsp-runner-records.jsonl"),
            r#"{"schema_version":"perl_core_harness.runner_record.v1","mode":"parse","path":"stale.t","status":"fail","assertions_passed":0,"assertions_total":1,"bucket":"parse_recovery","first_diagnostic":"stale"}"#,
        )?;
        let script = format!(
            r#"#!/bin/sh
set -eu
if [ "${{1:-}}" = "--dumptests" ]; then
  echo "base/ok.t"
  exit 0
fi
{run_body}"#
        );
        fs::write(t_dir.join("TEST"), script)?;
        Ok(perl_tree)
    }

    #[cfg(unix)]
    fn write_fake_runner(root: &Path, status: RunnerStatus) -> TestResult<PathBuf> {
        write_fake_runner_with_bucket(root, status, Some("parse_recovery"))
    }

    #[cfg(unix)]
    fn write_fake_execute_runner(root: &Path) -> TestResult<PathBuf> {
        let runner = root.join("fake-runner-execute-pass.sh");
        let body = r#"#!/bin/sh
set -eu
script="${1:-unknown.t}"
mode="${PERL_LSP_HARNESS_MODE:-execute}"
mkdir -p "$(dirname "$PERL_LSP_HARNESS_CONTEXT")"
printf '1..2\n'
printf 'ok 1 - if eq\n'
printf 'ok 2 - if ne\n'
printf '{"schema_version":"perl_core_harness.runner_record.v1","mode":"%s","path":"%s","status":"pass","assertions_passed":2,"assertions_total":2,"bucket":null,"first_diagnostic":null}\n' "$mode" "$script" >> "$PERL_LSP_HARNESS_CONTEXT"
"#;
        fs::write(&runner, body)?;
        set_executable(&runner)?;
        Ok(runner)
    }

    #[cfg(unix)]
    fn write_fake_runner_with_bucket(
        root: &Path,
        status: RunnerStatus,
        bucket: Option<&str>,
    ) -> TestResult<PathBuf> {
        let runner = match status {
            RunnerStatus::Pass => root.join("fake-runner-pass.sh"),
            RunnerStatus::Fail => root.join("fake-runner-fail.sh"),
        };
        let body = match status {
            RunnerStatus::Pass => {
                r#"#!/bin/sh
set -eu
script="${1:-unknown.t}"
mode="${PERL_LSP_HARNESS_MODE:-parse}"
mkdir -p "$(dirname "$PERL_LSP_HARNESS_CONTEXT")"
printf '1..1\n'
printf 'ok 1 - %s %s\n' "$mode" "$script"
printf '{"schema_version":"perl_core_harness.runner_record.v1","mode":"%s","path":"%s","status":"pass","assertions_passed":1,"assertions_total":1,"bucket":null,"first_diagnostic":null}\n' "$mode" "$script" >> "$PERL_LSP_HARNESS_CONTEXT"
"#
                .to_string()
            }
            RunnerStatus::Fail => {
                let bucket_comment = bucket.unwrap_or("unknown");
                let bucket_json = match bucket {
                    Some(bucket) => format!(r#""{bucket}""#),
                    None => "null".to_string(),
                };
                format!(
                    r#"#!/bin/sh
set -eu
script="${{1:-unknown.t}}"
mode="${{PERL_LSP_HARNESS_MODE:-parse}}"
mkdir -p "$(dirname "$PERL_LSP_HARNESS_CONTEXT")"
printf '1..1\n'
printf 'not ok 1 - %s %s\n' "$mode" "$script"
printf '# bucket: {bucket_comment}\n'
printf '# first diagnostic: expected expression\n'
printf '{{"schema_version":"perl_core_harness.runner_record.v1","mode":"%s","path":"%s","status":"fail","assertions_passed":0,"assertions_total":1,"bucket":{bucket_json},"first_diagnostic":"expected expression"}}\n' "$mode" "$script" >> "$PERL_LSP_HARNESS_CONTEXT"
exit 1
"#
                )
            }
        };
        fs::write(&runner, body)?;
        set_executable(&runner)?;
        Ok(runner)
    }
}
