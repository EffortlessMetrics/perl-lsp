//! `cargo xtask pr-close-proof` — canonical merge-base proof verifier.
//!
//! Implements the mandatory proof check from
//! [`docs/agents/CLOSE_PROOF_POLICY.md`](../../docs/agents/CLOSE_PROOF_POLICY.md):
//!
//! > Any close that claims "superseded", "already landed", or "duplicate of
//! > merged PR" **MUST** include the output of
//! > `git merge-base --is-ancestor <commit-sha> origin/main` pasted verbatim
//! > in the close comment.
//!
//! # Subcommand
//!
//! ```text
//! cargo xtask pr-close-proof --commit <sha> --canonical-main <ref>
//!                            [--substance-grep <string>]
//!                            [--format json]
//! ```
//!
//! # Exit codes
//!
//! | Code | Meaning |
//! |------|---------|
//! | `0` | Commit **is** an ancestor of canonical-main (safe to close) |
//! | `1` | Error (git not available, bad SHA format, I/O failure) |
//! | `2` | Commit is **not** an ancestor — do not close |
//!
//! Exit code 2 is distinct from 1 (error) so callers can branch on ancestry
//! without conflating "not reachable" with "git failed".
//!
//! # Output (human, default)
//!
//! ```text
//! REACHABLE   sha abc1234 is ancestor of origin/main
//! ```
//! or
//! ```text
//! NOT-REACHABLE   sha abc1234 is NOT ancestor of origin/main
//!                 allowed_close_reasons: []
//! ```
//!
//! # Output (--format json)
//!
//! ```json
//! {
//!   "reachable": true,
//!   "commit": "abc1234",
//!   "canonical_main": "origin/main",
//!   "allowed_close_reasons": ["superseded", "already-landed"],
//!   "content_survives": null
//! }
//! ```
//! `content_survives` is only present when `--substance-grep` is passed.

use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

// ---------------------------------------------------------------------------
// Public config
// ---------------------------------------------------------------------------

/// Output format for `pr-close-proof`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CloseProofFormat {
    Human,
    Json,
}

/// Configuration for `pr-close-proof`.
pub struct CloseProofConfig {
    /// The commit SHA to check.
    pub commit: String,
    /// The canonical main ref (e.g. `origin/main`).
    pub canonical_main: String,
    /// Optional substance grep string. When present, additionally runs
    /// `git grep -F <string> <canonical_main>` to detect the
    /// `in-ancestry-but-content-overwritten` class.
    pub substance_grep: Option<String>,
    /// Output format.
    pub format: CloseProofFormat,
}

// ---------------------------------------------------------------------------
// Output type
// ---------------------------------------------------------------------------

/// Machine-readable output for `--format json`.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct CloseProofOutput {
    /// Whether `commit` is an ancestor of `canonical_main`.
    pub reachable: bool,
    /// The commit SHA that was checked.
    pub commit: String,
    /// The canonical main ref that was checked against.
    pub canonical_main: String,
    /// Allowed close reasons when reachable; empty when not reachable.
    pub allowed_close_reasons: Vec<String>,
    /// Whether the substance grep string still exists in `canonical_main`.
    /// `None` when `--substance-grep` was not provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_survives: Option<bool>,
}

// ---------------------------------------------------------------------------
// Entry point — returns ExitCode so main.rs can propagate the exit code
// correctly without process::exit
// ---------------------------------------------------------------------------

/// Run the close-proof check.
///
/// Exit behaviour (caller must honour the returned value):
/// - `Ok(true)` — commit **is** ancestor; caller exits 0
/// - `Ok(false)` — commit is **not** ancestor; caller exits 2
/// - `Err(_)` — git error; caller exits 1
///
/// The caller (main.rs dispatch) is responsible for calling
/// `std::process::exit(2)` when `Ok(false)` is returned, keeping the
/// exit-code contract (0/1/2) intact.
pub fn run(config: CloseProofConfig) -> Result<bool> {
    let reachable = check_ancestry(&config.commit, &config.canonical_main)?;

    let content_survives = if let Some(ref grep_str) = config.substance_grep {
        Some(check_substance(grep_str, &config.canonical_main)?)
    } else {
        None
    };

    let allowed_close_reasons: Vec<String> = if reachable {
        vec!["superseded".to_string(), "already-landed".to_string()]
    } else {
        Vec::new()
    };

    let output = CloseProofOutput {
        reachable,
        commit: config.commit.clone(),
        canonical_main: config.canonical_main.clone(),
        allowed_close_reasons,
        content_survives,
    };

    match config.format {
        CloseProofFormat::Json => {
            let json = serde_json::to_string_pretty(&output).context("serializing JSON output")?;
            println!("{json}");
        }
        CloseProofFormat::Human => {
            print_human(&output);
        }
    }

    Ok(reachable)
}

// ---------------------------------------------------------------------------
// git operations
// ---------------------------------------------------------------------------

/// Returns `true` if `commit` is an ancestor of `canonical_main`.
///
/// Runs: `git merge-base --is-ancestor <commit> <canonical_main>`
/// Exit 0 → is ancestor; exit 1 → not ancestor; other → error.
fn check_ancestry(commit: &str, canonical_main: &str) -> Result<bool> {
    validate_sha_format(commit)?;

    let status = Command::new("git")
        .args(["merge-base", "--is-ancestor", commit, canonical_main])
        .status()
        .with_context(|| {
            format!("running `git merge-base --is-ancestor {commit} {canonical_main}`")
        })?;

    match status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        Some(code) => {
            color_eyre::eyre::bail!(
                "`git merge-base --is-ancestor` exited with unexpected code {code}"
            )
        }
        None => color_eyre::eyre::bail!("`git merge-base --is-ancestor` killed by signal"),
    }
}

/// Returns `true` if `grep_str` appears in `canonical_main` (Rule 3 of CLOSE_PROOF_POLICY.md).
///
/// Runs: `git grep -F <grep_str> <canonical_main>`
fn check_substance(grep_str: &str, canonical_main: &str) -> Result<bool> {
    let output = Command::new("git")
        .args(["grep", "-F", grep_str, canonical_main])
        .output()
        .with_context(|| format!("running `git grep -F {grep_str:?} {canonical_main}`"))?;

    // exit 0 = found, exit 1 = not found, other = error
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        Some(code) => {
            color_eyre::eyre::bail!("`git grep` exited with unexpected code {code}")
        }
        None => color_eyre::eyre::bail!("`git grep` killed by signal"),
    }
}

/// Minimal SHA validation — rejects obviously wrong inputs (empty, contains spaces,
/// obviously not hex).  Does not require exactly 40 chars to support short SHAs.
fn validate_sha_format(sha: &str) -> Result<()> {
    if sha.trim().is_empty() {
        color_eyre::eyre::bail!("commit SHA must not be empty");
    }
    if sha.contains(' ') || sha.contains('\t') {
        color_eyre::eyre::bail!("commit SHA must not contain whitespace: {sha:?}");
    }
    // Accept hex chars only (short or full SHA).
    if !sha.chars().all(|c| c.is_ascii_hexdigit()) {
        color_eyre::eyre::bail!("commit SHA must be hex characters only, got: {sha:?}");
    }
    if sha.len() < 7 {
        color_eyre::eyre::bail!("commit SHA too short (minimum 7 chars): {sha:?}");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Human output
// ---------------------------------------------------------------------------

fn print_human(output: &CloseProofOutput) {
    if output.reachable {
        println!("REACHABLE   {} is ancestor of {}", output.commit, output.canonical_main);
        println!(
            "            allowed_close_reasons: [{}]",
            output.allowed_close_reasons.join(", ")
        );
        if let Some(survives) = output.content_survives {
            if survives {
                println!(
                    "            content_survives: YES — substance grep found in {}",
                    output.canonical_main
                );
            } else {
                println!(
                    "WARNING     content_survives: NO — commit is ancestor but substance was overwritten"
                );
                println!(
                    "            Class: in-ancestry-but-content-overwritten (see CLOSE_PROOF_POLICY.md Rule 3)"
                );
            }
        }
    } else {
        println!(
            "NOT-REACHABLE   {} is NOT an ancestor of {}",
            output.commit, output.canonical_main
        );
        println!("                allowed_close_reasons: []");
        println!("                Do NOT close — commit has not landed on canonical main.");
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ----- SHA validation ---------------------------------------------------

    #[test]
    fn test_validate_sha_empty_fails() -> Result<()> {
        assert!(validate_sha_format("").is_err());
        assert!(validate_sha_format("   ").is_err());
        Ok(())
    }

    #[test]
    fn test_validate_sha_with_spaces_fails() -> Result<()> {
        assert!(validate_sha_format("abc 123").is_err());
        Ok(())
    }

    #[test]
    fn test_validate_sha_non_hex_fails() -> Result<()> {
        assert!(validate_sha_format("not-a-sha").is_err());
        assert!(validate_sha_format("zzzzzzzz").is_err());
        Ok(())
    }

    #[test]
    fn test_validate_sha_too_short_fails() -> Result<()> {
        assert!(validate_sha_format("abc123").is_err()); // 6 chars
        Ok(())
    }

    #[test]
    fn test_validate_sha_valid_short() -> Result<()> {
        validate_sha_format("abc1234")?; // 7-char short SHA
        Ok(())
    }

    #[test]
    fn test_validate_sha_valid_full() -> Result<()> {
        validate_sha_format("4b271dff5a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d")?;
        Ok(())
    }

    // ----- Output structure -------------------------------------------------

    #[test]
    fn test_output_reachable_has_close_reasons() -> Result<()> {
        let output = CloseProofOutput {
            reachable: true,
            commit: "abc1234".to_string(),
            canonical_main: "origin/main".to_string(),
            allowed_close_reasons: vec!["superseded".to_string(), "already-landed".to_string()],
            content_survives: None,
        };
        assert_eq!(output.allowed_close_reasons.len(), 2);
        assert!(output.allowed_close_reasons.contains(&"superseded".to_string()));
        assert!(output.allowed_close_reasons.contains(&"already-landed".to_string()));
        Ok(())
    }

    #[test]
    fn test_output_not_reachable_empty_close_reasons() -> Result<()> {
        let output = CloseProofOutput {
            reachable: false,
            commit: "deadbeef1".to_string(),
            canonical_main: "origin/main".to_string(),
            allowed_close_reasons: Vec::new(),
            content_survives: None,
        };
        assert!(output.allowed_close_reasons.is_empty());
        Ok(())
    }

    #[test]
    fn test_output_json_serialization_roundtrip() -> Result<()> {
        let output = CloseProofOutput {
            reachable: true,
            commit: "abc1234d".to_string(),
            canonical_main: "origin/main".to_string(),
            allowed_close_reasons: vec!["superseded".to_string()],
            content_survives: Some(true),
        };
        let json = serde_json::to_string(&output)?;
        let parsed: CloseProofOutput = serde_json::from_str(&json)?;
        assert_eq!(output, parsed);
        Ok(())
    }

    #[test]
    fn test_content_survives_omitted_when_none() -> Result<()> {
        let output = CloseProofOutput {
            reachable: true,
            commit: "abc1234d".to_string(),
            canonical_main: "origin/main".to_string(),
            allowed_close_reasons: vec![],
            content_survives: None,
        };
        let json = serde_json::to_string(&output)?;
        assert!(!json.contains("content_survives"), "should be omitted when None");
        Ok(())
    }

    // ----- Live repo ancestry checks ----------------------------------------
    //
    // Uses known commits from the repo's own history.  These are integration
    // tests — they shell to git and require the repo to be available.

    #[test]
    fn test_current_head_is_ancestor_of_itself() -> Result<()> {
        let output = Command::new("git").args(["rev-parse", "--verify", "HEAD^{commit}"]).output();
        let Ok(output) = output else {
            // git not available — skip
            return Ok(());
        };
        if !output.status.success() {
            // not a git checkout, shallow probe failure, etc. — skip
            return Ok(());
        }

        let sha = String::from_utf8(output.stdout)?.trim().to_string();
        if sha.is_empty() {
            return Ok(());
        }

        let reachable = check_ancestry(&sha, &sha)?;
        assert!(reachable, "current HEAD should be an ancestor of itself");
        Ok(())
    }

    #[test]
    fn test_fabricated_sha_is_not_ancestor_or_error() -> Result<()> {
        // A SHA that cannot exist in the repo.  Either "not ancestor" (exit 1)
        // or "object not found" which git reports as exit 128.
        // We accept both: check_ancestry returns Ok(false) or Err.
        let sha = "0000000000000000000000000000000000000000";
        // validate_sha_format should pass (it's hex).
        validate_sha_format(sha)?;
        // The actual ancestry check may error or return false — both are fine.
        // We just verify it doesn't claim "reachable".
        match check_ancestry(sha, "origin/main") {
            Ok(true) => panic!("all-zeros SHA should not be an ancestor"),
            Ok(false) => {} // expected: not ancestor
            Err(_) => {}    // also fine: git rejected it
        }
        Ok(())
    }
}
