//! `cargo xtask ci-doctor` — local CI parity diagnostics.
//!
//! Checks that the local environment matches what CI expects:
//! Rust toolchain, fmt/clippy availability, git cleanliness, fmt drift,
//! Perl interpreter, perl-lsp binary, and platform/env sanity.
//!
//! # Exit codes
//! - `0` in default (warn) mode — always.
//! - `0` in strict mode when all checks are `ok`.
//! - `1` in strict mode when any check is `warn` or `fail`.
//!
//! # JSON receipt (schema_version 1)
//! ```json
//! {
//!   "schema_version": 1,
//!   "overall": "ok",
//!   "checks": [
//!     {
//!       "name": "rustc-toolchain-match",
//!       "status": "ok",
//!       "expected": "1.95.0",
//!       "actual": "1.95.0",
//!       "note": "match"
//!     }
//!   ]
//! }
//! ```

use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Status of an individual check.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    /// Check passed — environment matches expectation.
    Ok,
    /// Discrepancy detected but not fatal (warn mode: exit 0; strict: exit 1).
    Warn,
    /// Hard failure — something is broken.
    Fail,
}

impl CheckStatus {
    fn severity(&self) -> u8 {
        match self {
            CheckStatus::Ok => 0,
            CheckStatus::Warn => 1,
            CheckStatus::Fail => 2,
        }
    }

    fn symbol(&self) -> &'static str {
        match self {
            CheckStatus::Ok => "ok  ",
            CheckStatus::Warn => "warn",
            CheckStatus::Fail => "FAIL",
        }
    }
}

/// Result of a single diagnostic check.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Check {
    /// Stable identifier for this check (kebab-case).
    pub name: String,
    /// Pass / warn / fail.
    pub status: CheckStatus,
    /// What was expected (empty string when not applicable).
    pub expected: String,
    /// What was actually found (empty string when not applicable).
    pub actual: String,
    /// Human-readable note explaining the outcome.
    pub note: String,
}

impl Check {
    fn ok(name: &str, value: &str, note: &str) -> Self {
        Check {
            name: name.to_string(),
            status: CheckStatus::Ok,
            expected: value.to_string(),
            actual: value.to_string(),
            note: note.to_string(),
        }
    }

    fn warn(name: &str, expected: &str, actual: &str, note: &str) -> Self {
        Check {
            name: name.to_string(),
            status: CheckStatus::Warn,
            expected: expected.to_string(),
            actual: actual.to_string(),
            note: note.to_string(),
        }
    }

    fn fail(name: &str, expected: &str, actual: &str, note: &str) -> Self {
        Check {
            name: name.to_string(),
            status: CheckStatus::Fail,
            expected: expected.to_string(),
            actual: actual.to_string(),
            note: note.to_string(),
        }
    }
}

/// The JSON receipt schema emitted by ci-doctor.
#[derive(Debug, Serialize, Deserialize)]
pub struct CiDoctorReceipt {
    /// Always 1 for this schema generation.
    pub schema_version: u32,
    /// Worst status across all checks.
    pub overall: CheckStatus,
    /// All checks in order.
    pub checks: Vec<Check>,
}

/// Configuration for the ci-doctor subcommand.
pub struct CiDoctorConfig {
    /// Emit JSON receipt to stdout instead of human-readable output.
    pub json: bool,
    /// Exit 1 when any check is warn or fail.
    pub strict: bool,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Run all ci-doctor checks and emit the result.
pub fn run(config: CiDoctorConfig) -> Result<()> {
    let checks = collect_checks();
    let overall = compute_overall(&checks);

    let receipt = CiDoctorReceipt { schema_version: 1, overall: overall.clone(), checks };

    if config.json {
        let json = serde_json::to_string_pretty(&receipt)
            .context("failed to serialize ci-doctor receipt to JSON")?;
        println!("{json}");
    } else {
        emit_human_summary(&receipt);
    }

    if config.strict && overall != CheckStatus::Ok {
        std::process::exit(1);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Aggregate helpers
// ---------------------------------------------------------------------------

fn compute_overall(checks: &[Check]) -> CheckStatus {
    let max_severity = checks.iter().map(|c| c.status.severity()).max().unwrap_or(0);
    match max_severity {
        0 => CheckStatus::Ok,
        1 => CheckStatus::Warn,
        _ => CheckStatus::Fail,
    }
}

fn collect_checks() -> Vec<Check> {
    vec![
        check_rustc_toolchain_match(),
        check_rustfmt_installed(),
        check_clippy_installed(),
        check_git_clean(),
        check_fmt_drift(),
        check_perl_interpreter(),
        check_perl_lsp_binary(),
        check_platform_sanity(),
    ]
}

// ---------------------------------------------------------------------------
// Individual checks
// ---------------------------------------------------------------------------

/// Check 1: rustc version matches rust-toolchain.toml channel.
fn check_rustc_toolchain_match() -> Check {
    let name = "rustc-toolchain-match";

    let pinned = match read_pinned_channel() {
        Some(v) => v,
        None => {
            return Check::warn(
                name,
                "rust-toolchain.toml",
                "not found",
                "rust-toolchain.toml not found or unreadable",
            );
        }
    };

    let actual = match run_version_cmd("rustc", &["--version"]) {
        Some(v) => v,
        None => {
            return Check::fail(
                name,
                &pinned,
                "not found",
                "rustc not found on PATH; install via rustup",
            );
        }
    };

    // Extract the version token (second word in "rustc X.Y.Z (hash date)").
    let actual_ver = actual.split_whitespace().nth(1).unwrap_or(&actual).to_string();

    if actual_ver == pinned {
        Check::ok(name, &pinned, "match")
    } else {
        Check::warn(
            name,
            &pinned,
            &actual_ver,
            &format!(
                "rustc {actual_ver} differs from pinned {pinned}; run `rustup override set {pinned}`"
            ),
        )
    }
}

/// Check 2: rustfmt is installed and callable.
fn check_rustfmt_installed() -> Check {
    let name = "rustfmt-installed";
    match run_version_cmd("cargo", &["fmt", "--version"]) {
        Some(v) => Check::ok(name, &v, "installed"),
        None => Check::fail(
            name,
            "installed",
            "not found",
            "cargo fmt not available; run `rustup component add rustfmt`",
        ),
    }
}

/// Check 3: clippy is installed and callable.
fn check_clippy_installed() -> Check {
    let name = "clippy-installed";
    match run_version_cmd("cargo", &["clippy", "--version"]) {
        Some(v) => Check::ok(name, &v, "installed"),
        None => Check::fail(
            name,
            "installed",
            "not found",
            "cargo clippy not available; run `rustup component add clippy`",
        ),
    }
}

/// Check 4: git working tree is clean.
fn check_git_clean() -> Check {
    let name = "git-clean";
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();

    match output {
        Err(_) => Check::warn(name, "clean", "unknown", "git not found; cannot check working tree"),
        Ok(out) => {
            if out.stdout.is_empty() {
                Check::ok(name, "clean", "working tree is clean")
            } else {
                let count = out.stdout.split(|&b| b == b'\n').filter(|l| !l.is_empty()).count();
                Check::warn(
                    name,
                    "clean",
                    &format!("{count} modified file(s)"),
                    "working tree has uncommitted changes (WIP is fine; mention if unexpected)",
                )
            }
        }
    }
}

/// Check 5: cargo xtask fmt --check exits 0 (no fmt drift).
fn check_fmt_drift() -> Check {
    let name = "fmt-drift";
    // We run `cargo fmt --check` rather than `cargo xtask fmt --check`
    // to avoid a recursive xtask invocation and reduce cost.
    let result = Command::new("cargo")
        .args(["fmt", "--check"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match result {
        Err(_) => {
            Check::warn(name, "formatted", "unknown", "cargo fmt not found; cannot check drift")
        }
        Ok(status) => {
            if status.success() {
                Check::ok(name, "formatted", "no fmt drift detected")
            } else {
                Check::warn(
                    name,
                    "formatted",
                    "drift detected",
                    "run `cargo xtask fmt` to fix formatting",
                )
            }
        }
    }
}

/// Check 6: Perl interpreter is available.
fn check_perl_interpreter() -> Check {
    let name = "perl-interpreter";
    match run_version_cmd("perl", &["-e", "print $^V"]) {
        Some(v) => Check::ok(name, &v, "perl found on PATH"),
        None => {
            // Perl absence is a warn — perl-lsp can be used without a local Perl installation
            // (e.g. editing Perl code on a machine without a runtime).
            Check::warn(
                name,
                "available",
                "not found",
                "perl not found on PATH; debugging Perl code requires a local interpreter",
            )
        }
    }
}

/// Check 7: perl-lsp binary is available (post-install parity check).
fn check_perl_lsp_binary() -> Check {
    let name = "perl-lsp-binary";
    let found = which_binary("perl-lsp");
    if found {
        Check::ok(name, "found", "perl-lsp found on PATH")
    } else {
        // Absence is a warn, not a fail — the user may not have installed the binary yet.
        Check::warn(
            name,
            "found",
            "not found",
            "perl-lsp not found on PATH; run `cargo install --path crates/perl-lsp-rs` or add target/debug to PATH",
        )
    }
}

/// Check 8: platform/env sanity (CARGO_TARGET_DIR, PERL5LIB).
fn check_platform_sanity() -> Check {
    let name = "platform-env-sanity";
    let mut issues: Vec<String> = Vec::new();

    // Check CARGO_TARGET_DIR if set.
    if let Ok(val) = std::env::var("CARGO_TARGET_DIR") {
        let path = std::path::Path::new(&val);
        if !path.is_absolute() {
            issues.push(format!("CARGO_TARGET_DIR={val:?} is not an absolute path"));
        }
        // On Windows, flag paths with drive letters but mixed separators.
        #[cfg(target_os = "windows")]
        if val.contains('/') && val.contains('\\') {
            issues.push(format!(
                "CARGO_TARGET_DIR={val:?} mixes forward and back slashes; prefer one style"
            ));
        }
    }

    // Check PERL5LIB if set.
    if let Ok(val) = std::env::var("PERL5LIB") {
        if val.trim().is_empty() {
            issues.push("PERL5LIB is set but empty".to_string());
        }
    }

    if issues.is_empty() {
        Check::ok(name, "sane", "no environment anomalies detected")
    } else {
        Check::warn(name, "sane", "anomalies detected", &issues.join("; "))
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn read_pinned_channel() -> Option<String> {
    let root = crate::utils::project_root().ok()?;
    let path = root.join("rust-toolchain.toml");
    let raw = std::fs::read_to_string(&path).ok()?;
    let table: toml::Value = toml::from_str(&raw).ok()?;
    let channel = table.get("toolchain")?.get("channel")?.as_str()?;
    Some(channel.trim().to_string())
}

/// Run a command and return its stdout as a trimmed string.
/// Returns `None` if the command is not found or exits non-zero.
fn run_version_cmd(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

/// Return `true` if the named binary exists somewhere on PATH.
fn which_binary(name: &str) -> bool {
    // Try `which` on Unix, `where` on Windows.
    #[cfg(target_os = "windows")]
    let checker = ("where", [name, ""]);
    #[cfg(not(target_os = "windows"))]
    let checker = ("which", [name, ""]);

    Command::new(checker.0)
        .arg(checker.1[0])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Human-readable output
// ---------------------------------------------------------------------------

fn emit_human_summary(receipt: &CiDoctorReceipt) {
    let overall_label = match &receipt.overall {
        CheckStatus::Ok => "ok",
        CheckStatus::Warn => "warn",
        CheckStatus::Fail => "FAIL",
    };
    println!("ci-doctor  overall: {overall_label}");
    println!();
    for check in &receipt.checks {
        let sym = check.status.symbol();
        if check.expected == check.actual || check.expected.is_empty() {
            println!("  [{sym}] {}: {}", check.name, check.note);
        } else {
            println!(
                "  [{sym}] {}: {} (expected: {}, got: {})",
                check.name, check.note, check.expected, check.actual
            );
        }
    }
    println!();
    if receipt.overall != CheckStatus::Ok {
        println!("  run with --strict to fail CI on the above warnings/failures");
    }
}
