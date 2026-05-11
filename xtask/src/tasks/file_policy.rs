//! Non-Rust file inventory for the file-policy rollout.
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
//! The inventory is **read-only** — it never mutates the allowlist.  The
//! gating behaviour (`cargo xtask check-file-policy`) lands in PR 4.
//!
//! Refs: #8174.

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
