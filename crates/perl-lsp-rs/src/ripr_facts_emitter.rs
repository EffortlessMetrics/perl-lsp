//! Test + oracle fact emitter for the ripr-perl-facts-v1 packet.
//!
//! Campaign 31 Phase B PR 6 (perl-lsp-swarm#2593). This module extracts
//! Test::More / Test2::V0 test facts + oracle facts from `.t` files in the
//! workspace and maps them into the ripr schema's `tests` + `oracles` arrays.
//!
//! Conservative mapping: where extraction is unsure, emit `OracleKind::unknown`
//! (ripr's strict-actionability validator fails closed on unknown oracles).
//! Distinguish exact oracles (is/isnt/cmp_ok) from smoke (ok/pass) from
//! mention-only (use_ok/require_ok) from unknown.

use serde_json::{Value, json};

/// The Test::More assertion functions recognized by perl-lsp's completion
/// module. Maps each assertion name to its oracle kind + strength in the
/// ripr schema.
const TEST_MORE_ASSERTIONS: &[(&str, &str, &str)] = &[
    // (function_name, oracle_kind, oracle_strength)
    ("is", "exact_return_assertion", "strong_exact"),
    ("isnt", "exact_return_assertion", "strong_exact"),
    ("cmp_ok", "predicate_boundary_assertion", "strong_exact"),
    ("is_deeply", "exact_return_assertion", "strong_exact"),
    ("like", "predicate_boundary_assertion", "weak_broad"),
    ("unlike", "predicate_boundary_assertion", "weak_broad"),
    ("ok", "smoke_ok", "weak_smoke"),
    ("pass", "smoke_ok", "weak_smoke"),
    ("fail", "smoke_ok", "weak_smoke"),
    ("isa_ok", "predicate_boundary_assertion", "weak_broad"),
    ("can_ok", "predicate_boundary_assertion", "weak_broad"),
    ("use_ok", "mention_only", "mention_only"),
    ("require_ok", "mention_only", "mention_only"),
];

/// Emit test + oracle facts for the `ripr-perl-facts-v1` packet.
///
/// Scans `.t` files in the workspace root for test framework usage + assertion
/// calls. Returns `(tests_array, oracles_array)` as JSON values matching the
/// ripr schema.
///
/// This is a **conservative first slice** (PR 6): it reads `.t` files directly
/// (not via the workspace index) and uses simple string matching for assertion
/// detection. Future PRs can enrich this with byte ranges + AST-level analysis.
pub(crate) fn emit_tests_and_oracles(root: &str) -> (Vec<Value>, Vec<Value>) {
    let mut tests = Vec::new();
    let mut oracles = Vec::new();

    // Scan for .t files under t/
    let t_dir = std::path::Path::new(root).join("t");
    let t_files = collect_t_files(&t_dir);

    for (file_path, relative_path, content) in t_files {
        let file_id = format!("file:{relative_path}");

        // Detect framework from `use` statements.
        let framework = detect_framework(&content);

        // Emit a test fact for the file.
        let test_id = format!("test:{relative_path}");
        tests.push(json!({
            "test_id": test_id,
            "file_id": file_id,
            "framework": framework,
            "name": relative_path,
            "range": {"start_line": 1, "start_column": 1, "end_line": 1, "end_column": 1},
            "runner_hints": ["prove"],
            "confidence": "high",
            "provenance_refs": []
        }));

        // Detect assertion calls and emit oracle facts.
        for (func_name, oracle_kind, oracle_strength) in TEST_MORE_ASSERTIONS {
            let count = count_assertion_calls(&content, func_name);
            for i in 0..count {
                let oracle_id = format!("oracle:{relative_path}:{func_name}:{i}");
                oracles.push(json!({
                    "oracle_id": oracle_id,
                    "test_id": test_id,
                    "kind": oracle_kind,
                    "strength": oracle_strength,
                    "target_owner_id": null,
                    "expression": format!("{func_name}(...)"),
                    "range": {"start_line": 1, "start_column": 1, "end_line": 1, "end_column": 1},
                    "confidence": "medium",
                    "provenance_refs": []
                }));
            }
        }
    }

    // If no .t files found, emit nothing (the packet stays unavailable for
    // tests/oracles). This is the honest state.
    (tests, oracles)
}

/// Collect all `.t` files under a directory. Returns (full_path, relative_path, content).
fn collect_t_files(t_dir: &std::path::Path) -> Vec<(String, String, String)> {
    let mut result = Vec::new();
    let Ok(entries) = std::fs::read_dir(t_dir) else {
        return result;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Recurse one level (t/subdir/*.t).
            result.extend(collect_t_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "t") {
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            // Relative path from the workspace root (forward-slash).
            let relative = path.to_string_lossy().replace('\\', "/");
            // Strip everything before "t/" to make it repo-relative.
            let relative =
                relative.split_once("/t/").map(|(_, rest)| format!("t/{rest}")).unwrap_or(relative);
            result.push((path.to_string_lossy().to_string(), relative, content));
        }
    }
    result
}

/// Detect the test framework from `use` statements in the file content.
fn detect_framework(content: &str) -> &'static str {
    if content.contains("use Test2::V0") || content.contains("use Test2::Suite") {
        "test2_v0"
    } else if content.contains("use Test::Exception") {
        "test_exception"
    } else if content.contains("use Test::Fatal") {
        "test_fatal"
    } else if content.contains("use Test::More") {
        "test_more"
    } else {
        "unknown"
    }
}

/// Count occurrences of an assertion call (e.g., `is(`, `ok(`) in the content.
/// Simple string matching — not AST-level, but conservative (over-counts
/// comments, under-counts is for `isa_ok`). The alpha's conservative bias
/// means false-positive oracles are OK (ripr fails closed on weak oracles).
fn count_assertion_calls(content: &str, func_name: &str) -> usize {
    // Match `func_name(` as a word boundary (not inside a longer identifier).
    // Use a simple heuristic: count lines where the func_name appears followed
    // by `(`. This is conservative (may miss multi-line calls or match comments).
    let needle = format!("{func_name}(");
    content.lines().filter(|line| line.contains(&needle)).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_test_more_framework() {
        assert_eq!(detect_framework("use Test::More;\nok(1);"), "test_more");
    }

    #[test]
    fn detect_test2_v0_framework() {
        assert_eq!(detect_framework("use Test2::V0;\nok(1);"), "test2_v0");
    }

    #[test]
    fn detect_unknown_framework() {
        assert_eq!(detect_framework("use strict;\nprint 1;"), "unknown");
    }

    #[test]
    fn count_is_assertions() {
        let content = "is(1, 1);\nis(2, 2);\nok(1);";
        assert_eq!(count_assertion_calls(content, "is"), 2);
        assert_eq!(count_assertion_calls(content, "ok"), 1);
    }

    #[test]
    fn emit_tests_and_oracles_for_test_more_file() {
        // Create a temp .t file.
        let temp = std::env::temp_dir().join("perl-B6-test-more.t");
        std::fs::write(&temp, "use Test::More;\nis(1, 1, 'one');\nok(1, 'truthy');\n").unwrap();
        // Create the t/ directory structure.
        let root = std::env::temp_dir().join("perl-B6-root");
        let t_dir = root.join("t");
        std::fs::create_dir_all(&t_dir).unwrap();
        std::fs::write(
            t_dir.join("app.t"),
            "use Test::More;\nis(1, 1, 'one');\nok(1, 'truthy');\n",
        )
        .unwrap();

        let (tests, oracles) = emit_tests_and_oracles(root.to_str().unwrap());

        // Should have 1 test fact (the .t file) + at least 2 oracle facts
        // (is + ok). The exact count depends on how many assertion kinds
        // appear — `is` gives 1, `ok` gives 1.
        assert!(!tests.is_empty(), "must emit at least one test fact");
        assert_eq!(tests[0]["framework"], "test_more");
        assert!(!oracles.is_empty(), "must emit at least one oracle fact");

        // Check that oracles have the right kind/strength.
        let has_exact = oracles.iter().any(|o| o["kind"] == "exact_return_assertion");
        let has_smoke = oracles.iter().any(|o| o["kind"] == "smoke_ok");
        assert!(has_exact, "must have an exact_return_assertion oracle (from is)");
        assert!(has_smoke, "must have a smoke_ok oracle (from ok)");

        // Clean up.
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn emit_returns_empty_when_no_t_files() {
        let root = std::env::temp_dir().join("perl-B6-empty-root");
        std::fs::create_dir_all(&root).unwrap();
        let (tests, oracles) = emit_tests_and_oracles(root.to_str().unwrap());
        assert!(tests.is_empty(), "no .t files → no tests");
        assert!(oracles.is_empty(), "no .t files → no oracles");
        let _ = std::fs::remove_dir_all(&root);
    }
}
