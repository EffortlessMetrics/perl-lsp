//! Corpus-level NodeKind coverage enforcement.
//!
//! These tests parse every file discovered by `perl_corpus::get_test_files()` and
//! verify that (a) every non-synthetic NodeKind appears at least once, and
//! (b) every required kind appears in at least 2 distinct contexts (files or
//! parent-kind diversity) unless explicitly allow-listed.
//!
//! Additionally, the tests track which kinds only appear through recovery parses
//! (files with diagnostics) and emit warnings — kinds observed only through
//! recovery are a sign of fragile coverage that should be addressed.

use std::collections::{BTreeMap, BTreeSet};

use perl_parser::Parser;

mod nodekind_helpers;
use nodekind_helpers::{
    collect_node_kinds, collect_node_kinds_labeled, collect_node_kinds_with_parents,
    corpus_required_kinds,
};

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
    let mut observed = BTreeSet::new();
    let mut clean_observed = BTreeSet::new();
    let mut clean_count: usize = 0;
    let mut recovery_count: usize = 0;

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

        let is_clean = output.diagnostics.is_empty();
        if is_clean {
            clean_count += 1;
            collect_node_kinds(&output.ast, &mut clean_observed);
        } else {
            recovery_count += 1;
        }

        // Always collect for the hard gate
        collect_node_kinds(&output.ast, &mut observed);
    }

    eprintln!(
        "INFO: {clean_count} clean / {recovery_count} recovery out of {} corpus files",
        files.len()
    );

    // Soft warning: kinds only seen through recovery
    let recovery_only: BTreeSet<_> = required
        .iter()
        .copied()
        .filter(|k| observed.contains(k) && !clean_observed.contains(k))
        .collect();
    if !recovery_only.is_empty() {
        eprintln!(
            "WARN: {} NodeKind(s) observed ONLY through recovery parses (fragile coverage): {:?}",
            recovery_only.len(),
            recovery_only
        );
    }

    // Hard gate: all required kinds must appear somewhere
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
// Test 2 — Hard fail on thin coverage ("angles") with parent-context diversity
// ---------------------------------------------------------------------------

#[test]
fn test_corpus_nodekind_angles() {
    let files = perl_corpus::get_test_files();
    assert!(!files.is_empty(), "perl_corpus::get_test_files() returned no files");

    let required = corpus_required_kinds();
    let allowlist_set: BTreeSet<&str> = SINGLE_FILE_ALLOWLIST.iter().map(|(k, _)| *k).collect();

    let mut kind_to_files: BTreeMap<&'static str, BTreeSet<String>> = BTreeMap::new();
    let mut kind_to_parents: BTreeMap<&'static str, BTreeSet<&'static str>> = BTreeMap::new();
    let mut clean_kind_to_files: BTreeMap<&'static str, BTreeSet<String>> = BTreeMap::new();

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

        let is_clean = output.diagnostics.is_empty();

        // Always collect for the hard gate
        collect_node_kinds_labeled(&output.ast, &label, &mut kind_to_files);
        collect_node_kinds_with_parents(&output.ast, None, &mut kind_to_parents);

        // Also track clean-only for informational reporting
        if is_clean {
            collect_node_kinds_labeled(&output.ast, &label, &mut clean_kind_to_files);
        }
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

    // Check 2: kinds with thin coverage (angle < 2)
    // Angle score = max(file_count, parent_context_count)
    let mut thin: Vec<(&str, usize, usize, usize)> = Vec::new();
    for kind in &required {
        if allowlist_set.contains(kind) {
            continue;
        }
        let file_count = kind_to_files.get(kind).map_or(0, |s| s.len());
        let parent_count = kind_to_parents.get(kind).map_or(0, |s| s.len());
        let angle = file_count.max(parent_count);
        if angle < 2 {
            thin.push((kind, file_count, parent_count, angle));
        }
    }
    thin.sort_by_key(|(k, _, _, _)| *k);

    // Summary to stderr (always printed, deterministic due to BTreeMap)
    eprintln!("\n=== NodeKind angle summary ===");
    for kind in &required {
        let file_count = kind_to_files.get(kind).map_or(0, |s| s.len());
        let parent_count = kind_to_parents.get(kind).map_or(0, |s| s.len());
        let clean_files = clean_kind_to_files.get(kind).map_or(0, |s| s.len());
        let angle = file_count.max(parent_count);
        let clean_marker = if clean_files == 0 { " [recovery-only]" } else { "" };
        eprintln!(
            "  {kind}: {angle} angle(s) ({file_count} file(s), {parent_count} parent(s)){clean_marker}"
        );
    }
    eprintln!("==============================\n");

    assert!(
        thin.is_empty(),
        "Thin NodeKind coverage (angle < 2, not in SINGLE_FILE_ALLOWLIST):\n{}\n\
         Either add more corpus fixtures or add to SINGLE_FILE_ALLOWLIST with justification.",
        thin.iter()
            .map(|(k, fc, pc, a)| format!("  {k}: angle={a} ({fc} file(s), {pc} parent(s))"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
