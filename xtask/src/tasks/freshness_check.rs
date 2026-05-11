//! `cargo xtask freshness-check` — source-tree staleness guard.
//!
//! Detects whether the current checkout is behind `origin/master` (or another
//! base ref) and emits a JSON receipt that downstream tools and hooks can
//! consume.
//!
//! # Exit codes
//! - `0` — always in warn mode (default).
//! - `0` — block mode AND `safe_for_code_state_claims == true`.
//! - `1` — block mode AND stale (unless `--allow-historical` was passed).
//!
//! # JSON receipt (schema_version 1)
//! ```json
//! {
//!   "schema_version": 1,
//!   "base_ref": "origin/master",
//!   "head": "abc1234",
//!   "base_head": "def5678",
//!   "behind_by": 5,
//!   "fetch_age_seconds": 3600,
//!   "worktree_dirty": false,
//!   "safe_for_code_state_claims": false,
//!   "mode": "warn",
//!   "allow_historical": false,
//!   "bypass_reason": null
//! }
//! ```

use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
};

/// Operating mode for the freshness check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FreshnessMode {
    /// Warn mode: always exit 0, emit receipt.
    Warn,
    /// Block mode: exit 1 when stale (unless `--allow-historical` was passed).
    Block,
}

impl std::fmt::Display for FreshnessMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FreshnessMode::Warn => write!(f, "warn"),
            FreshnessMode::Block => write!(f, "block"),
        }
    }
}

/// Configuration for the freshness check subcommand.
pub struct FreshnessCheckConfig {
    /// Base git reference to compare HEAD against. Default: `origin/master`.
    pub base: String,
    /// Operating mode (warn or block).
    pub mode: FreshnessMode,
    /// If `Some(path)`, write the JSON receipt to this file instead of stdout.
    pub json_output: Option<PathBuf>,
    /// Skip the `git fetch` step.
    pub no_fetch: bool,
    /// Bypass block mode for historical work. Requires `reason` to be `Some`.
    pub allow_historical: bool,
    /// Reason string required when `allow_historical` is true.
    pub reason: Option<String>,
}

/// The JSON receipt schema emitted by freshness-check.
#[derive(Debug, Serialize, Deserialize)]
pub struct FreshnessReceipt {
    /// Always 1 for this schema generation.
    pub schema_version: u32,
    /// The base git ref used for the comparison (e.g. `origin/master`).
    pub base_ref: String,
    /// Short SHA of HEAD.
    pub head: String,
    /// Short SHA of the base ref tip.
    pub base_head: String,
    /// Number of commits HEAD is behind the base ref.
    pub behind_by: u64,
    /// Seconds since the last `git fetch` (from `.git/FETCH_HEAD` mtime).
    /// `None` when FETCH_HEAD does not exist (never fetched or `--no-fetch`).
    pub fetch_age_seconds: Option<u64>,
    /// Whether the working tree has uncommitted changes.
    pub worktree_dirty: bool,
    /// `true` only when `behind_by == 0`.
    pub safe_for_code_state_claims: bool,
    /// The mode this invocation ran under.
    pub mode: FreshnessMode,
    /// Whether the historical-override escape hatch was used.
    pub allow_historical: bool,
    /// The caller-provided bypass reason, when `allow_historical` is `true`.
    pub bypass_reason: Option<String>,
}

/// Run the freshness-check subcommand.
///
/// Returns `Ok(())` when the check passes or when in warn mode.
/// Returns `Err` on fatal errors. Exits with code 1 when in block mode and
/// the checkout is stale (unless `--allow-historical` was passed).
pub fn run(config: FreshnessCheckConfig) -> Result<()> {
    // Validate allow-historical usage.
    if config.allow_historical && config.reason.is_none() {
        bail!("--allow-historical requires --reason <text>");
    }

    // Resolve the effective base ref.
    let base_ref = config.base.clone();

    // Optionally fetch.
    if !config.no_fetch {
        fetch_base(&base_ref)?;
    }

    // Gather git data.
    let head = git_short_sha("HEAD")?;
    let base_head = git_short_sha(&base_ref)?;
    let behind_by = compute_behind_by(&base_ref)?;
    let fetch_age_seconds = read_fetch_age_seconds();
    let worktree_dirty = is_worktree_dirty()?;
    let safe_for_code_state_claims = behind_by == 0;

    let receipt = FreshnessReceipt {
        schema_version: 1,
        base_ref: base_ref.clone(),
        head,
        base_head,
        behind_by,
        fetch_age_seconds,
        worktree_dirty,
        safe_for_code_state_claims,
        mode: config.mode,
        allow_historical: config.allow_historical,
        bypass_reason: config.reason.clone(),
    };

    // Emit receipt.
    let json = serde_json::to_string_pretty(&receipt)
        .context("failed to serialize freshness receipt to JSON")?;

    if let Some(ref path) = config.json_output {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {}", parent.display()))?;
        }
        fs::write(path, &json)
            .with_context(|| format!("failed to write receipt to {}", path.display()))?;
        // Also print a human summary to stderr when writing to a file.
        emit_human_summary(&receipt);
    } else {
        // Default: JSON to stdout.
        println!("{json}");
        emit_human_summary(&receipt);
    }

    // Decide exit code.
    let stale = !safe_for_code_state_claims;
    if config.mode == FreshnessMode::Block && stale && !config.allow_historical {
        // Use std::process::exit so tests can observe the exit code.
        std::process::exit(1);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn git_output(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).stderr(Stdio::null()).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn git_short_sha(rev: &str) -> Result<String> {
    git_output(&["rev-parse", "--short", rev])
        .ok_or_else(|| color_eyre::eyre::eyre!("failed to resolve git rev: {rev}"))
}

fn fetch_base(base_ref: &str) -> Result<()> {
    // Extract the remote and branch from e.g. "origin/master".
    let (remote, branch) = if let Some(slash) = base_ref.find('/') {
        (&base_ref[..slash], &base_ref[slash + 1..])
    } else {
        // If no slash, treat the whole thing as the remote and fetch all.
        (base_ref, "")
    };

    let status = if branch.is_empty() {
        Command::new("git")
            .args(["fetch", remote])
            .stderr(Stdio::null())
            .stdout(Stdio::null())
            .status()
            .context("failed to run git fetch")?
    } else {
        Command::new("git")
            .args(["fetch", remote, branch])
            .stderr(Stdio::null())
            .stdout(Stdio::null())
            .status()
            .context("failed to run git fetch")?
    };

    if !status.success() {
        // Fetch errors are non-fatal — the user may be offline.
        eprintln!(
            "warning: git fetch {remote} {branch} failed (offline?); proceeding with cached data"
        );
    }
    Ok(())
}

fn compute_behind_by(base_ref: &str) -> Result<u64> {
    let count_str = git_output(&["rev-list", "--count", &format!("HEAD..{base_ref}")])
        .unwrap_or_else(|| "0".to_string());
    count_str
        .trim()
        .parse::<u64>()
        .with_context(|| format!("failed to parse rev-list --count output: {count_str:?}"))
}

fn read_fetch_age_seconds() -> Option<u64> {
    // Walk up from cwd to find .git/FETCH_HEAD.
    let git_dir = git_output(&["rev-parse", "--git-dir"])?;
    let fetch_head = std::path::Path::new(&git_dir).join("FETCH_HEAD");
    let metadata = fs::metadata(&fetch_head).ok()?;
    let modified = metadata.modified().ok()?;
    let age = std::time::SystemTime::now().duration_since(modified).ok()?;
    Some(age.as_secs())
}

fn is_worktree_dirty() -> Result<bool> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .context("failed to run git status")?;
    Ok(!output.stdout.is_empty())
}

fn emit_human_summary(receipt: &FreshnessReceipt) {
    if receipt.safe_for_code_state_claims {
        eprintln!("freshness-check: HEAD is up-to-date with {} (behind_by=0)", receipt.base_ref);
    } else {
        eprintln!(
            "freshness-check: HEAD is {} commit(s) behind {} — safe_for_code_state_claims=false",
            receipt.behind_by, receipt.base_ref
        );
        if receipt.mode == FreshnessMode::Warn {
            eprintln!("  (warn mode — exit 0; run `git pull --rebase` to update)");
        } else if receipt.allow_historical {
            eprintln!(
                "  (block mode — historical override accepted: {:?})",
                receipt.bypass_reason.as_deref().unwrap_or("")
            );
        } else {
            eprintln!("  (block mode — exit 1; run `git pull --rebase` to update)");
        }
    }
}
