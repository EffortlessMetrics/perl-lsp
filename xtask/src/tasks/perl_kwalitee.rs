//! `cargo xtask perl-kwalitee` — Perl distribution Kwalitee evaluation.
//!
//! This is the repo-local wrapper around the [`perl_kwalitee`] crate. The crate
//! owns the indicator model, scoring, profiles, and receipt schema and is
//! deliberately pure (no process spawning, no network). This module supplies
//! the parts that require touching the live repository:
//!
//! - repository paths (workspace root, default receipt locations),
//! - the current git commit and a timestamp for the receipt envelope,
//! - the results of the heavier gates the crate does not run itself
//!   (`release artifact-check`, `update-status --check`), fed in as
//!   [`ExternalResult`]s.
//!
//! Receipt-backed indicators (native-tooling readiness, quality-gate) are read
//! by the crate directly from their JSON receipts when present; this wrapper
//! only points it at the default receipt paths.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::ValueEnum;
use color_eyre::eyre::{Result, bail};

use perl_kwalitee::{
    EvidencePaths, EvidenceRef, ExternalResult, KwaliteeOptions, KwaliteeProfile, KwaliteeReceipt,
    evaluate, explain as explain_indicator, indicator_ids,
};

use crate::tasks::{release_artifact_check, update_status};
use crate::utils::project_root;

/// Default receipt locations, relative to the workspace root.
const READINESS_RECEIPT_REL: &str = "target/receipts/native-tooling/readiness.json";
const QUALITY_GATE_RECEIPT_REL: &str = "target/receipts/quality/quality-gate.json";
const FORMAT_CORPUS_RECEIPT_REL: &str = "target/receipts/format/native-format-corpus.json";
const CRITIC_FALSE_POSITIVE_RECEIPT_REL: &str =
    "target/receipts/native-tooling/native-critic-false-positive.json";
const PERLTIDY_COMPAT_RECEIPT_REL: &str =
    "target/receipts/format/native-format-perltidy-compat.json";
const PERLCRITIC_COMPAT_RECEIPT_REL: &str = "target/receipts/native-tooling/perlcritic-compat.json";
const DEFAULT_JSON_REL: &str = "target/receipts/kwalitee/perl-kwalitee.json";
const DEFAULT_MARKDOWN_REL: &str = "target/receipts/kwalitee/perl-kwalitee.md";

/// CLI-facing profile mirror of [`perl_kwalitee::KwaliteeProfile`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum PerlKwaliteeProfile {
    /// Fast per-PR profile; release-artifact indicators are not applicable.
    Pr,
    /// Strict release profile; requires `--dist`.
    Release,
    /// Broad nightly profile.
    Nightly,
}

impl PerlKwaliteeProfile {
    fn to_lib(self) -> KwaliteeProfile {
        match self {
            PerlKwaliteeProfile::Pr => KwaliteeProfile::Pr,
            PerlKwaliteeProfile::Release => KwaliteeProfile::Release,
            PerlKwaliteeProfile::Nightly => KwaliteeProfile::Nightly,
        }
    }
}

/// `cargo xtask perl-kwalitee check` — evaluate and fail on a non-clean verdict.
///
/// `repo_root` overrides the tree being evaluated (see [`resolve_root`]).
pub fn check(
    profile: PerlKwaliteeProfile,
    dist: Option<PathBuf>,
    strict: bool,
    repo_root: Option<PathBuf>,
) -> Result<()> {
    let (root, live) = resolve_root(repo_root)?;
    let receipt = build_and_evaluate(&root, live, profile, dist, strict)?;

    println!("{}", receipt.to_markdown());
    println!(
        "Perl Kwalitee: {} (score {}/100, profile {})",
        receipt.verdict.label(),
        receipt.score,
        receipt.profile
    );

    if receipt.verdict.is_failure() {
        bail!(
            "Perl Kwalitee check failed (verdict {}, score {}/100): {} mandatory failed, {} unverified",
            receipt.verdict.label(),
            receipt.score,
            receipt.mandatory_failed_count,
            receipt.unverified_count
        );
    }
    Ok(())
}

/// `cargo xtask perl-kwalitee report` — evaluate and write JSON + Markdown
/// receipts. Reporting does not fail the process on a non-clean verdict; use
/// `check` as the gate.
pub fn report(
    profile: PerlKwaliteeProfile,
    dist: Option<PathBuf>,
    json: PathBuf,
    markdown: PathBuf,
    repo_root: Option<PathBuf>,
) -> Result<()> {
    let (root, live) = resolve_root(repo_root)?;
    // Report is not strict — it records the state, it does not gate.
    let receipt = build_and_evaluate(&root, live, profile, dist, false)?;

    write_file(&json, &receipt.to_json_pretty()?)?;
    write_file(&markdown, &receipt.to_markdown())?;

    println!(
        "Perl Kwalitee report written: {} / {} — {} (score {}/100)",
        json.display(),
        markdown.display(),
        receipt.verdict.label(),
        receipt.score
    );
    Ok(())
}

/// `cargo xtask perl-kwalitee explain <indicator-id>`.
pub fn explain(id: &str) -> Result<()> {
    match explain_indicator(id) {
        Some(e) => {
            println!("{}  [{}]", e.id, e.area);
            println!("  {}", e.title);
            println!("  mandatory: {}", e.mandatory);
            println!();
            println!("  Why: {}", e.rationale);
            println!();
            println!("  Fix: {}", e.remediation);
            Ok(())
        }
        None => {
            eprintln!("Unknown indicator id: {id}");
            eprintln!("Known indicators:");
            for known in indicator_ids() {
                eprintln!("  {known}");
            }
            bail!("unknown indicator id `{id}`");
        }
    }
}

/// Resolve the tree to evaluate.
///
/// Returns `(root, live)`. With no override we evaluate the live workspace
/// (`live = true`) and run the shell-out gates. With an explicit `--repo-root`
/// we evaluate that tree using only native + receipt-backed indicators
/// (`live = false`) — the live-repo gates (`update-status`) assume the current
/// workspace and are skipped.
fn resolve_root(repo_root: Option<PathBuf>) -> Result<(PathBuf, bool)> {
    match repo_root {
        Some(root) => {
            // Fail loudly on a typo/missing path rather than silently evaluating
            // an empty tree (every native indicator would just read missing
            // files and report Unverified, letting a non-strict `check` pass
            // without having evaluated anything).
            if !root.is_dir() {
                bail!("--repo-root {} is not an existing directory", root.display());
            }
            Ok((root, false))
        }
        None => Ok((project_root()?, true)),
    }
}

/// Assemble [`KwaliteeOptions`], run the external gates, and evaluate.
fn build_and_evaluate(
    root: &Path,
    live: bool,
    profile: PerlKwaliteeProfile,
    dist: Option<PathBuf>,
    strict: bool,
) -> Result<KwaliteeReceipt> {
    let lib_profile = profile.to_lib();
    let commit = current_commit(root);

    let evidence = EvidencePaths {
        native_tooling_readiness: existing(root.join(READINESS_RECEIPT_REL)),
        quality_gate_receipt: existing(root.join(QUALITY_GATE_RECEIPT_REL)),
        native_format_corpus: existing(root.join(FORMAT_CORPUS_RECEIPT_REL)),
        native_critic_false_positive: existing(root.join(CRITIC_FALSE_POSITIVE_RECEIPT_REL)),
        native_format_perltidy_compat: existing(root.join(PERLTIDY_COMPAT_RECEIPT_REL)),
        native_tooling_perlcritic_compat: existing(root.join(PERLCRITIC_COMPAT_RECEIPT_REL)),
    };

    let mut external_results = BTreeMap::new();
    // `update-status --check` targets the live workspace, so only run it when
    // evaluating the live repo; under a `--repo-root` override docs.status_current
    // is left unverified rather than measured against the wrong tree.
    if live {
        add_docs_status_result(&mut external_results);
    }
    if lib_profile.requires_release_artifacts() {
        add_release_results(&mut external_results, dist.clone());
    }

    let options = KwaliteeOptions {
        repo_root: root.to_path_buf(),
        profile: lib_profile,
        dist_dir: dist,
        strict,
        commit,
        generated_at: timestamp(),
        evidence,
        external_results,
    };

    Ok(evaluate(&options))
}

/// Run `release artifact-check` once and map its single pass/fail result onto
/// the three release indicators (the gate validates all three concerns in one
/// pass, so they share the outcome).
fn add_release_results(results: &mut BTreeMap<String, ExternalResult>, dist: Option<PathBuf>) {
    let Some(dist) = dist else {
        // No dist: leave the release indicators unset so the crate marks them
        // Fail with the "--dist required" remediation.
        return;
    };

    let cmd = format!("cargo xtask release artifact-check --dist {}", dist.display());
    let run = release_artifact_check::run(release_artifact_check::Config {
        dist,
        contract: None,
        version: None,
        allow_partial: false,
    });

    let evidence = vec![EvidenceRef::command(cmd)];
    for id in [
        "release.native_binaries_present",
        "release.no_external_tooling",
        "release.checksums_valid",
    ] {
        results.insert(
            id.to_string(),
            ExternalResult::from_gate(
                run.as_ref().map(|_| ()).map_err(|e| e.to_string()),
                evidence.clone(),
                "Run `cargo xtask release artifact-check --dist <dir>` and resolve the reported \
                 release-archive violations.",
            ),
        );
    }
}

/// Run `update-status --check` and record the docs.status_current result.
///
/// Note: `update-status --check` returns `Err` both for genuine doc drift and
/// for a tooling failure; we map any `Err` to Fail (drift is the expected
/// common case). This is why the Kwalitee CI job starts advisory — a tooling
/// error would otherwise spuriously fail a mandatory indicator. The appended
/// error `note` distinguishes the two on inspection.
fn add_docs_status_result(results: &mut BTreeMap<String, ExternalResult>) {
    let run = update_status::run(false, true, None);
    let evidence = vec![EvidenceRef::command("cargo xtask update-status --check")];
    results.insert(
        "docs.status_current".to_string(),
        ExternalResult::from_gate(
            run.map_err(|e| e.to_string()),
            evidence,
            "Run `cargo xtask update-status --check`; regenerate with `--write` if drift is \
             reported.",
        ),
    );
}

/// Default JSON receipt path relative to the workspace root.
pub fn default_json_path(root: &Path) -> PathBuf {
    root.join(DEFAULT_JSON_REL)
}

/// Default Markdown receipt path relative to the workspace root.
pub fn default_markdown_path(root: &Path) -> PathBuf {
    root.join(DEFAULT_MARKDOWN_REL)
}

/// `Some(path)` if it exists on disk, else `None`.
fn existing(path: PathBuf) -> Option<PathBuf> {
    if path.exists() { Some(path) } else { None }
}

/// Write `contents` to `path`, creating parent directories as needed.
fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, contents)?;
    Ok(())
}

/// Git HEAD of the evaluated tree; `"unknown"` when `root` is not the top level
/// of a git repo or git is unavailable.
///
/// `git -C <root> rev-parse HEAD` walks *up* the directory tree, so if `root`
/// is a subdirectory of a repo (e.g. `--repo-root crates/foo` or a `target/`
/// subdir of the live workspace) it would return the parent repo's HEAD. Guard
/// with `--show-toplevel` and only stamp HEAD when `root` is itself the repo
/// top level; otherwise the receipt records `"unknown"` rather than leaking an
/// unrelated commit.
fn current_commit(root: &Path) -> String {
    let git = |args: &[&str]| -> Option<String> {
        Command::new("git")
            .current_dir(root)
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };

    let is_own_toplevel = git(&["rev-parse", "--show-toplevel"])
        .is_some_and(|top| std::fs::canonicalize(&top).ok() == std::fs::canonicalize(root).ok());
    if !is_own_toplevel {
        return "unknown".to_string();
    }

    git(&["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".to_string())
}

/// RFC 3339 timestamp for the receipt envelope.
fn timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_maps_to_lib() {
        assert_eq!(PerlKwaliteeProfile::Pr.to_lib(), KwaliteeProfile::Pr);
        assert_eq!(PerlKwaliteeProfile::Release.to_lib(), KwaliteeProfile::Release);
        assert_eq!(PerlKwaliteeProfile::Nightly.to_lib(), KwaliteeProfile::Nightly);
    }

    #[test]
    fn default_paths_are_under_receipts_kwalitee() {
        let root = Path::new("/repo");
        assert!(default_json_path(root).ends_with("target/receipts/kwalitee/perl-kwalitee.json"));
        assert!(default_markdown_path(root).ends_with("target/receipts/kwalitee/perl-kwalitee.md"));
    }

    #[test]
    fn existing_reflects_disk_state() {
        let dir = tempfile::tempdir().expect("tmp");
        let missing = dir.path().join("nope.json");
        assert!(existing(missing).is_none());
        let present = dir.path().join("here.json");
        std::fs::write(&present, "{}").expect("write");
        assert_eq!(existing(present.clone()), Some(present));
    }

    #[test]
    fn explain_rejects_unknown_indicator() {
        assert!(explain("does.not.exist").is_err());
    }
}
