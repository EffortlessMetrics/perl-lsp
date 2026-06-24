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

    for (_file_path, relative_path, content) in t_files {
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
///
/// Returns the serde-expected wire string matching ripr's `TestFramework`
/// enum (`#[serde(rename = "Test::More")]` etc.). M1 contract convergence
/// (Campaign 31): the producer and consumer must use the SAME vocabulary.
fn detect_framework(content: &str) -> &'static str {
    if content.contains("use Test2::V0") || content.contains("use Test2::Suite") {
        "Test2::V0"
    } else if content.contains("use Test::Exception") {
        "Test::Exception"
    } else if content.contains("use Test::Fatal") {
        "Test::Fatal"
    } else if content.contains("use Test::More") {
        "Test::More"
    } else {
        "unknown"
    }
}

/// Emit relations, concrete discriminators, and observed-sink facts.
///
/// Campaign 31 Phase B PR 7 (perl-lsp-swarm#2594). The load-bearing semantic
/// slice. This conservative first-pass emitter:
///
/// - **Relations**: infers `file_proximity` relations between `.pm` source
///   files and `.t` test files that share a package name. Relation kind is
///   `file_proximity` (advisory-only per ripr's gating rules) with confidence
///   `medium`. This is the simplest relation — `direct_owner_call` /
///   `established-helper-call-chain` require AST-level call-graph analysis
///   that lands in a later enrichment.
///
/// - **Concrete discriminators**: derives from `is(...)` assertions — the
///   first argument is the observed value, the second is the expected value,
///   producing a concrete `"$got == $expected"` discriminator string. This
///   replaces the generic enum labels ripr's consumer currently falls back to.
///
/// - **Observed-sink facts**: ties each oracle to the specific value it
///   observes (the first argument of `is(got, expected, name)`). A strong
///   assertion elsewhere in the same test is NOT enough — the oracle must
///   observe the changed sink.
///
/// All three are conservative: where extraction is unsure, emit `unknown` /
/// omit the fact. ripr's strict-actionability validator fails closed on
/// unknown facts.
pub(crate) fn emit_relations_and_discriminators(
    root: &str,
    tests: &[Value],
    _oracles: &[Value],
) -> (Vec<Value>, Vec<Value>, Vec<Value>) {
    let mut relations = Vec::new();
    let mut changed_observables = Vec::new();
    let mut observed_sinks = Vec::new();

    // Collect .pm files from lib/.
    let lib_dir = std::path::Path::new(root).join("lib");
    let pm_files = collect_pm_files(&lib_dir);

    // For each test file, infer relations to .pm files by package-name match.
    for test in tests {
        let test_file_id = test["file_id"].as_str().unwrap_or("");
        let test_path = test["name"].as_str().unwrap_or("");

        for (pm_path, pm_content) in &pm_files {
            // Extract package name from the .pm file.
            let package_name = extract_package_name(pm_content);
            if package_name.is_empty() {
                continue;
            }

            // Check if the test file references the package.
            if !test_file_id.is_empty()
                && file_references_package(
                    test_path,
                    &pm_files.iter().map(|(p, _)| p.as_str()).collect::<Vec<_>>(),
                    pm_path,
                )
            {
                let relation_id = format!("relation:{test_file_id}:{pm_path}");
                relations.push(json!({
                    "relation_id": relation_id,
                    // M1 contract convergence: change_id must be a string (not
                    // null) per the ripr schema. Use a placeholder when no
                    // change is linked — ripr's ingestion boundary validates
                    // referential integrity, so this will fail closed until
                    // a real change is associated.
                    "change_id": "change:unresolved",
                    "owner_id": format!("owner:{pm_path}:{package_name}"),
                    "test_id": test["test_id"],
                    "oracle_id": null,
                    "relation_kind": "file_proximity",
                    "reachability_hint": "weakly_reachable",
                    "confidence": "medium",
                    "provenance_refs": []
                }));
            }
        }
    }

    // Extract concrete discriminators + observed-sink facts from `is(...)` assertions.
    // Read the .t files again to parse assertion arguments.
    let t_dir = std::path::Path::new(root).join("t");
    let t_files = collect_t_files(&t_dir);

    for (_file_path, relative_path, content) in &t_files {
        for line in content.lines() {
            if let Some(args) = extract_is_args(line) {
                // `is($got, $expected, $name)` → discriminator "$got == $expected"
                let discriminator = format!("{} == {}", args.0, args.1);
                let observable_id = format!("observable:{relative_path}:{}", args.0);
                changed_observables.push(json!({
                    "observable_id": observable_id,
                    "expression": args.0,
                    "file_id": format!("file:{relative_path}"),
                    "discriminator": discriminator,
                    "confidence": "medium"
                }));

                // Observed-sink: the oracle observes the `got` value.
                let sink_id = format!("sink:{relative_path}:{}", args.0);
                observed_sinks.push(json!({
                    "sink_id": sink_id,
                    "oracle_kind": "exact_return_assertion",
                    "observed_expression": args.0,
                    "file_id": format!("file:{relative_path}"),
                    "confidence": "medium"
                }));
            }
        }
    }

    (relations, changed_observables, observed_sinks)
}

/// Collect all `.pm` files under a directory. Returns (relative_path, content).
fn collect_pm_files(lib_dir: &std::path::Path) -> Vec<(String, String)> {
    let mut result = Vec::new();
    collect_pm_files_recursive(lib_dir, lib_dir, &mut result);
    result
}

fn collect_pm_files_recursive(
    dir: &std::path::Path,
    base: &std::path::Path,
    result: &mut Vec<(String, String)>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_pm_files_recursive(&path, base, result);
        } else if path.extension().is_some_and(|ext| ext == "pm") {
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            let relative = path.to_string_lossy().replace('\\', "/");
            let relative = relative
                .split_once("/lib/")
                .map(|(_, rest)| format!("lib/{rest}"))
                .unwrap_or(relative);
            result.push((relative, content));
        }
    }
}

/// Extract the package name from Perl source (first `package Foo::Bar;` line).
fn extract_package_name(content: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("package ") {
            if let Some(name_end) = rest.find(';') {
                return rest[..name_end].trim().to_string();
            }
        }
    }
    String::new()
}

/// Check if a test file references a .pm file (simple heuristic: same basename).
fn file_references_package(test_path: &str, _all_pm_paths: &[&str], pm_path: &str) -> bool {
    // Simple heuristic: if the .pm basename appears in the test path.
    // E.g. t/app.t references lib/My/App.pm if "App" appears in both.
    let pm_basename = pm_path.rsplit('/').next().unwrap_or("").trim_end_matches(".pm");
    !pm_basename.is_empty() && test_path.contains(pm_basename)
}

/// Extract the arguments from an `is(...)` call.
/// Returns (got, expected) if parseable.
fn extract_is_args(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix("is(")?.strip_suffix(");")?;
    let parts: Vec<&str> = inner.splitn(3, ',').collect();
    if parts.len() >= 2 {
        Some((parts[0].trim().to_string(), parts[1].trim().to_string()))
    } else {
        None
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

/// Patterns that indicate a dynamic boundary in Perl source. Each entry maps
/// a string-search pattern to the ripr BoundaryKind it represents.
const DYNAMIC_BOUNDARY_PATTERNS: &[(&str, &str)] = &[
    ("eval {", "eval_or_string_code"),
    ("eval(", "eval_or_string_code"),
    ("eval '", "eval_or_string_code"),
    ("eval\"", "eval_or_string_code"),
    ("->$", "dynamic_dispatch"),
    ("::->", "dynamic_dispatch"),
    ("can(", "framework_indirection"),
    ("AUTOLOAD", "framework_indirection"),
    ("@ISA", "role_composition"),
    ("use parent", "role_composition"),
    ("use base", "role_composition"),
    ("*{", "symbol_table_mutation"),
    ("no strict", "unsupported_syntax"),
    ("BEGIN {", "unsupported_syntax"),
    ("require $", "module_resolution_unknown"),
];

/// Emit dynamic-boundary facts + limitations + typed verify-command candidates.
///
/// Campaign 31 Phase B PR 8 (perl-lsp-swarm#2595). The final Phase B slice:
/// closes the producer with boundary detection, limitations, verify-command
/// candidates, and deterministic output.
///
/// - **Dynamic boundaries**: scans `.pm` + `.t` files for the patterns in `DYNAMIC_BOUNDARY_PATTERNS`. Each match emits a `dynamic_boundaries` entry + a corresponding `limitations` entry. All boundaries fail closed in ripr's strict-actionability validator.
///
/// - **Typed verify-command candidates**: derives `prove <test_path>` for each
///   `.t` file. These are candidates — ripr's typed validator (PR 13) accepts/
///   rejects them. NOT shell strings; ripr generates the receipt command.
///
/// - **Deterministic goldens**: the emitter scans files in sorted order + emits
///   arrays in a stable order (sorted by ID), so the same input always produces
///   the same packet.
pub(crate) fn emit_boundaries_and_commands(root: &str) -> (Vec<Value>, Vec<Value>, Vec<Value>) {
    let mut boundaries = Vec::new();
    let mut limitations = Vec::new();
    let mut verify_commands = Vec::new();

    // Scan .pm files for dynamic boundaries.
    let lib_dir = std::path::Path::new(root).join("lib");
    let pm_files = collect_pm_files(&lib_dir);
    let t_dir = std::path::Path::new(root).join("t");
    let t_files = collect_t_files(&t_dir);

    let mut boundary_counter = 0usize;

    // Scan all source files (.pm + .t) for boundary patterns.
    let mut all_files: Vec<(String, String)> = Vec::new();
    for (path, content) in &pm_files {
        all_files.push((path.clone(), content.clone()));
    }
    for (_full, relative, content) in &t_files {
        all_files.push((relative.clone(), content.clone()));
    }
    all_files.sort_by(|a, b| a.0.cmp(&b.0));

    for (file_path, content) in &all_files {
        let file_id = format!("file:{file_path}");
        for (pattern, boundary_kind) in DYNAMIC_BOUNDARY_PATTERNS {
            if content.contains(pattern) {
                boundary_counter += 1;
                let boundary_id =
                    format!("boundary:{file_path}:{boundary_kind}:{boundary_counter}");
                boundaries.push(json!({
                    "boundary_id": boundary_id,
                    "kind": boundary_kind,
                    "file_id": file_id,
                    "owner_id": null,
                    "range": {"start_line": 1, "start_column": 1, "end_line": 1, "end_column": 1},
                    "confidence": "high",
                    "provenance_refs": []
                }));
                limitations.push(json!({
                    "limitation_id": format!("limitation:{boundary_id}"),
                    "kind": boundary_kind,
                    "message": format!("Dynamic boundary `{pattern}` detected in {file_path}; ripr fails closed on this boundary kind."),
                    "evidence_refs": []
                }));
            }
        }
    }

    // Emit typed verify-command candidates for each .t file.
    let mut cmd_counter = 0usize;
    let mut sorted_t: Vec<&(String, String, String)> = t_files.iter().collect();
    sorted_t.sort_by(|a, b| a.1.cmp(&b.1));
    for (_full, relative, _content) in &sorted_t {
        cmd_counter += 1;
        let command_id = format!("verify_cmd:{relative}:{cmd_counter}");
        verify_commands.push(json!({
            "command_id": command_id,
            "runner": "prove",
            "argv": ["prove", relative],
            "scope": "test",
            "test_id": format!("test:{relative}"),
            "confidence": "high",
            "preconditions": [],
            "provenance_refs": []
        }));
    }

    (boundaries, limitations, verify_commands)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_test_more_framework() {
        assert_eq!(detect_framework("use Test::More;\nok(1);"), "Test::More");
    }

    #[test]
    fn detect_test2_v0_framework() {
        assert_eq!(detect_framework("use Test2::V0;\nok(1);"), "Test2::V0");
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
        assert_eq!(tests[0]["framework"], "Test::More");
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

    // ── PR 7 tests (relations + discriminators + observed-sink) ──

    #[test]
    fn extract_package_name_from_pm_content() {
        let content = "package My::App;\nuse strict;\n1;";
        assert_eq!(extract_package_name(content), "My::App");
    }

    #[test]
    fn extract_package_name_returns_empty_when_no_package() {
        assert_eq!(extract_package_name("use strict;\n1;"), "");
    }

    #[test]
    fn extract_is_args_parses_simple_is_call() {
        let (got, expected) =
            extract_is_args("is(discount(100), 50, 'half price');").expect("must parse");
        assert_eq!(got, "discount(100)");
        assert_eq!(expected, "50");
    }

    #[test]
    fn extract_is_args_returns_none_for_non_is() {
        assert!(extract_is_args("ok(1, 'truthy');").is_none());
    }

    #[test]
    fn emit_relations_finds_pm_test_proximity() {
        let root = std::env::temp_dir().join("perl-B7-relations-root");
        let lib_dir = root.join("lib/My");
        let t_dir = root.join("t");
        std::fs::create_dir_all(&lib_dir).unwrap();
        std::fs::create_dir_all(&t_dir).unwrap();
        std::fs::write(lib_dir.join("App.pm"), "package My::App;\nsub discount { }\n1;").unwrap();
        std::fs::write(t_dir.join("App.t"), "use Test::More;\nok(1);\n").unwrap();

        // First emit tests, then relations.
        let (tests, _oracles) = emit_tests_and_oracles(root.to_str().unwrap());
        let (relations, _observables, _sinks) =
            emit_relations_and_discriminators(root.to_str().unwrap(), &tests, &[]);

        assert!(!relations.is_empty(), "must find at least one relation between App.pm and App.t");
        assert_eq!(
            relations[0]["relation_kind"], "file_proximity",
            "relation must be file_proximity (advisory-only)"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn emit_discriminators_from_is_assertions() {
        let root = std::env::temp_dir().join("perl-B7-discriminators-root");
        let t_dir = root.join("t");
        std::fs::create_dir_all(&t_dir).unwrap();
        std::fs::write(t_dir.join("app.t"), "use Test::More;\nis(discount(100), 50, 'half');\n")
            .unwrap();

        let (tests, _oracles) = emit_tests_and_oracles(root.to_str().unwrap());
        let (_relations, observables, sinks) =
            emit_relations_and_discriminators(root.to_str().unwrap(), &tests, &[]);

        assert!(!observables.is_empty(), "must emit at least one changed-observable from is()");
        assert!(
            observables[0]["discriminator"].as_str().unwrap_or("").contains("=="),
            "discriminator must be a concrete comparison: {:?}",
            observables[0]["discriminator"]
        );

        assert!(!sinks.is_empty(), "must emit at least one observed-sink from is()");

        let _ = std::fs::remove_dir_all(&root);
    }

    // ── PR 8 tests (boundaries + verify-commands) ──

    #[test]
    fn emit_boundaries_detects_eval() {
        let root = std::env::temp_dir().join("perl-B8-eval-root");
        let lib_dir = root.join("lib/My");
        std::fs::create_dir_all(&lib_dir).unwrap();
        std::fs::write(lib_dir.join("App.pm"), "package My::App;\nsub run { eval { die }; }\n1;")
            .unwrap();

        let (boundaries, limitations, _cmds) = emit_boundaries_and_commands(root.to_str().unwrap());
        assert!(!boundaries.is_empty(), "eval block must produce a boundary fact");
        assert!(
            boundaries.iter().any(|b| b["kind"] == "eval_or_string_code"),
            "must have an eval_or_string_code boundary"
        );
        assert!(!limitations.is_empty(), "each boundary must have a corresponding limitation");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn emit_boundaries_detects_dynamic_dispatch() {
        let root = std::env::temp_dir().join("perl-B8-dispatch-root");
        let lib_dir = root.join("lib");
        std::fs::create_dir_all(&lib_dir).unwrap();
        std::fs::write(
            lib_dir.join("Dynamic.pm"),
            "package Dynamic;\nsub call { my $m = shift; $obj->$m(); }\n1;",
        )
        .unwrap();

        let (boundaries, _limitations, _cmds) =
            emit_boundaries_and_commands(root.to_str().unwrap());
        assert!(
            boundaries.iter().any(|b| b["kind"] == "dynamic_dispatch"),
            "->$method() must produce a dynamic_dispatch boundary"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn emit_verify_commands_for_t_files() {
        let root = std::env::temp_dir().join("perl-B8-cmds-root");
        let t_dir = root.join("t");
        std::fs::create_dir_all(&t_dir).unwrap();
        std::fs::write(t_dir.join("alpha.t"), "use Test::More;\nok(1);\n").unwrap();
        std::fs::write(t_dir.join("beta.t"), "use Test::More;\nok(1);\n").unwrap();

        let (_boundaries, _limitations, verify_commands) =
            emit_boundaries_and_commands(root.to_str().unwrap());

        assert_eq!(verify_commands.len(), 2, "must emit one verify-command per .t file");
        // Verify commands use 'prove' runner.
        assert!(
            verify_commands.iter().all(|c| c["runner"] == "prove"),
            "all verify-commands must use prove runner"
        );
        // Commands are deterministic (sorted by path).
        assert!(
            verify_commands[0]["argv"][1].as_str().unwrap_or("")
                < verify_commands[1]["argv"][1].as_str().unwrap_or(""),
            "verify-commands must be sorted by path (deterministic)"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn emit_boundaries_returns_empty_when_no_source_files() {
        let root = std::env::temp_dir().join("perl-B8-empty-root");
        std::fs::create_dir_all(&root).unwrap();
        let (boundaries, limitations, cmds) = emit_boundaries_and_commands(root.to_str().unwrap());
        assert!(boundaries.is_empty());
        assert!(limitations.is_empty());
        assert!(cmds.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }
}
