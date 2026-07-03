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

use perl_parser_core::line_index::LineIndex;
use perl_parser_core::{Node, NodeKind, Parser};
use perl_symbol::SymbolKind;
use perl_symbol::surface::{SymbolRefKind, extract_symbol_decls, extract_symbol_refs};
use serde_json::{Value, json};

/// Recognized test frameworks, most-specific first. Maps a module name (as it
/// appears in a parsed `use`) to the ripr `test.framework` wire enum. Order
/// matters: the `Test2::*` bundles and the `Test::Exception`/`Test::Fatal`
/// add-ons are checked before `Test::More` so a file that pulls both a bundle
/// and an add-on reports the more specific harness. Parser-backed
/// (`NodeKind::Use.module`), never a substring match.
const TEST_FRAMEWORKS: &[(&str, &str)] = &[
    ("Test2::V1", "Test2::V1"),
    ("Test2::V0", "Test2::V0"),
    ("Test2::Suite", "Test2::Suite"),
    ("Test::Exception", "Test::Exception"),
    ("Test::Fatal", "Test::Fatal"),
    ("Test::More", "Test::More"),
];

/// Assertion / exception-observer / warning-observer call names → ripr
/// `oracle.kind` + `oracle.strength`. Matched against the **real callee name of
/// a parsed call node** (via `extract_symbol_refs`), so `isa_ok` never counts as
/// `is`, and names inside comments or strings never match. `diag`/`note`/`plan`/
/// `done_testing`/`subtest` are intentionally absent — they are diagnostics or
/// test structure, not behavioral oracles.
const ASSERTION_ORACLES: &[(&str, &str, &str)] = &[
    // (call_name, oracle_kind, oracle_strength)
    // Test::More / Test2 comparisons
    ("is", "exact_return_assertion", "strong_exact"),
    ("isnt", "exact_return_assertion", "strong_exact"),
    ("is_deeply", "exact_return_assertion", "strong_exact"),
    ("cmp_ok", "predicate_boundary_assertion", "strong_exact"),
    ("like", "predicate_boundary_assertion", "weak_broad"),
    ("unlike", "predicate_boundary_assertion", "weak_broad"),
    ("isa_ok", "predicate_boundary_assertion", "weak_broad"),
    ("can_ok", "predicate_boundary_assertion", "weak_broad"),
    ("ok", "smoke_ok", "weak_smoke"),
    ("pass", "smoke_ok", "weak_smoke"),
    ("fail", "smoke_ok", "weak_smoke"),
    ("use_ok", "mention_only", "mention_only"),
    ("require_ok", "mention_only", "mention_only"),
    // Test::Exception / Test::Fatal / Test2 exception observers
    ("throws_ok", "exception_observer", "weak_broad"),
    ("dies_ok", "exception_observer", "weak_broad"),
    ("lives_ok", "smoke_ok", "weak_smoke"),
    ("lives_and", "exception_observer", "weak_broad"),
    ("exception", "exception_observer", "weak_broad"),
    ("dies", "exception_observer", "weak_broad"),
    ("lives", "smoke_ok", "weak_smoke"),
    // Test::Warn observers (commonly paired with the above)
    ("warning_is", "warn_observer", "weak_broad"),
    ("warning_like", "warn_observer", "weak_broad"),
    ("warnings_are", "warn_observer", "weak_broad"),
];

/// Emit parser-backed `tests[]`, `oracles[]`, `provenance[]`, and
/// `limitations[]` for the `ripr-perl-facts-v1` packet (#3293 PR 4).
///
/// For each `.t` file under `root/t`, this parses the file with
/// `perl-parser-core` and:
/// - detects the framework from parsed `use` statements (`NodeKind::Use.module`,
///   never `content.contains`) and emits one `test` fact with the file's **real
///   full-file range** plus a `test_discovery` provenance carrying the `use`
///   statement's range;
/// - emits one `oracle` fact per recognized assertion **call node**
///   (`extract_symbol_refs`, filtered to `SubroutineCall`), with the call's
///   **real source range**, a kind/strength from [`ASSERTION_ORACLES`], and the
///   call's source text as `expression`; and
/// - records limitations for unparseable files, recognized-framework-but-no-
///   oracle files (wrapped/aliased/dynamic helpers), and the narrower schema
///   representation (single call range, no observed/expected sub-ranges).
///
/// No string-scan assertion counting and no placeholder `1:1` ranges. On a parse
/// failure the file still yields a `test` fact (framework `unknown`) plus a
/// limitation — never silent, never string-fallback oracles.
pub(crate) fn emit_tests_and_oracles(
    root: &str,
) -> (Vec<Value>, Vec<Value>, Vec<Value>, Vec<Value>) {
    let mut tests = Vec::new();
    let mut oracles = Vec::new();
    let mut provenance = Vec::new();
    let mut limitations = Vec::new();

    let t_dir = std::path::Path::new(root).join("t");
    let mut t_files = collect_t_files(&t_dir);
    // Deterministic order: sort discovered test files by repo-relative path.
    t_files.sort_by(|a, b| a.1.cmp(&b.1));

    let mut any_oracles = false;

    for (_full_path, relative_path, content) in t_files {
        let file_id = format!("file:{relative_path}");
        let test_id = format!("test:{relative_path}");
        let import_prov_id = format!("prov:test_discovery:{file_id}");

        // Parse first (borrowing `content`), so `content` can then move into
        // `LineIndex` without a per-file clone.
        let parsed = {
            let mut parser = Parser::new(&content);
            parser.parse()
        };
        let ast = match parsed {
            Ok(ast) => ast,
            Err(_) => {
                // Never silent, never string-fallback: emit the test fact with
                // an unknown framework and record why oracles are absent.
                let content_len = content.len();
                let line_index = LineIndex::new(content);
                let full_range = full_file_range(&line_index, content_len);
                tests.push(test_fact(
                    "unknown",
                    &test_id,
                    &file_id,
                    &relative_path,
                    &full_range,
                    &import_prov_id,
                ));
                provenance.push(test_discovery_provenance(
                    &import_prov_id,
                    &file_id,
                    None,
                    "medium",
                ));
                limitations.push(json!({
                    "limitation_id": format!("test-parse-failed:{file_id}"),
                    "kind": "parse_failure",
                    "message": format!(
                        "could not parse test file `{relative_path}`; emitted the test fact with unknown framework and no oracles"
                    ),
                    "evidence_refs": [file_id],
                }));
                continue;
            }
        };

        // Detect the framework and collect each oracle's span + source-text
        // `expression` while `content` is still borrowed — then `content` can
        // move into `LineIndex` (no clone) for the range conversions.
        let (framework, use_span) = framework_from_ast(&ast);
        let oracle_hits: Vec<(String, &'static str, &'static str, usize, usize, String)> =
            extract_symbol_refs(&ast)
                .into_iter()
                .filter(|reference| reference.kind == SymbolRefKind::SubroutineCall)
                .filter_map(|reference| {
                    oracle_for(&reference.name).map(|(kind, strength)| {
                        let (start_byte, end_byte) = reference.full_span;
                        let expression = call_expression(&content, start_byte, end_byte);
                        (reference.name, kind, strength, start_byte, end_byte, expression)
                    })
                })
                .collect();

        let content_len = content.len();
        let line_index = LineIndex::new(content);
        let full_range = full_file_range(&line_index, content_len);
        let import_range = use_span.map(|(s, e)| range_string(&line_index, s, e));

        tests.push(test_fact(
            framework,
            &test_id,
            &file_id,
            &relative_path,
            &full_range,
            &import_prov_id,
        ));
        provenance.push(test_discovery_provenance(
            &import_prov_id,
            &file_id,
            import_range,
            if framework == "unknown" { "medium" } else { "high" },
        ));

        // Oracles from the parsed call sites (byte spans → real ranges).
        let oracle_prov_id = format!("prov:oracle_extraction:{file_id}");
        let mut file_oracles = 0usize;
        for (name, kind, strength, start_byte, end_byte, expression) in oracle_hits {
            let ((sl, sc), (el, ec)) = line_index.range(start_byte, end_byte);
            oracles.push(json!({
                "oracle_id": format!("oracle:{relative_path}:{name}:{start_byte}-{end_byte}"),
                "test_id": test_id,
                "kind": kind,
                "strength": strength,
                "target_owner_id": null,
                "expression": expression,
                "observed_sink": null,
                "expected_expression": null,
                "range": {"start_line": sl, "start_column": sc, "end_line": el, "end_column": ec},
                "confidence": "medium",
                "provenance_refs": [oracle_prov_id.clone()],
            }));
            file_oracles += 1;
        }

        if file_oracles > 0 {
            any_oracles = true;
            provenance.push(json!({
                "provenance_id": oracle_prov_id,
                "source": "oracle_extraction",
                "file_id": file_id,
                "range": null,
                "confidence": "medium",
            }));
        } else if framework != "unknown" {
            // A recognized framework but no extractable assertion: assertions may
            // be wrapped, aliased, or dynamically generated — flag, do not guess.
            limitations.push(json!({
                "limitation_id": format!("no-oracles:{file_id}"),
                "kind": "framework_indirection",
                "message": format!(
                    "`{relative_path}` uses {framework} but no supported assertion call was found; assertions may be wrapped, aliased, or dynamically generated"
                ),
                "evidence_refs": [file_id],
            }));
        }
    }

    // Representation note: the schema carries one call `range` + string
    // expressions per oracle, not separate observed/expected sub-ranges.
    if any_oracles {
        limitations.push(json!({
            "limitation_id": "oracle-representation",
            "kind": "narrowed_representation",
            "message": "oracle facts locate the whole assertion call range; the ripr-perl-facts-v1 schema has no observed/expected expression sub-range fields, so those are conveyed as string expressions only.",
            "evidence_refs": [],
        }));
    }

    (tests, oracles, provenance, limitations)
}

/// The whole-file range `(0,0)..(end)` as a schema range value.
fn full_file_range(line_index: &LineIndex, content_len: usize) -> Value {
    let ((sl, sc), (el, ec)) = line_index.range(0, content_len);
    json!({"start_line": sl, "start_column": sc, "end_line": el, "end_column": ec})
}

/// Build a schema-valid `test` fact. The `test` schema has no `owner_id`
/// (`additionalProperties: false`), so subtest ownership, when added later, must
/// ride `test_id` naming rather than a field. Test2 harnesses run under `yath`
/// or `prove`; everything else under `prove`.
fn test_fact(
    framework: &str,
    test_id: &str,
    file_id: &str,
    name: &str,
    range: &Value,
    provenance_id: &str,
) -> Value {
    let runner_hints =
        if framework.starts_with("Test2") { json!(["yath", "prove"]) } else { json!(["prove"]) };
    json!({
        "test_id": test_id,
        "file_id": file_id,
        "framework": framework,
        "name": name,
        "range": range,
        "runner_hints": runner_hints,
        "confidence": "high",
        "provenance_refs": [provenance_id],
    })
}

/// Build a `test_discovery` provenance fact (framework/import evidence).
fn test_discovery_provenance(
    provenance_id: &str,
    file_id: &str,
    range: Option<String>,
    confidence: &str,
) -> Value {
    json!({
        "provenance_id": provenance_id,
        "source": "test_discovery",
        "file_id": file_id,
        "range": range,
        "confidence": confidence,
    })
}

/// Look up a call name in [`ASSERTION_ORACLES`], returning `(kind, strength)`.
fn oracle_for(name: &str) -> Option<(&'static str, &'static str)> {
    ASSERTION_ORACLES
        .iter()
        .find(|(n, _, _)| *n == name)
        .map(|(_, kind, strength)| (*kind, *strength))
}

/// The oracle `expression`: the call's source text, trimmed and length-bounded.
fn call_expression(content: &str, start: usize, end: usize) -> String {
    let slice = content.get(start..end).unwrap_or("").trim();
    const MAX_CHARS: usize = 120;
    if slice.chars().count() > MAX_CHARS {
        let head: String = slice.chars().take(MAX_CHARS).collect();
        format!("{head}…")
    } else {
        slice.to_owned()
    }
}

/// A compact `sl:sc-el:ec` range string for the schema's string-typed
/// `provenance.range` field.
fn range_string(line_index: &LineIndex, start: usize, end: usize) -> String {
    let ((sl, sc), (el, ec)) = line_index.range(start, end);
    format!("{sl}:{sc}-{el}:{ec}")
}

/// Detect the test framework from parsed `use` statements. Returns the ripr
/// wire name (`"unknown"` if none matched) and the byte span of the matched
/// `use` node (for provenance). When several frameworks are imported, the most
/// specific one wins (the earliest entry in [`TEST_FRAMEWORKS`]). Single-pass —
/// no intermediate allocation or module-name cloning.
fn framework_from_ast(ast: &Node) -> (&'static str, Option<(usize, usize)>) {
    // (framework index, wire name, span) of the best (lowest-index) match so far.
    let mut best: Option<(usize, &'static str, (usize, usize))> = None;
    find_framework_use(ast, &mut best);
    match best {
        Some((_, wire, span)) => (wire, Some(span)),
        None => ("unknown", None),
    }
}

/// Recursively update `best` with the most specific test-framework `use` node.
fn find_framework_use(node: &Node, best: &mut Option<(usize, &'static str, (usize, usize))>) {
    if let NodeKind::Use { module, .. } = &node.kind
        && let Some(index) = TEST_FRAMEWORKS.iter().position(|(m, _)| m == module)
        && best.is_none_or(|(best_index, _, _)| index < best_index)
    {
        let (_, wire) = TEST_FRAMEWORKS[index];
        *best = Some((index, wire, (node.location.start, node.location.end)));
    }
    node.for_each_child(|child| find_framework_use(child, best));
}

/// Collect all `.t` files under a directory. Returns (full_path, relative_path, content).
fn collect_t_files(t_dir: &std::path::Path) -> Vec<(String, String, String)> {
    let mut result = Vec::new();
    let Ok(entries) = std::fs::read_dir(t_dir) else {
        return result;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_symlink = entry.file_type().is_ok_and(|file_type| file_type.is_symlink());
        if path.is_dir() {
            // Do not descend into symlinked directories — a directory-symlink
            // loop would otherwise recurse infinitely (`is_dir()` follows the
            // link). A symlinked `.t` *file* (below) is still read.
            if is_symlink {
                continue;
            }
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
/// Build the canonical `owners[]` `owner_id` string for a declaration. The
/// single source of truth for the id shape, shared by [`emit_files_and_owners`]
/// (which emits the `owner` facts) and [`resolve_package_owner_id`] (which
/// rebuilds a package owner's id so a `relation` can reference it) — so the two
/// can never drift into the dangling cross-reference #3342 corrected.
fn owner_fact_id(
    relative_path: &str,
    kind: &str,
    qualified_name: &str,
    start_byte: usize,
    end_byte: usize,
) -> String {
    format!("owner:{relative_path}:{kind}:{qualified_name}:{start_byte}-{end_byte}")
}

/// Rebuild the exact `owners[]` `owner_id` that [`emit_files_and_owners`] assigns
/// to the container declaration (`package`/`class`/`role`) named `package_name`
/// in `pm_path`. Returns `None` when the parse exposes no such container decl
/// (a name-mismatched or unparsed package) — the caller then omits the relation
/// with a limitation rather than emitting a dangling `owner_id`. Uses the same
/// `extract_symbol_decls` + `owner_kind` + `full_span` path as the owner emitter,
/// via the shared [`owner_fact_id`], so the reconstructed id is byte-identical.
fn resolve_package_owner_id(ast: &Node, pm_path: &str, package_name: &str) -> Option<String> {
    extract_symbol_decls(ast, Some("main")).into_iter().find_map(|decl| {
        if !matches!(decl.kind, SymbolKind::Package | SymbolKind::Class | SymbolKind::Role) {
            return None;
        }
        if decl.qualified_name != package_name {
            return None;
        }
        let kind = owner_kind(&decl.kind)?;
        let (start_byte, end_byte) = decl.full_span;
        Some(owner_fact_id(pm_path, kind, &decl.qualified_name, start_byte, end_byte))
    })
}

pub(crate) fn emit_relations_and_discriminators(
    root: &str,
    tests: &[Value],
    _oracles: &[Value],
) -> (Vec<Value>, Vec<Value>, Vec<Value>, Vec<Value>) {
    let mut relations = Vec::new();
    let mut changed_observables = Vec::new();
    let mut observed_sinks = Vec::new();
    let mut limitations = Vec::new();

    // Collect .pm files from lib/, sorted for deterministic traversal order
    // (`std::fs::read_dir` order is filesystem/OS-dependent).
    let mut pm_files = collect_pm_files(std::path::Path::new(root));
    pm_files.sort_by(|a, b| a.0.cmp(&b.0));

    // Parse each candidate `.pm` once, hoisted above the test loop below —
    // NOT once per (test, pm) pair. `extract_package_name` stays the existing
    // string scan (out of scope); only the declared-sub proof is parser-backed.
    // The same parse also resolves the package's real `owners[]` `owner_id`
    // (#3342) so a relation can carry a resolvable cross-reference; `None` means
    // the parse exposed no matching container decl (unparsed/name-mismatched).
    type PmFact = (String, String, std::collections::HashSet<String>, Option<String>);
    let pm_facts: Vec<PmFact> = pm_files
        .iter()
        .map(|(pm_path, pm_content)| {
            let package_name = extract_package_name(pm_content);
            let (declared_subs, owner_id) = if package_name.is_empty() {
                (std::collections::HashSet::new(), None)
            } else {
                match Parser::new(pm_content).parse() {
                    Ok(ast) => (
                        declared_sub_names(&ast, &package_name),
                        resolve_package_owner_id(&ast, pm_path, &package_name),
                    ),
                    // parse failure → cannot prove calls or resolve an owner
                    Err(_) => (std::collections::HashSet::new(), None),
                }
            };
            (pm_path.clone(), package_name, declared_subs, owner_id)
        })
        .collect();

    // Collect + parse every `.t` file once, hoisted and reused by both the
    // relation loop below and the discriminator loop further down (which
    // previously called `collect_t_files` a second time).
    let t_dir = std::path::Path::new(root).join("t");
    let mut t_files = collect_t_files(&t_dir);
    t_files.sort_by(|a, b| a.1.cmp(&b.1));

    let t_call_facts: std::collections::HashMap<&str, TestCallFacts> = t_files
        .iter()
        .map(|(_, relative_path, content)| {
            let facts = match Parser::new(content).parse() {
                Ok(ast) => TestCallFacts::from_ast(&ast),
                Err(_) => TestCallFacts::unparsed(),
            };
            (relative_path.as_str(), facts)
        })
        .collect();

    // For each test file, infer relations to .pm files by package-name match.
    for test in tests {
        let test_file_id = test["file_id"].as_str().unwrap_or("");
        let test_path = test["name"].as_str().unwrap_or("");

        for (pm_path, package_name, declared_subs, owner_id) in &pm_facts {
            if package_name.is_empty() {
                continue;
            }

            // Check if the test file references the package.
            if !test_file_id.is_empty() && file_references_package(test_path, &[], pm_path) {
                // #3342: a relation's `owner_id` must resolve to a real
                // `owners[]` fact. Resolve the package's actual owner id from
                // the same parse the owner emitter uses; if the package exposed
                // no container decl (unparsed/name-mismatched), omit the
                // relation with a limitation rather than dangle the reference.
                let Some(resolved_owner_id) = owner_id else {
                    limitations.push(json!({
                        "limitation_id": format!("relation-owner-unresolved:{pm_path}"),
                        "kind": "unresolved_owner",
                        "message": format!(
                            "test references package `{package_name}` in `{pm_path}` but no matching owners[] fact was found (unparsed or name-mismatched package); the relation is omitted to avoid a dangling owner_id"
                        ),
                        "evidence_refs": []
                    }));
                    continue;
                };

                // #3293 PR 6: upgrade to direct_owner_call only when the
                // test file's parsed call graph provably includes a call to
                // a sub the candidate package's AST actually declares.
                let is_direct = t_call_facts.get(test_path).is_some_and(|facts| {
                    test_calls_declared_sub(facts, package_name, declared_subs)
                });

                let relation_kind = if is_direct { "direct_owner_call" } else { "file_proximity" };
                let reachability = if is_direct { "reachable" } else { "weakly_reachable" };

                let relation_id = format!("relation:{test_file_id}:{pm_path}");
                relations.push(json!({
                    "relation_id": relation_id,
                    "change_id": "change:unresolved",
                    "owner_id": resolved_owner_id,
                    "test_id": test["test_id"],
                    "oracle_id": null,
                    "relation_kind": relation_kind,
                    "reachability_hint": reachability,
                    "confidence": "medium",
                    "provenance_refs": []
                }));
            }
        }
    }

    // Sort relations by id for deterministic output order regardless of the
    // caller's `tests` ordering.
    relations.sort_by(|a, b| a["relation_id"].as_str().cmp(&b["relation_id"].as_str()));

    // Extract concrete discriminators + observed-sink facts from `is(...)` assertions.
    // Reuses the `t_files` collected above — no second directory walk.
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

    // Deterministic order + dedup: the same unresolvable package referenced by
    // several test files would otherwise push a duplicate limitation per test.
    limitations.sort_by(|a, b| a["limitation_id"].as_str().cmp(&b["limitation_id"].as_str()));
    limitations.dedup_by(|a, b| a["limitation_id"] == b["limitation_id"]);

    (relations, changed_observables, observed_sinks, limitations)
}

/// Collect all `.pm` files under `<root>/lib`. Returns (relative_path, content),
/// where `relative_path` is `strip_prefix(root)` — byte-identical to the path
/// [`emit_files_and_owners`] derives for the same file. #3342: the previous
/// `split_once("/lib/")` heuristic diverged from that path whenever `root` had
/// an ancestor segment named `lib` (e.g. `vendor/lib/proj`, `t/lib/...`),
/// re-dangling a relation's resolved `owner_id`; both now strip the same `root`.
fn collect_pm_files(root: &std::path::Path) -> Vec<(String, String)> {
    let mut result = Vec::new();
    collect_pm_files_recursive(&root.join("lib"), root, &mut result);
    result
}

fn collect_pm_files_recursive(
    dir: &std::path::Path,
    root: &std::path::Path,
    result: &mut Vec<(String, String)>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_pm_files_recursive(&path, root, result);
        } else if path.extension().is_some_and(|ext| ext == "pm") {
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            let relative = path
                .strip_prefix(root)
                .unwrap_or(path.as_path())
                .to_string_lossy()
                .replace('\\', "/");
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

/// Per-`.t`-file facts computed once (parse + call/`use` extraction) and
/// reused across every candidate `.pm` in [`emit_relations_and_discriminators`]
/// — avoids re-parsing/re-walking `t/` once per (test, pm) pair.
struct TestCallFacts {
    /// Subroutine/method/static-method call sites from the test file.
    calls: Vec<perl_symbol::surface::SymbolRef>,
}

impl TestCallFacts {
    /// Conservative fallback for a test file that failed to parse: proves
    /// nothing, so [`test_calls_declared_sub`] always degrades for it.
    fn unparsed() -> Self {
        Self { calls: Vec::new() }
    }

    fn from_ast(ast: &Node) -> Self {
        let calls = extract_symbol_refs(ast)
            .into_iter()
            .filter(|reference| {
                matches!(
                    reference.kind,
                    SymbolRefKind::SubroutineCall
                        | SymbolRefKind::MethodCall
                        | SymbolRefKind::StaticMethodCall
                )
            })
            .collect();
        Self { calls }
    }
}

/// Bare sub/method names a `.pm`'s AST declares *inside* `package_name`.
///
/// Filters [`extract_symbol_decls`] to callables whose `container` matches the
/// package being evaluated for this relation, so a multi-package file only
/// credits the package actually being checked.
fn declared_sub_names(pm_ast: &Node, package_name: &str) -> std::collections::HashSet<String> {
    extract_symbol_decls(pm_ast, Some("main"))
        .into_iter()
        .filter(|decl| matches!(decl.kind, SymbolKind::Subroutine | SymbolKind::Method))
        .filter(|decl| decl.container.as_deref() == Some(package_name))
        .map(|decl| decl.name)
        .collect()
}

/// Parser-backed replacement for the former string-heuristic
/// `test_calls_package` (#3293 PR 6). Proves a `direct_owner_call` relation
/// using AST call nodes (`perl-symbol`) instead of substring scans over raw
/// file content.
///
/// A call counts as direct **only** when it is a fully-qualified
/// `SymbolRefKind::SubroutineCall` — `package_name::sub(...)` with
/// `package_qualifier == Some(package_name)` and `sub` in `declared_subs`. A
/// fully-qualified call names its owning package unambiguously and reaches the
/// declared sub regardless of exports, so this is the one form the AST can
/// *prove*.
///
/// Deliberately conservative — never a false `true`:
///  - A **bare** call (`sub(...)`) after `use package_name` is NOT proof: the
///    sub is only callable bareword if the package exports it
///    (`Exporter`/`@EXPORT`), which `perl-symbol` cannot see, and a shared
///    basename could make the same bare name belong to any of several `use`d
///    packages. Bare calls fall through to the conservative
///    `file_proximity`/`weakly_reachable` branch rather than a false
///    `reachable`. (Export-aware bare-call attribution is a later slice.)
///  - `MethodCall` / `StaticMethodCall` (`$obj->m` / `Pkg->m`) are excluded —
///    dynamic dispatch can resolve to an inherited, `AUTOLOAD`'d, or
///    role-composed method and is never provable from the call site.
///  - An empty `declared_subs` (the `.pm` failed to parse or declares nothing
///    in `package_name`) or an unparsed test file proves nothing.
///
/// This is a lexical call-graph proof, not a control-flow-verified one: a call
/// inside a dead branch or a `SKIP:` block still counts, matching this crate's
/// convention that every other "reachable"-adjacent claim here is also lexical.
fn test_calls_declared_sub(
    facts: &TestCallFacts,
    package_name: &str,
    declared_subs: &std::collections::HashSet<String>,
) -> bool {
    if declared_subs.is_empty() {
        return false;
    }
    facts.calls.iter().any(|call| {
        call.kind == SymbolRefKind::SubroutineCall
            && call.package_qualifier.as_deref() == Some(package_name)
            && declared_subs.contains(&call.name)
    })
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
    let pm_files = collect_pm_files(std::path::Path::new(root));
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

/// Map a content_hash (u64) to a hex digest string for the packet.
fn content_hash_to_digest(hash: u64) -> String {
    format!("fnv64:{hash:016x}")
}

/// Deterministic content fingerprint (`fnv64:` prefix) of a serialized packet
/// (#3293 PR 7). Non-cryptographic FNV-1a — a content/identity hash the consumer
/// can dedup or cache on, not a security digest; it keeps the crate's
/// no-crypto-dependency contract and matches the `fnv64:` style used by file
/// `digest`s.
pub(crate) fn content_fingerprint(serialized: &str) -> String {
    content_hash_to_digest(fnv1a_hash(serialized))
}

/// Strip a `file:///` prefix from a source URI and normalize to forward-slash.
#[allow(dead_code)]
fn uri_to_relative_path(uri: &str) -> String {
    uri.strip_prefix("file:///")
        .or_else(|| uri.strip_prefix("file://"))
        .unwrap_or(uri)
        .replace('\\', "/")
}

/// Determine file role from path extension.
fn file_role_from_path(path: &str) -> &'static str {
    if path.ends_with(".t") {
        "test"
    } else if path.ends_with(".pm") || path.ends_with(".pl") || path.ends_with(".psgi") {
        "source"
    } else if path.ends_with("Makefile.PL")
        || path.ends_with("Build.PL")
        || path.ends_with("cpanfile")
    {
        "config"
    } else {
        "unknown"
    }
}

/// One contiguous run of added (`+`) lines within a single file's diff, tagged
/// with the head-file line range it lands on (0-based, tracked from the
/// `@@ -a,b +c,d @@` header). Pure text — no filesystem or byte offsets.
struct DiffHunkRun {
    file_path: String,
    start_line: u32,
    end_line: u32,
    lines: Vec<String>,
}

/// Parse a unified diff into contiguous added-line runs, one per uninterrupted
/// block of `+` lines, tracking the head-file line cursor from each
/// `@@ -a,b +c,d @@` header. Removed (`-`) lines do not advance the head cursor;
/// context lines do. Pure text parsing — no filesystem access, no subprocess.
fn parse_diff_hunks(diff_text: &str) -> Vec<DiffHunkRun> {
    fn flush(run: &mut Option<DiffHunkRun>, runs: &mut Vec<DiffHunkRun>) {
        if let Some(finished) = run.take() {
            runs.push(finished);
        }
    }

    let mut runs = Vec::new();
    let mut current_file: Option<String> = None;
    let mut head_line: u32 = 0;
    let mut run: Option<DiffHunkRun> = None;

    for line in diff_text.lines() {
        if let Some(rest) = line.strip_prefix("+++ b/") {
            flush(&mut run, &mut runs);
            current_file = Some(rest.trim().to_string());
            continue;
        }
        if line.starts_with("+++") || line.starts_with("---") {
            flush(&mut run, &mut runs);
            continue;
        }
        if let Some(header_rest) = line.strip_prefix("@@") {
            flush(&mut run, &mut runs);
            head_line = parse_hunk_new_start(header_rest).unwrap_or(0);
            continue;
        }
        if line.starts_with('\\') {
            // `\ No newline at end of file` — metadata about the preceding line,
            // present in neither file version. Do not flush the open run or
            // advance the head cursor (advancing it would shift every following
            // added line down by one).
            continue;
        }
        if let Some(added) = line.strip_prefix('+') {
            let file = current_file.clone().unwrap_or_default();
            match run {
                Some(ref mut open) if open.file_path == file => {
                    open.end_line = head_line;
                    open.lines.push(added.to_string());
                }
                _ => {
                    flush(&mut run, &mut runs);
                    if current_file.is_some() {
                        run = Some(DiffHunkRun {
                            file_path: file,
                            start_line: head_line,
                            end_line: head_line,
                            lines: vec![added.to_string()],
                        });
                    }
                }
            }
            head_line += 1;
        } else if line.starts_with('-') {
            // Removed line: not present in the head file, so the cursor holds.
            flush(&mut run, &mut runs);
        } else {
            // Context (or blank) line: present in the head file, advances cursor.
            flush(&mut run, &mut runs);
            head_line += 1;
        }
    }
    flush(&mut run, &mut runs);
    runs
}

/// From a hunk header body ` -a,b +c,d @@ ...`, return the new-file start line
/// `c` as a 0-based line (`c - 1`). `None` if the `+c` token is unparseable.
fn parse_hunk_new_start(header_rest: &str) -> Option<u32> {
    let plus = header_rest.split_whitespace().find(|tok| tok.starts_with('+'))?;
    let start: u32 = plus.trim_start_matches('+').split(',').next()?.parse().ok()?;
    Some(start.saturating_sub(1))
}

/// Smallest owner (by line span) in `owners` that belongs to `file_id` and whose
/// `[range.start_line, range.end_line]` inclusively contains `[start_line,
/// end_line]`. Ties (equal span) break toward `sub`/`method` over `package`, so
/// the result is deterministic and independent of `owners` order. `None` when no
/// owner contains the range (file-/script-level code, or a file with zero owners).
fn find_enclosing_owner<'a>(
    owners: &'a [Value],
    file_id: &str,
    start_line: u32,
    end_line: u32,
) -> Option<&'a Value> {
    let mut best: Option<&Value> = None;
    let mut best_span = u64::MAX;
    let mut best_is_sub = false;
    for owner in owners {
        if owner["file_id"].as_str() != Some(file_id) {
            continue;
        }
        let range = &owner["range"];
        let (Some(owner_start), Some(owner_end)) =
            (range["start_line"].as_u64(), range["end_line"].as_u64())
        else {
            continue;
        };
        let (owner_start, owner_end) = (owner_start as u32, owner_end as u32);
        if owner_start <= start_line && end_line <= owner_end {
            let span = u64::from(owner_end - owner_start);
            let is_sub = matches!(owner["kind"].as_str(), Some("sub") | Some("method"));
            if span < best_span || (span == best_span && is_sub && !best_is_sub) {
                best = Some(owner);
                best_span = span;
                best_is_sub = is_sub;
            }
        }
    }
    best
}

/// Pick a `behavior_hint` + discriminator for a hunk by scanning its added lines
/// top-to-bottom; the first line matching a known pattern (predicate boundary →
/// return value → exception path) wins. No match on any line → `"unknown"`.
fn behavior_hint_for_hunk(lines: &[String]) -> (&'static str, String) {
    for line in lines {
        // A whole-line comment is never executable, so it must never yield a
        // concrete behavior hint (e.g. `# return $x;` is not a return). This is
        // a cheap, safe filter; false positives from `die`/`return` substrings
        // *inside string literals* remain possible and are documented as a
        // limitation (a robust fix needs tokenization, out of this slice's scope).
        if line.trim_start().starts_with('#') {
            continue;
        }
        let (hint, discriminator) = infer_behavior_and_discriminator(line);
        if hint != "unknown" {
            return (hint, discriminator);
        }
    }
    ("unknown", String::new())
}

/// Normalize a `git diff` head path (repo-root-relative, e.g.
/// `crates/perl-parser/src/Foo.pm`) to a `root`-relative path matching the
/// `file_id`s [`emit_files_and_owners`] emits (e.g. `src/Foo.pm` when `root` is
/// `crates/perl-parser`). A `.` / empty root, or a path not under `root`, is
/// returned unchanged (an outside-root path stays unmatched → `diff-file-not-found`).
fn strip_root_prefix<'a>(path: &'a str, root: &str) -> &'a str {
    if root == "." || root.is_empty() {
        return path;
    }
    path.strip_prefix(root).and_then(|rest| rest.strip_prefix('/')).unwrap_or(path)
}

/// Emit `changes[]` + `limitations[]` from a **caller-supplied** unified diff
/// (`RiprFactsRequest.diff`), resolving each contiguous added-line hunk's owner
/// by smallest-enclosing-line-range containment against `owners` (as emitted by
/// [`emit_files_and_owners`]). `files` supplies the set of parsed file ids so a
/// hunk touching a path outside `root` is surfaced as a limitation, not silently
/// dropped. `root` normalizes the diff's repo-root-relative paths to the
/// `root`-relative `file_id`s the packet uses (#3293 PR 5).
///
/// This is pure text processing — no filesystem access, no subprocess, no git.
/// Referential integrity: a `change` is emitted only when its file is a known
/// `file_id` **and** the hunk lands inside a real `owners[]` fact; otherwise a
/// limitation records the gap. Of the schema's nine `behavior_hint` values, only
/// the three syntactically-detectable ones (`predicate_boundary`,
/// `return_value`, `exception_path`) are inferred; everything else is
/// `"unknown"` and `missing_discriminator` is always `null` in this slice.
pub(crate) fn emit_changes_from_diff(
    diff_text: &str,
    root: &str,
    files: &[Value],
    owners: &[Value],
) -> (Vec<Value>, Vec<Value>) {
    let mut changes = Vec::new();
    let mut limitations = Vec::new();

    // base/head/diff are caller-asserted; this crate never runs git to confirm
    // the supplied diff is the actual base→head diff. Always surface that.
    limitations.push(json!({
        "limitation_id": "diff-provenance-unverified",
        "kind": "unverified_provenance",
        "message": "base/head/diff are caller-asserted and not verified against a repository; this packet does not confirm the supplied diff is the actual base->head diff.",
        "evidence_refs": [],
    }));

    let known_files: std::collections::HashSet<&str> =
        files.iter().filter_map(|file| file["file_id"].as_str()).collect();

    for hunk in parse_diff_hunks(diff_text) {
        // git diff paths are repo-root-relative; file_ids are root-relative.
        let rel_path = strip_root_prefix(&hunk.file_path, root);
        let file_id = format!("file:{rel_path}");

        if !known_files.contains(file_id.as_str()) {
            limitations.push(json!({
                "limitation_id": format!("diff-file-not-found:{}", hunk.file_path),
                "kind": "unresolved_diff_path",
                "message": format!(
                    "diff hunk touches `{}`, which was not parsed under the packet root; no change fact emitted",
                    hunk.file_path
                ),
                "evidence_refs": [file_id],
            }));
            continue;
        }

        let Some(owner) = find_enclosing_owner(owners, &file_id, hunk.start_line, hunk.end_line)
        else {
            limitations.push(json!({
                "limitation_id": format!("unattributable-change:{file_id}:{}", hunk.start_line),
                "kind": "unattributable_change",
                "message": format!(
                    "diff hunk at lines {}-{} of `{}` is not inside any package/sub/method owner (file- or script-level code); no change fact emitted",
                    hunk.start_line, hunk.end_line, hunk.file_path
                ),
                "evidence_refs": [file_id],
            }));
            continue;
        };

        let (behavior_hint, discriminator) = behavior_hint_for_hunk(&hunk.lines);
        let end_column = hunk.lines.last().map_or(0, |last| last.chars().count()) as u32;
        let changed_observable =
            if behavior_hint == "unknown" { Value::Null } else { Value::String(discriminator) };

        changes.push(json!({
            "change_id": format!("change:{file_id}:{}:{}", hunk.start_line, hunk.end_line),
            "file_id": file_id,
            "owner_id": owner["owner_id"].clone(),
            "range": {
                "start_line": hunk.start_line,
                "start_column": 0,
                "end_line": hunk.end_line,
                "end_column": end_column,
            },
            "behavior_hint": behavior_hint,
            "changed_text_digest": content_hash_to_digest(fnv1a_hash(&hunk.lines.join("\n"))),
            "changed_observable": changed_observable,
            "missing_discriminator": Value::Null,
            "provenance_refs": [],
        }));
    }

    // Packet-level honesty notes — once each, only when changes were emitted.
    if !changes.is_empty() {
        limitations.push(json!({
            "limitation_id": "change-range-imprecise",
            "kind": "range_precision",
            "message": "change ranges are line-granular with best-effort column data derived from the diff, not the byte-accurate LineIndex ranges used for owners.",
            "evidence_refs": [],
        }));
        limitations.push(json!({
            "limitation_id": "change-behavior-hint-partial",
            "kind": "partial_inference",
            "message": "only predicate_boundary / return_value / exception_path behavior_hints are inferred from added-line text; every other change resolves to \"unknown\", and missing_discriminator is always null in this slice. Whole-line comments are skipped, but a die/return/comparison token inside a string literal can still be misclassified (a robust fix needs tokenization).",
            "evidence_refs": [],
        }));
    }

    (changes, limitations)
}

/// Infer behavior kind + concrete discriminator from a changed Perl line.
///
/// Conservative: only the three alpha-supported classes produce concrete
/// discriminators. Everything else is "unknown" with an empty discriminator
/// (ripr's strict-actionability fails closed on unknown).
fn infer_behavior_and_discriminator(line: &str) -> (&'static str, String) {
    let trimmed = line.trim();

    // Predicate boundary: a LEADING conditional (if/unless/while/elsif at the
    // start of the line, after trim) with a comparison operator. A trailing
    // modifier-if is NOT a predicate boundary — `return $x if $y > 5;` is a
    // return_value and `die "x" if $y > 5;` is an exception_path, so those must
    // fall through to the branches below rather than match on the mere presence
    // of `if `.
    if (trimmed.starts_with("if ")
        || trimmed.starts_with("unless ")
        || trimmed.starts_with("while ")
        || trimmed.starts_with("elsif ")
        || trimmed.starts_with("} elsif "))
        && (trimmed.contains("==")
            || trimmed.contains("!=")
            || trimmed.contains(">=")
            || trimmed.contains("<=")
            || trimmed.contains(">")
            || trimmed.contains("<"))
    {
        // Extract the condition text as the discriminator.
        let disc = extract_condition(trimmed).unwrap_or_else(|| trimmed.to_string());
        return ("predicate_boundary", disc);
    }

    // Return value.
    if trimmed.starts_with("return") || trimmed.contains("return ") {
        let expr = trimmed
            .strip_prefix("return")
            .unwrap_or(trimmed)
            .trim()
            .trim_end_matches(';')
            .to_string();
        return ("return_value", expr);
    }

    // Exception path.
    if trimmed.contains("die ") || trimmed.contains("croak ") || trimmed.contains("confess ") {
        let msg = extract_die_message(trimmed).unwrap_or_else(|| "exception".to_string());
        return ("exception_path", msg);
    }

    ("unknown", String::new())
}

/// Extract the condition expression from a leading if/unless/while/elsif line.
fn extract_condition(line: &str) -> Option<String> {
    let after_kw = line
        .strip_prefix("if ")
        .or_else(|| line.strip_prefix("unless "))
        .or_else(|| line.strip_prefix("while "))
        .or_else(|| line.strip_prefix("elsif "))
        .or_else(|| line.strip_prefix("} elsif "))?;
    let cond = after_kw.trim_end_matches('{').trim().trim_end_matches('{').trim();
    Some(cond.to_string())
}

/// Extract the message from a die/croak/confess call.
fn extract_die_message(line: &str) -> Option<String> {
    for kw in &["die ", "croak ", "confess "] {
        if let Some(idx) = line.find(kw) {
            let rest = &line[idx + kw.len()..];
            let msg = rest
                .trim_start_matches('"')
                .trim_start_matches("'")
                .trim_end_matches(';')
                .trim_end_matches('"')
                .trim_end_matches("'");
            return Some(msg.to_string());
        }
    }
    None
}

/// Simple FNV-1a hash for deterministic digests.
fn fnv1a_hash(text: &str) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Emit `files[]`, `owners[]`, per-file `provenance[]`, and parse/read
/// `limitations[]` by parsing every Perl source/test file under `root`
/// (#3293 PR 3).
///
/// For each discovered `.pm` / `.pl` / `.psgi` / `.t` file this produces:
/// - one `file` fact (repo-relative path, role, deterministic FNV-1a digest,
///   parser-derived package names);
/// - one `owner` fact per `package` / `class` / `role` / `sub` / `method`
///   declaration, carrying the parser's real source range and a byte-span-derived
///   `owner_id` (stable, never traversal-order); and
/// - one `syntax`-sourced `provenance` fact that the file and its owners
///   reference by id.
///
/// Files that cannot be read or parsed are **not** silently dropped: a read
/// failure records a limitation and emits no file fact (a digest needs the
/// content); a parse failure still emits the file fact with zero owners plus a
/// limitation.
///
/// The `range` uses the schema's flat `{start_line, start_column, end_line,
/// end_column}` shape (0-based, UTF-16 columns from `LineIndex`), and provenance
/// uses the schema's `source` enum (`"syntax"`) — the packet contract has no
/// nested-LSP range or free-form provenance `producer`/`kind` fields.
///
/// Parser-backed via `perl-parser-core` (parse + `LineIndex` byte→line/column)
/// and `perl-symbol` (`extract_symbol_decls`) — both leaf crates with no
/// forbidden dependencies. `perl-workspace` is intentionally avoided (it pulls
/// `lsp-types`).
pub(crate) fn emit_files_and_owners(
    root: &str,
) -> (Vec<Value>, Vec<Value>, Vec<Value>, Vec<Value>) {
    let mut files = Vec::new();
    let mut owners = Vec::new();
    let mut provenance = Vec::new();
    let mut limitations = Vec::new();

    for relative_path in collect_perl_files(root) {
        let file_id = format!("file:{relative_path}");
        let absolute = std::path::Path::new(root).join(&relative_path);

        let content = match std::fs::read_to_string(&absolute) {
            Ok(content) => content,
            Err(error) => {
                // Do not silently drop: a digest needs the content, so emit no
                // file fact — just a limitation recording why.
                limitations.push(json!({
                    "limitation_id": format!("read-failed:{file_id}"),
                    "kind": "read_failure",
                    "message": format!("could not read `{relative_path}`: {error}"),
                    "evidence_refs": [file_id],
                }));
                continue;
            }
        };

        let digest = content_hash_to_digest(fnv1a_hash(&content));
        let role = file_role_from_path(&relative_path);
        let provenance_id = format!("prov:syntax:{file_id}");
        let mut package_names: Vec<String> = Vec::new();

        // Parse and project declarations into owner facts. Scope the parser's
        // borrow of `content` to the parse so `content` can move into
        // `LineIndex` afterwards (no per-file clone).
        let parsed = {
            let mut parser = Parser::new(&content);
            parser.parse()
        };
        match parsed {
            Ok(ast) => {
                let line_index = LineIndex::new(content);
                for decl in extract_symbol_decls(&ast, Some("main")) {
                    let Some(kind) = owner_kind(&decl.kind) else {
                        continue;
                    };
                    if matches!(
                        decl.kind,
                        SymbolKind::Package | SymbolKind::Class | SymbolKind::Role
                    ) && !package_names.contains(&decl.qualified_name)
                    {
                        package_names.push(decl.qualified_name.clone());
                    }

                    let (start_byte, end_byte) = decl.full_span;
                    let ((start_line, start_column), (end_line, end_column)) =
                        line_index.range(start_byte, end_byte);

                    // Byte-span-derived id: stable and independent of traversal
                    // order (a decl is uniquely located by its span).
                    owners.push(json!({
                        "owner_id": owner_fact_id(
                            &relative_path,
                            kind,
                            &decl.qualified_name,
                            start_byte,
                            end_byte,
                        ),
                        "file_id": file_id,
                        "kind": kind,
                        "package": decl.container,
                        "name": decl.name,
                        "range": {
                            "start_line": start_line,
                            "start_column": start_column,
                            "end_line": end_line,
                            "end_column": end_column,
                        },
                        "confidence": "high",
                        "provenance_refs": [provenance_id.clone()],
                    }));
                }
            }
            Err(_error) => {
                // Fail soft: still emit the file fact (below) with zero owners,
                // and record why parsing yielded none.
                limitations.push(json!({
                    "limitation_id": format!("parse-failed:{file_id}"),
                    "kind": "parse_failure",
                    "message": format!(
                        "could not parse `{relative_path}` as Perl; emitted the file fact with no owners"
                    ),
                    "evidence_refs": [file_id.clone()],
                }));
            }
        }

        package_names.sort();
        package_names.dedup();

        provenance.push(json!({
            "provenance_id": provenance_id.clone(),
            "source": "syntax",
            "file_id": file_id.clone(),
            "confidence": "high",
        }));

        files.push(json!({
            "file_id": file_id,
            "path": relative_path,
            "role": [role],
            "digest": digest,
            "package_names": package_names,
            "provenance_refs": [provenance_id],
        }));
    }

    // Honest digest note: the packet digest is a deterministic FNV-1a hash
    // (`fnv64:` prefix), not SHA-256 — a SHA-256 digest would require adding a
    // hashing dependency the crate's dep contract keeps out for now.
    if !files.is_empty() {
        limitations.push(json!({
            "limitation_id": "digest-algorithm",
            "kind": "digest_algorithm",
            "message": "file digests are deterministic FNV-1a (fnv64:), not SHA-256; a SHA-256 digest would require an added hashing dependency.",
            "evidence_refs": [],
        }));
    }

    (files, owners, provenance, limitations)
}

/// Map a `perl-symbol` [`SymbolKind`] to the ripr `owner.kind` vocabulary.
///
/// Only namespace and callable declarations are owners; variables, constants,
/// and imports are not. `Class` / `Role` are namespace declarations, so they map
/// to `package`.
fn owner_kind(kind: &SymbolKind) -> Option<&'static str> {
    match kind {
        SymbolKind::Package | SymbolKind::Class | SymbolKind::Role => Some("package"),
        SymbolKind::Subroutine => Some("sub"),
        SymbolKind::Method => Some("method"),
        _ => None,
    }
}

/// Recursively collect the repo-relative forward-slash paths of Perl
/// source/test files under `root`, in deterministic (sorted) order.
///
/// Skips hidden directories and common build trees so a workspace scan stays
/// bounded. Content is read per-file by the caller (not here) so that read
/// failures can be reported as limitations rather than silently dropped.
fn collect_perl_files(root: &str) -> Vec<String> {
    let root_path = std::path::Path::new(root);
    let mut result = Vec::new();
    collect_perl_files_recursive(root_path, root_path, &mut result);
    result.sort();
    result
}

fn collect_perl_files_recursive(
    dir: &std::path::Path,
    root: &std::path::Path,
    result: &mut Vec<String>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        let is_symlink = entry.file_type().is_ok_and(|file_type| file_type.is_symlink());
        if path.is_dir() {
            // Do not descend into symlinked directories — a directory-symlink
            // loop would otherwise recurse infinitely (`is_dir()` follows the
            // link). Also skip hidden dirs and non-source build trees. A
            // symlinked source *file* (below) is still read.
            if is_symlink
                || name.starts_with('.')
                || matches!(name.as_ref(), "target" | "blib" | "node_modules" | "_build")
            {
                continue;
            }
            collect_perl_files_recursive(&path, root, result);
        } else if is_perl_source_file(&name) {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(path.as_path())
                .to_string_lossy()
                .replace('\\', "/");
            result.push(relative);
        }
    }
}

/// Whether a file name is a Perl source or test file the emitter parses.
fn is_perl_source_file(name: &str) -> bool {
    name.ends_with(".pm")
        || name.ends_with(".pl")
        || name.ends_with(".psgi")
        || name.ends_with(".t")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_src(src: &str) -> Node {
        let mut parser = Parser::new(src);
        parser.parse().expect("test source parses")
    }

    #[test]
    fn framework_from_ast_detects_test_more() {
        assert_eq!(framework_from_ast(&parse_src("use Test::More;\nok(1);\n")).0, "Test::More");
    }

    #[test]
    fn framework_from_ast_detects_test2_bundles() {
        // Contract freeze: each Test2 bundle emits its own distinct wire name
        // (early builds collapsed Suite/V1 into "Test2::V0").
        for (src, want) in [
            ("use Test2::V0;\nok(1);", "Test2::V0"),
            ("use Test2::V1;\nok(1);", "Test2::V1"),
            ("use Test2::Suite;\nok(1);", "Test2::Suite"),
        ] {
            assert_eq!(framework_from_ast(&parse_src(src)).0, want, "src: {src}");
        }
    }

    #[test]
    fn framework_from_ast_detects_exception_and_fatal() {
        assert_eq!(
            framework_from_ast(&parse_src("use Test::Exception;\nthrows_ok { die } qr/x/;")).0,
            "Test::Exception"
        );
        assert_eq!(framework_from_ast(&parse_src("use Test::Fatal;\nok(1);")).0, "Test::Fatal");
    }

    #[test]
    fn framework_from_ast_returns_unknown_for_non_test() {
        assert_eq!(framework_from_ast(&parse_src("use strict;\nprint 1;\n")).0, "unknown");
    }

    #[test]
    fn oracle_for_maps_known_assertions_only() {
        // Independent contract list — deliberately NOT derived from ASSERTION_ORACLES,
        // so a rename or kind/strength drift in the table (e.g. `is_deeply` →
        // `is-deeply`) breaks this test instead of silently dropping coverage.
        let expected: &[(&str, &str, &str)] = &[
            ("is", "exact_return_assertion", "strong_exact"),
            ("isnt", "exact_return_assertion", "strong_exact"),
            ("is_deeply", "exact_return_assertion", "strong_exact"),
            ("cmp_ok", "predicate_boundary_assertion", "strong_exact"),
            ("like", "predicate_boundary_assertion", "weak_broad"),
            ("unlike", "predicate_boundary_assertion", "weak_broad"),
            ("isa_ok", "predicate_boundary_assertion", "weak_broad"),
            ("can_ok", "predicate_boundary_assertion", "weak_broad"),
            ("ok", "smoke_ok", "weak_smoke"),
            ("pass", "smoke_ok", "weak_smoke"),
            ("fail", "smoke_ok", "weak_smoke"),
            ("use_ok", "mention_only", "mention_only"),
            ("require_ok", "mention_only", "mention_only"),
            ("throws_ok", "exception_observer", "weak_broad"),
            ("dies_ok", "exception_observer", "weak_broad"),
            ("lives_ok", "smoke_ok", "weak_smoke"),
            ("lives_and", "exception_observer", "weak_broad"),
            ("exception", "exception_observer", "weak_broad"),
            ("dies", "exception_observer", "weak_broad"),
            ("lives", "smoke_ok", "weak_smoke"),
            ("warning_is", "warn_observer", "weak_broad"),
            ("warning_like", "warn_observer", "weak_broad"),
            ("warnings_are", "warn_observer", "weak_broad"),
        ];
        for (name, kind, strength) in expected {
            assert_eq!(
                oracle_for(name),
                Some((*kind, *strength)),
                "{name} must map to ({kind}, {strength})"
            );
        }
        // Length guard: a new table entry that isn't added to `expected` above
        // fails here, forcing coverage to track the table.
        assert_eq!(
            ASSERTION_ORACLES.len(),
            expected.len(),
            "every ASSERTION_ORACLES entry must have a contract assertion above"
        );
        // Non-assertions never map — no diagnostics, no arbitrary calls.
        assert!(oracle_for("diag").is_none(), "diag is a diagnostic, not an oracle");
        assert!(oracle_for("note").is_none(), "note is a diagnostic, not an oracle");
        assert!(oracle_for("not_an_assertion").is_none());
    }

    #[test]
    fn emit_tests_and_oracles_for_test_more_file() {
        let root = std::env::temp_dir().join("perl-p4-test-more-root");
        let t_dir = root.join("t");
        std::fs::create_dir_all(&t_dir).expect("mkdir");
        std::fs::write(
            t_dir.join("app.t"),
            "use Test::More;\nis(1, 1, 'one');\nok(1, 'truthy');\n",
        )
        .expect("write");

        let (tests, oracles, provenance, _limitations) =
            emit_tests_and_oracles(root.to_str().expect("utf8 root"));

        assert_eq!(tests.len(), 1, "one test fact for the .t file");
        assert_eq!(tests[0]["framework"], "Test::More");
        assert!(oracles.iter().any(|o| o["kind"] == "exact_return_assertion"), "is → exact");
        assert!(oracles.iter().any(|o| o["kind"] == "smoke_ok"), "ok → smoke");
        // Parser-backed: no placeholder 1:1 ranges.
        let placeholder =
            json!({"start_line": 1, "start_column": 1, "end_line": 1, "end_column": 1});
        assert!(oracles.iter().all(|o| o["range"] != placeholder), "no placeholder oracle ranges");
        assert!(provenance.iter().any(|p| p["source"] == "test_discovery"));
        assert!(provenance.iter().any(|p| p["source"] == "oracle_extraction"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn emit_returns_empty_when_no_t_files() {
        let root = std::env::temp_dir().join("perl-p4-empty-root");
        std::fs::create_dir_all(&root).expect("mkdir");
        let (tests, oracles, provenance, limitations) =
            emit_tests_and_oracles(root.to_str().expect("utf8 root"));
        assert!(tests.is_empty() && oracles.is_empty(), "no .t files → no tests/oracles");
        assert!(
            provenance.is_empty() && limitations.is_empty(),
            "no .t files → no prov/limitations"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn collect_t_files_reads_symlinked_t_file() {
        use std::os::unix::fs::symlink;
        // A symlinked `.t` file under `t/` must still be discovered + read — only
        // symlinked *directories* are skipped (loop safety).
        let root = std::env::temp_dir().join("perl-p4-symlink-t");
        let _ = std::fs::remove_dir_all(&root);
        let shared = root.join("shared");
        let t_dir = root.join("t");
        std::fs::create_dir_all(&shared).expect("mkdir shared");
        std::fs::create_dir_all(&t_dir).expect("mkdir t");
        std::fs::write(shared.join("real.t"), "use Test::More;\nok(1);\n").expect("write real");
        symlink(shared.join("real.t"), t_dir.join("linked.t")).expect("symlink .t");

        let (tests, _oracles, _provenance, _limitations) =
            emit_tests_and_oracles(root.to_str().expect("utf8 root"));
        assert!(
            tests.iter().any(|t| t["name"] == "t/linked.t"),
            "symlinked .t file must be read, not silently dropped"
        );
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
        let (tests, _oracles, _provenance, _limitations) =
            emit_tests_and_oracles(root.to_str().unwrap());
        let (relations, _observables, _sinks, _relation_limitations) =
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

        let (tests, _oracles, _provenance, _limitations) =
            emit_tests_and_oracles(root.to_str().unwrap());
        let (_relations, observables, sinks, _relation_limitations) =
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

    // ── P1 tests (FileFactShard mapping helpers) ──

    #[test]
    fn content_hash_to_digest_formats_hex() {
        assert_eq!(content_hash_to_digest(0), "fnv64:0000000000000000");
        assert_eq!(content_hash_to_digest(255), "fnv64:00000000000000ff");
        assert_eq!(content_hash_to_digest(0xcbf29ce484222325), "fnv64:cbf29ce484222325");
    }

    #[test]
    fn uri_to_relative_path_strips_file_prefix() {
        assert_eq!(uri_to_relative_path("file:///lib/My/App.pm"), "lib/My/App.pm");
        assert_eq!(uri_to_relative_path("lib/App.pm"), "lib/App.pm");
        assert_eq!(uri_to_relative_path("file://C:/repo/lib/App.pm"), "C:/repo/lib/App.pm");
    }

    #[test]
    fn file_role_from_path_classifies_correctly() {
        assert_eq!(file_role_from_path("lib/My/App.pm"), "source");
        assert_eq!(file_role_from_path("script/run.pl"), "source");
        assert_eq!(file_role_from_path("t/app.t"), "test");
        assert_eq!(file_role_from_path("app.psgi"), "source");
        assert_eq!(file_role_from_path("Makefile.PL"), "config");
        assert_eq!(file_role_from_path("cpanfile"), "config");
        assert_eq!(file_role_from_path("README.md"), "unknown");
    }

    // ── P2 tests (diff-derived changes + discriminators) ──

    #[test]
    fn infer_predicate_boundary_from_if_condition() {
        let (kind, disc) = infer_behavior_and_discriminator("    if ($amount >= $threshold) {");
        assert_eq!(kind, "predicate_boundary");
        assert!(disc.contains(">="), "discriminator must contain the comparison: {disc}");
    }

    #[test]
    fn infer_return_value_from_return() {
        let (kind, disc) = infer_behavior_and_discriminator("    return $discounted;");
        assert_eq!(kind, "return_value");
        assert_eq!(disc, "$discounted");
    }

    #[test]
    fn infer_exception_from_die() {
        let (kind, disc) = infer_behavior_and_discriminator("    die \"Invalid amount: $amount\";");
        assert_eq!(kind, "exception_path");
        assert!(
            disc.contains("Invalid amount"),
            "discriminator must contain the die message: {disc}"
        );
    }

    #[test]
    fn infer_unknown_for_assignment() {
        let (kind, disc) = infer_behavior_and_discriminator("    my $x = 1;");
        assert_eq!(kind, "unknown");
        assert!(disc.is_empty(), "unknown must have empty discriminator");
    }

    #[test]
    fn infer_modifier_if_is_not_predicate_boundary() {
        // A trailing modifier-if/unless must NOT hijack the classification — the
        // statement's own category wins (droid P1).
        assert_eq!(infer_behavior_and_discriminator("    return $x if $y > 5;").0, "return_value");
        assert_eq!(
            infer_behavior_and_discriminator("    die \"bad\" if $y > 5;").0,
            "exception_path"
        );
        assert_eq!(
            infer_behavior_and_discriminator("    croak \"x\" unless $y >= 3;").0,
            "exception_path"
        );
        // A LEADING conditional is still a predicate boundary.
        assert_eq!(infer_behavior_and_discriminator("    if ($x > 5) {").0, "predicate_boundary");
        assert_eq!(
            infer_behavior_and_discriminator("    while ($i < 10) {").0,
            "predicate_boundary"
        );
        let (kind, disc) = infer_behavior_and_discriminator("    } elsif ($y < 3) {");
        assert_eq!(kind, "predicate_boundary");
        assert!(disc.contains("$y < 3"), "elsif condition extracted cleanly: {disc}");
    }

    // ── PR 5 (#3293): diff-owned changes[] ──

    /// A `lib/My/App.pm` file fact + a `sub discount` owner spanning 0-based
    /// lines 4..8, for the diff-change tests below.
    fn app_files_and_owners() -> (Vec<Value>, Vec<Value>) {
        let files = vec![json!({ "file_id": "file:lib/My/App.pm" })];
        let owners = vec![
            json!({
                "owner_id": "owner:lib/My/App.pm:package:main::App:0-200",
                "file_id": "file:lib/My/App.pm",
                "kind": "package",
                "range": {"start_line": 0, "start_column": 0, "end_line": 20, "end_column": 1},
            }),
            json!({
                "owner_id": "owner:lib/My/App.pm:sub:main::discount:60-140",
                "file_id": "file:lib/My/App.pm",
                "kind": "sub",
                "range": {"start_line": 4, "start_column": 0, "end_line": 8, "end_column": 1},
            }),
        ];
        (files, owners)
    }

    #[test]
    fn emit_changes_from_diff_emits_change_for_hunk_inside_a_sub() {
        let (files, owners) = app_files_and_owners();
        // New start line 6 (1-based) → 0-based 5; the added line lands at line 5,
        // inside the sub's 4..8 range.
        let diff = "\
--- a/lib/My/App.pm
+++ b/lib/My/App.pm
@@ -5,3 +5,4 @@
 sub discount {
     my ($amount) = @_;
+    return $amount / 2;
 }
";
        let (changes, _limitations) = emit_changes_from_diff(diff, ".", &files, &owners);
        assert_eq!(changes.len(), 1, "one added line inside a sub → one change");
        assert_eq!(changes[0]["owner_id"], "owner:lib/My/App.pm:sub:main::discount:60-140");
        assert_eq!(changes[0]["behavior_hint"], "return_value");
        assert_eq!(changes[0]["file_id"], "file:lib/My/App.pm");
        // Schema-contract parity: both nullable observation keys are present.
        assert!(changes[0].get("changed_observable").is_some());
        assert!(changes[0].get("missing_discriminator").is_some());
        assert!(
            changes[0]["changed_observable"].as_str().unwrap_or("").contains("$amount"),
            "changed_observable carries the return expression"
        );
    }

    #[test]
    fn emit_changes_from_diff_hunk_at_file_scope_produces_no_change_but_a_limitation() {
        // File is known, but the added line is above every owner's range.
        let files = vec![json!({ "file_id": "file:lib/My/App.pm" })];
        let owners = vec![json!({
            "owner_id": "owner:lib/My/App.pm:sub:main::foo:60-140",
            "file_id": "file:lib/My/App.pm",
            "kind": "sub",
            "range": {"start_line": 10, "start_column": 0, "end_line": 14, "end_column": 1},
        })];
        let diff = "\
+++ b/lib/My/App.pm
@@ -1,0 +1,1 @@
+use strict;
";
        let (changes, limitations) = emit_changes_from_diff(diff, ".", &files, &owners);
        assert!(changes.is_empty(), "file-scope hunk (no enclosing owner) → no change fact");
        assert!(
            limitations.iter().any(|l| l["limitation_id"]
                .as_str()
                .is_some_and(|s| s.starts_with("unattributable-change:"))),
            "must record an unattributable-change limitation"
        );
    }

    #[test]
    fn emit_changes_from_diff_unknown_file_records_diff_file_not_found() {
        // The diff touches a file the packet never parsed (outside root) — no
        // change, but a diff-file-not-found limitation instead of a silent drop.
        let files = vec![json!({ "file_id": "file:lib/My/App.pm" })];
        let owners: Vec<Value> = Vec::new();
        let diff = "\
+++ b/other/Thing.pm
@@ -1,0 +1,1 @@
+return 1;
";
        let (changes, limitations) = emit_changes_from_diff(diff, ".", &files, &owners);
        assert!(changes.is_empty(), "hunk in an unknown file → no change");
        assert!(
            limitations.iter().any(|l| l["limitation_id"]
                .as_str()
                .is_some_and(|s| s.starts_with("diff-file-not-found:"))),
            "must record a diff-file-not-found limitation"
        );
    }

    #[test]
    fn emit_changes_from_diff_is_deterministic_and_stable_across_reordering() {
        let (files, owners) = app_files_and_owners();
        let hunk_a = "@@ -5,2 +5,3 @@\n sub discount {\n+    return 1;\n";
        let hunk_b = "@@ -6,2 +6,3 @@\n     my $x = 1;\n+    return 2;\n";
        let header = "+++ b/lib/My/App.pm\n";
        let ab = format!("{header}{hunk_a}{hunk_b}");
        let ba = format!("{header}{hunk_b}{hunk_a}");
        let (changes_ab, _) = emit_changes_from_diff(&ab, ".", &files, &owners);
        let (changes_ab2, _) = emit_changes_from_diff(&ab, ".", &files, &owners);
        let (changes_ba, _) = emit_changes_from_diff(&ba, ".", &files, &owners);
        // Same input → byte-identical output.
        assert_eq!(changes_ab, changes_ab2, "same diff → identical changes");
        // change_id is derived from (file_id, start_line, end_line), so each
        // hunk's id is stable no matter the order the hunks appear in the diff.
        let ids = |cs: &[Value]| {
            let mut v: Vec<String> =
                cs.iter().filter_map(|c| c["change_id"].as_str().map(str::to_owned)).collect();
            v.sort();
            v
        };
        assert_eq!(ids(&changes_ab), ids(&changes_ba), "change_ids stable across reordering");
    }

    #[test]
    fn find_enclosing_owner_picks_smallest_of_nested_package_and_sub() {
        let (_files, owners) = app_files_and_owners();
        // Line 5 is inside both the package (0..20) and the sub (4..8) → sub wins.
        let owner = find_enclosing_owner(&owners, "file:lib/My/App.pm", 5, 5).expect("an owner");
        assert_eq!(owner["kind"], "sub", "smallest enclosing owner is the sub, not the package");
    }

    #[test]
    fn find_enclosing_owner_returns_none_when_no_owner_contains_the_range() {
        let (_files, owners) = app_files_and_owners();
        assert!(
            find_enclosing_owner(&owners, "file:lib/My/App.pm", 50, 50).is_none(),
            "a range outside every owner yields None"
        );
    }

    #[test]
    fn behavior_hint_for_hunk_first_matching_line_wins() {
        assert_eq!(behavior_hint_for_hunk(&["    my $x = 1;".into()]).0, "unknown");
        assert_eq!(behavior_hint_for_hunk(&["    return $x + 1;".into()]).0, "return_value");
        assert_eq!(behavior_hint_for_hunk(&["    if ($x >= 10) {".into()]).0, "predicate_boundary");
        assert_eq!(behavior_hint_for_hunk(&["    die \"bad\";".into()]).0, "exception_path");
        // First recognized line wins over a later one.
        let lines = vec!["    my $x = 1;".into(), "    return $x;".into(), "    die \"z\";".into()];
        assert_eq!(behavior_hint_for_hunk(&lines).0, "return_value");
    }

    #[test]
    fn behavior_hint_for_hunk_skips_comment_lines() {
        // A whole-line comment is not executable → never a concrete hint.
        assert_eq!(behavior_hint_for_hunk(&["# return $x;".into()]).0, "unknown");
        assert_eq!(behavior_hint_for_hunk(&["    # die now".into()]).0, "unknown");
        // Real code after a comment still wins.
        let lines = vec!["# comment".into(), "    return $x;".into()];
        assert_eq!(behavior_hint_for_hunk(&lines).0, "return_value");
    }

    #[test]
    fn strip_root_prefix_normalizes_subdir_paths() {
        assert_eq!(strip_root_prefix("crates/p/lib/A.pm", "crates/p"), "lib/A.pm");
        assert_eq!(strip_root_prefix("lib/A.pm", "."), "lib/A.pm");
        // A path not under root is left unchanged (→ diff-file-not-found).
        assert_eq!(strip_root_prefix("other/A.pm", "crates/p"), "other/A.pm");
    }

    #[test]
    fn emit_changes_from_diff_normalizes_git_paths_against_subdir_root() {
        // git diff paths are repo-root-relative; file_ids are root-relative. A
        // subdir root must not make every hunk diff-file-not-found.
        let files = vec![json!({ "file_id": "file:lib/App.pm" })];
        let owners = vec![json!({
            "owner_id": "owner:lib/App.pm:sub:main::f:0-40",
            "file_id": "file:lib/App.pm",
            "kind": "sub",
            "range": {"start_line": 0, "start_column": 0, "end_line": 10, "end_column": 1},
        })];
        let diff =
            "+++ b/crates/perl-parser/lib/App.pm\n@@ -1,1 +1,2 @@\n sub f {\n+    return 1;\n";
        let (changes, _l) = emit_changes_from_diff(diff, "crates/perl-parser", &files, &owners);
        assert_eq!(changes.len(), 1, "subdir-root git path must normalize and match the file_id");
        assert_eq!(changes[0]["file_id"], "file:lib/App.pm");
    }

    #[test]
    fn emit_changes_from_diff_digest_uses_fnv64_prefix_not_sha256() {
        let (files, owners) = app_files_and_owners();
        let diff = "+++ b/lib/My/App.pm\n@@ -5,2 +5,3 @@\n sub discount {\n+    return 1;\n";
        let (changes, _) = emit_changes_from_diff(diff, ".", &files, &owners);
        let digest = changes[0]["changed_text_digest"].as_str().expect("digest string");
        assert!(
            digest.starts_with("fnv64:"),
            "digest must use the real fnv64: prefix, not sha256:"
        );
        assert_eq!(digest, content_hash_to_digest(fnv1a_hash("    return 1;")));
    }

    #[test]
    fn emit_changes_from_diff_missing_discriminator_is_always_null() {
        let (files, owners) = app_files_and_owners();
        let diff = "+++ b/lib/My/App.pm\n@@ -5,2 +5,3 @@\n sub discount {\n+    return 1;\n";
        let (changes, _) = emit_changes_from_diff(diff, ".", &files, &owners);
        assert!(
            changes.iter().all(|c| c["missing_discriminator"].is_null()),
            "missing_discriminator is always null in this slice"
        );
    }

    #[test]
    fn parse_diff_hunks_ignores_no_newline_marker() {
        // The `\ No newline at end of file` marker is metadata about the
        // preceding line; it must not advance the head-file cursor.
        let with_marker =
            "+++ b/f.pm\n@@ -5,1 +5,2 @@\n-old\n\\ No newline at end of file\n+new1\n+new2\n";
        let hunks = parse_diff_hunks(with_marker);
        assert_eq!(hunks.len(), 1, "one added-line run");
        // `+5` → 0-based 4; new1/new2 land at head lines 4 and 5, unshifted.
        assert_eq!(hunks[0].start_line, 4, "marker must not shift the head cursor");
        assert_eq!(hunks[0].end_line, 5);
    }

    #[test]
    fn emit_changes_from_diff_no_newline_marker_does_not_misattribute() {
        // A tight owner (lines 4..5). Without the marker fix, the added line would
        // shift to 6 and fall outside the owner → wrong result.
        let files = vec![json!({ "file_id": "file:lib/My/App.pm" })];
        let owners = vec![json!({
            "owner_id": "owner:lib/My/App.pm:sub:main::tiny:40-70",
            "file_id": "file:lib/My/App.pm",
            "kind": "sub",
            "range": {"start_line": 4, "start_column": 0, "end_line": 5, "end_column": 1},
        })];
        let diff = "+++ b/lib/My/App.pm\n@@ -5,1 +5,2 @@\n-old\n\\ No newline at end of file\n+    return 1;\n";
        let (changes, _limitations) = emit_changes_from_diff(diff, ".", &files, &owners);
        assert_eq!(changes.len(), 1, "the added line stays inside the owner");
        assert_eq!(changes[0]["owner_id"], "owner:lib/My/App.pm:sub:main::tiny:40-70");
    }

    #[test]
    fn oracle_facts_carry_observed_sink_and_expected_expression_fields() {
        // Contract-freeze parity (Campaign 31 step 2): the schema declares
        // observed_sink + expected_expression on oracle (nullable). The emitter
        // must write both keys — null when not yet derived — so ripr's
        // deserializer sees them. This test runs the test/oracle emitter and
        // asserts every oracle carries both keys.
        let temp = std::env::temp_dir().join("ripr_facts_oracle_contract_test");
        let t_dir = temp.join("t");
        std::fs::create_dir_all(&t_dir).unwrap();
        std::fs::write(
            t_dir.join("app.t"),
            "use Test::More;\nis(disco(100), 50, 'half');\nok(1);\n",
        )
        .unwrap();
        let (_tests, oracles, _provenance, _limitations) =
            emit_tests_and_oracles(temp.to_str().unwrap());
        let _ = std::fs::remove_dir_all(&temp);
        assert!(!oracles.is_empty(), "expected at least one oracle fact");
        for oracle in &oracles {
            assert!(
                oracle.get("observed_sink").is_some(),
                "oracle must carry observed_sink (schema contract): {oracle}"
            );
            assert!(
                oracle.get("expected_expression").is_some(),
                "oracle must carry expected_expression (schema contract): {oracle}"
            );
        }
    }

    #[test]
    fn emit_changes_empty_when_diff_has_no_added_lines() {
        // A pure-deletion diff (no `+` content) yields no hunks → no changes.
        let files = vec![json!({ "file_id": "file:lib/My/App.pm" })];
        let owners: Vec<Value> = Vec::new();
        let diff = "+++ b/lib/My/App.pm\n@@ -5,2 +5,1 @@\n sub discount {\n-    return $x;\n";
        let (changes, _limitations) = emit_changes_from_diff(diff, ".", &files, &owners);
        assert!(changes.is_empty(), "a deletion-only hunk produces no change facts");
    }

    #[test]
    fn fnv1a_hash_is_deterministic() {
        assert_eq!(fnv1a_hash("hello"), fnv1a_hash("hello"));
        assert_ne!(fnv1a_hash("hello"), fnv1a_hash("world"));
    }

    // ── P3 / #3293 PR 6 tests (direct_owner_call relations) ──

    #[test]
    fn emit_relations_upgrades_to_direct_owner_call() {
        let root = std::env::temp_dir().join("perl-P3-direct-call-root");
        let lib_dir = root.join("lib/My");
        let t_dir = root.join("t");
        std::fs::create_dir_all(&lib_dir).unwrap();
        std::fs::create_dir_all(&t_dir).unwrap();
        std::fs::write(lib_dir.join("App.pm"), "package My::App;\nsub discount { }\n1;\n").unwrap();
        // Test file uses the package directly.
        std::fs::write(t_dir.join("App.t"), "use My::App;\nis(My::App::discount(100), 50);\n")
            .unwrap();

        let (tests, _oracles, _provenance, _limitations) =
            emit_tests_and_oracles(root.to_str().unwrap());
        let (relations, _observables, _sinks, _relation_limitations) =
            emit_relations_and_discriminators(root.to_str().unwrap(), &tests, &[]);

        assert!(!relations.is_empty(), "must find at least one relation");
        assert!(
            relations.iter().any(|r| r["relation_kind"] == "direct_owner_call"),
            "must emit at least one direct_owner_call relation"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    // ── #3293 PR 6: parser-backed `test_calls_declared_sub` predicate ──

    /// Build `TestCallFacts` directly from source, mirroring what
    /// `emit_relations_and_discriminators` does per `.t` file.
    fn facts_from_src(src: &str) -> TestCallFacts {
        TestCallFacts::from_ast(&parse_src(src))
    }

    fn one_sub(name: &str) -> std::collections::HashSet<String> {
        std::collections::HashSet::from([name.to_string()])
    }

    #[test]
    fn declared_sub_matches_qualified_call_without_use() {
        let facts = facts_from_src("My::App::discount(100);\n");
        assert!(
            test_calls_declared_sub(&facts, "My::App", &one_sub("discount")),
            "qualified call must prove direct_owner_call even without a `use`"
        );
    }

    #[test]
    fn declared_sub_rejects_bare_call_after_use_export_unproven() {
        // A bare call after `use My::App` is NOT proof of direct_owner_call: the
        // sub is only callable bareword if My::App exports it (Exporter/@EXPORT),
        // which perl-symbol can't see. Marking it `reachable` would be a false
        // positive (the call fails at runtime if the sub isn't exported), so the
        // bare-call case degrades to the conservative file_proximity branch.
        let facts = facts_from_src("use My::App;\ndiscount(100);\n");
        assert!(
            !test_calls_declared_sub(&facts, "My::App", &one_sub("discount")),
            "a bare call after `use` is export-unproven — must not claim direct_owner_call"
        );
    }

    #[test]
    fn declared_sub_rejects_use_without_matching_call() {
        let facts = facts_from_src("use My::App;\nok(1);\n");
        assert!(
            !test_calls_declared_sub(&facts, "My::App", &one_sub("discount")),
            "`use` alone, with no call to a declared sub, must not prove direct_owner_call"
        );
    }

    #[test]
    fn declared_sub_rejects_call_name_in_comment() {
        // The key false positive `test_calls_package`'s substring scan had:
        // a comment mentioning the qualified name is not a call node.
        let facts = facts_from_src("# My::App::discount(100);\nok(1);\n");
        assert!(
            !test_calls_declared_sub(&facts, "My::App", &one_sub("discount")),
            "a call name inside a comment must never prove direct_owner_call"
        );
    }

    #[test]
    fn declared_sub_rejects_call_name_in_string_literal() {
        let facts = facts_from_src("my $x = \"My::App::discount(100)\";\nok(1);\n");
        assert!(
            !test_calls_declared_sub(&facts, "My::App", &one_sub("discount")),
            "a call name inside a string literal must never prove direct_owner_call"
        );
    }

    #[test]
    fn declared_sub_rejects_static_method_call() {
        let facts = facts_from_src("My::App->discount(100);\n");
        assert!(
            !test_calls_declared_sub(&facts, "My::App", &one_sub("discount")),
            "a static method call must degrade — dispatch can resolve to an \
             inherited/AUTOLOAD'd method the AST can't distinguish from a direct hit"
        );
    }

    #[test]
    fn declared_sub_rejects_instance_method_call() {
        let facts = facts_from_src("my $o = My::App->new;\n$o->discount(100);\n");
        assert!(
            !test_calls_declared_sub(&facts, "My::App", &one_sub("discount")),
            "an instance method call on a dynamic receiver must degrade"
        );
    }

    #[test]
    fn declared_sub_rejects_qualified_call_to_different_package() {
        let facts = facts_from_src("Other::Pkg::discount(100);\n");
        assert!(
            !test_calls_declared_sub(&facts, "My::App", &one_sub("discount")),
            "a call qualified to a different package must not prove this package's relation"
        );
    }

    #[test]
    fn declared_sub_rejects_bare_call_without_use() {
        let facts = facts_from_src("discount(100);\n");
        assert!(
            !test_calls_declared_sub(&facts, "My::App", &one_sub("discount")),
            "a bare call with no matching `use` must not prove direct_owner_call \
             (same bare name could belong to any package)"
        );
    }

    #[test]
    fn declared_sub_rejects_when_declared_subs_empty() {
        // Even a real, unambiguous qualified call must degrade when the
        // candidate package's `.pm` declared no subs (e.g. it failed to
        // parse, or only declared anonymous subs) — never a false `true`.
        let facts = facts_from_src("My::App::discount(100);\n");
        assert!(
            !test_calls_declared_sub(&facts, "My::App", &std::collections::HashSet::new()),
            "empty declared_subs must always degrade, regardless of call shape"
        );
    }

    #[test]
    fn declared_sub_rejects_use_with_empty_import_list_and_no_call() {
        // `test_calls_package`'s `content.contains("(")` used to trip on the
        // parens of `use My::App ();` alone, with zero actual calls.
        let facts = facts_from_src("use My::App ();\nok(1);\n");
        assert!(
            !test_calls_declared_sub(&facts, "My::App", &one_sub("discount")),
            "an empty-import-list `use` with no call must not prove direct_owner_call"
        );
    }

    #[test]
    fn declared_sub_names_filters_by_container() {
        // A file declaring two packages must only credit the one being asked
        // about — `declared_sub_names` filters on `container`, not just name.
        let ast = parse_src(
            "package My::App;\nsub discount { }\npackage Other::Pkg;\nsub discount { }\n",
        );
        let app_subs = declared_sub_names(&ast, "My::App");
        assert!(app_subs.contains("discount"));
        let other_subs = declared_sub_names(&ast, "Other::Pkg");
        assert!(other_subs.contains("discount"));
        // Cross-check: a package name that appears nowhere yields no subs.
        assert!(declared_sub_names(&ast, "Unrelated::Pkg").is_empty());
    }

    #[test]
    fn emit_relations_bare_call_after_use_stays_file_proximity() {
        // End-to-end: a bare call after `use` is export-unproven, so the relation
        // stays file_proximity/weakly_reachable — NOT a false direct_owner_call.
        let root = std::env::temp_dir().join("perl-P6-bare-call-root");
        let lib_dir = root.join("lib/My");
        let t_dir = root.join("t");
        std::fs::create_dir_all(&lib_dir).unwrap();
        std::fs::create_dir_all(&t_dir).unwrap();
        std::fs::write(lib_dir.join("App.pm"), "package My::App;\nsub discount { }\n1;\n").unwrap();
        std::fs::write(t_dir.join("App.t"), "use My::App;\ndiscount(100);\nok(1);\n").unwrap();

        let (tests, _oracles, _provenance, _limitations) =
            emit_tests_and_oracles(root.to_str().unwrap());
        let (relations, _observables, _sinks, _relation_limitations) =
            emit_relations_and_discriminators(root.to_str().unwrap(), &tests, &[]);

        assert!(!relations.is_empty(), "the file_proximity relation is still emitted");
        assert!(
            relations.iter().all(|r| r["relation_kind"] != "direct_owner_call"),
            "a bare (export-unproven) call must NOT be direct_owner_call, got: {relations:?}"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn emit_relations_shared_basename_bare_call_marks_neither_direct() {
        // Two packages share the basename `Widget` and both declare `run`; the
        // test `use`s both and calls bare `run()`. At most one is the real owner,
        // so neither may be claimed direct_owner_call (the string version would
        // have marked both). Qualified calls would disambiguate; bare ones can't.
        let root = std::env::temp_dir().join("perl-P6-shared-basename-root");
        let t_dir = root.join("t");
        std::fs::create_dir_all(root.join("lib/V1")).unwrap();
        std::fs::create_dir_all(root.join("lib/V2")).unwrap();
        std::fs::create_dir_all(&t_dir).unwrap();
        std::fs::write(root.join("lib/V1/Widget.pm"), "package V1::Widget;\nsub run { }\n1;\n")
            .unwrap();
        std::fs::write(root.join("lib/V2/Widget.pm"), "package V2::Widget;\nsub run { }\n1;\n")
            .unwrap();
        std::fs::write(
            t_dir.join("Widget.t"),
            "use V1::Widget;\nuse V2::Widget;\nrun();\nok(1);\n",
        )
        .unwrap();

        let (tests, _oracles, _provenance, _limitations) =
            emit_tests_and_oracles(root.to_str().unwrap());
        let (relations, _observables, _sinks, _relation_limitations) =
            emit_relations_and_discriminators(root.to_str().unwrap(), &tests, &[]);

        assert!(
            relations.iter().all(|r| r["relation_kind"] != "direct_owner_call"),
            "an ambiguous shared-basename bare call must mark neither package direct, got: {relations:?}"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn emit_relations_comment_mention_stays_file_proximity() {
        // The headline false positive the string heuristic had, proven fixed
        // through the full emitter, not just the predicate.
        let root = std::env::temp_dir().join("perl-P6-comment-mention-root");
        let lib_dir = root.join("lib/My");
        let t_dir = root.join("t");
        std::fs::create_dir_all(&lib_dir).unwrap();
        std::fs::create_dir_all(&t_dir).unwrap();
        std::fs::write(lib_dir.join("App.pm"), "package My::App;\nsub discount { }\n1;\n").unwrap();
        std::fs::write(t_dir.join("App.t"), "# My::App::discount(100) is deprecated\nok(1);\n")
            .unwrap();

        let (tests, _oracles, _provenance, _limitations) =
            emit_tests_and_oracles(root.to_str().unwrap());
        let (relations, _observables, _sinks, _relation_limitations) =
            emit_relations_and_discriminators(root.to_str().unwrap(), &tests, &[]);

        assert!(!relations.is_empty(), "must still find a file_proximity relation");
        assert!(
            relations.iter().all(|r| r["relation_kind"] == "file_proximity"),
            "a comment-only mention must never upgrade to direct_owner_call, got: {relations:?}"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn emit_relations_deterministic_across_runs_and_fs_order() {
        let root = std::env::temp_dir().join("perl-P6-determinism-root");
        let lib_dir = root.join("lib/My");
        let t_dir = root.join("t");
        std::fs::create_dir_all(&lib_dir).unwrap();
        std::fs::create_dir_all(&t_dir).unwrap();
        // Create files in reverse-alphabetical order on disk.
        std::fs::write(lib_dir.join("Zeta.pm"), "package My::Zeta;\nsub run { }\n1;\n").unwrap();
        std::fs::write(lib_dir.join("Alpha.pm"), "package My::Alpha;\nsub run { }\n1;\n").unwrap();
        std::fs::write(t_dir.join("Zeta.t"), "use My::Zeta;\nrun();\nok(1);\n").unwrap();
        std::fs::write(t_dir.join("Alpha.t"), "use My::Alpha;\nrun();\nok(1);\n").unwrap();

        let (tests, _oracles, _provenance, _limitations) =
            emit_tests_and_oracles(root.to_str().unwrap());
        let (relations_1, _observables_1, _sinks_1, _limitations_1) =
            emit_relations_and_discriminators(root.to_str().unwrap(), &tests, &[]);
        let (relations_2, _observables_2, _sinks_2, _limitations_2) =
            emit_relations_and_discriminators(root.to_str().unwrap(), &tests, &[]);

        assert_eq!(
            relations_1, relations_2,
            "two runs over the same root must produce byte-identical relations JSON"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn emit_relations_sorted_by_relation_id() {
        let root = std::env::temp_dir().join("perl-P6-sorted-root");
        let lib_dir = root.join("lib/My");
        let t_dir = root.join("t");
        std::fs::create_dir_all(&lib_dir).unwrap();
        std::fs::create_dir_all(&t_dir).unwrap();
        std::fs::write(lib_dir.join("Zeta.pm"), "package My::Zeta;\nsub run { }\n1;\n").unwrap();
        std::fs::write(lib_dir.join("Mid.pm"), "package My::Mid;\nsub run { }\n1;\n").unwrap();
        std::fs::write(lib_dir.join("Alpha.pm"), "package My::Alpha;\nsub run { }\n1;\n").unwrap();
        std::fs::write(t_dir.join("Zeta.t"), "use My::Zeta;\nrun();\n").unwrap();
        std::fs::write(t_dir.join("Mid.t"), "use My::Mid;\nrun();\n").unwrap();
        std::fs::write(t_dir.join("Alpha.t"), "use My::Alpha;\nrun();\n").unwrap();

        let (tests, _oracles, _provenance, _limitations) =
            emit_tests_and_oracles(root.to_str().unwrap());
        let (relations, _observables, _sinks, _relation_limitations) =
            emit_relations_and_discriminators(root.to_str().unwrap(), &tests, &[]);

        assert!(relations.len() >= 3, "expected at least 3 relations, got {}", relations.len());
        let ids: Vec<&str> = relations.iter().map(|r| r["relation_id"].as_str().unwrap()).collect();
        let mut sorted_ids = ids.clone();
        sorted_ids.sort_unstable();
        assert_eq!(ids, sorted_ids, "relations must be sorted by relation_id");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn relation_owner_id_resolves_to_owner_fact() {
        // #3342: a relation's `owner_id` must be the real `owners[]` id (kind +
        // qualified name + byte span), not the old dangling `owner:{path}:{pkg}`
        // shape — so a consumer walking `relation.owner_id → owners[]` resolves.
        let root = std::env::temp_dir().join("perl-3342-owner-id-root");
        let lib_dir = root.join("lib/My");
        let t_dir = root.join("t");
        std::fs::create_dir_all(&lib_dir).unwrap();
        std::fs::create_dir_all(&t_dir).unwrap();
        std::fs::write(lib_dir.join("App.pm"), "package My::App;\nsub discount { }\n1;\n").unwrap();
        std::fs::write(t_dir.join("App.t"), "use My::App;\ndiscount(100);\n").unwrap();

        let (tests, _oracles, _provenance, _limitations) =
            emit_tests_and_oracles(root.to_str().unwrap());
        let (relations, _observables, _sinks, _relation_limitations) =
            emit_relations_and_discriminators(root.to_str().unwrap(), &tests, &[]);
        let (_files, owners, _file_provenance, _file_limitations) =
            emit_files_and_owners(root.to_str().unwrap());

        assert!(!relations.is_empty());
        let owner_id = relations[0]["owner_id"].as_str().unwrap();
        // New shape: `owner:{path}:package:{qualified_name}:{span}` (not the bare
        // package name), with a byte span so it is not hard-pinned here.
        assert!(
            owner_id.starts_with("owner:lib/My/App.pm:package:My::App:"),
            "relation owner_id should use the resolvable owners[] shape, got {owner_id}"
        );
        // And it must actually resolve to an emitted `owners[]` fact.
        assert!(
            owners.iter().any(|o| o["owner_id"].as_str() == Some(owner_id)),
            "relation.owner_id {owner_id} must resolve to a present owners[] fact"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn emit_files_and_owners_extracts_packages_and_subs() {
        let root = std::env::temp_dir().join("perl-P3-files-owners-root");
        let lib_dir = root.join("lib/My");
        std::fs::create_dir_all(&lib_dir).unwrap();
        std::fs::write(
            lib_dir.join("App.pm"),
            "package My::App;\nsub discount { return 42; }\nsub total { }\n1;\n",
        )
        .unwrap();

        let (files, owners, provenance, limitations) =
            emit_files_and_owners(root.to_str().unwrap());

        // One file fact for the .pm — source role, an fnv64 digest, the package name.
        assert_eq!(files.len(), 1, "one .pm file → one file fact");
        let file = &files[0];
        assert_eq!(file["path"], "lib/My/App.pm");
        assert_eq!(file["role"], json!(["source"]));
        assert!(
            file["digest"].as_str().unwrap().starts_with("fnv64:"),
            "digest is an fnv64 hex string, got {:?}",
            file["digest"]
        );
        assert_eq!(file["package_names"], json!(["My::App"]));
        assert_eq!(file["file_id"], "file:lib/My/App.pm");
        assert_eq!(file["provenance_refs"], json!(["prov:syntax:file:lib/My/App.pm"]));

        // Owners: the package + both subs, with parser-derived kinds.
        let kinds: Vec<&str> = owners.iter().filter_map(|o| o["kind"].as_str()).collect();
        assert!(kinds.contains(&"package"), "package My::App must be an owner, got {kinds:?}");
        assert_eq!(
            kinds.iter().filter(|k| **k == "sub").count(),
            2,
            "both subs must be owners, got {kinds:?}"
        );

        // A sub owner carries a real range + the per-file syntax provenance ref.
        let sub = owners.iter().find(|o| o["name"] == "discount").expect("discount owner");
        assert_eq!(sub["kind"], "sub");
        assert_eq!(sub["file_id"], "file:lib/My/App.pm");
        assert_eq!(sub["confidence"], "high");
        assert_eq!(sub["provenance_refs"], json!(["prov:syntax:file:lib/My/App.pm"]));
        // `sub discount` is declared on the second line (0-based line 1).
        assert_eq!(sub["range"]["start_line"], 1, "discount is on the second line (0-based)");
        // The owner id is byte-span-derived, not traversal-order (no trailing `:N`).
        let owner_id = sub["owner_id"].as_str().unwrap();
        assert!(owner_id.contains("discount"), "owner id names the decl: {owner_id}");

        // A per-file `syntax` provenance fact exists, plus the digest-algorithm note.
        assert!(
            provenance
                .iter()
                .any(|p| p["source"] == "syntax" && p["file_id"] == "file:lib/My/App.pm"),
            "a per-file syntax provenance entry must exist"
        );
        assert!(
            limitations.iter().any(|l| l["limitation_id"] == "digest-algorithm"),
            "the fnv64 digest limitation must be recorded"
        );

        let _ = std::fs::remove_dir_all(&root);
    }
}
