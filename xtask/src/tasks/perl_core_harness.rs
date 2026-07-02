//! Upstream Perl core harness integration scaffold.
//!
//! This first slice owns discovery only: given a prepared Perl source tree and
//! a host Perl, invoke upstream `t/TEST` or `t/harness` with `--dumptests` and
//! write a normalized manifest for later parse/compile/execute runner slices.

use crate::utils::project_root;
use chrono::Utc;
use clap::ValueEnum;
use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const DISCOVERY_SCHEMA_VERSION: &str = "perl_core_harness.discovery.v1";

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

/// Stub for future parse/compile/execute runner implementation.
pub fn run_mode(mode: HarnessMode) -> Result<()> {
    bail!(
        "perl-core-harness run --mode {mode} is not implemented in this discovery scaffold; use discover first"
    )
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

fn write_discovery_report(path: &Path, report: &DiscoveryReport) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating output directory {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(report).context("serializing discovery report")?;
    fs::write(path, format!("{json}\n"))
        .with_context(|| format!("writing discovery report {}", path.display()))
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
}
