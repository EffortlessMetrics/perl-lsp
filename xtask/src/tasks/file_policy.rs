//! Non-Rust file inventory and policy enforcement for the file-policy rollout.
//!
//! ## Commands
//!
//! - `cargo xtask non-rust inventory` — walks `git ls-files`, classifies
//!   tracked files as Rust or non-Rust, looks each non-Rust file up in
//!   `policy/non-rust-allowlist.toml`, and emits:
//!   - `target/policy/non-rust-inventory.md` — human-readable markdown table.
//!   - `target/policy/non-rust-inventory.json` — machine-readable JSON array.
//!   - `docs/policy/NON_RUST_INVENTORY.md` — regenerated from the same data.
//!
//! - `cargo xtask non-rust check [--mode <mode>] [--json <path>] [--allowlist <path>]` —
//!   classify tracked files against the allowlist and report violations.
//!   Modes: `advisory` (default, always exit 0), `blocking-allowlist` (exit 1 on
//!   unallowlisted files or expired entries), `blocking-strict` (also fail on stale
//!   `review_after`, duplicate ids, absolute/backslashed paths, broad globs without
//!   `broad_glob_reason`).
//!
//! The inventory is **read-only** — it never mutates the allowlist.
//!
//! Refs: #8174, #8566.

use color_eyre::eyre::{Context, Result, eyre};
use glob::Pattern;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

// ---------------------------------------------------------------------------
// Allowlist schema (mirrors `policy/non-rust-allowlist.toml`)
// ---------------------------------------------------------------------------

/// Top-level structure of `policy/non-rust-allowlist.toml`.
#[derive(Debug, Deserialize)]
pub struct Allowlist {
    #[serde(default)]
    pub allow: Vec<AllowEntry>,
}

/// A single `[[allow]]` entry.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct AllowEntry {
    pub id: String,
    /// Glob pattern (mutually exclusive with `path`).
    pub glob: Option<String>,
    /// Exact path (mutually exclusive with `glob`).
    pub path: Option<String>,
    pub kind: String,
    pub language: String,
    pub surface: String,
    pub classification: String,
    pub owner: String,
    pub reason: String,
    #[serde(default)]
    pub covered_by: Vec<String>,
    pub created: String,
    pub review_after: String,
    pub expires: Option<String>,
    pub broad_glob_reason: Option<String>,
    #[serde(default)]
    pub retired: bool,
}

// ---------------------------------------------------------------------------
// Inventory output schema
// ---------------------------------------------------------------------------

/// Classification of a single tracked file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRecord {
    /// Repo-relative path (forward slashes, no leading `./`).
    pub path: String,
    /// File extension without the leading dot, or empty string for
    /// files with no extension.
    pub extension: String,
    /// `"rust"` for Rust-family files; the allowlist `classification`
    /// value for non-Rust files that are allowlisted; `"unclassified"`
    /// otherwise.
    pub category: String,
    /// Whether the file matches at least one non-retired allowlist entry.
    pub allowlisted: bool,
    /// The first matching allowlist entry, if any.
    pub entry: Option<AllowEntry>,
}

// ---------------------------------------------------------------------------
// Rust-family classifier
// ---------------------------------------------------------------------------

/// Returns `true` when the path is a Rust-family file that does not require
/// an allowlist entry.
pub fn is_rust_file(path: &str) -> bool {
    // Source and build artefacts that are fully Rust-owned.
    if path.ends_with(".rs") {
        return true;
    }
    // Well-known filenames (no extension or fixed name).
    let basename = path.rsplit('/').next().unwrap_or(path);
    matches!(
        basename,
        "Cargo.toml" | "Cargo.lock" | "rust-toolchain.toml" | "clippy.toml" | "rustfmt.toml"
    )
}

// ---------------------------------------------------------------------------
// Allowlist loading and glob matching
// ---------------------------------------------------------------------------

/// Load `policy/non-rust-allowlist.toml` from the workspace root.
pub fn load_allowlist(root: &Path) -> Result<Allowlist> {
    let path = root.join("policy/non-rust-allowlist.toml");
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

struct PreparedAllowEntry<'a> {
    entry: &'a AllowEntry,
    glob: Option<Pattern>,
}

fn prepare_allow_entries(entries: &[AllowEntry]) -> Vec<PreparedAllowEntry<'_>> {
    let mut prepared = Vec::new();
    for entry in entries {
        if entry.retired {
            continue;
        }
        let glob = match entry.glob.as_deref() {
            Some(glob_str) => match Pattern::new(glob_str) {
                Ok(pattern) => Some(pattern),
                Err(_) => continue,
            },
            None => None,
        };
        prepared.push(PreparedAllowEntry { entry, glob });
    }
    prepared
}

fn find_matching_prepared_entry<'a>(
    file_path: &str,
    entries: &[PreparedAllowEntry<'a>],
) -> Option<&'a AllowEntry> {
    for prepared in entries {
        let matched = if let Some(pattern) = prepared.glob.as_ref() {
            pattern.matches(file_path)
        } else if let Some(ref exact) = prepared.entry.path {
            exact == file_path
        } else {
            false
        };
        if matched {
            return Some(prepared.entry);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// git ls-files
// ---------------------------------------------------------------------------

/// Run `git ls-files` from `root` and return a sorted list of repo-relative
/// paths (forward slashes, no leading `./`).
pub fn list_tracked_files(root: &Path) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(root)
        .output()
        .with_context(|| "running `git ls-files -z`")?;
    if !output.status.success() {
        return Err(eyre!("`git ls-files -z` failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    let mut files: Vec<String> = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| {
            let path = String::from_utf8(path.to_vec())
                .with_context(|| "`git ls-files -z` produced a non-UTF-8 path")?;
            Ok(path.trim_start_matches("./").replace('\\', "/"))
        })
        .collect::<Result<_>>()?;
    files.sort_unstable();
    files.dedup();
    Ok(files)
}

// ---------------------------------------------------------------------------
// Core inventory logic
// ---------------------------------------------------------------------------

#[cfg(test)]
fn classify_file(path: &str, entries: &[AllowEntry]) -> FileRecord {
    let prepared = prepare_allow_entries(entries);
    classify_file_with_prepared(path, &prepared)
}

fn classify_file_with_prepared(path: &str, entries: &[PreparedAllowEntry<'_>]) -> FileRecord {
    let extension = path
        .rsplit('/')
        .next()
        .and_then(|file_name| file_name.rsplit_once('.'))
        .filter(|(stem, ext)| !stem.is_empty() && !ext.is_empty())
        .map(|(_, ext)| ext)
        .unwrap_or("")
        .to_string();

    if is_rust_file(path) {
        return FileRecord {
            path: path.to_string(),
            extension,
            category: "rust".to_string(),
            allowlisted: false,
            entry: None,
        };
    }

    match find_matching_prepared_entry(path, entries) {
        Some(e) => FileRecord {
            path: path.to_string(),
            extension,
            category: e.classification.clone(),
            allowlisted: true,
            entry: Some(e.clone()),
        },
        None => FileRecord {
            path: path.to_string(),
            extension,
            category: "unclassified".to_string(),
            allowlisted: false,
            entry: None,
        },
    }
}

/// Build a full inventory from `root`.
pub fn build_inventory(root: &Path) -> Result<Vec<FileRecord>> {
    let allowlist = load_allowlist(root)?;
    let tracked = list_tracked_files(root)?;
    let prepared = prepare_allow_entries(&allowlist.allow);

    let records: Vec<FileRecord> =
        tracked.iter().map(|p| classify_file_with_prepared(p, &prepared)).collect();
    Ok(records)
}

// ---------------------------------------------------------------------------
// Markdown renderer
// ---------------------------------------------------------------------------

/// Render the inventory as a Markdown document.
pub fn render_markdown(records: &[FileRecord]) -> String {
    let total = records.len();
    let rust_count = records.iter().filter(|r| r.category == "rust").count();
    let non_rust: Vec<&FileRecord> = records.iter().filter(|r| r.category != "rust").collect();
    let allowlisted_count = non_rust.iter().filter(|r| r.allowlisted).count();
    let unclassified_count = non_rust.iter().filter(|r| !r.allowlisted).count();

    // Group non-Rust files by category for a summary table.
    let mut by_category: BTreeMap<&str, usize> = BTreeMap::new();
    for r in &non_rust {
        *by_category.entry(r.category.as_str()).or_insert(0) += 1;
    }

    let mut out = String::new();
    out.push_str("# Non-Rust File Inventory\n\n");
    out.push_str("> Generated by `cargo xtask non-rust inventory`. Do not edit by hand.\n\n");
    out.push_str("## Summary\n\n");
    out.push_str(&format!(
        "| Metric | Count |\n|---|---|\n\
         | Total tracked files | {total} |\n\
         | Rust-family files | {rust_count} |\n\
         | Non-Rust files | {} |\n\
         | Allowlisted | {allowlisted_count} |\n\
         | Unclassified | {unclassified_count} |\n\n",
        non_rust.len()
    ));

    out.push_str("## Non-Rust files by category\n\n");
    out.push_str("| Category | Count |\n|---|---|\n");
    for (cat, count) in &by_category {
        out.push_str(&format!("| {cat} | {count} |\n"));
    }
    out.push('\n');

    if unclassified_count > 0 {
        out.push_str("## Unclassified files\n\n");
        out.push_str(
            "> These files have no matching allowlist entry. Add an entry to \
             `policy/non-rust-allowlist.toml` or run `cargo xtask non-rust propose`.\n\n",
        );
        out.push_str("| Path | Extension |\n|---|---|\n");
        for r in non_rust.iter().filter(|r| !r.allowlisted) {
            out.push_str(&format!("| `{}` | `{}` |\n", r.path, r.extension));
        }
        out.push('\n');
    }

    out.push_str("## Allowlisted non-Rust files\n\n");
    out.push_str("| Path | Category | Entry id | Owner |\n|---|---|---|---|\n");
    for r in non_rust.iter().filter(|r| r.allowlisted) {
        let (id, owner) =
            r.entry.as_ref().map(|e| (e.id.as_str(), e.owner.as_str())).unwrap_or(("", ""));
        out.push_str(&format!("| `{}` | {} | `{}` | {} |\n", r.path, r.category, id, owner));
    }
    out.push('\n');

    out.push_str("## See also\n\n");
    out.push_str(
        "- [FILE_POLICY.md](FILE_POLICY.md) — the doctrine.\n\
         - [NON_RUST_POLICY.md](NON_RUST_POLICY.md) — the schema.\n\
         - [POLICY_ALLOWLISTS.md](POLICY_ALLOWLISTS.md) — all seven ledgers.\n",
    );

    out
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Entry point for `cargo xtask non-rust inventory`.
pub fn non_rust_inventory(root: &Path) -> Result<()> {
    println!("Building non-Rust file inventory...");

    let records = build_inventory(root)?;

    // Write outputs under target/policy/.
    let target_dir = root.join("target/policy");
    fs::create_dir_all(&target_dir)
        .with_context(|| format!("creating {}", target_dir.display()))?;

    let md_path = target_dir.join("non-rust-inventory.md");
    let json_path = target_dir.join("non-rust-inventory.json");

    let markdown = render_markdown(&records);
    fs::write(&md_path, &markdown).with_context(|| format!("writing {}", md_path.display()))?;
    println!("  wrote {}", md_path.display());

    let json =
        serde_json::to_string_pretty(&records).with_context(|| "serialising inventory to JSON")?;
    fs::write(&json_path, &json).with_context(|| format!("writing {}", json_path.display()))?;
    println!("  wrote {}", json_path.display());

    // Regenerate docs/policy/NON_RUST_INVENTORY.md.
    let docs_path = root.join("docs/policy/NON_RUST_INVENTORY.md");
    if let Some(parent) = docs_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(&docs_path, &markdown).with_context(|| format!("writing {}", docs_path.display()))?;
    println!("  wrote {}", docs_path.display());

    // Print a brief summary.
    let total = records.len();
    let rust_count = records.iter().filter(|r| r.category == "rust").count();
    let non_rust_count = total - rust_count;
    let allowlisted = records.iter().filter(|r| r.allowlisted).count();
    let unclassified = non_rust_count - allowlisted;

    println!(
        "\nInventory complete: {total} tracked files\n\
         - Rust-family:   {rust_count}\n\
         - Non-Rust:      {non_rust_count}\n\
         - Allowlisted:   {allowlisted}\n\
         - Unclassified:  {unclassified}"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// check-file-policy — enforcement subcommand
// ---------------------------------------------------------------------------

/// Operating mode for `cargo xtask non-rust check`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CheckFilePolicyMode {
    /// Report only — never exit with a non-zero code.
    Advisory,
    /// Fail when any non-Rust file has no allowlist entry, or any entry has
    /// an expired `expires` date. Does not check `review_after`.
    BlockingAllowlist,
    /// `blocking-allowlist` plus: stale `review_after`, duplicate entry ids,
    /// absolute or backslash paths, and broad globs without `broad_glob_reason`.
    BlockingStrict,
}

impl std::fmt::Display for CheckFilePolicyMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckFilePolicyMode::Advisory => write!(f, "advisory"),
            CheckFilePolicyMode::BlockingAllowlist => write!(f, "blocking-allowlist"),
            CheckFilePolicyMode::BlockingStrict => write!(f, "blocking-strict"),
        }
    }
}

/// Configuration for `cargo xtask non-rust check`.
pub struct CheckFilePolicyConfig {
    /// Operating mode.
    pub mode: CheckFilePolicyMode,
    /// If `Some(path)`, write the JSON receipt to this file.
    pub json_output: Option<std::path::PathBuf>,
    /// Override the default allowlist path (`policy/non-rust-allowlist.toml`).
    pub allowlist_path: Option<std::path::PathBuf>,
    /// Override the workspace root used for `git ls-files`.
    /// When `None`, the binary resolves `project_root()` at runtime.
    /// Intended as a test seam — production invocations omit this.
    pub root_override: Option<std::path::PathBuf>,
}

/// A single policy violation found during `check-file-policy`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyViolation {
    /// Machine-readable violation kind.
    pub kind: String,
    /// Human-readable description.
    pub message: String,
    /// Path of the file or entry involved (if applicable).
    pub path: Option<String>,
    /// Allowlist entry id involved (if applicable).
    pub entry_id: Option<String>,
}

/// JSON receipt emitted by `cargo xtask non-rust check`.
#[derive(Debug, Serialize, Deserialize)]
pub struct FilePolicyReceipt {
    /// Always 1 for this schema generation.
    pub schema_version: u32,
    /// Mode used for this run.
    pub mode: String,
    /// Total number of tracked files (Rust + non-Rust).
    pub total_tracked: usize,
    /// Number of non-Rust files.
    pub non_rust: usize,
    /// Number of non-Rust files with no allowlist entry.
    pub unclassified: usize,
    /// Number of allowlist entries with an expired `expires` date.
    pub expired: usize,
    /// Number of allowlist entries with a stale `review_after` date (past today).
    pub stale_review_after: usize,
    /// Number of duplicate entry ids across the allowlist.
    pub duplicate_ids: usize,
    /// Number of non-retired allowlist entries that match no tracked file.
    pub unused_entries: usize,
    /// Violations that fail the selected mode.
    pub violations: Vec<PolicyViolation>,
}

/// Check whether a date string (YYYY-MM-DD) is in the past relative to today.
fn is_past_date(date_str: &str) -> bool {
    // Parse YYYY-MM-DD by splitting on '-'.
    let parts: Vec<&str> = date_str.trim().split('-').collect();
    if parts.len() != 3 {
        // Malformed date — treat as in the past so it gets flagged.
        return true;
    }
    let (Ok(y), Ok(m), Ok(d)) =
        (parts[0].parse::<u32>(), parts[1].parse::<u32>(), parts[2].parse::<u32>())
    else {
        return true;
    };
    // Use chrono if available; otherwise fall back to a manual comparison
    // against the compile-time UTC date (good enough for CI).
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    // Approximate: days since epoch → year/month/day (Gregorian).
    let days = secs / 86400;
    // Epoch = 1970-01-01.
    let (ey, em, ed) = days_to_ymd(days);
    (y, m, d) < (ey, em, ed)
}

/// Convert days-since-Unix-epoch to (year, month, day) using the proleptic
/// Gregorian calendar. Accurate for years 1970–2200 (sufficient for policy).
fn days_to_ymd(days: u64) -> (u32, u32, u32) {
    // Algorithm: Julian Day Number method.
    let jdn = days + 2_440_588; // Unix epoch = JDN 2440588
    let a = jdn + 32044;
    let b = (4 * a + 3) / 146097;
    let c = a - 146097 * b / 4;
    let d = (4 * c + 3) / 1461;
    let e = c - 1461 * d / 4;
    let m = (5 * e + 2) / 153;
    let day = e - (153 * m + 2) / 5 + 1;
    let month = m + 3 - 12 * (m / 10);
    let year = 100 * b + d - 4800 + m / 10;
    (year as u32, month as u32, day as u32)
}

/// Returns `true` when the glob pattern looks like a "broad" glob
/// (e.g. `**/*`, `**`, `*`).
fn is_broad_glob(glob_str: &str) -> bool {
    matches!(glob_str.trim(), "**" | "**/*" | "*" | "*.*")
        || glob_str.starts_with("**/*.")
            && glob_str.trim_start_matches("**/").trim_start_matches("*.").is_empty()
}

fn expired_entry_count(entries: &[AllowEntry]) -> usize {
    entries
        .iter()
        .filter(|entry| !entry.retired)
        .filter(|entry| entry.expires.as_deref().is_some_and(is_past_date))
        .count()
}

fn stale_review_after_count(entries: &[AllowEntry]) -> usize {
    entries
        .iter()
        .filter(|entry| !entry.retired)
        .filter(|entry| !entry.review_after.is_empty() && is_past_date(&entry.review_after))
        .count()
}

fn duplicate_id_count(entries: &[AllowEntry]) -> usize {
    let mut seen_ids: BTreeMap<&str, usize> = BTreeMap::new();
    for entry in entries {
        *seen_ids.entry(entry.id.as_str()).or_insert(0) += 1;
    }
    seen_ids.values().filter(|count| **count > 1).count()
}

fn entry_matches_any_tracked_file(entry: &AllowEntry, tracked: &[String]) -> bool {
    if let Some(path) = entry.path.as_deref() {
        return tracked.iter().any(|tracked_path| tracked_path == path);
    }
    if let Some(glob_str) = entry.glob.as_deref() {
        let Ok(pattern) = Pattern::new(glob_str) else {
            return false;
        };
        return tracked.iter().any(|tracked_path| pattern.matches(tracked_path));
    }
    false
}

fn unused_entry_count(entries: &[AllowEntry], tracked: &[String]) -> usize {
    entries
        .iter()
        .filter(|entry| !entry.retired)
        .filter(|entry| entry.glob.is_some() ^ entry.path.is_some())
        .filter(|entry| !entry_matches_any_tracked_file(entry, tracked))
        .count()
}

/// Load the allowlist from the given path (overrides root-relative default).
fn load_allowlist_from(path: &std::path::Path) -> Result<Allowlist> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

fn render_policy_report_markdown(receipt: &FilePolicyReceipt) -> String {
    let mut out = String::new();
    out.push_str("# Non-Rust File Policy Report\n\n");
    out.push_str("> Generated by `cargo xtask check-file-policy`. Do not edit by hand.\n\n");
    out.push_str("## Summary\n\n");
    out.push_str("| Metric | Value |\n|---|---:|\n");
    out.push_str(&format!("| Mode | `{}` |\n", receipt.mode));
    out.push_str(&format!("| Total tracked | {} |\n", receipt.total_tracked));
    out.push_str(&format!("| Non-Rust | {} |\n", receipt.non_rust));
    out.push_str(&format!("| Unclassified | {} |\n", receipt.unclassified));
    out.push_str(&format!("| Expired entries | {} |\n", receipt.expired));
    out.push_str(&format!("| Stale review_after | {} |\n", receipt.stale_review_after));
    out.push_str(&format!("| Duplicate ids | {} |\n", receipt.duplicate_ids));
    out.push_str(&format!("| Unused entries | {} |\n", receipt.unused_entries));
    out.push_str(&format!("| Violations | {} |\n\n", receipt.violations.len()));

    if receipt.violations.is_empty() {
        out.push_str("## Violations\n\nNo violations for the selected mode.\n");
        return out;
    }

    out.push_str("## Violations\n\n");
    out.push_str("| Kind | Location | Entry | Message |\n|---|---|---|---|\n");
    for violation in &receipt.violations {
        let path = violation.path.as_deref().unwrap_or("");
        let entry_id = violation.entry_id.as_deref().unwrap_or("");
        out.push_str(&format!(
            "| `{}` | `{}` | `{}` | {} |\n",
            violation.kind, path, entry_id, violation.message
        ));
    }
    out
}

/// Run all allowlist-level validations and return violations.
fn check_allowlist_entries(
    entries: &[AllowEntry],
    mode: CheckFilePolicyMode,
    tracked: &[String],
) -> Vec<PolicyViolation> {
    let mut violations: Vec<PolicyViolation> = Vec::new();

    // --- Duplicate id check (strict only) ---
    if mode == CheckFilePolicyMode::BlockingStrict {
        let mut seen_ids: BTreeMap<&str, usize> = BTreeMap::new();
        for entry in entries {
            *seen_ids.entry(entry.id.as_str()).or_insert(0) += 1;
        }
        for (id, count) in &seen_ids {
            if *count > 1 {
                violations.push(PolicyViolation {
                    kind: "duplicate-id".to_string(),
                    message: format!("Allowlist entry id {id:?} appears {count} times"),
                    path: None,
                    entry_id: Some(id.to_string()),
                });
            }
        }
    }

    for entry in entries {
        if entry.retired {
            continue;
        }

        let has_glob = entry.glob.is_some();
        let has_path = entry.path.is_some();

        // --- Blocking-allowlist+ entry validity checks ---
        if mode != CheckFilePolicyMode::Advisory {
            if let Some(ref expires) = entry.expires {
                if is_past_date(expires) {
                    violations.push(PolicyViolation {
                        kind: "expired-entry".to_string(),
                        message: format!("Entry {:?} has expired (expires={})", entry.id, expires),
                        path: None,
                        entry_id: Some(entry.id.clone()),
                    });
                }
            }

            if !has_glob && !has_path {
                violations.push(PolicyViolation {
                    kind: "missing-matcher".to_string(),
                    message: format!("Entry {:?} must define `path` or `glob`", entry.id),
                    path: None,
                    entry_id: Some(entry.id.clone()),
                });
            }
            if has_glob && has_path {
                violations.push(PolicyViolation {
                    kind: "multiple-matchers".to_string(),
                    message: format!("Entry {:?} must not define both `path` and `glob`", entry.id),
                    path: None,
                    entry_id: Some(entry.id.clone()),
                });
            }
            if let Some(glob_str) = entry.glob.as_deref() {
                if Pattern::new(glob_str).is_err() {
                    violations.push(PolicyViolation {
                        kind: "invalid-glob".to_string(),
                        message: format!("Entry {:?} has invalid glob {:?}", entry.id, glob_str),
                        path: Some(glob_str.to_string()),
                        entry_id: Some(entry.id.clone()),
                    });
                }
            }
            if entry.kind.trim().is_empty() {
                violations.push(PolicyViolation {
                    kind: "missing-kind".to_string(),
                    message: format!("Entry {:?} is missing required field `kind`", entry.id),
                    path: None,
                    entry_id: Some(entry.id.clone()),
                });
            }
            if entry.language.trim().is_empty() {
                violations.push(PolicyViolation {
                    kind: "missing-language".to_string(),
                    message: format!("Entry {:?} is missing required field `language`", entry.id),
                    path: None,
                    entry_id: Some(entry.id.clone()),
                });
            }
            if entry.owner.trim().is_empty() {
                violations.push(PolicyViolation {
                    kind: "missing-owner".to_string(),
                    message: format!("Entry {:?} is missing required field `owner`", entry.id),
                    path: None,
                    entry_id: Some(entry.id.clone()),
                });
            }
            if entry.reason.trim().is_empty() {
                violations.push(PolicyViolation {
                    kind: "missing-reason".to_string(),
                    message: format!("Entry {:?} is missing required field `reason`", entry.id),
                    path: None,
                    entry_id: Some(entry.id.clone()),
                });
            }
            if entry.surface.trim().is_empty() {
                violations.push(PolicyViolation {
                    kind: "missing-surface".to_string(),
                    message: format!("Entry {:?} is missing required field `surface`", entry.id),
                    path: None,
                    entry_id: Some(entry.id.clone()),
                });
            }
            if entry.classification.trim().is_empty() {
                violations.push(PolicyViolation {
                    kind: "missing-classification".to_string(),
                    message: format!(
                        "Entry {:?} is missing required field `classification`",
                        entry.id
                    ),
                    path: None,
                    entry_id: Some(entry.id.clone()),
                });
            }
            if entry.covered_by.is_empty() {
                violations.push(PolicyViolation {
                    kind: "missing-covered-by".to_string(),
                    message: format!("Entry {:?} is missing required field `covered_by`", entry.id),
                    path: None,
                    entry_id: Some(entry.id.clone()),
                });
            }
        }

        // The following checks are strict-only.
        if mode != CheckFilePolicyMode::BlockingStrict {
            continue;
        }

        // --- Stale review_after ---
        if !entry.review_after.is_empty() && is_past_date(&entry.review_after) {
            violations.push(PolicyViolation {
                kind: "stale-review-after".to_string(),
                message: format!(
                    "Entry {:?} review_after={} is in the past",
                    entry.id, entry.review_after
                ),
                path: None,
                entry_id: Some(entry.id.clone()),
            });
        }

        // --- Unused entries ---
        if has_glob ^ has_path && !entry_matches_any_tracked_file(entry, tracked) {
            violations.push(PolicyViolation {
                kind: "unused-entry".to_string(),
                message: format!("Entry {:?} matches no tracked file", entry.id),
                path: entry.path.clone().or_else(|| entry.glob.clone()),
                entry_id: Some(entry.id.clone()),
            });
        }

        // --- Absolute or backslash paths ---
        let path_or_glob = entry.glob.as_deref().or(entry.path.as_deref()).unwrap_or("");
        if path_or_glob.starts_with('/') {
            violations.push(PolicyViolation {
                kind: "absolute-path".to_string(),
                message: format!("Entry {:?} uses an absolute path: {:?}", entry.id, path_or_glob),
                path: Some(path_or_glob.to_string()),
                entry_id: Some(entry.id.clone()),
            });
        }
        if path_or_glob.contains('\\') {
            violations.push(PolicyViolation {
                kind: "backslash-path".to_string(),
                message: format!(
                    "Entry {:?} uses backslashes in path: {:?}",
                    entry.id, path_or_glob
                ),
                path: Some(path_or_glob.to_string()),
                entry_id: Some(entry.id.clone()),
            });
        }

        // --- Broad glob without reason ---
        if let Some(ref glob_str) = entry.glob {
            if is_broad_glob(glob_str) && entry.broad_glob_reason.is_none() {
                violations.push(PolicyViolation {
                    kind: "broad-glob-no-reason".to_string(),
                    message: format!(
                        "Entry {:?} has a broad glob {:?} but no `broad_glob_reason`",
                        entry.id, glob_str
                    ),
                    path: Some(glob_str.clone()),
                    entry_id: Some(entry.id.clone()),
                });
            }
        }
    }

    violations
}

/// Entry point for `cargo xtask non-rust check`.
pub fn check_file_policy(root: &std::path::Path, config: CheckFilePolicyConfig) -> Result<()> {
    // Resolve effective workspace root (allows test seam override).
    let effective_root: std::path::PathBuf =
        if let Some(ref r) = config.root_override { r.clone() } else { root.to_path_buf() };
    let root = effective_root.as_path();

    // Load allowlist.
    let allowlist = if let Some(ref custom_path) = config.allowlist_path {
        load_allowlist_from(custom_path)?
    } else {
        load_allowlist(root)?
    };

    let entries = &allowlist.allow;

    // Build inventory.
    let tracked = list_tracked_files(root)?;
    let prepared = prepare_allow_entries(entries);

    let mut violations: Vec<PolicyViolation> = Vec::new();

    // --- Per-file classification ---
    let mut non_rust_count = 0usize;
    let mut unclassified_count = 0usize;

    for path in &tracked {
        let record = classify_file_with_prepared(path, &prepared);
        if record.category == "rust" {
            continue;
        }
        non_rust_count += 1;
        if !record.allowlisted {
            unclassified_count += 1;
            if config.mode != CheckFilePolicyMode::Advisory {
                violations.push(PolicyViolation {
                    kind: "unallowlisted-file".to_string(),
                    message: format!("Non-Rust file {path:?} has no allowlist entry"),
                    path: Some(path.clone()),
                    entry_id: None,
                });
            }
        }
    }

    // --- Allowlist entry checks ---
    let entry_violations = check_allowlist_entries(entries, config.mode, &tracked);
    let expired_count = expired_entry_count(entries);
    let stale_review_after_count = stale_review_after_count(entries);
    let duplicate_ids_count = duplicate_id_count(entries);
    let unused_entries = unused_entry_count(entries, &tracked);
    violations.extend(entry_violations);

    // --- Build receipt ---
    let receipt = FilePolicyReceipt {
        schema_version: 1,
        mode: config.mode.to_string(),
        total_tracked: tracked.len(),
        non_rust: non_rust_count,
        unclassified: unclassified_count,
        expired: expired_count,
        stale_review_after: stale_review_after_count,
        duplicate_ids: duplicate_ids_count,
        unused_entries,
        violations: violations.clone(),
    };

    // --- Emit output ---
    let json =
        serde_json::to_string_pretty(&receipt).context("failed to serialize policy receipt")?;

    let json_path = config
        .json_output
        .clone()
        .unwrap_or_else(|| root.join("target/policy/file-policy-report.json"));
    let markdown_path = if config.json_output.is_none() {
        Some(root.join("target/policy/file-policy-report.md"))
    } else {
        None
    };

    if let Some(parent) = json_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(&json_path, &json)
        .with_context(|| format!("writing receipt to {}", json_path.display()))?;
    println!("  wrote {}", json_path.display());

    if let Some(markdown_path) = markdown_path {
        if let Some(parent) = markdown_path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        let markdown = render_policy_report_markdown(&receipt);
        fs::write(&markdown_path, &markdown)
            .with_context(|| format!("writing report to {}", markdown_path.display()))?;
        println!("  wrote {}", markdown_path.display());
    }

    // Human-readable summary.
    println!("check-file-policy (mode: {})", config.mode);
    println!(
        "  total tracked: {}  non-Rust: {}  unclassified: {}",
        receipt.total_tracked, receipt.non_rust, receipt.unclassified
    );
    println!(
        "  expired entries: {}  stale review_after: {}  unused entries: {}",
        expired_count, stale_review_after_count, unused_entries
    );
    if violations.is_empty() {
        println!("  result: OK — no violations");
    } else {
        println!("  result: {} violation(s)", violations.len());
        for v in &violations {
            let loc = v.path.as_deref().or(v.entry_id.as_deref()).unwrap_or("");
            println!(
                "    [{}] {}{}",
                v.kind,
                if loc.is_empty() { String::new() } else { format!("{loc}: ") },
                v.message
            );
        }
    }

    // Decide exit code based on mode.
    if config.mode != CheckFilePolicyMode::Advisory && !violations.is_empty() {
        std::process::exit(1);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(
        id: &str,
        glob: Option<&str>,
        path: Option<&str>,
        classification: &str,
    ) -> AllowEntry {
        AllowEntry {
            id: id.to_string(),
            glob: glob.map(str::to_string),
            path: path.map(str::to_string),
            kind: "test".to_string(),
            language: "mixed".to_string(),
            surface: "test".to_string(),
            classification: classification.to_string(),
            owner: "test".to_string(),
            reason: "test".to_string(),
            covered_by: vec![],
            created: "2026-01-01".to_string(),
            review_after: "2026-06-01".to_string(),
            expires: None,
            broad_glob_reason: None,
            retired: false,
        }
    }

    // --- is_rust_file ---

    #[test]
    fn rust_extension_is_rust() {
        assert!(is_rust_file("src/main.rs"));
        assert!(is_rust_file("crates/foo/src/lib.rs"));
    }

    #[test]
    fn rust_well_known_names_are_rust() {
        assert!(is_rust_file("Cargo.toml"));
        assert!(is_rust_file("path/to/Cargo.toml"));
        assert!(is_rust_file("Cargo.lock"));
        assert!(is_rust_file("rust-toolchain.toml"));
        assert!(is_rust_file("clippy.toml"));
        assert!(is_rust_file("rustfmt.toml"));
    }

    #[test]
    fn non_rust_files_return_false() {
        assert!(!is_rust_file("README.md"));
        assert!(!is_rust_file("justfile"));
        assert!(!is_rust_file("flake.nix"));
        assert!(!is_rust_file("features.toml"));
        assert!(!is_rust_file(".github/workflows/ci.yml"));
        assert!(!is_rust_file("test_corpus/foo.pl"));
    }

    // --- classify_file ---

    #[test]
    fn classify_rs_file_as_rust() {
        let rec = classify_file("src/lib.rs", &[]);
        assert_eq!(rec.category, "rust");
        assert!(!rec.allowlisted);
        assert!(rec.entry.is_none());
    }

    #[test]
    fn classify_unknown_file_as_unclassified() {
        let rec = classify_file("strange/file.xyz", &[]);
        assert_eq!(rec.category, "unclassified");
        assert!(!rec.allowlisted);
    }

    #[test]
    fn classify_exact_path_match() {
        let entries = vec![make_entry("e1", None, Some("justfile"), "tooling")];
        let rec = classify_file("justfile", &entries);
        assert_eq!(rec.category, "tooling");
        assert!(rec.allowlisted);
        assert_eq!(rec.entry.as_ref().map(|e| e.id.as_str()), Some("e1"));
    }

    #[test]
    fn exact_path_does_not_match_other_paths() {
        let entries = vec![make_entry("e1", None, Some("justfile"), "tooling")];
        let rec = classify_file("other-file", &entries);
        assert_eq!(rec.category, "unclassified");
        assert!(!rec.allowlisted);
    }

    #[test]
    fn classify_glob_match() {
        let entries = vec![make_entry("docs", Some("docs/**"), None, "documentation")];
        let rec = classify_file("docs/policy/FILE_POLICY.md", &entries);
        assert_eq!(rec.category, "documentation");
        assert!(rec.allowlisted);
    }

    #[test]
    fn glob_does_not_match_outside_tree() {
        let entries = vec![make_entry("docs", Some("docs/**"), None, "documentation")];
        let rec = classify_file("README.md", &entries);
        assert_eq!(rec.category, "unclassified");
        assert!(!rec.allowlisted);
    }

    #[test]
    fn retired_entry_is_skipped() {
        let mut entry = make_entry("retired", Some("docs/**"), None, "documentation");
        entry.retired = true;
        let entries = vec![entry];
        let rec = classify_file("docs/policy/FILE_POLICY.md", &entries);
        assert_eq!(rec.category, "unclassified", "retired entry must not match");
    }

    // --- extension extraction ---

    #[test]
    fn extension_extracted_correctly() {
        let rec = classify_file("foo/bar.md", &[]);
        assert_eq!(rec.extension, "md");
    }

    #[test]
    fn file_without_extension() {
        let rec = classify_file("justfile", &[]);
        assert_eq!(rec.extension, "");
        assert!(!rec.allowlisted);
    }

    // --- JSON round-trip ---

    #[test]
    fn file_record_serde_round_trip() -> Result<()> {
        let record = FileRecord {
            path: "justfile".to_string(),
            extension: String::new(),
            category: "tooling".to_string(),
            allowlisted: true,
            entry: Some(make_entry("e1", None, Some("justfile"), "tooling")),
        };
        let json = serde_json::to_string(&record)?;
        let back: FileRecord = serde_json::from_str(&json)?;
        assert_eq!(record, back);
        Ok(())
    }

    // --- render_markdown smoke ---

    #[test]
    fn render_markdown_contains_summary_heading() {
        let records = vec![
            FileRecord {
                path: "src/lib.rs".to_string(),
                extension: "rs".to_string(),
                category: "rust".to_string(),
                allowlisted: false,
                entry: None,
            },
            FileRecord {
                path: "README.md".to_string(),
                extension: "md".to_string(),
                category: "documentation".to_string(),
                allowlisted: true,
                entry: Some(make_entry("e1", Some("*.md"), None, "documentation")),
            },
            FileRecord {
                path: "unknown.xyz".to_string(),
                extension: "xyz".to_string(),
                category: "unclassified".to_string(),
                allowlisted: false,
                entry: None,
            },
        ];
        let md = render_markdown(&records);
        assert!(md.contains("# Non-Rust File Inventory"), "missing H1");
        assert!(md.contains("## Summary"), "missing Summary section");
        assert!(md.contains("## Unclassified files"), "missing Unclassified section");
        assert!(md.contains("## Allowlisted non-Rust files"), "missing Allowlisted section");
    }
}
