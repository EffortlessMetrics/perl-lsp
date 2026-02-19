//! Corpus-level NodeKind coverage enforcement.
//!
//! These tests parse every file discovered by `perl_corpus::get_test_files()` and
//! verify that (a) every non-synthetic NodeKind appears at least once, and
//! (b) every required kind appears in at least 2 distinct files ("angles") unless
//! explicitly allow-listed.

use std::collections::{HashMap, HashSet};

use perl_parser::Parser;

mod nodekind_helpers;
use nodekind_helpers::{collect_node_kinds, collect_node_kinds_labeled, corpus_required_kinds};

/// Kinds that genuinely cannot appear in more than one corpus file.
/// Each entry is `(kind_name, reason)`.
///
/// This allowlist is intentionally small — most kinds should be exercisable from
/// multiple syntactic contexts.  Add entries only after confirming the kind truly
/// has only one natural surface syntax.
const SINGLE_FILE_ALLOWLIST: &[(&str, &str)] = &[
    // Tuned after first run — add entries as needed.
];

// ---------------------------------------------------------------------------
// Test 1 — Hard fail on missing kinds
// ---------------------------------------------------------------------------

#[test]
fn test_corpus_nodekind_coverage() {
    let files = perl_corpus::get_test_files();
    assert!(!files.is_empty(), "perl_corpus::get_test_files() returned no files");

    let required = corpus_required_kinds();
    let mut observed = HashSet::new();
    let mut parse_failure_count: usize = 0;

    for path in &files {
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("WARN: could not read {}: {e}", path.display());
                continue;
            }
        };

        let mut parser = Parser::new(&source);
        let output = parser.parse_with_recovery();

        if !output.diagnostics.is_empty() {
            parse_failure_count += 1;
        }

        collect_node_kinds(&output.ast, &mut observed);
    }

    if parse_failure_count > 0 {
        eprintln!(
            "INFO: {parse_failure_count}/{} corpus files had parse diagnostics (non-blocking)",
            files.len()
        );
    }

    let mut missing: Vec<&str> =
        required.iter().copied().filter(|k| !observed.contains(k)).collect();
    missing.sort();

    assert!(
        missing.is_empty(),
        "Corpus is missing NodeKind coverage for: {missing:?}\n\
         Add corpus fixtures (test_corpus/*.pl) that exercise these kinds."
    );
}

// ---------------------------------------------------------------------------
// Test 2 — Hard fail on thin coverage ("angles")
// ---------------------------------------------------------------------------

#[test]
fn test_corpus_nodekind_angles() {
    let files = perl_corpus::get_test_files();
    assert!(!files.is_empty(), "perl_corpus::get_test_files() returned no files");

    let required = corpus_required_kinds();
    let allowlist_set: HashSet<&str> = SINGLE_FILE_ALLOWLIST.iter().map(|(k, _)| *k).collect();

    let mut kind_to_files: HashMap<&'static str, HashSet<String>> = HashMap::new();

    for path in &files {
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("WARN: could not read {}: {e}", path.display());
                continue;
            }
        };

        let label = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();

        let mut parser = Parser::new(&source);
        let output = parser.parse_with_recovery();

        collect_node_kinds_labeled(&output.ast, &label, &mut kind_to_files);
    }

    // Check 1: completely missing kinds
    let mut completely_missing: Vec<&str> =
        required.iter().copied().filter(|k| !kind_to_files.contains_key(k)).collect();
    completely_missing.sort();

    assert!(
        completely_missing.is_empty(),
        "Corpus is completely missing NodeKind(s): {completely_missing:?}\n\
         Add corpus fixtures that exercise these kinds."
    );

    // Check 2: kinds appearing in only 1 file (thin coverage)
    let mut thin: Vec<(&str, usize)> = Vec::new();
    for kind in &required {
        if allowlist_set.contains(kind) {
            continue;
        }
        if let Some(file_set) = kind_to_files.get(kind) {
            if file_set.len() < 2 {
                thin.push((kind, file_set.len()));
            }
        }
    }
    thin.sort_by_key(|(k, _)| *k);

    // Summary to stderr (always printed)
    eprintln!("\n=== NodeKind angle summary ===");
    let mut summary: Vec<_> = kind_to_files
        .iter()
        .filter(|(k, _)| required.contains(*k))
        .map(|(k, files)| (*k, files.len()))
        .collect();
    summary.sort_by_key(|(k, _)| *k);
    for (kind, count) in &summary {
        eprintln!("  {kind}: {count} file(s)");
    }
    eprintln!("==============================\n");

    assert!(
        thin.is_empty(),
        "Thin NodeKind coverage (appears in only 1 file, not in SINGLE_FILE_ALLOWLIST):\n{}\n\
         Either add more corpus fixtures or add to SINGLE_FILE_ALLOWLIST with justification.",
        thin.iter().map(|(k, n)| format!("  {k}: {n} file(s)")).collect::<Vec<_>>().join("\n")
    );
}
