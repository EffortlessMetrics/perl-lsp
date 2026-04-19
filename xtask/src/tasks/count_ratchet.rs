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
use color_eyre::eyre::{Result, bail, eyre};
use serde::Deserialize;
use std::fs;
use std::path::Path;

/// Relative path (from project root) of the baseline file.
const BASELINE_FILE: &str = "xtask/published-crate-baseline.txt";

#[derive(Deserialize)]
struct Metadata {
    #[serde(rename = "metadata")]
    workspace_metadata: Option<PublishSettings>,
}

#[derive(Deserialize)]
struct PublishSettings {
    publish: Option<PublishAllowList>,
}

#[derive(Deserialize)]
struct PublishAllowList {
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
///
/// Uses `Ord` derived comparison to map the three possible orderings into
/// the corresponding `CountStatus` variants.
pub fn check_count(current: u32, baseline: u32) -> CountStatus {
    match current.cmp(&baseline) {
        std::cmp::Ordering::Greater => CountStatus::Fail,
        std::cmp::Ordering::Less => CountStatus::Ratchet {
            new_baseline: current,
        },
        std::cmp::Ordering::Equal => CountStatus::Pass,
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
        assert_eq!(
            check_count(29, 30),
            CountStatus::Ratchet { new_baseline: 29 }
        );
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

    // =============================================================================
    // PROPERTY-BASED TESTS
    // These verify invariants across many generated inputs, not just examples.
    // =============================================================================

    /// Property: The ratchet never loosens - new_baseline is always <= original baseline
    #[test]
    fn property_ratchet_never_loosens() {
        for baseline in 0u32..1000 {
            for current in 0u32..1000 {
                let result = check_count(current, baseline);
                if let CountStatus::Ratchet { new_baseline } = result {
                    assert!(
                        new_baseline <= baseline,
                        "Ratchet loosened: current={}, baseline={}, new_baseline={}",
                        current,
                        baseline,
                        new_baseline
                    );
                }
            }
        }
    }

    /// Property: Idempotency - current == baseline always means Pass
    #[test]
    fn property_idempotent_pass() {
        for baseline in 0u32..500 {
            let result = check_count(baseline, baseline);
            assert_eq!(
                result,
                CountStatus::Pass,
                "baseline={} should always Pass",
                baseline
            );
            for current in 0u32..500 {
                if current == baseline {
                    assert_eq!(
                        check_count(current, baseline),
                        CountStatus::Pass,
                        "current={} == baseline={} should Pass",
                        current,
                        baseline
                    );
                }
            }
        }
    }

    /// Property: Ratchet only fires when current < baseline
    #[test]
    fn property_ratchet_only_when_lower() {
        for baseline in 0u32..500 {
            for current in 0u32..500 {
                let result = check_count(current, baseline);
                if let CountStatus::Ratchet { new_baseline } = result {
                    assert!(
                        current < baseline,
                        "Ratchet fired when current={} >= baseline={}",
                        current,
                        baseline
                    );
                    assert_eq!(
                        new_baseline, current,
                        "Ratchet new_baseline={} should equal current={}",
                        new_baseline, current
                    );
                }
            }
        }
    }

    /// Property: Fail only occurs when current > baseline
    #[test]
    fn property_fail_only_when_higher() {
        for baseline in 0u32..500 {
            for current in 0u32..500 {
                let result = check_count(current, baseline);
                if matches!(result, CountStatus::Fail) {
                    assert!(
                        current > baseline,
                        "Fail occurred when current={} <= baseline={}",
                        current,
                        baseline
                    );
                }
            }
        }
    }

    /// Property: After Ratchet fires and "updates" baseline, same current should Pass
    #[test]
    fn property_ratchet_after_update_passes() {
        for original_baseline in 10u32..500 {
            for current in 0u32..original_baseline {
                let first_result = check_count(current, original_baseline);
                assert_eq!(
                    first_result,
                    CountStatus::Ratchet {
                        new_baseline: current
                    },
                    "First check should Ratchet: current={}, baseline={}",
                    current,
                    original_baseline
                );
                let second_result = check_count(current, current);
                assert_eq!(
                    second_result,
                    CountStatus::Pass,
                    "After ratchet update, same current should Pass: current={}",
                    current
                );
            }
        }
    }

    /// Property: check_count is a pure function of current vs baseline comparison
    #[test]
    fn property_check_count_determined_by_comparison() {
        for baseline in 0u32..200 {
            for current in 0u32..200 {
                let result = check_count(current, baseline);
                let cmp = current.cmp(&baseline);
                let expected = match cmp {
                    std::cmp::Ordering::Greater => CountStatus::Fail,
                    std::cmp::Ordering::Less => CountStatus::Ratchet {
                        new_baseline: current,
                    },
                    std::cmp::Ordering::Equal => CountStatus::Pass,
                };
                assert_eq!(
                    result, expected,
                    "check_count({}, {}) = {:?}, expected {:?}",
                    current, baseline, result, expected
                );
            }
        }
    }

    /// Property: parse_baseline roundtrip with newlines
    #[test]
    fn property_parse_write_roundtrip() {
        let test_values = [
            0u32,
            1,
            42,
            98,
            100,
            1000,
            9999,
            100000,
            u32::MAX,
            u32::MAX - 1,
            1 << 20,
        ];
        for value in test_values {
            let written = format!("{}\n", value);
            let parsed = parse_baseline(&written);
            assert_eq!(
                parsed,
                Some(value),
                "Roundtrip failed for value {}: wrote '{}', parsed back as {:?}",
                value,
                written.trim(),
                parsed
            );
            let written_no_nl = format!("{}", value);
            let parsed_no_nl = parse_baseline(&written_no_nl);
            assert_eq!(
                parsed_no_nl,
                Some(value),
                "Roundtrip (no newline) failed for value {}",
                value
            );
        }
    }

    /// Property: parse_baseline rejects invalid inputs
    #[test]
    fn property_parse_rejects_invalid() {
        let invalid_inputs = [
            "",
            "   ",
            "\t",
            "\n",
            "abc",
            "12abc",
            "abc123",
            "-1",
            "-42",
            "3.14",
            "1e10",
            "4294967296",
            "99999999999999999999999",
        ];
        for input in invalid_inputs {
            let result = parse_baseline(input);
            assert!(
                result.is_none(),
                "parse_baseline({:?}) should return None, got {:?}",
                input,
                result
            );
        }
    }

    /// Property: write_baseline creates newline-terminated output
    #[test]
    fn property_write_newline_terminated() {
        let temp_dir = std::env::temp_dir();
        for value in [0u32, 42, 100, u32::MAX] {
            let temp_path = temp_dir.join(format!("prop_baseline_{}.txt", value));
            write_baseline(&temp_path, value).expect("write_baseline should succeed");
            let contents = std::fs::read_to_string(&temp_path).expect("file should be readable");
            assert!(
                contents.ends_with('\n'),
                "write_baseline({}) should create newline-terminated file, got {:?}",
                value,
                contents
            );
            let trimmed = contents.trim();
            assert_eq!(
                trimmed.parse::<u32>().ok(),
                Some(value),
                "Content {:?} should parse to {}",
                contents,
                value
            );
            std::fs::remove_file(&temp_path).ok();
        }
    }

    /// Property: write/read roundtrip across a wide range of values
    #[test]
    fn property_write_read_roundtrip() {
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("prop_roundtrip_baseline.txt");
        let test_values: Vec<u32> = vec![
            0,
            1,
            2,
            10,
            42,
            50,
            81,
            98,
            100,
            500,
            1000,
            5000,
            10000,
            u32::MAX,
            u32::MAX - 1,
            1 << 20,
        ];
        for value in test_values {
            write_baseline(&temp_path, value).expect("write should succeed");
            let read_value = read_baseline(&temp_path)
                .unwrap_or_else(|e| panic!("read_baseline failed for value {}: {}", value, e));
            assert_eq!(
                read_value, value,
                "Roundtrip failed: wrote {}, read back {}",
                value, read_value
            );
        }
    }

    /// Property: read_baseline fails for various invalid contents
    #[test]
    fn property_read_baseline_fails_for_various_invalid() {
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("prop_invalid_baseline.txt");
        let invalid_contents = [
            "",
            "   \t\n",
            "not-a-number",
            "12.34",
            "-1",
            "abc",
            "12abc",
            "\x00",
        ];
        for content in invalid_contents {
            std::fs::write(&temp_path, content).expect("write should succeed");
            let result = read_baseline(&temp_path);
            assert!(
                result.is_err(),
                "read_baseline({:?}) should fail, got {:?}",
                content,
                result
            );
        }
    }
}
