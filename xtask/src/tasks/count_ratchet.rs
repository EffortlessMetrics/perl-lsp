//! Ratchet gate: published crate count must not increase.
//!
//! This guards against accidentally re-expanding the `[workspace.metadata.publish.allow]`
//! list during the microcrate collapse (parent issue #4410) and beyond.
//!
//! Behavior:
//!   * Reads the current count of entries in `[workspace.metadata.publish.allow]`
//!     from `cargo metadata --no-deps`.
//!   * Reads the baseline from `xtask/published-crate-baseline.txt` (single integer).
//!   * current > baseline  -> ERROR (gate fails).
//!   * current < baseline  -> INFO + auto-write new baseline (ratchet tightens).
//!   * current == baseline -> pass silently.
//!
//! The baseline file is the single source of truth. Every wave PR that collapses
//! crates should see the baseline tighten automatically; the diff is committed as
//! part of that wave.
//!
//! Related: #4416, ADR-0041 (#4413), parent collapse #4410.

use crate::utils::{project_root, run_cargo_metadata};
use color_eyre::eyre::{bail, eyre, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

/// Relative path (from project root) of the baseline file.
const BASELINE_FILE: &str = "xtask/published-crate-baseline.txt";

#[derive(Deserialize)]
struct Metadata {
    #[serde(rename = "metadata")]
    workspace_metadata: Option<WorkspacePublishMeta>,
}

#[derive(Deserialize)]
struct WorkspacePublishMeta {
    publish: Option<AllowList>,
}

#[derive(Deserialize)]
struct AllowList {
    allow: Option<Vec<String>>,
}

/// Entry point for the `published-crate-count` xtask subcommand.
pub fn run() -> Result<()> {
    let root = project_root()?;
    let baseline_path = root.join(BASELINE_FILE);

    let current = current_count()?;
    let baseline = read_baseline(&baseline_path)?;

    match check_count(current, baseline) {
        CountStatus::Pass => {
            println!("published-crate-count: OK ({current} crates, baseline {baseline})");
            Ok(())
        }
        CountStatus::Ratchet { new_baseline } => {
            println!(
                "published-crate-count: RATCHET — count dropped from {baseline} to {new_baseline}, updating {BASELINE_FILE}"
            );
            write_baseline(&baseline_path, new_baseline)?;
            Ok(())
        }
        CountStatus::Fail => {
            bail!(
                "published-crate-count: FAIL — {current} crates published, baseline is {baseline}.\n\
                 The published crate count increased. Either remove crates from\n\
                 [workspace.metadata.publish.allow] in Cargo.toml, or if the increase is\n\
                 intentional, update {BASELINE_FILE} explicitly in a reviewed commit."
            );
        }
    }
}

/// Outcome of comparing the current count to the baseline.
#[derive(Debug, PartialEq, Eq)]
pub enum CountStatus {
    /// current == baseline (no action).
    Pass,
    /// current < baseline (auto-tighten baseline to `new_baseline`).
    Ratchet { new_baseline: u32 },
    /// current > baseline (gate fails).
    Fail,
}

/// Pure comparison helper — the core ratchet logic, extracted for unit tests.
pub fn check_count(current: u32, baseline: u32) -> CountStatus {
    if current > baseline {
        CountStatus::Fail
    } else if current < baseline {
        CountStatus::Ratchet { new_baseline: current }
    } else {
        CountStatus::Pass
    }
}

/// Queries `cargo metadata --no-deps` and returns the current count of entries
/// in `[workspace.metadata.publish.allow]` from the root `Cargo.toml`.
///
/// # Errors
///
/// Returns an error if:
/// - `cargo metadata` fails or exits non-zero
/// - The metadata JSON cannot be parsed
/// - The `workspace.metadata.publish.allow` key is missing from `Cargo.toml`
fn current_count() -> Result<u32> {
    let bytes = run_cargo_metadata(true)?;
    let meta: Metadata =
        serde_json::from_slice(&bytes).map_err(|e| eyre!("Failed to parse cargo metadata: {e}"))?;
    let allowlist = meta
        .workspace_metadata
        .as_ref()
        .and_then(|m| m.publish.as_ref())
        .and_then(|p| p.allow.as_ref())
        .ok_or_else(|| eyre!("No [workspace.metadata.publish.allow] found in root Cargo.toml"))?;
    Ok(allowlist.len() as u32)
}

/// Reads the baseline integer from the given path.
///
/// The baseline file is expected to contain a single integer (possibly with
/// trailing whitespace/newlines).
///
/// # Errors
///
/// Returns an error if the file cannot be read or the content is not a valid u32.
fn read_baseline(path: &Path) -> Result<u32> {
    let raw = fs::read_to_string(path)
        .map_err(|e| eyre!("Failed to read baseline file {}: {e}", path.display()))?;
    parse_baseline(&raw)
        .ok_or_else(|| eyre!("Invalid baseline value in {}: {:?}", path.display(), raw))
}

/// Parses a baseline value from a string.
///
/// Strips whitespace and newlines before parsing. Returns `None` if the content
/// is not a valid non-negative integer.
fn parse_baseline(raw: &str) -> Option<u32> {
    raw.trim().parse::<u32>().ok()
}

/// Writes the baseline value to the given path, followed by a newline.
///
/// The newline-terminated format matches typical text-file conventions and keeps
/// `git diff` output clean.
///
/// # Errors
///
/// Returns an error if the file cannot be written.
fn write_baseline(path: &Path, value: u32) -> Result<()> {
    // Newline-terminated to match typical text-file conventions and keep `git diff`
    // output clean.
    let contents = format!("{value}\n");
    fs::write(path, contents)
        .map_err(|e| eyre!("Failed to write baseline file {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_count_passes_when_equal() {
        assert_eq!(check_count(30, 30), CountStatus::Pass);
        assert_eq!(check_count(0, 0), CountStatus::Pass);
    }

    #[test]
    fn check_count_fails_when_current_exceeds_baseline() {
        assert_eq!(check_count(31, 30), CountStatus::Fail);
        assert_eq!(check_count(99, 98), CountStatus::Fail);
    }

    #[test]
    fn check_count_ratchets_when_current_is_lower() {
        assert_eq!(check_count(29, 30), CountStatus::Ratchet { new_baseline: 29 });
        assert_eq!(check_count(0, 5), CountStatus::Ratchet { new_baseline: 0 });
    }

    #[test]
    fn parse_baseline_trims_whitespace_and_newlines() {
        assert_eq!(parse_baseline("98"), Some(98));
        assert_eq!(parse_baseline("98\n"), Some(98));
        assert_eq!(parse_baseline("  42  \n"), Some(42));
        assert_eq!(parse_baseline("0"), Some(0));
    }

    #[test]
    fn parse_baseline_rejects_non_integers() {
        assert_eq!(parse_baseline(""), None);
        assert_eq!(parse_baseline("abc"), None);
        assert_eq!(parse_baseline("-5"), None);
        assert_eq!(parse_baseline("3.14"), None);
    }
}
