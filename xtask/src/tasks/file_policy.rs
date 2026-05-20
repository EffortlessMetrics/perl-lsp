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
// validate-policy — schema-only non-Rust policy validation
// ---------------------------------------------------------------------------

const REQUIRED_ALLOW_FIELDS: &[&str] = &[
    "id",
    "kind",
    "language",
    "surface",
    "classification",
    "owner",
    "reason",
    "covered_by",
    "created",
    "review_after",
];

const ALLOWED_ALLOW_FIELDS: &[&str] = &[
    "id",
    "glob",
    "path",
    "kind",
    "language",
    "surface",
    "classification",
    "owner",
    "reason",
    "covered_by",
    "created",
    "review_after",
    "expires",
    "broad_glob_reason",
    "retired",
    "generated_by",
];

const KNOWN_CLASSIFICATIONS: &[&str] =
    &["production", "test", "tooling", "config", "documentation", "generated"];

const COVERAGE_REQUIRING_CLASSIFICATIONS: &[&str] = &["production", "test", "tooling"];

/// Configuration for `cargo xtask non-rust validate-policy`.
pub struct ValidateNonRustPolicyConfig {
    /// Override the default allowlist path (`policy/non-rust-allowlist.toml`).
    pub allowlist_path: std::path::PathBuf,
    /// Override the default debt path (`policy/non-rust-debt.toml`).
    pub debt_path: std::path::PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonRustPolicyValidation {
    pub allow_entries: usize,
    pub debt_entries: usize,
    pub errors: Vec<String>,
}

impl NonRustPolicyValidation {
    fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Validate the non-Rust allowlist and debt TOML schema without walking git.
///
/// This is the Rust successor to `scripts/policy/validate_non_rust_allowlist.py`:
/// fast schema validation lives next to the main file-policy engine, while the
/// compatibility script only delegates to this command.
pub fn validate_non_rust_policy(config: ValidateNonRustPolicyConfig) -> Result<()> {
    let validation = validate_non_rust_policy_files(&config.allowlist_path, &config.debt_path);

    if validation.is_ok() {
        let allow_word = if validation.allow_entries == 1 { "entry" } else { "entries" };
        let debt_word = if validation.debt_entries == 1 { "entry" } else { "entries" };
        println!(
            "OK: validated {} allow {} and {} debt {}.",
            validation.allow_entries, allow_word, validation.debt_entries, debt_word
        );
        return Ok(());
    }

    eprintln!("FAIL: {} non-Rust policy validation error(s):", validation.errors.len());
    for error in &validation.errors {
        eprintln!("  - {error}");
    }
    Err(eyre!("non-Rust policy validation failed with {} error(s)", validation.errors.len()))
}

fn validate_non_rust_policy_files(
    allowlist_path: &std::path::Path,
    debt_path: &std::path::Path,
) -> NonRustPolicyValidation {
    let mut errors = Vec::new();
    let allow_entries = validate_policy_table(allowlist_path, "allow", true, &mut errors);
    let debt_entries = validate_policy_table(debt_path, "debt", false, &mut errors);
    NonRustPolicyValidation { allow_entries, debt_entries, errors }
}

fn validate_policy_table(
    path: &std::path::Path,
    table_name: &str,
    strict_allow_schema: bool,
    errors: &mut Vec<String>,
) -> usize {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) => {
            errors.push(format!("FAIL: read {}: {err}", path.display()));
            return 0;
        }
    };
    let data = match toml::from_str::<toml::Value>(&text) {
        Ok(data) => data,
        Err(err) => {
            errors.push(format!("FAIL: parse {}: {err}", path.display()));
            return 0;
        }
    };

    let Some(entries) = data.get(table_name) else {
        return 0;
    };
    let Some(entries) = entries.as_array() else {
        errors.push(format!("{}: `{table_name}` must be a list of tables", path.display()));
        return 0;
    };

    let mut seen_ids: BTreeMap<String, usize> = BTreeMap::new();
    let mut seen_matchers: BTreeMap<String, String> = BTreeMap::new();
    for (index, entry) in entries.iter().enumerate() {
        let Some(table) = entry.as_table() else {
            errors
                .push(format!("{}: `{table_name}` entry #{index} must be a table", path.display()));
            continue;
        };

        if strict_allow_schema {
            validate_allow_schema_entry(table, index, errors);
        }

        let entry_id = table
            .get("id")
            .and_then(toml::Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| format!("<unnamed entry #{index}>"));

        if let Some(id) = table.get("id").and_then(toml::Value::as_str) {
            if let Some(previous) = seen_ids.insert(id.to_string(), index) {
                errors.push(format!("{id}: duplicate id (also at index {previous})"));
            }
        }

        let matcher = table.get("glob").or_else(|| table.get("path")).and_then(toml::Value::as_str);
        if let Some(matcher) = matcher {
            if let Some(previous_id) = seen_matchers.insert(matcher.to_string(), entry_id.clone()) {
                errors.push(format!(
                    "{entry_id}: duplicate matcher `{matcher}` (also used by id `{previous_id}`)"
                ));
            }
        }
    }

    entries.len()
}

fn validate_allow_schema_entry(
    entry: &toml::map::Map<String, toml::Value>,
    index: usize,
    errors: &mut Vec<String>,
) {
    let entry_id = entry.get("id").and_then(toml::Value::as_str).unwrap_or("<unnamed entry>");
    let fallback_id = format!("<unnamed entry #{index}>");
    let entry_id = if entry_id == "<unnamed entry>" { fallback_id.as_str() } else { entry_id };

    let has_glob = entry.contains_key("glob");
    let has_path = entry.contains_key("path");
    if has_glob && has_path {
        errors.push(format!("{entry_id}: cannot set both `glob` and `path`"));
    }
    if !has_glob && !has_path {
        errors.push(format!("{entry_id}: must set either `glob` or `path`"));
    }

    if let Some(matcher) =
        entry.get("glob").or_else(|| entry.get("path")).and_then(toml::Value::as_str)
    {
        validate_repo_relative_matcher(entry_id, matcher, errors);
        if has_glob && is_policy_broad_glob(matcher) {
            let has_reason = entry
                .get("broad_glob_reason")
                .and_then(toml::Value::as_str)
                .is_some_and(|reason| !reason.trim().is_empty());
            if !has_reason {
                errors.push(format!(
                    "{entry_id}: glob `{matcher}` is broad; declare `broad_glob_reason`"
                ));
            }
        }
    }

    for field in REQUIRED_ALLOW_FIELDS {
        if !entry.contains_key(*field) {
            errors.push(format!("{entry_id}: missing required field `{field}`"));
        }
    }

    for field in entry.keys() {
        if !ALLOWED_ALLOW_FIELDS.contains(&field.as_str()) {
            errors.push(format!("{entry_id}: unknown field `{field}`"));
        }
    }

    if let Some(classification) = entry.get("classification").and_then(toml::Value::as_str) {
        if !KNOWN_CLASSIFICATIONS.contains(&classification) {
            errors.push(format!(
                "{entry_id}: classification `{classification}` not in {:?}",
                KNOWN_CLASSIFICATIONS
            ));
        }
    }

    validate_covered_by(entry_id, entry, errors);
    validate_policy_dates(entry_id, entry, errors);

    if let Some(retired) = entry.get("retired") {
        if retired.as_bool().is_none() {
            errors.push(format!("{entry_id}: `retired` must be a boolean"));
        }
    }
}

fn validate_repo_relative_matcher(entry_id: &str, matcher: &str, errors: &mut Vec<String>) {
    if matcher.starts_with("./") || matcher.starts_with('/') {
        errors.push(format!(
            "{entry_id}: matcher `{matcher}` must be repo-relative without leading `./` or `/`"
        ));
    }
    if matcher.contains('\\') {
        errors.push(format!("{entry_id}: matcher `{matcher}` contains Windows backslashes"));
    }
    if matcher.trim() != matcher {
        errors.push(format!("{entry_id}: matcher `{matcher}` has surrounding whitespace"));
    }
}

fn validate_covered_by(
    entry_id: &str,
    entry: &toml::map::Map<String, toml::Value>,
    errors: &mut Vec<String>,
) {
    let covered_by = entry.get("covered_by");
    let covered_by_strings = covered_by
        .and_then(toml::Value::as_array)
        .map(|items| items.iter().all(|item| item.as_str().is_some()));
    if covered_by.is_some() && covered_by_strings != Some(true) {
        errors.push(format!("{entry_id}: `covered_by` must be a list of strings"));
        return;
    }

    let classification = entry.get("classification").and_then(toml::Value::as_str);
    let coverage_required = classification
        .is_some_and(|classification| COVERAGE_REQUIRING_CLASSIFICATIONS.contains(&classification));
    let coverage_empty = covered_by.and_then(toml::Value::as_array).is_none_or(Vec::is_empty);
    if coverage_required && coverage_empty {
        let classification = classification.unwrap_or("unknown");
        errors.push(format!(
            "{entry_id}: classification `{classification}` requires at least one `covered_by` entry"
        ));
    }
}

fn validate_policy_dates(
    entry_id: &str,
    entry: &toml::map::Map<String, toml::Value>,
    errors: &mut Vec<String>,
) {
    let created = parse_policy_date(entry_id, entry, "created", errors);
    let review_after = parse_policy_date(entry_id, entry, "review_after", errors);
    let expires = if entry.contains_key("expires") {
        parse_policy_date(entry_id, entry, "expires", errors)
    } else {
        None
    };

    if let (Some(created), Some(review_after)) = (created, review_after) {
        if review_after <= created {
            errors.push(format!("{entry_id}: `review_after` must be after `created`"));
        }
    }
    if let (Some(created), Some(expires)) = (created, expires) {
        if expires <= created {
            errors.push(format!("{entry_id}: `expires` must be after `created`"));
        }
    }
}

fn parse_policy_date(
    entry_id: &str,
    entry: &toml::map::Map<String, toml::Value>,
    field: &str,
    errors: &mut Vec<String>,
) -> Option<chrono::NaiveDate> {
    let value = entry.get(field)?;
    let Some(value) = value.as_str() else {
        errors.push(format!("{entry_id}: `{field}` must be a YYYY-MM-DD string"));
        return None;
    };
    match chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        Ok(date) => Some(date),
        Err(_) => {
            errors.push(format!("{entry_id}: `{field}` is not a real date: {value:?}"));
            None
        }
    }
}

/// Broad-glob heuristic for policy schema validation. Mirrors the original
/// Python gate and intentionally catches more than the strict enforcement
/// broad-glob helper.
fn is_policy_broad_glob(glob_str: &str) -> bool {
    glob_str.starts_with("**")
        || glob_str.ends_with("/**")
        || glob_str == "*.md"
        || glob_str.starts_with("**/")
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
// Proposal generator — `cargo xtask non-rust propose`
// ---------------------------------------------------------------------------

/// Grouping strategy for `cargo xtask non-rust propose`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProposeGroupBy {
    /// Group by top-level directory (default).
    Directory,
    /// Group by file extension.
    Extension,
}

/// Configuration for `cargo xtask non-rust propose`.
pub struct ProposeConfig {
    /// Output directory (defaults to `target/policy`).
    pub output_dir: std::path::PathBuf,
    /// How to group unclassified files.
    pub group_by: ProposeGroupBy,
    /// Override the workspace root used for `git ls-files` (test seam).
    pub root_override: Option<std::path::PathBuf>,
}

/// Return today's date as `YYYY-MM-DD` using Unix timestamp arithmetic.
pub fn today_ymd() -> (u32, u32, u32) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let days = secs / 86400;
    days_to_ymd(days)
}

/// Add `n` days to a `(year, month, day)` tuple using the Julian Day Number
/// method. Accurate for years 1970-2200.
pub fn add_days(ymd: (u32, u32, u32), n: u32) -> (u32, u32, u32) {
    // Convert (y, m, d) → JDN using the standard proleptic Gregorian formula.
    // All arithmetic is signed to avoid underflow.
    let (year, month, day) = (ymd.0 as i64, ymd.1 as i64, ymd.2 as i64);
    let a = (14 - month) / 12;
    let y = year + 4800 - a;
    let m = month + 12 * a - 3;
    let jdn = day + (153 * m + 2) / 5 + 365 * y + y / 4 - y / 100 + y / 400 - 32045;
    // JDN for Unix epoch (1970-01-01) = 2440588.
    let unix_days = (jdn - 2_440_588 + n as i64) as u64;
    days_to_ymd(unix_days)
}

/// Format a `(year, month, day)` tuple as `YYYY-MM-DD`.
pub fn fmt_ymd(ymd: (u32, u32, u32)) -> String {
    format!("{:04}-{:02}-{:02}", ymd.0, ymd.1, ymd.2)
}

/// Heuristic: infer classification from a top-level directory name.
fn classify_dir(dir: &str) -> &'static str {
    match dir {
        "docs" | "doc" | "book" | "guide" | "guides" | "wiki" | "website" | "pages" => "docs",
        "test" | "tests" | "t" | "spec" | "specs" | "fixtures" | "test_corpus" | "test-corpus" => {
            "test"
        }
        "vendor" | "third_party" | "third-party" | "extern" | "external" => "vendor",
        "scripts" | "bin" | "tools" | "tool" | "ci" | ".ci" | ".github" | "xtask" => "build",
        "data" | "assets" | "static" | "public" | "resources" | "corpus" | "samples" => "data",
        "vscode-extension" | "vscode" | "editor" | "editors" => "data",
        _ => "tbd",
    }
}

/// Heuristic: infer classification from a file extension.
fn classify_ext(ext: &str) -> &'static str {
    match ext {
        "md" | "rst" | "txt" | "adoc" | "asciidoc" => "docs",
        "toml" | "yaml" | "yml" | "json" | "ron" | "json5" => "build",
        "sh" | "bash" | "zsh" | "fish" | "ps1" | "bat" | "cmd" => "build",
        "py" | "js" | "ts" | "rb" | "pl" | "pm" | "lua" | "tcl" => "build",
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "ico" | "webp" => "data",
        "woff" | "woff2" | "ttf" | "eot" | "otf" => "data",
        "pdf" | "docx" | "xlsx" | "pptx" => "docs",
        "nix" | "lock" | "makefile" | "mk" | "cmake" => "build",
        "html" | "css" | "scss" | "less" => "data",
        "proto" | "thrift" | "avsc" => "data",
        "csv" | "tsv" | "parquet" => "data",
        "" => "tbd",
        _ => "tbd",
    }
}

/// Entry point for `cargo xtask non-rust propose`.
///
/// Reads the current inventory, groups unclassified files by the chosen
/// strategy, and writes two output files:
///
/// - `<output_dir>/non-rust-proposed-allowlist.toml` — draft allowlist entries.
/// - `<output_dir>/non-rust-proposal.md` — human-readable summary.
///
/// The canonical `policy/non-rust-allowlist.toml` is NEVER modified.
pub fn non_rust_propose(root: &Path, config: ProposeConfig) -> Result<()> {
    let effective_root: std::path::PathBuf =
        if let Some(ref r) = config.root_override { r.clone() } else { root.to_path_buf() };
    let root = effective_root.as_path();

    println!("Building inventory for proposal generation...");

    let allowlist = load_allowlist(root)?;
    let tracked = list_tracked_files(root)?;
    let prepared = prepare_allow_entries(&allowlist.allow);

    // Collect unclassified non-Rust files.
    let unclassified: Vec<String> = tracked
        .iter()
        .filter_map(|p| {
            let record = classify_file_with_prepared(p, &prepared);
            if record.category == "unclassified" { Some(p.clone()) } else { None }
        })
        .collect();

    println!("  {} unclassified files to group", unclassified.len());

    // Group files.
    let groups: BTreeMap<String, Vec<String>> = match config.group_by {
        ProposeGroupBy::Directory => group_by_directory(&unclassified),
        ProposeGroupBy::Extension => group_by_extension(&unclassified),
    };

    let today = today_ymd();
    let review_after = add_days(today, 30);
    let today_str = fmt_ymd(today);
    let review_after_str = fmt_ymd(review_after);

    // Build proposed AllowEntry list.
    let mut entries: Vec<AllowEntry> = Vec::new();
    for (group_key, files) in &groups {
        let (glob_pattern, entry_id) = match config.group_by {
            ProposeGroupBy::Directory => {
                // "(root)" is a virtual key for files that have no parent directory.
                // Their glob is simply "*" (all root-level files).
                let glob = if group_key == "(root)" {
                    "*".to_string()
                } else {
                    format!("{group_key}/**/*")
                };
                let sanitized = group_key
                    .chars()
                    .map(|c| if c == '/' || c == '.' || c == '(' || c == ')' { '-' } else { c })
                    .collect::<String>()
                    .to_lowercase();
                let id = format!("proposed-dir-{sanitized}");
                (glob, id)
            }
            ProposeGroupBy::Extension => {
                let glob = if group_key.is_empty() {
                    // Files with no extension — list individually or use a tbd glob.
                    "**/*".to_string()
                } else {
                    format!("**/*.{group_key}")
                };
                let id = if group_key.is_empty() {
                    "proposed-ext-no-extension".to_string()
                } else {
                    format!("proposed-ext-{}", group_key.to_lowercase())
                };
                (glob, id)
            }
        };

        let classification = match config.group_by {
            ProposeGroupBy::Directory => {
                let top = group_key.split('/').next().unwrap_or(group_key.as_str());
                classify_dir(top)
            }
            ProposeGroupBy::Extension => classify_ext(group_key.as_str()),
        };

        let reason = match config.group_by {
            ProposeGroupBy::Directory => {
                format!("auto-proposed: {} files in {}/", files.len(), group_key)
            }
            ProposeGroupBy::Extension => {
                let ext_label = if group_key.is_empty() {
                    "(no extension)".to_string()
                } else {
                    format!(".{group_key}")
                };
                format!("auto-proposed: {} {} files", files.len(), ext_label)
            }
        };

        let broad_glob_reason = Some(
            "auto-proposed bulk classification — refine per-directory before promotion".to_string(),
        );

        entries.push(AllowEntry {
            id: entry_id,
            glob: Some(glob_pattern.clone()),
            path: None,
            kind: "non-rust".to_string(),
            language: "mixed".to_string(),
            surface: "unclassified".to_string(),
            classification: classification.to_string(),
            owner: "TBD".to_string(),
            reason,
            covered_by: vec![glob_pattern],
            created: today_str.clone(),
            review_after: review_after_str.clone(),
            expires: None,
            broad_glob_reason,
            retired: false,
        });
    }

    // Write output files.
    fs::create_dir_all(&config.output_dir)
        .with_context(|| format!("creating {}", config.output_dir.display()))?;

    let toml_path = config.output_dir.join("non-rust-proposed-allowlist.toml");
    let md_path = config.output_dir.join("non-rust-proposal.md");

    let toml_content = render_proposed_toml(&entries, config.group_by, today_str.as_str())?;
    fs::write(&toml_path, &toml_content)
        .with_context(|| format!("writing {}", toml_path.display()))?;
    println!("  wrote {}", toml_path.display());

    let md_content = render_proposal_markdown(&groups, &entries, config.group_by, &unclassified);
    fs::write(&md_path, &md_content).with_context(|| format!("writing {}", md_path.display()))?;
    println!("  wrote {}", md_path.display());

    println!(
        "\nProposal complete: {} unclassified files → {} groups\n\
         Review {} and {} before promoting to policy/non-rust-allowlist.toml",
        unclassified.len(),
        groups.len(),
        toml_path.display(),
        md_path.display()
    );

    Ok(())
}

/// Group files by their top-level directory component.
fn group_by_directory(files: &[String]) -> BTreeMap<String, Vec<String>> {
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for file in files {
        let top_dir = file.split('/').next().unwrap_or(file.as_str());
        // If a file has no directory component, group under "(root)".
        let key = if file.contains('/') { top_dir.to_string() } else { "(root)".to_string() };
        groups.entry(key).or_default().push(file.clone());
    }
    groups
}

/// Group files by their file extension (without leading dot).
fn group_by_extension(files: &[String]) -> BTreeMap<String, Vec<String>> {
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for file in files {
        let basename = file.rsplit('/').next().unwrap_or(file.as_str());
        let ext = basename
            .rsplit_once('.')
            .filter(|(stem, ext)| !stem.is_empty() && !ext.is_empty())
            .map(|(_, e)| e)
            .unwrap_or("")
            .to_lowercase();
        groups.entry(ext).or_default().push(file.clone());
    }
    groups
}

/// Render the proposed allowlist as TOML.
fn render_proposed_toml(
    entries: &[AllowEntry],
    group_by: ProposeGroupBy,
    today: &str,
) -> Result<String> {
    let group_label = match group_by {
        ProposeGroupBy::Directory => "directory",
        ProposeGroupBy::Extension => "extension",
    };

    let mut out = String::new();
    out.push_str("# Non-Rust Proposed Allowlist\n");
    out.push_str("#\n");
    out.push_str("# AUTO-GENERATED by `cargo xtask non-rust propose`.\n");
    out.push_str("# DO NOT edit directly. Review each entry and promote to\n");
    out.push_str("# policy/non-rust-allowlist.toml after setting owner/surface/classification.\n");
    out.push_str("#\n");
    out.push_str(&format!("# Generated: {today}\n"));
    out.push_str(&format!("# Grouped by: {group_label}\n"));
    out.push_str("#\n");
    out.push_str("# Fields marked TBD MUST be set by a human reviewer\n");
    out.push_str("# before promoting any entry into the canonical ledger.\n\n");

    out.push_str("schema_version = 1\n");
    out.push_str("policy = \"non-rust-allowlist\"\n");
    out.push_str("owner = \"TBD\"\n");
    out.push_str("status = \"proposed\"\n");
    out.push_str(&format!("updated = \"{today}\"\n\n"));

    out.push_str("[defaults]\n");
    out.push_str("rust_is_default = true\n");
    out.push_str("xtask_is_default_for_repo_automation = true\n");
    out.push_str("new_non_rust_requires_review = true\n");
    out.push_str("broad_globs_require_reason = true\n");
    out.push_str("coverage_required_for_production_surfaces = true\n\n");

    for entry in entries {
        out.push_str("[[allow]]\n");
        out.push_str(&format!("id = {:?}\n", entry.id));
        if let Some(ref g) = entry.glob {
            out.push_str(&format!("glob = {:?}\n", g));
        }
        if let Some(ref p) = entry.path {
            out.push_str(&format!("path = {:?}\n", p));
        }
        out.push_str(&format!("kind = {:?}\n", entry.kind));
        out.push_str(&format!("language = {:?}\n", entry.language));
        out.push_str(&format!("surface = {:?}\n", entry.surface));
        out.push_str(&format!("classification = {:?}\n", entry.classification));
        out.push_str(&format!("owner = {:?}\n", entry.owner));
        out.push_str(&format!("reason = {:?}\n", entry.reason));
        // covered_by array
        out.push_str("covered_by = [");
        for (i, c) in entry.covered_by.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            out.push_str(&format!("{c:?}"));
        }
        out.push_str("]\n");
        out.push_str(&format!("created = {:?}\n", entry.created));
        out.push_str(&format!("review_after = {:?}\n", entry.review_after));
        if let Some(ref bgr) = entry.broad_glob_reason {
            out.push_str(&format!("broad_glob_reason = {:?}\n", bgr));
        }
        out.push('\n');
    }

    Ok(out)
}

/// Render a human-readable markdown summary of the proposal.
fn render_proposal_markdown(
    groups: &BTreeMap<String, Vec<String>>,
    entries: &[AllowEntry],
    group_by: ProposeGroupBy,
    all_unclassified: &[String],
) -> String {
    let group_label = match group_by {
        ProposeGroupBy::Directory => "directory",
        ProposeGroupBy::Extension => "extension",
    };

    // Extension breakdown for summary.
    let mut ext_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for file in all_unclassified {
        let basename = file.rsplit('/').next().unwrap_or(file.as_str());
        let ext = basename.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
        *ext_counts.entry(ext).or_insert(0) += 1;
    }

    let mut out = String::new();
    out.push_str("# Non-Rust Allowlist Proposal\n\n");
    out.push_str("> AUTO-GENERATED by `cargo xtask non-rust propose`. Do not edit by hand.\n");
    out.push_str("> Review each group, set `owner`/`surface`/`classification`, then promote\n");
    out.push_str("> to `policy/non-rust-allowlist.toml`.\n\n");

    out.push_str("## Summary\n\n");
    out.push_str(&format!(
        "| Metric | Value |\n|---|---|\n\
         | Unclassified files | {} |\n\
         | Groups ({group_label}) | {} |\n\
         | Proposed entries | {} |\n\n",
        all_unclassified.len(),
        groups.len(),
        entries.len(),
    ));

    out.push_str("## Top extensions\n\n");
    out.push_str("| Extension | Count |\n|---|---|\n");
    let mut ext_vec: Vec<(&&str, &usize)> = ext_counts.iter().collect();
    ext_vec.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (ext, count) in ext_vec.iter().take(20) {
        let label = if ext.is_empty() { "(no ext)" } else { ext };
        out.push_str(&format!("| `.{label}` | {count} |\n"));
    }
    out.push('\n');

    let migration_candidates: Vec<_> = groups
        .iter()
        .filter_map(|(group_key, files)| {
            group_migration_rule(files).map(|rule| (group_key, files, rule))
        })
        .collect();

    out.push_str("## Rust migration candidates\n\n");
    if migration_candidates.is_empty() {
        out.push_str("No unclassified repo-automation scripts were detected in this proposal.\n\n");
    } else {
        out.push_str(
            "These groups contain non-Rust automation that should be reviewed for conversion \
             into Rust-owned tooling before broad allowlist promotion.\n\n",
        );
        out.push_str(
            "| Group | Files | Recommended destination | Rationale |\n|---|---:|---|---|\n",
        );
        for (group_key, files, rule) in &migration_candidates {
            out.push_str(&format!(
                "| `{group_key}` | {} | `{}` | {} |\n",
                files.len(),
                rule.target,
                rule.rationale
            ));
        }
        out.push('\n');
    }

    out.push_str(&format!("## Groups by {group_label}\n\n"));
    for (group_key, files) in groups {
        let entry_id = entries
            .iter()
            .find(|e| {
                e.reason.contains(&format!("{}/", group_key))
                    || e.reason.contains(group_key.as_str())
            })
            .map(|e| e.id.as_str())
            .unwrap_or("—");
        out.push_str(&format!("### `{group_key}` ({} files)\n\n", files.len()));
        out.push_str(&format!("- Proposed entry: `{entry_id}`\n"));
        out.push_str("- `owner`: TBD — must be set before promotion\n");
        out.push_str("- `surface`: unclassified — must be refined\n");
        if let Some(rule) = group_migration_rule(files) {
            out.push_str(&format!(
                "- Rust migration review: {} Target: `{}`.\n",
                rule.rationale, rule.target
            ));
        }
        // Show first 10 files as examples.
        if !files.is_empty() {
            out.push_str("- Sample files:\n");
            for f in files.iter().take(10) {
                out.push_str(&format!("  - `{f}`\n"));
            }
            if files.len() > 10 {
                out.push_str(&format!("  - … and {} more\n", files.len() - 10));
            }
        }
        out.push('\n');
    }

    out.push_str("## Next steps\n\n");
    out.push_str("1. Review `target/policy/non-rust-proposed-allowlist.toml`.\n");
    out.push_str("2. For each entry: set `owner`, `surface`, refine `classification`.\n");
    out.push_str("3. Copy approved entries into `policy/non-rust-allowlist.toml`.\n");
    out.push_str("4. Run `cargo xtask check-file-policy --mode advisory` to verify.\n");
    out.push_str("5. Do NOT promote entries with `owner = \"TBD\"`.\n");

    out
}

// ---------------------------------------------------------------------------
// Migration candidate finder — `cargo xtask non-rust migration-candidates`
// ---------------------------------------------------------------------------

/// Output format for non-Rust migration candidate reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MigrationCandidateFormat {
    /// Human-readable Markdown.
    Markdown,
    /// Machine-readable JSON.
    Json,
}

/// Configuration for `cargo xtask non-rust migration-candidates`.
pub struct MigrationCandidatesConfig {
    /// Output format.
    pub format: MigrationCandidateFormat,
    /// Optional output path. Prints to stdout when omitted.
    pub output: Option<std::path::PathBuf>,
    /// Maximum number of candidates to include.
    pub limit: Option<usize>,
    /// Override the workspace root used for `git ls-files` (test seam).
    pub root_override: Option<std::path::PathBuf>,
}

/// One non-Rust file that looks like a Rust migration candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MigrationCandidate {
    /// Repo-relative path.
    pub path: String,
    /// Language family inferred from extension and filename.
    pub language: String,
    /// Suggested Rust-owned destination.
    pub target: String,
    /// Stable priority bucket for review ordering.
    pub priority: String,
    /// Why the file belongs in the suggested target.
    pub rationale: String,
}

#[derive(Clone, Copy)]
struct MigrationRule {
    language: &'static str,
    target: &'static str,
    priority: &'static str,
    rationale: &'static str,
}

/// Entry point for `cargo xtask non-rust migration-candidates`.
///
/// The command is intentionally read-only. It identifies script-style tooling
/// that is already in a Rust-owned architectural lane (for example corpus
/// tooling belongs in `perl-corpus`, repo automation belongs in `xtask`) and
/// emits a deterministic review queue for future focused migration PRs.
pub fn non_rust_migration_candidates(root: &Path, config: MigrationCandidatesConfig) -> Result<()> {
    let effective_root: std::path::PathBuf =
        if let Some(ref r) = config.root_override { r.clone() } else { root.to_path_buf() };

    let mut candidates = collect_migration_candidates(effective_root.as_path())?;
    if let Some(limit) = config.limit {
        candidates.truncate(limit);
    }

    let rendered = match config.format {
        MigrationCandidateFormat::Markdown => render_migration_candidates_markdown(&candidates),
        MigrationCandidateFormat::Json => serde_json::to_string_pretty(&candidates)?,
    };

    if let Some(output) = config.output {
        if let Some(parent) = output.parent().filter(|parent| !parent.as_os_str().is_empty()) {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        fs::write(&output, rendered).with_context(|| format!("writing {}", output.display()))?;
        println!("wrote {} migration candidates to {}", candidates.len(), output.display());
    } else {
        println!("{rendered}");
    }

    Ok(())
}

fn collect_migration_candidates(root: &Path) -> Result<Vec<MigrationCandidate>> {
    let tracked = list_tracked_files(root)?;
    let mut candidates: Vec<MigrationCandidate> = tracked
        .iter()
        .filter_map(|path| {
            migration_rule_for_path(path).map(|rule| candidate_from_rule(path, rule))
        })
        .collect();

    candidates.sort_by(|a, b| {
        priority_rank(&a.priority)
            .cmp(&priority_rank(&b.priority))
            .then_with(|| a.target.cmp(&b.target))
            .then_with(|| a.path.cmp(&b.path))
    });
    Ok(candidates)
}

fn candidate_from_rule(path: &str, rule: MigrationRule) -> MigrationCandidate {
    MigrationCandidate {
        path: path.to_string(),
        language: rule.language.to_string(),
        target: rule.target.to_string(),
        priority: rule.priority.to_string(),
        rationale: rule.rationale.to_string(),
    }
}

fn priority_rank(priority: &str) -> u8 {
    match priority {
        "high" => 0,
        "medium" => 1,
        "low" => 2,
        _ => 3,
    }
}

fn migration_rule_for_path(path: &str) -> Option<MigrationRule> {
    let language = script_language(path)?;

    if path.starts_with("tools/corpus_") || path == "tools/add_metadata.py" {
        return Some(MigrationRule {
            language,
            target: "crates/perl-corpus",
            priority: "high",
            rationale: "Corpus linting, indexing, and metadata helpers belong with the Rust corpus crate and its `perl-corpus` CLI.",
        });
    }

    if path.starts_with("benchmarks/scripts/") {
        return Some(MigrationRule {
            language,
            target: "xtask benchmark/metrics tasks",
            priority: "medium",
            rationale: "Benchmark orchestration and result formatting should share Rust workspace metadata, receipts, and error handling through xtask.",
        });
    }

    if path.starts_with("ci/") {
        return Some(MigrationRule {
            language,
            target: "xtask policy/check tasks",
            priority: "medium",
            rationale: "CI policy checks should use the same Rust policy modules that local agent and gate commands exercise.",
        });
    }

    if path.starts_with("scripts/ci/") || path.starts_with("scripts/policy/") {
        return Some(MigrationRule {
            language,
            target: "xtask policy/check tasks",
            priority: "medium",
            rationale: "Policy and CI receipts are core repository automation and should be implemented as typed xtask tasks.",
        });
    }

    if path.starts_with("scripts/") && path.ends_with(".py") {
        return Some(MigrationRule {
            language,
            target: "xtask tasks",
            priority: "medium",
            rationale: "Python repository automation should migrate to xtask when it does not require a Python-specific ecosystem API.",
        });
    }

    if path.starts_with("bin/") || path.starts_with("tools/") {
        return Some(MigrationRule {
            language,
            target: "xtask tasks",
            priority: "medium",
            rationale: "Repository helper scripts should migrate to typed xtask tasks when they do not require a language-specific ecosystem API.",
        });
    }

    if path == "install.sh" || path == "install.ps1" {
        return Some(MigrationRule {
            language,
            target: "install-surface checks",
            priority: "low",
            rationale: "Installer validation and shared install-surface logic should live in Rust-owned release checks.",
        });
    }

    if path.starts_with("scripts/check-") || path.starts_with("scripts/validate-") {
        return Some(MigrationRule {
            language,
            target: "xtask policy/check tasks",
            priority: "low",
            rationale: "Shell validation wrappers are candidates for typed xtask checks once their external command surface is stable.",
        });
    }

    None
}

fn group_migration_rule(files: &[String]) -> Option<MigrationRule> {
    files.iter().find_map(|file| migration_rule_for_path(file))
}

fn script_language(path: &str) -> Option<&'static str> {
    let basename = path.rsplit('/').next().unwrap_or(path);
    let ext = basename.rsplit_once('.').map(|(_, ext)| ext).unwrap_or("");
    match ext {
        "py" => Some("python"),
        "sh" | "bash" | "zsh" | "fish" => Some("shell"),
        "ps1" | "bat" | "cmd" => Some("shell"),
        "js" => Some("javascript"),
        "ts" => Some("typescript"),
        "rb" => Some("ruby"),
        "pl" => Some("perl"),
        _ => None,
    }
}

fn render_migration_candidates_markdown(candidates: &[MigrationCandidate]) -> String {
    let mut by_priority: BTreeMap<&str, usize> = BTreeMap::new();
    let mut by_target: BTreeMap<&str, usize> = BTreeMap::new();
    for candidate in candidates {
        *by_priority.entry(candidate.priority.as_str()).or_default() += 1;
        *by_target.entry(candidate.target.as_str()).or_default() += 1;
    }

    let mut out = String::new();
    out.push_str("# Non-Rust Migration Candidates\n\n");
    out.push_str("> AUTO-GENERATED by `cargo xtask non-rust migration-candidates`.\n");
    out.push_str("> Use this as a review queue; migrate one concern per PR.\n\n");

    out.push_str("## Summary\n\n");
    out.push_str("| Metric | Value |\n|---|---|\n");
    out.push_str(&format!("| Candidates | {} |\n", candidates.len()));
    out.push_str(&format!("| Targets | {} |\n", by_target.len()));
    out.push('\n');

    out.push_str("## By priority\n\n");
    out.push_str("| Priority | Count |\n|---|---:|\n");
    for priority in ["high", "medium", "low"] {
        if let Some(count) = by_priority.get(priority) {
            out.push_str(&format!("| {priority} | {count} |\n"));
        }
    }
    out.push('\n');

    out.push_str("## By target\n\n");
    out.push_str("| Target | Count |\n|---|---:|\n");
    for (target, count) in by_target {
        out.push_str(&format!("| `{target}` | {count} |\n"));
    }
    out.push('\n');

    out.push_str("## Candidates\n\n");
    out.push_str("| Priority | Path | Language | Target | Rationale |\n");
    out.push_str("|---|---|---|---|---|\n");
    for candidate in candidates {
        out.push_str(&format!(
            "| {} | `{}` | {} | `{}` | {} |\n",
            candidate.priority,
            candidate.path,
            candidate.language,
            candidate.target,
            candidate.rationale
        ));
    }

    out
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

    // --- migration candidate finder ---

    #[test]
    fn migration_rule_routes_corpus_tools_to_perl_corpus() -> Result<()> {
        let candidate = migration_rule_for_path("tools/corpus_lint.py")
            .ok_or_else(|| eyre!("expected corpus lint tool to be a migration candidate"))?;

        assert_eq!(candidate.language, "python");
        assert_eq!(candidate.priority, "high");
        assert_eq!(candidate.target, "crates/perl-corpus");
        assert!(candidate.rationale.contains("perl-corpus"));
        Ok(())
    }

    #[test]
    fn migration_rule_routes_ci_shell_checks_to_xtask_policy() -> Result<()> {
        let candidate = migration_rule_for_path("ci/check_doc_hygiene.sh")
            .ok_or_else(|| eyre!("expected CI shell check to be a migration candidate"))?;

        assert_eq!(candidate.language, "shell");
        assert_eq!(candidate.priority, "medium");
        assert_eq!(candidate.target, "xtask policy/check tasks");
        Ok(())
    }

    #[test]
    fn migration_rule_ignores_data_and_rust_files() {
        assert!(migration_rule_for_path("test_corpus/basic_constructs.pl").is_none());
        assert!(migration_rule_for_path("xtask/src/main.rs").is_none());
    }

    #[test]
    fn render_migration_candidates_markdown_summarizes_targets() {
        let candidates = vec![candidate_from_rule(
            "tools/corpus_index.py",
            MigrationRule {
                language: "python",
                target: "crates/perl-corpus",
                priority: "high",
                rationale: "Corpus indexing belongs in the Rust corpus CLI.",
            },
        )];

        let report = render_migration_candidates_markdown(&candidates);

        assert!(report.contains("# Non-Rust Migration Candidates"));
        assert!(report.contains("| Candidates | 1 |"));
        assert!(report.contains("`tools/corpus_index.py`"));
        assert!(report.contains("`crates/perl-corpus`"));
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

    #[test]
    fn validate_non_rust_policy_accepts_current_schema_extensions() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let allowlist = temp.path().join("allow.toml");
        let debt = temp.path().join("debt.toml");
        fs::write(
            &allowlist,
            r#"
[[allow]]
id = "generated-badge"
glob = "badges/*.json"
kind = "generated_badge_endpoint"
language = "json"
surface = "docs"
classification = "generated"
owner = "release/ci"
reason = "Generated badge data."
generated_by = "cargo xtask badges"
covered_by = []
created = "2026-05-13"
review_after = "2026-08-13"
"#,
        )?;
        fs::write(&debt, "# empty debt ledger\n")?;

        let validation = validate_non_rust_policy_files(&allowlist, &debt);

        assert_eq!(validation.allow_entries, 1);
        assert_eq!(validation.debt_entries, 0);
        assert!(validation.errors.is_empty(), "unexpected errors: {:?}", validation.errors);
        Ok(())
    }

    #[test]
    fn validate_non_rust_policy_reports_schema_errors() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let allowlist = temp.path().join("allow.toml");
        let debt = temp.path().join("debt.toml");
        fs::write(
            &allowlist,
            r#"
[[allow]]
id = "bad"
glob = "docs/**"
path = "docs/README.md"
kind = "documentation"
language = "markdown"
surface = "docs"
classification = "production"
owner = "docs"
reason = "Broken fixture."
covered_by = []
created = "2026-05-13"
review_after = "2026-05-13"
unknown = "field"
"#,
        )?;
        fs::write(&debt, "debt = []\n")?;

        let validation = validate_non_rust_policy_files(&allowlist, &debt);

        assert!(
            validation.errors.iter().any(|error| error.contains("cannot set both")),
            "missing matcher conflict error: {:?}",
            validation.errors
        );
        assert!(
            validation.errors.iter().any(|error| error.contains("requires at least one")),
            "missing coverage requirement error: {:?}",
            validation.errors
        );
        assert!(
            validation.errors.iter().any(|error| error.contains("unknown field")),
            "missing unknown field error: {:?}",
            validation.errors
        );
        assert!(
            validation.errors.iter().any(|error| error.contains("review_after")),
            "missing date ordering error: {:?}",
            validation.errors
        );
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
