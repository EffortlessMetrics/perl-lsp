//! Editor Intelligence Scorecard — Gold Corpus Harness
//!
//! Replays hover, goto-definition, and completion requests against real
//! gold-corpus fixtures and reports pass rates.  This is the headless
//! scorecard described in issue #4066.
//!
//! ## How it works
//!
//! 1. Discover all `test_corpus/gold/<fixture-name>/` directories.
//! 2. For each fixture directory that has `expected_hover.json`,
//!    `expected_goto.json`, or `expected_completion.json`, load the
//!    assertions and replay LSP requests.
//! 3. Print per-kind pass rates to stdout (`--nocapture`).
//! 4. Fail the test if *any* assertion fails (CI gate).
//!
//! ## Verify
//!
//! ```bash
//! RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --test editor_intelligence_scorecard -- --nocapture
//! ```

mod common;

use common::test_utils::TestServerBuilder;
use perl_corpus::gold::{
    CompletionAssertionKind, CompletionGoldFixture, GoldAssertion, GoldFixture, GotoAssertionKind,
    GotoGoldFixture, HoverAssertionKind, HoverGoldFixture, load_completion_gold_fixtures,
    load_document_symbol_gold_fixtures, load_gold_fixtures, load_goto_gold_fixtures,
    load_hover_gold_fixtures,
};
use perl_corpus::{DocumentSymbolAssertionKind, DocumentSymbolGoldFixture};
use serde_json::Value;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn gold_corpus_root() -> PathBuf {
    // Walk up from the test binary location to find the workspace root,
    // then resolve test_corpus/gold/.
    // CARGO_MANIFEST_DIR is set to the crate directory during tests.
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let crate_dir = PathBuf::from(manifest);
    // crates/perl-lsp-rs → workspace root is two levels up
    let workspace_root = crate_dir
        .parent() // crates/
        .and_then(|p| p.parent()) // workspace root
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| crate_dir.clone());
    workspace_root.join("test_corpus").join("gold")
}

fn hover_content_from_response(resp: &Value) -> Option<String> {
    let result = resp.get("result")?;
    if result.is_null() {
        return None;
    }
    let contents = result.get("contents")?;
    let value = contents.get("value")?.as_str()?;
    if value.is_empty() { None } else { Some(value.to_string()) }
}

fn completion_labels_from_response(resp: &Value) -> Vec<String> {
    let items = match resp["result"]["items"].as_array() {
        Some(arr) => arr,
        None => match resp["result"].as_array() {
            Some(arr) => arr,
            None => return Vec::new(),
        },
    };
    items.iter().filter_map(|item| item["label"].as_str().map(|s| s.to_string())).collect()
}

fn first_goto_line(resp: &Value) -> Option<u32> {
    let arr = resp.get("result")?.as_array()?;
    let first = arr.first()?;
    let line = first["range"]["start"]["line"].as_u64()? as u32;
    Some(line)
}

fn diagnostic_codes_from_response(resp: &Value) -> Vec<String> {
    resp["result"]["items"].as_array().map_or_else(Vec::new, |items| {
        items
            .iter()
            .filter_map(|item| {
                let code = item.get("code")?;
                code.as_str()
                    .map(ToString::to_string)
                    .or_else(|| code.as_i64().map(|n| n.to_string()))
            })
            .collect()
    })
}

fn document_symbol_names(resp: &Value) -> Vec<String> {
    fn collect_document_symbols(symbols: &Value, out: &mut Vec<String>) {
        if let Some(array) = symbols.as_array() {
            for symbol in array {
                if let Some(name) = symbol.get("name").and_then(Value::as_str) {
                    out.push(name.to_string());
                }

                if let Some(children) = symbol.get("children") {
                    collect_document_symbols(children, out);
                }
            }
        }
    }

    let mut names = Vec::new();
    if let Some(result) = resp.get("result") {
        collect_document_symbols(result, &mut names);
    }
    names
}

// ---------------------------------------------------------------------------
// Hover correctness test
// ---------------------------------------------------------------------------

/// Run all hover gold fixtures and assert every assertion passes.
/// Reports a pass-rate summary to stdout under --nocapture.
#[test]
fn test_hover_gold_corpus() {
    let root = gold_corpus_root();
    let fixtures: Vec<HoverGoldFixture> = match load_hover_gold_fixtures(&root) {
        Ok(f) if !f.is_empty() => f,
        Ok(_) => {
            eprintln!("SKIP: no hover gold fixtures found in {}", root.display());
            return;
        }
        Err(e) => panic!("Failed to load hover gold fixtures: {e}"),
    };

    let server = TestServerBuilder::new().build();

    let mut total = 0usize;
    let mut passed = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for fixture in &fixtures {
        let code = std::fs::read_to_string(&fixture.fixture_path).unwrap_or_else(|e| {
            panic!("Cannot read fixture {}: {e}", fixture.fixture_path.display())
        });

        let uri = format!("file:///gold/{}.pl", fixture.name);
        server.open_document(&uri, &code);

        for assertion in &fixture.hover_assertions {
            total += 1;
            let resp = server.get_hover(&uri, assertion.line, assertion.character);
            let content = hover_content_from_response(&resp);

            let ok = match &assertion.kind {
                HoverAssertionKind::HoverNonNull => content.is_some(),
                HoverAssertionKind::HoverNull => content.is_none(),
                HoverAssertionKind::HoverContains { needle } => {
                    content.as_deref().is_some_and(|c| c.contains(needle.as_str()))
                }
                HoverAssertionKind::HoverAbsent { needle } => {
                    content.as_deref().map_or(true, |c| !c.contains(needle.as_str()))
                }
            };

            if ok {
                passed += 1;
            } else {
                failures.push(format!(
                    "  FAIL [{}] {:?} at line:{} char:{} — got: {:?}",
                    fixture.name, assertion.kind, assertion.line, assertion.character, content
                ));
            }
        }
    }

    println!(
        "\nHover gold corpus: {}/{} assertions passed ({:.0}%)",
        passed,
        total,
        if total > 0 { passed as f64 / total as f64 * 100.0 } else { 100.0 }
    );
    for f in &failures {
        println!("{f}");
    }

    assert!(
        failures.is_empty(),
        "Hover gold corpus: {} assertion(s) failed out of {}:\n{}",
        failures.len(),
        total,
        failures.join("\n")
    );
}

// ---------------------------------------------------------------------------
// Goto-definition correctness test
// ---------------------------------------------------------------------------

/// Run all goto-definition gold fixtures and assert every assertion passes.
#[test]
fn test_goto_gold_corpus() {
    let root = gold_corpus_root();
    let fixtures: Vec<GotoGoldFixture> = match load_goto_gold_fixtures(&root) {
        Ok(f) if !f.is_empty() => f,
        Ok(_) => {
            eprintln!("SKIP: no goto gold fixtures found in {}", root.display());
            return;
        }
        Err(e) => panic!("Failed to load goto gold fixtures: {e}"),
    };

    let server = TestServerBuilder::new().build();

    let mut total = 0usize;
    let mut passed = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for fixture in &fixtures {
        let code = std::fs::read_to_string(&fixture.fixture_path).unwrap_or_else(|e| {
            panic!("Cannot read fixture {}: {e}", fixture.fixture_path.display())
        });

        let uri = format!("file:///gold/{}.pl", fixture.name);
        server.open_document(&uri, &code);

        for assertion in &fixture.goto_assertions {
            total += 1;
            let resp = server.get_definition(&uri, assertion.line, assertion.character);
            let def_line = first_goto_line(&resp);

            let ok = match &assertion.kind {
                GotoAssertionKind::GotoNonNull => def_line.is_some(),
                GotoAssertionKind::GotoNull => def_line.is_none(),
                GotoAssertionKind::GotoLine { expected_line } => def_line == Some(*expected_line),
            };

            if ok {
                passed += 1;
            } else {
                failures.push(format!(
                    "  FAIL [{}] {:?} at line:{} char:{} — got def_line: {:?}",
                    fixture.name, assertion.kind, assertion.line, assertion.character, def_line
                ));
            }
        }
    }

    println!(
        "\nGoto gold corpus: {}/{} assertions passed ({:.0}%)",
        passed,
        total,
        if total > 0 { passed as f64 / total as f64 * 100.0 } else { 100.0 }
    );
    for f in &failures {
        println!("{f}");
    }

    assert!(
        failures.is_empty(),
        "Goto gold corpus: {} assertion(s) failed out of {}:\n{}",
        failures.len(),
        total,
        failures.join("\n")
    );
}

// ---------------------------------------------------------------------------
// Completion relevance test
// ---------------------------------------------------------------------------

/// Run all completion gold fixtures and assert every assertion passes.
/// Reports top-1 accuracy, top-5 accuracy, non-empty rate, and noise-free rate.
#[test]
fn test_completion_gold_corpus() {
    let root = gold_corpus_root();
    let fixtures: Vec<CompletionGoldFixture> = match load_completion_gold_fixtures(&root) {
        Ok(f) if !f.is_empty() => f,
        Ok(_) => {
            eprintln!("SKIP: no completion gold fixtures found in {}", root.display());
            return;
        }
        Err(e) => panic!("Failed to load completion gold fixtures: {e}"),
    };

    let server = TestServerBuilder::new().build();

    let mut total = 0usize;
    let mut passed = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for fixture in &fixtures {
        let code = std::fs::read_to_string(&fixture.fixture_path).unwrap_or_else(|e| {
            panic!("Cannot read fixture {}: {e}", fixture.fixture_path.display())
        });

        let uri = format!("file:///gold/{}.pl", fixture.name);
        server.open_document(&uri, &code);

        for assertion in &fixture.completion_assertions {
            total += 1;
            let resp = server.get_completion(&uri, assertion.line, assertion.character);
            let labels = completion_labels_from_response(&resp);

            let ok = match &assertion.kind {
                CompletionAssertionKind::CompletionNonEmpty => !labels.is_empty(),
                CompletionAssertionKind::CompletionTop1 { expected_label } => {
                    labels.first().map(|l| l.as_str()) == Some(expected_label.as_str())
                }
                CompletionAssertionKind::CompletionTop5 { expected_label } => {
                    labels.iter().take(5).any(|l| l == expected_label)
                }
                CompletionAssertionKind::CompletionPresent { expected_label } => {
                    labels.iter().any(|l| l == expected_label)
                }
                CompletionAssertionKind::CompletionNoiseAbsent { forbidden_label } => {
                    !labels.iter().any(|l| l == forbidden_label)
                }
            };

            if ok {
                passed += 1;
            } else {
                failures.push(format!(
                    "  FAIL [{}] {:?} at line:{} char:{} — labels: {:?}",
                    fixture.name,
                    assertion.kind,
                    assertion.line,
                    assertion.character,
                    &labels[..labels.len().min(10)]
                ));
            }
        }
    }

    println!(
        "\nCompletion gold corpus: {}/{} assertions passed ({:.0}%)",
        passed,
        total,
        if total > 0 { passed as f64 / total as f64 * 100.0 } else { 100.0 }
    );
    for f in &failures {
        println!("{f}");
    }

    assert!(
        failures.is_empty(),
        "Completion gold corpus: {} assertion(s) failed out of {}:\n{}",
        failures.len(),
        total,
        failures.join("\n")
    );
}

// ---------------------------------------------------------------------------
<<<<<<< HEAD
// Diagnostics correctness test
// ---------------------------------------------------------------------------

/// Run all diagnostics gold fixtures (`expected.json`) and assert every assertion passes.
#[test]
fn test_diagnostics_gold_corpus() {
    let root = gold_corpus_root();
    let fixtures: Vec<GoldFixture> = match load_gold_fixtures(&root) {
        Ok(f) if !f.is_empty() => f,
        Ok(_) => {
            eprintln!("SKIP: no diagnostics gold fixtures found in {}", root.display());
            return;
        }
        Err(e) => panic!("Failed to load diagnostics gold fixtures: {e}"),
=======
// Document symbols correctness test
// ---------------------------------------------------------------------------

/// Run all document-symbol gold fixtures and assert every assertion passes.
#[test]
fn test_document_symbols_gold_corpus() {
    let root = gold_corpus_root();
    let fixtures: Vec<DocumentSymbolGoldFixture> = match load_document_symbol_gold_fixtures(&root) {
        Ok(f) if !f.is_empty() => f,
        Ok(_) => {
            eprintln!("SKIP: no document-symbol gold fixtures found in {}", root.display());
            return;
        }
        Err(e) => panic!("Failed to load document-symbol gold fixtures: {e}"),
>>>>>>> 0fe556b31 (test(editor-intelligence): add document-symbol gold fixture lane (#0000))
    };

    let server = TestServerBuilder::new().build();

    let mut total = 0usize;
    let mut passed = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for fixture in &fixtures {
        let code = std::fs::read_to_string(&fixture.fixture_path).unwrap_or_else(|e| {
            panic!("Cannot read fixture {}: {e}", fixture.fixture_path.display())
        });

        let uri = format!("file:///gold/{}.pl", fixture.name);
        server.open_document(&uri, &code);
<<<<<<< HEAD

        let diagnostics = server.get_diagnostics(&uri);
        let diagnostic_codes = diagnostic_codes_from_response(&diagnostics);

        for assertion in &fixture.expected.diagnostics {
            total += 1;
            let ok = match assertion {
                GoldAssertion::NoDiagnostics => diagnostic_codes.is_empty(),
                GoldAssertion::NoDiagnostic { code } => !diagnostic_codes.iter().any(|c| c == code),
                GoldAssertion::DiagnosticPresent { code, message_contains, .. } => {
                    let has_code = diagnostic_codes.iter().any(|c| c == code);
                    if !has_code {
                        false
                    } else if let Some(needle) = message_contains {
                        diagnostics["result"]["items"].as_array().is_some_and(|items| {
                            items.iter().any(|item| {
                                // Use the same normalization as diagnostic_codes_from_response:
                                // accept both string and integer coded diagnostics.
                                let item_code = item.get("code").and_then(|v| {
                                    v.as_str()
                                        .map(ToString::to_string)
                                        .or_else(|| v.as_i64().map(|n| n.to_string()))
                                });
                                let item_code_matches =
                                    item_code.as_deref().is_some_and(|c| c == code);
                                let message_matches = item["message"]
                                    .as_str()
                                    .is_some_and(|message| message.contains(needle));
                                item_code_matches && message_matches
                            })
                        })
                    } else {
                        true
                    }
                }
                GoldAssertion::DiagnosticCount { code, count } => {
                    diagnostic_codes.iter().filter(|c| *c == code).count() == *count
                }
=======
        let resp = server.get_symbols(&uri);
        let names = document_symbol_names(&resp);

        for assertion in &fixture.symbol_assertions {
            total += 1;
            let ok = match &assertion.kind {
                DocumentSymbolAssertionKind::SymbolNonEmpty => !names.is_empty(),
                DocumentSymbolAssertionKind::SymbolPresent { name } => {
                    names.iter().any(|candidate| candidate == name)
                }
                DocumentSymbolAssertionKind::SymbolAbsent { name } => {
                    names.iter().all(|candidate| candidate != name)
                }
                DocumentSymbolAssertionKind::SymbolCount { count } => names.len() == *count,
>>>>>>> 0fe556b31 (test(editor-intelligence): add document-symbol gold fixture lane (#0000))
            };

            if ok {
                passed += 1;
            } else {
                failures.push(format!(
<<<<<<< HEAD
                    "  FAIL [{}] {:?} — diagnostic codes: {:?}",
                    fixture.name, assertion, diagnostic_codes
=======
                    "  FAIL [{}] {:?} — symbols: {:?}",
                    fixture.name, assertion.kind, names
>>>>>>> 0fe556b31 (test(editor-intelligence): add document-symbol gold fixture lane (#0000))
                ));
            }
        }
    }

    println!(
<<<<<<< HEAD
        "\nDiagnostics gold corpus: {}/{} assertions passed ({:.0}%)",
=======
        "\nDocument symbols gold corpus: {}/{} assertions passed ({:.0}%)",
>>>>>>> 0fe556b31 (test(editor-intelligence): add document-symbol gold fixture lane (#0000))
        passed,
        total,
        if total > 0 { passed as f64 / total as f64 * 100.0 } else { 100.0 }
    );
    for f in &failures {
        println!("{f}");
    }

    assert!(
        failures.is_empty(),
<<<<<<< HEAD
        "Diagnostics gold corpus: {} assertion(s) failed out of {}:\n{}",
=======
        "Document symbols gold corpus: {} assertion(s) failed out of {}:\n{}",
>>>>>>> 0fe556b31 (test(editor-intelligence): add document-symbol gold fixture lane (#0000))
        failures.len(),
        total,
        failures.join("\n")
    );
}
