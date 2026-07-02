//! Upstream Perl core harness integration scaffold.
//!
//! The scaffold can discover tests from a prepared Perl source tree and run the
//! staged profile through a `t/perl` compatibility wrapper in parse mode.
//! Compile and execute modes remain fail-closed for later slices.

use crate::utils::project_root;
use chrono::Utc;
use clap::ValueEnum;
use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const DISCOVERY_SCHEMA_VERSION: &str = "perl_core_harness.discovery.v1";
const RUN_REPORT_SCHEMA_VERSION: &str = "perl_core_harness.report.v1";

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
    fn script_name(self) -> &'static str {
        match self {
            Self::Test => "TEST",
            Self::Harness => "harness",
        }
    }

    fn as_str(self) -> &'static str {
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
    fn as_str(self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::Comp => "comp",
            Self::Run => "run",
            Self::Core => "core",
            Self::Lib => "lib",
            Self::Full => "full",
        }
    }

    fn roots(self) -> &'static [&'static str] {
        match self {
            Self::Base => &["base"],
            Self::Comp => &["comp"],
            Self::Run => &["run"],
            Self::Core => &["base", "comp", "run", "cmd", "io", "re", "opbasic", "op"],
            Self::Lib => &["lib"],
            Self::Full => &["base", "comp", "run", "cmd", "io", "re", "opbasic", "op", "uni"],
        }
    }

    fn runner_args(self, runner: HarnessRunner) -> Vec<String> {
        match runner {
            HarnessRunner::Test => self.roots().iter().map(|root| (*root).to_string()).collect(),
            HarnessRunner::Harness => {
                self.roots().iter().map(|root| format!("{root}/*.t")).collect()
            }
        }
    }
}

impl fmt::Display for HarnessProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
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

/// Configuration for `perl-core-harness run`.
#[derive(Debug, Clone)]
pub struct RunConfig {
    pub perl_tree: PathBuf,
    pub host_perl: PathBuf,
    pub runner: HarnessRunner,
    pub mode: HarnessMode,
    pub profile: HarnessProfile,
    pub output: Option<PathBuf>,
    pub runner_binary: Option<PathBuf>,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunnerStatus {
    Pass,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct RunnerRecord {
    schema_version: String,
    mode: String,
    path: String,
    status: RunnerStatus,
    assertions_passed: usize,
    assertions_total: usize,
    bucket: Option<String>,
    first_diagnostic: Option<String>,
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
        &config.profile.runner_args(config.runner),
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
    println!(
        "perl-core-harness: discovered {} tests for profile {} via {}",
        report.tests.len(),
        report.profile,
        report.runner
    );
    println!("wrote {}", output_path.display());
    Ok(())
}

/// Stub for future `prepare` implementation.
pub fn prepare() -> Result<()> {
    bail!(
        "perl-core-harness prepare is not implemented in this discovery scaffold; pass --perl-tree to discover"
    )
}

/// Run upstream Perl core tests through the compatibility runner.
pub fn run_mode(config: RunConfig) -> Result<()> {
    if config.mode != HarnessMode::Parse {
        bail!(
            "perl-core-harness run --mode {} is not implemented yet; this slice supports parse only",
            config.mode
        );
    }

    let perl_tree = canonicalize_existing_dir(&config.perl_tree, "prepared Perl tree")?;
    let run_tree = prepare_run_copy(&perl_tree, config.runner, config.mode, config.profile)?;
    let t_dir = run_tree.join("t");
    let script = validate_runner_script(&t_dir, config.runner)?;
    let dumptests_args = config.profile.runner_args(config.runner);
    let dumptests_output = invoke_dumptests(&config.host_perl, &t_dir, &script, &dumptests_args)?;
    let discovered = parse_dumptests_output(&dumptests_output.stdout)?;

    let runner_binary = resolve_runner_binary(config.runner_binary.as_deref())?;
    let context_path = run_tree.join("target").join("perl-lsp-runner-records.jsonl");
    if context_path.exists() {
        let context = format!("removing stale context {}", context_path.display());
        fs::remove_file(&context_path).context(context)?;
    }
    install_t_perl_wrapper(&run_tree)?;

    let output = invoke_harness_run(
        &config.host_perl,
        &t_dir,
        &script,
        &config.profile.runner_args(config.runner),
        &runner_binary,
        &context_path,
        config.mode,
    )
    .with_context(|| format!("running Perl core tests via {} {}", config.runner, config.profile))?;

    let records = read_runner_records(&context_path)?;
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

    println!(
        "perl-core-harness: {} {}/{} files passed via {}",
        report.mode, report.summary.files_passed, report.summary.files_total, report.runner
    );
    println!("wrote {}", output_path.display());

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
    if !output.status.success() {
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

/// Stub for future baseline management.
pub fn baseline(accept: bool) -> Result<()> {
    if accept {
        bail!("perl-core-harness baseline --accept is not implemented until run receipts exist");
    }
    bail!("perl-core-harness baseline is not implemented until run receipts exist")
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

fn write_discovery_report(path: &Path, report: &DiscoveryReport) -> Result<()> {
    if let Some(parent) = path.parent() {
        let context = format!("creating output directory {}", parent.display());
        fs::create_dir_all(parent).context(context)?;
    }
    let json = serde_json::to_string_pretty(report).context("serializing discovery report")?;
    fs::write(path, format!("{json}\n"))
        .with_context(|| format!("writing discovery report {}", path.display()))
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

fn prepare_run_copy(
    perl_tree: &Path,
    runner: HarnessRunner,
    mode: HarnessMode,
    profile: HarnessProfile,
) -> Result<PathBuf> {
    let run_tree = project_root()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("target")
        .join("perl-core")
        .join("runs")
        .join(format!("{runner}-{mode}-{profile}"));
    if run_tree.exists() {
        fs::remove_dir_all(&run_tree)
            .with_context(|| format!("removing prior run tree {}", run_tree.display()))?;
    }
    copy_dir_all(perl_tree, &run_tree)?;
    Ok(run_tree)
}

fn copy_dir_all(source: &Path, destination: &Path) -> Result<()> {
    let create_context = format!("creating directory {}", destination.display());
    fs::create_dir_all(destination).context(create_context)?;
    let read_context = format!("reading {}", source.display());
    for entry in fs::read_dir(source).context(read_context)? {
        let entry_context = format!("reading entry in {}", source.display());
        let entry = entry.context(entry_context)?;
        let entry_path = entry.path();
        let type_context = format!("reading file type for {}", entry_path.display());
        let ty = entry.file_type().context(type_context)?;
        let child_destination = destination.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry_path, &child_destination)?;
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

fn workstream_for_bucket(bucket: &str) -> &'static str {
    match bucket {
        "parse_recovery" => "parser_recovery",
        "source_decode" => "source_loading",
        "cli_switch" => "harness_cli_compat",
        "harness_prepare" => "harness_integration",
        _ => "compiler_conformance",
    }
}

fn lsp_impact_for_bucket(bucket: &str) -> Vec<&'static str> {
    match bucket {
        "parse_recovery" => vec!["diagnostics", "syntax_tree", "semantic_tokens"],
        "source_decode" => vec!["workspace_index", "diagnostics"],
        "cli_switch" | "harness_prepare" => vec!["compiler_conformance"],
        _ => vec!["compiler_conformance"],
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
    fn profile_base_uses_bootstrap_root_for_test_runner() {
        assert_eq!(HarnessProfile::Base.runner_args(HarnessRunner::Test), vec!["base"]);
    }

    #[test]
    fn profile_base_uses_glob_for_tap_harness_runner() {
        assert_eq!(HarnessProfile::Base.runner_args(HarnessRunner::Harness), vec!["base/*.t"]);
    }

    #[test]
    fn compile_mode_remains_fail_closed() -> TestResult {
        let config = RunConfig {
            perl_tree: PathBuf::from("unused"),
            host_perl: PathBuf::from("perl"),
            runner: HarnessRunner::Test,
            mode: HarnessMode::Compile,
            profile: HarnessProfile::Base,
            output: None,
            runner_binary: None,
        };

        let Err(err) = run_mode(config) else {
            bail!("compile mode should remain fail-closed in parse slice");
        };

        assert!(err.to_string().contains("run --mode compile is not implemented yet"));
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
        assert_eq!(workstream_for_bucket("cli_switch"), "harness_cli_compat");
        assert_eq!(workstream_for_bucket("harness_prepare"), "harness_integration");
        assert_eq!(workstream_for_bucket("unknown_bucket"), "compiler_conformance");
        assert_eq!(lsp_impact_for_bucket("source_decode"), vec!["workspace_index", "diagnostics"]);
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
            output: Some(output.clone()),
            runner_binary: Some(runner),
        })?;

        let raw = fs::read_to_string(output)?;
        let report: RunReport = serde_json::from_str(&raw)?;
        assert_eq!(report.summary.files_total, 2);
        assert_eq!(report.summary.files_passed, 2);
        assert_eq!(report.summary.files_failed, 0);
        assert_eq!(
            report.file_results.iter().map(|result| result.path.as_str()).collect::<Vec<_>>(),
            vec!["base/ok.t", "base/lex.t"]
        );
        assert!(report.file_results.iter().all(|result| result.status == RunnerStatus::Pass));
        assert!(!perl_tree.join("t").join("perl").exists(), "source Perl tree must not be mutated");
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
        let runner = match status {
            RunnerStatus::Pass => root.join("fake-runner-pass.sh"),
            RunnerStatus::Fail => root.join("fake-runner-fail.sh"),
        };
        let body = match status {
            RunnerStatus::Pass => {
                r#"#!/bin/sh
set -eu
script="${1:-unknown.t}"
mkdir -p "$(dirname "$PERL_LSP_HARNESS_CONTEXT")"
printf '1..1\n'
printf 'ok 1 - parse %s\n' "$script"
printf '{"schema_version":"perl_core_harness.runner_record.v1","mode":"parse","path":"%s","status":"pass","assertions_passed":1,"assertions_total":1,"bucket":null,"first_diagnostic":null}\n' "$script" >> "$PERL_LSP_HARNESS_CONTEXT"
"#
            }
            RunnerStatus::Fail => {
                r#"#!/bin/sh
set -eu
script="${1:-unknown.t}"
mkdir -p "$(dirname "$PERL_LSP_HARNESS_CONTEXT")"
printf '1..1\n'
printf 'not ok 1 - parse %s\n' "$script"
printf '# bucket: parse_recovery\n'
printf '# first diagnostic: expected expression\n'
printf '{"schema_version":"perl_core_harness.runner_record.v1","mode":"parse","path":"%s","status":"fail","assertions_passed":0,"assertions_total":1,"bucket":"parse_recovery","first_diagnostic":"expected expression"}\n' "$script" >> "$PERL_LSP_HARNESS_CONTEXT"
exit 1
"#
            }
        };
        fs::write(&runner, body)?;
        set_executable(&runner)?;
        Ok(runner)
    }
}
