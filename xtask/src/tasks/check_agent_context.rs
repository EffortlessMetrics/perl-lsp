//! Package-local agent-context coverage gate (M6, issue #3848 / epic #3612).
//!
//! Every workspace member (as reported by `cargo metadata --no-deps`) must be
//! accounted for as exactly one of:
//!   - **has-context**: a `CLAUDE.md` file exists in the crate directory.
//!   - **exempt**: a genuine infra/test-only crate with no product surface to
//!     document, listed in `.ci/policies/agent-context-policy.toml`. Permanent.
//!   - **needs_context**: a core-product crate that should have package-local
//!     context but does not yet, also listed in the policy file. This is
//!     explicit tracked debt -- the validator prints it loudly on every run
//!     and never treats it as satisfying the gate silently.
//!
//! A member found in none of the three buckets is **unaccounted** and fails
//! the gate. This is deliberate: it is the difference between "every crate
//! is accounted for, and the gap is visible" (honest) and "the check is
//! green because we quietly exempted the crates that should have context"
//! (gamed). See the M4-advisory lesson referenced in the M6 spec.
//!
//! ## M6 phase 2 (issue #3848): the "no volatile facts" + paths-exist bar
//!
//! Phase 1 only proved *coverage* (every member has-context/exempt/
//! needs_context). Phase 2 enforces the actual M6 exit criterion: package-local
//! context must load on demand and never go stale, which means it must not
//! embed facts that rot the moment a PR merges or a release ships. Every
//! `has_context` member's `CLAUDE.md` is additionally checked for:
//!
//!   - **no volatile facts**: PR/issue numbers, commit SHAs, release
//!     versions, calendar dates, and status/label words (see
//!     [`scan_volatile_facts`] for the exact patterns and the false-positive
//!     heuristics applied to each).
//!   - **paths exist**: every file path cited in a "Read first" or "Focused
//!     validation" section must resolve on disk, relative to either the
//!     crate directory or the repo root (see [`scan_dead_paths`]).
//!
//! `validate_policy` additionally requires every `needs_context` entry to
//! carry a `tracking_issue` (the coderabbit follow-up from #3876) -- tracked
//! debt without a tracking issue is not tracked debt, it is a TODO comment.

use crate::utils::{project_root, run_cargo_metadata};
use color_eyre::eyre::{Result, bail, eyre};
use regex::Regex;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

const POLICY_PATH: &str = ".ci/policies/agent-context-policy.toml";
const CONTEXT_FILE_NAME: &str = "CLAUDE.md";

/// Markdown `## ` section headings whose cited paths are in scope for the
/// paths-exist check. Case-insensitive, matched against the trimmed heading
/// text (with any leading `#`s and whitespace stripped).
const PATH_CHECKED_SECTIONS: &[&str] = &["read first", "focused validation"];

/// File extensions that make a slash-free backtick span (e.g. `` `Cargo.toml` ``)
/// a path candidate rather than a type/function name or crate identifier.
const PATH_LIKE_EXTENSIONS: &[&str] =
    &["rs", "md", "toml", "yaml", "yml", "py", "sh", "txt", "json", "lock"];

#[derive(Debug, Deserialize)]
struct CargoMetadata {
    packages: Vec<MetadataPackage>,
}

#[derive(Debug, Deserialize)]
struct MetadataPackage {
    name: String,
    manifest_path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct AgentContextPolicy {
    version: u64,
    #[serde(default)]
    exempt: Vec<PolicyEntry>,
    #[serde(default)]
    needs_context: Vec<PolicyEntry>,
}

#[derive(Debug, Deserialize)]
struct PolicyEntry {
    name: String,
    reason: String,
    #[serde(default)]
    tracking_issue: Option<u64>,
}

/// A workspace member reduced to what classification needs: its name,
/// whether a package-local `CLAUDE.md` already exists for it, and its crate
/// directory (needed to locate that file and resolve crate-relative paths
/// it cites).
#[derive(Debug, Clone)]
struct Member {
    name: String,
    has_context: bool,
    crate_dir: PathBuf,
}

/// Result of classifying every workspace member against the policy file.
#[derive(Debug, Default)]
struct Coverage {
    total: usize,
    has_context: Vec<String>,
    exempt: Vec<String>,
    needs_context: Vec<String>,
    unaccounted: Vec<String>,
}

/// A single volatile-fact hit: a line in a package-local `CLAUDE.md` that
/// matched one of the volatile-fact categories in [`scan_volatile_facts`].
#[derive(Debug, Clone)]
struct VolatileFactHit {
    member: String,
    path: PathBuf,
    line_number: usize,
    category: &'static str,
    line: String,
}

pub fn run() -> Result<()> {
    let root = project_root()?;
    let policy = load_policy(&root.join(POLICY_PATH))?;
    let members = load_members(&root)?;
    let coverage = classify(&members, &policy);

    print_report(&coverage, &policy);

    let volatile_hits = collect_volatile_fact_hits(&members)?;
    let dead_paths = collect_dead_paths(&members, &root)?;
    print_enforcement_report(&volatile_hits, &dead_paths);

    if !coverage.unaccounted.is_empty() {
        bail!(
            "agent-context gate: {} workspace member(s) unaccounted (neither {CONTEXT_FILE_NAME}, exempt, nor needs_context in {POLICY_PATH}): {}",
            coverage.unaccounted.len(),
            coverage.unaccounted.join(", ")
        );
    }

    if !volatile_hits.is_empty() || !dead_paths.is_empty() {
        bail!(
            "agent-context gate: {} volatile-fact hit(s) and {} dead path(s) in package-local {CONTEXT_FILE_NAME} files -- see report above",
            volatile_hits.len(),
            dead_paths.len()
        );
    }

    Ok(())
}

/// Scan every `has_context` member's `CLAUDE.md` for volatile facts.
fn collect_volatile_fact_hits(members: &[Member]) -> Result<Vec<VolatileFactHit>> {
    let mut hits = Vec::new();
    for member in members {
        if !member.has_context {
            continue;
        }
        let claude_md = member.crate_dir.join(CONTEXT_FILE_NAME);
        hits.extend(scan_volatile_facts(&member.name, &claude_md)?);
    }
    Ok(hits)
}

/// Scan every `has_context` member's `CLAUDE.md` for dead paths cited in a
/// "Read first" or "Focused validation" section.
fn collect_dead_paths(members: &[Member], repo_root: &Path) -> Result<Vec<String>> {
    let mut dead = Vec::new();
    for member in members {
        if !member.has_context {
            continue;
        }
        let claude_md = member.crate_dir.join(CONTEXT_FILE_NAME);
        dead.extend(scan_dead_paths(&member.name, &claude_md, &member.crate_dir, repo_root)?);
    }
    Ok(dead)
}

fn print_enforcement_report(volatile_hits: &[VolatileFactHit], dead_paths: &[String]) {
    if volatile_hits.is_empty() && dead_paths.is_empty() {
        println!();
        println!("no-volatile-facts + paths-exist: clean (no hits)");
        return;
    }

    if !volatile_hits.is_empty() {
        println!();
        println!("VOLATILE FACTS -- package-local CLAUDE.md must not embed facts that rot:");
        for hit in volatile_hits {
            println!(
                "  - {} {}:{} [{}] {}",
                hit.member,
                hit.path.display(),
                hit.line_number,
                hit.category,
                hit.line.trim()
            );
        }
    }

    if !dead_paths.is_empty() {
        println!();
        println!("DEAD PATHS -- cited in Read first / Focused validation but absent on disk:");
        for path in dead_paths {
            println!("  - {path}");
        }
    }
}

// ---------------------------------------------------------------------------
// No-volatile-facts check
// ---------------------------------------------------------------------------
//
// Each pattern below targets one volatile-fact category. Heuristics applied
// to reduce false positives:
//   - Fenced code blocks (```...```) are skipped entirely: a version number
//     or SHA inside a worked command example ("cargo build # v0.12.3") is a
//     literal example, not a stale prose claim, and is not what this gate is
//     for.
//   - The SHA pattern additionally requires the match contain at least one
//     hex letter (a-f); a bare run of 7+ digits (a line count, an iteration
//     count) is far more likely to be a plain number than a commit SHA.
//   - Status words are matched case-insensitively since prose capitalizes
//     freely ("Merged", "Status:").
//
// Known limitation (documented, not fixed): the version regex will also
// match a dotted-quad substring inside an IPv4 address (e.g. the "0.0.1" in
// "127.0.0.1"). No such text exists in the current corpus; if it ever does,
// prefer rewording the doc over relaxing this pattern.

#[allow(clippy::expect_used, reason = "static LazyLock regex with known-good pattern")]
static PR_ISSUE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"#\d{3,}").expect("static PR/issue regex is valid"));
#[allow(clippy::expect_used, reason = "static LazyLock regex with known-good pattern")]
static SHA_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b[0-9a-f]{7,40}\b").expect("static SHA regex is valid"));
#[allow(clippy::expect_used, reason = "static LazyLock regex with known-good pattern")]
static VERSION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bv?\d+\.\d+\.\d+\b").expect("static version regex is valid"));
#[allow(clippy::expect_used, reason = "static LazyLock regex with known-good pattern")]
static DATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b20\d\d-\d\d-\d\d\b").expect("static date regex is valid"));
#[allow(clippy::expect_used, reason = "static LazyLock regex with known-good pattern")]
static STATUS_WORD_RE: LazyLock<Regex> = LazyLock::new(|| {
    // `\b` before `needs-` so this only matches at a word boundary (e.g. not
    // a false hit on a substring like "kneads-foo").
    Regex::new(r"(?i)\b(?:merged|deep-reviewed)\b|\bneeds-[a-z]+|\bstatus:")
        .expect("static status-word regex is valid")
});

/// Classify one line against the volatile-fact patterns, in priority order.
/// Returns the first matching category name, or `None` if the line is clean.
fn classify_volatile_fact(line: &str) -> Option<&'static str> {
    if PR_ISSUE_RE.is_match(line) {
        return Some("pr-or-issue-number");
    }
    if let Some(m) = SHA_RE.find(line) {
        // Require at least one hex letter so plain multi-digit numbers
        // (line counts, iteration counts) aren't misread as commit SHAs.
        if m.as_str().chars().any(|c| c.is_ascii_alphabetic()) {
            return Some("commit-sha");
        }
    }
    if VERSION_RE.is_match(line) {
        return Some("release-version");
    }
    if DATE_RE.is_match(line) {
        return Some("calendar-date");
    }
    if STATUS_WORD_RE.is_match(line) {
        return Some("status-or-label-word");
    }
    None
}

/// Scan a package-local `CLAUDE.md` for volatile facts, skipping fenced code
/// blocks (a line whose trimmed content starts with ` ``` ` toggles fence
/// state). Returns one [`VolatileFactHit`] per offending line.
fn scan_volatile_facts(member_name: &str, path: &Path) -> Result<Vec<VolatileFactHit>> {
    let Ok(content) = fs::read_to_string(path) else {
        // A has_context member without a readable CLAUDE.md is a coverage
        // problem, not a volatile-facts problem -- load_members() already
        // requires the file to exist to set has_context, so this should be
        // unreachable in practice; treat it as clean rather than panicking.
        return Ok(Vec::new());
    };

    let mut hits = Vec::new();
    let mut in_fence = false;
    for (index, line) in content.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if let Some(category) = classify_volatile_fact(line) {
            hits.push(VolatileFactHit {
                member: member_name.to_string(),
                path: path.to_path_buf(),
                line_number: index + 1,
                category,
                line: line.to_string(),
            });
        }
    }
    Ok(hits)
}

// ---------------------------------------------------------------------------
// Paths-exist check
// ---------------------------------------------------------------------------

#[allow(clippy::expect_used, reason = "static LazyLock regex with known-good pattern")]
static BACKTICK_SPAN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"`([^`]+)`").expect("static backtick-span regex is valid"));

/// Whether a backtick-quoted span looks like a repo/crate-relative path
/// rather than a type name, crate identifier, or shell command. A span
/// qualifies if it uses only path-safe characters (alphanumeric, `_`, `.`,
/// `-`, `/`) AND either contains a `/` or ends in a known path-like
/// extension (see [`PATH_LIKE_EXTENSIONS`]). This deliberately excludes
/// spans with spaces (shell commands like `` `cargo test -p foo` ``),
/// `::` (Rust paths like `` `perllsp::run_cli` ``), and parens/backtick
/// call-syntax (`` `analyze_file(...)` ``).
fn is_path_candidate(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    let path_safe =
        text.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-' | '/'));
    if !path_safe {
        return false;
    }
    text.contains('/') || has_path_like_extension(text)
}

fn has_path_like_extension(text: &str) -> bool {
    match text.rsplit_once('.') {
        Some((_, ext)) => PATH_LIKE_EXTENSIONS.contains(&ext),
        None => false,
    }
}

/// Extract candidate paths cited in "Read first" / "Focused validation"
/// `## ` sections of a package-local `CLAUDE.md`. Fenced code blocks are
/// skipped, matching [`scan_volatile_facts`]'s fence-handling.
fn extract_cited_paths(content: &str) -> Vec<String> {
    let mut in_checked_section = false;
    let mut in_fence = false;
    let mut candidates = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if let Some(heading) = trimmed.strip_prefix("## ") {
            let heading = heading.trim().to_ascii_lowercase();
            in_checked_section = PATH_CHECKED_SECTIONS.contains(&heading.as_str());
            continue;
        }
        // Any other heading level ends the checked section too.
        if trimmed.starts_with('#') {
            in_checked_section = false;
            continue;
        }
        if !in_checked_section {
            continue;
        }
        for capture in BACKTICK_SPAN_RE.captures_iter(line) {
            if let Some(span) = capture.get(1) {
                let candidate = span.as_str();
                if is_path_candidate(candidate) {
                    candidates.push(candidate.to_string());
                }
            }
        }
    }

    candidates
}

/// Resolve a cited path candidate against the crate directory first, then
/// the repo root (paths like `crates/perl-lsp-rs/CLAUDE.md` are repo-root
/// relative; paths like `src/lib.rs` are crate-directory relative).
fn resolve_cited_path(candidate: &str, crate_dir: &Path, repo_root: &Path) -> bool {
    crate_dir.join(candidate).exists() || repo_root.join(candidate).exists()
}

fn scan_dead_paths(
    member_name: &str,
    claude_md: &Path,
    crate_dir: &Path,
    repo_root: &Path,
) -> Result<Vec<String>> {
    let Ok(content) = fs::read_to_string(claude_md) else {
        return Ok(Vec::new());
    };

    let mut dead = Vec::new();
    for candidate in extract_cited_paths(&content) {
        if !resolve_cited_path(&candidate, crate_dir, repo_root) {
            dead.push(format!("{member_name} {}: `{candidate}` (not found)", claude_md.display()));
        }
    }
    Ok(dead)
}

fn print_report(coverage: &Coverage, policy: &AgentContextPolicy) {
    let accounted =
        coverage.has_context.len() + coverage.exempt.len() + coverage.needs_context.len();
    println!(
        "agent-context coverage: {accounted}/{} workspace members accounted for",
        coverage.total
    );
    println!("  has {CONTEXT_FILE_NAME}:  {}", coverage.has_context.len());
    println!("  exempt (infra/test-only): {}", coverage.exempt.len());
    println!("  needs_context (core-product debt): {}", coverage.needs_context.len());

    if !coverage.needs_context.is_empty() {
        println!();
        println!(
            "TRACKED CONTEXT DEBT -- M6 (issue #3848) is NOT complete until this list is empty:"
        );
        for name in &coverage.needs_context {
            let issue = policy
                .needs_context
                .iter()
                .find(|entry| entry.name == *name)
                .and_then(|entry| entry.tracking_issue);
            match issue {
                Some(number) => println!("  - {name} (tracked in #{number})"),
                None => println!("  - {name}"),
            }
        }
    }

    if !coverage.unaccounted.is_empty() {
        println!();
        println!("UNACCOUNTED -- add {CONTEXT_FILE_NAME}, or classify in {POLICY_PATH}:");
        for name in &coverage.unaccounted {
            println!("  - {name}");
        }
    }
}

fn load_policy(path: &Path) -> Result<AgentContextPolicy> {
    let content = fs::read_to_string(path)
        .map_err(|error| eyre!("failed to read {}: {error}", path.display()))?;
    let policy: AgentContextPolicy = toml::from_str(&content)
        .map_err(|error| eyre!("failed to parse {}: {error}", path.display()))?;
    validate_policy(&policy, path)?;
    Ok(policy)
}

fn validate_policy(policy: &AgentContextPolicy, path: &Path) -> Result<()> {
    if policy.version != 1 {
        bail!("{} version must be 1", path.display());
    }

    let mut seen = BTreeSet::new();
    for entry in policy.exempt.iter().chain(policy.needs_context.iter()) {
        if entry.name.trim().is_empty() {
            bail!("{} has an entry with an empty name", path.display());
        }
        if entry.reason.trim().is_empty() {
            bail!("{} entry {} must have a non-empty reason", path.display(), entry.name);
        }
        if !seen.insert(entry.name.as_str()) {
            bail!(
                "{} lists {} more than once (exempt and needs_context must be disjoint, and each list must be duplicate-free)",
                path.display(),
                entry.name
            );
        }
    }

    for entry in &policy.needs_context {
        if entry.tracking_issue.is_none() {
            bail!(
                "{} needs_context entry {} must have a tracking_issue -- tracked debt without a tracking issue is not tracked",
                path.display(),
                entry.name
            );
        }
    }

    Ok(())
}

fn load_members(root: &Path) -> Result<Vec<Member>> {
    let bytes = run_cargo_metadata(true)?;
    let metadata: CargoMetadata = serde_json::from_slice(&bytes)
        .map_err(|error| eyre!("failed to parse cargo metadata: {error}"))?;

    let mut members = Vec::with_capacity(metadata.packages.len());
    for package in metadata.packages {
        let crate_dir = package
            .manifest_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| root.to_path_buf());
        let has_context = crate_dir.join(CONTEXT_FILE_NAME).is_file();
        members.push(Member { name: package.name, has_context, crate_dir });
    }
    Ok(members)
}

fn classify(members: &[Member], policy: &AgentContextPolicy) -> Coverage {
    let exempt_names: BTreeSet<&str> =
        policy.exempt.iter().map(|entry| entry.name.as_str()).collect();
    let needs_context_names: BTreeSet<&str> =
        policy.needs_context.iter().map(|entry| entry.name.as_str()).collect();

    let mut coverage = Coverage { total: members.len(), ..Coverage::default() };

    for member in members {
        if member.has_context {
            coverage.has_context.push(member.name.clone());
        } else if exempt_names.contains(member.name.as_str()) {
            coverage.exempt.push(member.name.clone());
        } else if needs_context_names.contains(member.name.as_str()) {
            coverage.needs_context.push(member.name.clone());
        } else {
            coverage.unaccounted.push(member.name.clone());
        }
    }

    coverage.has_context.sort();
    coverage.exempt.sort();
    coverage.needs_context.sort();
    coverage.unaccounted.sort();

    coverage
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(exempt: &[&str], needs_context: &[&str]) -> AgentContextPolicy {
        AgentContextPolicy {
            version: 1,
            exempt: exempt
                .iter()
                .map(|name| PolicyEntry {
                    name: (*name).to_string(),
                    reason: "test reason".to_string(),
                    tracking_issue: None,
                })
                .collect(),
            needs_context: needs_context
                .iter()
                .map(|name| PolicyEntry {
                    name: (*name).to_string(),
                    reason: "test reason".to_string(),
                    tracking_issue: Some(3874),
                })
                .collect(),
        }
    }

    fn member(name: &str, has_context: bool) -> Member {
        Member { name: name.to_string(), has_context, crate_dir: PathBuf::from(name) }
    }

    #[test]
    fn member_with_context_is_accounted_for() {
        let members = vec![member("has-context-crate", true)];
        let policy = policy(&[], &[]);

        let coverage = classify(&members, &policy);

        assert_eq!(coverage.has_context, vec!["has-context-crate".to_string()]);
        assert!(coverage.exempt.is_empty());
        assert!(coverage.needs_context.is_empty());
        assert!(coverage.unaccounted.is_empty());
    }

    #[test]
    fn exempt_member_without_context_is_accounted_for() {
        let members = vec![member("perl-test-must", false)];
        let policy = policy(&["perl-test-must"], &[]);

        let coverage = classify(&members, &policy);

        assert!(coverage.has_context.is_empty());
        assert_eq!(coverage.exempt, vec!["perl-test-must".to_string()]);
        assert!(coverage.needs_context.is_empty());
        assert!(coverage.unaccounted.is_empty());
    }

    #[test]
    fn needs_context_member_passes_with_debt_printed() {
        let members = vec![member("perllsp", false)];
        let policy = policy(&[], &["perllsp"]);

        let coverage = classify(&members, &policy);

        assert!(coverage.has_context.is_empty());
        assert!(coverage.exempt.is_empty());
        assert_eq!(coverage.needs_context, vec!["perllsp".to_string()]);
        assert!(coverage.unaccounted.is_empty());
        // needs_context does not fail the gate on its own -- only unaccounted does.
    }

    #[test]
    fn truly_unaccounted_member_fails() {
        let members = vec![member("brand-new-crate", false)];
        let policy = policy(&["perl-test-must"], &["perllsp"]);

        let coverage = classify(&members, &policy);

        assert_eq!(coverage.unaccounted, vec!["brand-new-crate".to_string()]);
    }

    #[test]
    fn mixed_membership_classifies_each_bucket_independently() {
        let members = vec![
            member("perl-ast", true),
            member("perl-test-must", false),
            member("perllsp", false),
            member("mystery-crate", false),
        ];
        let policy = policy(&["perl-test-must"], &["perllsp"]);

        let coverage = classify(&members, &policy);

        assert_eq!(coverage.has_context, vec!["perl-ast".to_string()]);
        assert_eq!(coverage.exempt, vec!["perl-test-must".to_string()]);
        assert_eq!(coverage.needs_context, vec!["perllsp".to_string()]);
        assert_eq!(coverage.unaccounted, vec!["mystery-crate".to_string()]);
    }

    #[test]
    fn validate_policy_rejects_wrong_version() {
        let mut bad = policy(&[], &[]);
        bad.version = 2;

        let result = validate_policy(&bad, Path::new("policy.toml"));

        assert!(result.is_err());
    }

    #[test]
    fn validate_policy_rejects_duplicate_names_across_lists() -> Result<()> {
        let bad = policy(&["dup-crate"], &["dup-crate"]);

        let result = validate_policy(&bad, Path::new("policy.toml"));

        let Err(error) = result else {
            bail!("duplicate name across exempt/needs_context should be rejected");
        };
        assert!(error.to_string().contains("more than once"));
        Ok(())
    }

    #[test]
    fn validate_policy_rejects_empty_reason() {
        let mut bad = policy(&[], &[]);
        bad.exempt.push(PolicyEntry {
            name: "some-crate".to_string(),
            reason: String::new(),
            tracking_issue: None,
        });

        let result = validate_policy(&bad, Path::new("policy.toml"));

        assert!(result.is_err());
    }

    #[test]
    fn real_policy_file_parses_and_is_internally_consistent() -> Result<()> {
        let root = project_root()?;
        let policy = load_policy(&root.join(POLICY_PATH))?;

        // `exempt` is permanent tracked infra/test-only debt and is expected to
        // stay non-empty. `needs_context` is the opposite: it is expected to
        // shrink to empty as M6 (#3874/#3848) authors package-local CLAUDE.md
        // files for the remaining core-product crates -- an empty list here is
        // the gate succeeding, not a fixture regression, so this test only
        // asserts that the field parses (any length, including zero, is valid).
        assert!(!policy.exempt.is_empty(), "expected at least one exempt entry");
        let _ = &policy.needs_context;
        Ok(())
    }

    #[test]
    fn real_workspace_has_no_unaccounted_members() -> Result<()> {
        let root = project_root()?;
        let policy = load_policy(&root.join(POLICY_PATH))?;
        let members = load_members(&root)?;

        let coverage = classify(&members, &policy);

        assert!(
            coverage.unaccounted.is_empty(),
            "unaccounted workspace members found (add {CONTEXT_FILE_NAME} or classify in {POLICY_PATH}): {:?}",
            coverage.unaccounted
        );
        Ok(())
    }

    #[test]
    fn validate_policy_rejects_needs_context_without_tracking_issue() {
        let mut bad = policy(&[], &["some-crate"]);
        bad.needs_context[0].tracking_issue = None;

        let result = validate_policy(&bad, Path::new("policy.toml"));

        assert!(result.is_err());
    }

    #[test]
    fn validate_policy_accepts_needs_context_with_tracking_issue() -> Result<()> {
        let good = policy(&[], &["some-crate"]);

        validate_policy(&good, Path::new("policy.toml"))?;
        Ok(())
    }

    // -----------------------------------------------------------------
    // No-volatile-facts: classify_volatile_fact (pure, no I/O)
    // -----------------------------------------------------------------

    #[test]
    fn classify_volatile_fact_detects_pr_or_issue_number() {
        assert_eq!(classify_volatile_fact("Fixed in #3848."), Some("pr-or-issue-number"));
    }

    #[test]
    fn classify_volatile_fact_detects_commit_sha_with_hex_letter() {
        assert_eq!(classify_volatile_fact("See commit abc1234 for the fix."), Some("commit-sha"));
    }

    #[test]
    fn classify_volatile_fact_ignores_plain_digit_run_without_hex_letter() {
        // A run of 7+ plain digits (a count, not a SHA) must not be flagged.
        assert_eq!(
            classify_volatile_fact("This function runs 1234567 iterations per second."),
            None
        );
    }

    #[test]
    fn classify_volatile_fact_detects_release_version() {
        assert_eq!(
            classify_volatile_fact("Version: workspace (currently 0.12.3)"),
            Some("release-version")
        );
    }

    #[test]
    fn classify_volatile_fact_detects_calendar_date() {
        assert_eq!(
            classify_volatile_fact("Written on 2026-07-10 during the sprint."),
            Some("calendar-date")
        );
    }

    #[test]
    fn classify_volatile_fact_detects_status_word() {
        assert_eq!(
            classify_volatile_fact("This work was merged into main."),
            Some("status-or-label-word")
        );
    }

    #[test]
    fn classify_volatile_fact_detects_deep_reviewed_status_word() {
        // The other half of the `merged|deep-reviewed` alternation -- only
        // `merged` had a test above.
        assert_eq!(
            classify_volatile_fact("This PR is deep-reviewed and ready."),
            Some("status-or-label-word")
        );
    }

    #[test]
    fn classify_volatile_fact_detects_needs_dash_label_word() {
        // `needs-[a-z]+` branch had no test at all before this: a crate's
        // CLAUDE.md describing its own pipeline label (e.g. `needs-deep-review`)
        // is exactly the kind of point-in-time status this gate exists to catch.
        assert_eq!(
            classify_volatile_fact("Currently carries needs-deep-review."),
            Some("status-or-label-word")
        );
    }

    #[test]
    fn classify_volatile_fact_ignores_needs_dash_glued_to_preceding_word() {
        // Regression guard for the `\b` added before `needs-`: without it, a
        // missing-space typo like "thisneeds-review" would false-positive on
        // the `needs-review` substring it happens to contain, since the old
        // pattern had no boundary requirement before `needs-`.
        assert_eq!(
            classify_volatile_fact("A typo like thisneeds-review should not misfire here."),
            None
        );
    }

    #[test]
    fn classify_volatile_fact_detects_status_colon() {
        // `\bstatus:` branch had no test at all before this.
        assert_eq!(
            classify_volatile_fact("Status: blocked on upstream."),
            Some("status-or-label-word")
        );
    }

    #[test]
    fn classify_volatile_fact_known_limitation_ipv4_octets_read_as_version() {
        // Pins the documented, unfixed limitation from the module-level doc
        // comment: the version pattern also matches the first three octets of
        // an IPv4 address. This test exists so the limitation stays *visible*
        // (a characterization test, not a correctness assertion) -- if this
        // ever starts returning None, the module doc comment's claim is now
        // false and must be updated alongside whatever fixed it.
        assert_eq!(
            classify_volatile_fact("The dev server listens on 127.0.0.1 by default."),
            Some("release-version")
        );
    }

    #[test]
    fn classify_volatile_fact_clean_prose_line_returns_none() {
        assert_eq!(classify_volatile_fact("This crate implements the parser core."), None);
    }

    // -----------------------------------------------------------------
    // No-volatile-facts: scan_volatile_facts (file I/O + fence handling)
    // -----------------------------------------------------------------

    #[test]
    fn scan_volatile_facts_flags_prose_but_skips_fenced_code() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let claude_md = dir.path().join(CONTEXT_FILE_NAME);
        fs::write(
            &claude_md,
            "# CLAUDE.md\n\nSee issue #3848 for background.\n\n```bash\n# example: fixed in #9999\n```\n",
        )?;

        let hits = scan_volatile_facts("test-crate", &claude_md)?;

        assert_eq!(hits.len(), 1, "expected exactly the prose hit, got {hits:?}");
        assert_eq!(hits[0].category, "pr-or-issue-number");
        assert_eq!(hits[0].line_number, 3);
        Ok(())
    }

    #[test]
    fn scan_volatile_facts_clean_file_reports_no_hits() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let claude_md = dir.path().join(CONTEXT_FILE_NAME);
        fs::write(&claude_md, "# CLAUDE.md\n\n## Role\n\nA thin facade crate.\n")?;

        let hits = scan_volatile_facts("test-crate", &claude_md)?;

        assert!(hits.is_empty());
        Ok(())
    }

    // -----------------------------------------------------------------
    // Paths-exist: is_path_candidate (pure, no I/O)
    // -----------------------------------------------------------------

    #[test]
    fn is_path_candidate_accepts_slash_paths_and_known_extensions() {
        assert!(is_path_candidate("src/lib.rs"));
        assert!(is_path_candidate("docs/adr/PLSP-ADR-0006-foo.md"));
        assert!(is_path_candidate("Cargo.toml"));
    }

    #[test]
    fn is_path_candidate_rejects_commands_and_rust_paths() {
        assert!(!is_path_candidate("cargo test -p perl-pod"));
        assert!(!is_path_candidate("perllsp::run_cli(...)"));
        assert!(!is_path_candidate("NodeKind"));
        assert!(!is_path_candidate("@INC"));
    }

    // -----------------------------------------------------------------
    // Paths-exist: extract_cited_paths (section scoping)
    // -----------------------------------------------------------------

    #[test]
    fn extract_cited_paths_only_scans_checked_sections() {
        let content = "# CLAUDE.md\n\n\
## Owns\n\n\
- `src/other.rs` -- not in a checked section.\n\n\
## Read first\n\n\
- `src/lib.rs` -- the entry point.\n\
- `cargo test -p foo` -- not a path.\n\n\
## Focused validation\n\n\
- `tests/roundtrip.rs`\n";

        let paths = extract_cited_paths(content);

        assert_eq!(paths, vec!["src/lib.rs".to_string(), "tests/roundtrip.rs".to_string()]);
    }

    // -----------------------------------------------------------------
    // Paths-exist: scan_dead_paths (file I/O + resolution)
    // -----------------------------------------------------------------

    #[test]
    fn scan_dead_paths_flags_missing_and_passes_existing() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let crate_dir = dir.path().join("some-crate");
        fs::create_dir_all(crate_dir.join("src"))?;
        fs::write(crate_dir.join("src").join("lib.rs"), "// stub\n")?;

        let claude_md = crate_dir.join(CONTEXT_FILE_NAME);
        fs::write(
            &claude_md,
            "# CLAUDE.md\n\n## Read first\n\n\
- `src/lib.rs` -- exists.\n\
- `src/missing.rs` -- does not exist.\n",
        )?;

        let dead = scan_dead_paths("some-crate", &claude_md, &crate_dir, dir.path())?;

        assert_eq!(dead.len(), 1, "expected exactly one dead path, got {dead:?}");
        assert!(dead[0].contains("src/missing.rs"));
        Ok(())
    }

    #[test]
    fn scan_dead_paths_resolves_repo_root_relative_citations() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let repo_root = dir.path();
        let crate_dir = repo_root.join("crates").join("some-crate");
        fs::create_dir_all(&crate_dir)?;
        let sibling_dir = repo_root.join("crates").join("other-crate");
        fs::create_dir_all(&sibling_dir)?;
        fs::write(sibling_dir.join(CONTEXT_FILE_NAME), "# other crate\n")?;

        let claude_md = crate_dir.join(CONTEXT_FILE_NAME);
        fs::write(
            &claude_md,
            "# CLAUDE.md\n\n## Read first\n\n\
- `crates/other-crate/CLAUDE.md` -- repo-root relative, not crate-relative.\n",
        )?;

        let dead = scan_dead_paths("some-crate", &claude_md, &crate_dir, repo_root)?;

        assert!(dead.is_empty(), "expected repo-root relative path to resolve, got {dead:?}");
        Ok(())
    }

    // -----------------------------------------------------------------
    // End-to-end: the 12 crates authored specifically for M6 phase 2 must
    // already be clean. Pre-existing package-local CLAUDE.md files (authored
    // before this bar existed) may still carry volatile facts -- tracked as
    // follow-up cleanup, not asserted clean here, so this test stays green
    // without papering over that debt (see the PR body for the honest
    // required-vs-advisory disposition).
    // -----------------------------------------------------------------

    #[test]
    fn m6_phase2_authored_context_files_are_clean() -> Result<()> {
        const M6_PHASE2_CRATES: &[&str] = &[
            "perllsp",
            "perl-lsp-rs-core",
            "perl-module",
            "perl-workspace-core",
            "perl-pod",
            "perl-lsp-perltidy",
            "perl-diagnostics",
            "perl-line-index",
            "perl-ast-v2",
            "perl-semantic-facts",
            "perl-tree-sitter-compat",
            "perl-dead-code",
        ];

        let root = project_root()?;
        for name in M6_PHASE2_CRATES {
            let crate_dir = root.join("crates").join(name);
            let claude_md = crate_dir.join(CONTEXT_FILE_NAME);

            let hits = scan_volatile_facts(name, &claude_md)?;
            assert!(hits.is_empty(), "{name}: expected no volatile facts, found {hits:?}");

            let dead = scan_dead_paths(name, &claude_md, &crate_dir, &root)?;
            assert!(dead.is_empty(), "{name}: expected no dead paths, found {dead:?}");
        }
        Ok(())
    }
}
