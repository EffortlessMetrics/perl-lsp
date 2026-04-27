//! Parser Ratchet — scoped pre-merge CI lane.
//!
//! Currently a no-op scaffolding implementation. Future PRs will add:
//! - PR 5: scope selection via ci-scope (decide selected based on changed paths)
//! - PR 6: live base-vs-head metric comparison
//! - PR 7: perl-corpus concept-tagged fixtures
//! - PR 8: system Perl corpus discovery
//! - PR 9: golden receipt tests
//! - PR 10: hard enforcement
//!
//! See issue #6847 for the full design.

use color_eyre::eyre::{Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The stable receipt schema emitted after every run.
#[derive(Debug, Serialize)]
pub struct Receipt {
    pub check: &'static str,
    pub schema_version: u32,
    pub event: String,
    pub base_sha: String,
    pub head_sha: String,
    pub selected: bool,
    pub reason: String,
    pub profile: String,
    pub verdict: &'static str,
    pub repro: Repro,
}

/// Reproducibility hint included in the receipt.
#[derive(Debug, Serialize)]
pub struct Repro {
    pub command: String,
}

/// Arguments for the parser-ratchet subcommand.
pub struct ParserRatchetArgs {
    pub profile: String,
    pub base: String,
    pub receipt: PathBuf,
}

/// Entry point — always exits 0 in this scaffolding phase.
///
/// Emits a JSON receipt with `selected=false` until PR 5 wires ci-scope
/// selection. This establishes the stable `Parser Ratchet` check name that
/// subsequent PRs build upon without changing the workflow file.
pub fn run(args: ParserRatchetArgs) -> Result<()> {
    let event = std::env::var("GITHUB_EVENT_NAME").unwrap_or_else(|_| "local".to_string());
    let base_sha = resolve_base_sha(&args.base)?;
    let head_sha = resolve_head_sha()?;

    // Scaffolding: always not-selected until PR 5 wires ci-scope.
    let receipt = Receipt {
        check: "Parser Ratchet",
        schema_version: 1,
        event,
        base_sha,
        head_sha,
        selected: false,
        reason: "not_implemented_scope_default — see #6847 for redesign sequence".to_string(),
        profile: args.profile.clone(),
        verdict: "pass",
        repro: Repro {
            command: format!(
                "cargo run --locked -p xtask -- parser-ratchet --profile {} --base {}",
                args.profile, args.base
            ),
        },
    };

    write_receipt(&args.receipt, &receipt)?;
    eprintln!(
        "Parser Ratchet: selected=false (scaffolding); receipt at {}",
        args.receipt.display()
    );
    Ok(())
}

fn resolve_base_sha(base: &str) -> Result<String> {
    let output =
        Command::new("git").args(["rev-parse", base]).output().wrap_err("running git rev-parse")?;
    if !output.status.success() {
        // Detached HEAD or unresolved base — record as "unknown" not error.
        return Ok("unknown".to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn resolve_head_sha() -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .wrap_err("running git rev-parse HEAD")?;
    if !output.status.success() {
        return Ok("unknown".to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn write_receipt(path: &Path, receipt: &Receipt) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).wrap_err("creating receipt parent dir")?;
    }
    let json = serde_json::to_string_pretty(receipt).wrap_err("serializing receipt")?;
    std::fs::write(path, json).wrap_err("writing receipt")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn no_op_receipt_is_valid_json() -> Result<()> {
        let tmp = tempdir()?;
        let receipt_path = tmp.path().join("parser-ratchet.json");
        run(ParserRatchetArgs {
            profile: "pr".to_string(),
            base: "HEAD".to_string(),
            receipt: receipt_path.clone(),
        })?;
        let raw = std::fs::read_to_string(&receipt_path)?;
        let parsed: serde_json::Value = serde_json::from_str(&raw)?;
        assert_eq!(parsed["check"], "Parser Ratchet");
        assert_eq!(parsed["selected"], false);
        assert_eq!(parsed["verdict"], "pass");
        assert!(
            parsed["repro"]["command"]
                .as_str()
                .map(|s| s.contains("parser-ratchet"))
                .unwrap_or(false),
            "repro.command should contain 'parser-ratchet'"
        );
        Ok(())
    }

    #[test]
    fn receipt_contains_all_required_fields() -> Result<()> {
        let tmp = tempdir()?;
        let receipt_path = tmp.path().join("receipt.json");
        run(ParserRatchetArgs {
            profile: "pr".to_string(),
            base: "HEAD".to_string(),
            receipt: receipt_path.clone(),
        })?;
        let raw = std::fs::read_to_string(&receipt_path)?;
        let parsed: serde_json::Value = serde_json::from_str(&raw)?;
        // All required schema fields must be present
        assert!(parsed["check"].is_string());
        assert!(parsed["schema_version"].is_number());
        assert!(parsed["event"].is_string());
        assert!(parsed["base_sha"].is_string());
        assert!(parsed["head_sha"].is_string());
        assert!(parsed["selected"].is_boolean());
        assert!(parsed["reason"].is_string());
        assert!(parsed["profile"].is_string());
        assert!(parsed["verdict"].is_string());
        assert!(parsed["repro"].is_object());
        assert_eq!(parsed["schema_version"], 1);
        Ok(())
    }

    #[test]
    fn receipt_profile_is_preserved() -> Result<()> {
        let tmp = tempdir()?;
        let receipt_path = tmp.path().join("receipt.json");
        run(ParserRatchetArgs {
            profile: "nightly".to_string(),
            base: "HEAD".to_string(),
            receipt: receipt_path.clone(),
        })?;
        let raw = std::fs::read_to_string(&receipt_path)?;
        let parsed: serde_json::Value = serde_json::from_str(&raw)?;
        assert_eq!(parsed["profile"], "nightly");
        Ok(())
    }

    #[test]
    fn receipt_creates_parent_dirs() -> Result<()> {
        let tmp = tempdir()?;
        let nested = tmp.path().join("a").join("b").join("c").join("receipt.json");
        run(ParserRatchetArgs {
            profile: "pr".to_string(),
            base: "HEAD".to_string(),
            receipt: nested.clone(),
        })?;
        assert!(nested.exists(), "receipt file should exist after nested dir creation");
        Ok(())
    }
}
