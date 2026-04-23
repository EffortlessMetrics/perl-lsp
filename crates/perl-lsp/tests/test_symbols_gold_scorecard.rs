//! Editor Intelligence Scorecard — Document Symbols Gold Corpus Tests (RED)
//!
//! These tests define what the document symbols scorecard must look like.
//! They FAIL until the implementation provides:
//!   - `SymbolsGoldFixture` struct in `perl-corpus/src/gold.rs`
//!   - `SymbolsAssertionKind` enum with assertion variants
//!   - `load_symbols_gold_fixtures()` function
//!   - Gold fixtures in `test_corpus/gold/<name>/expected_symbols.json`
//!
//! ## What Correct Behavior Looks Like
//!
//! 1. `SymbolsNonEmpty` — document symbols returns non-empty list for files with symbols
//! 2. `SymbolsContains { name, kind }` — expected symbol present with correct name
//! 3. `SymbolsKindMatches { name, expected_kind }` — symbol has expected LSP SymbolKind
//! 4. `SymbolsCountAtLeast { min }` — document has at least N top-level symbols
//! 5. `SymbolsPackagePresent { name }` — package declaration appears as a symbol

mod common;

use common::test_utils::TestServerBuilder;
use perl_corpus::gold::{SymbolsAssertionKind, SymbolsGoldFixture, load_symbols_gold_fixtures};
use serde_json::Value;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn gold_corpus_root() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let crate_dir = PathBuf::from(manifest);
    let workspace_root = crate_dir
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| crate_dir.clone());
    workspace_root.join("test_corpus").join("gold")
}

/// LSP SymbolKind values (from LSP spec)
mod symbol_kind {
    pub const FILE: i64 = 1;
    pub const MODULE: i64 = 2;
    pub const NAMESPACE: i64 = 3;
    pub const PACKAGE: i64 = 4;
    pub const CLASS: i64 = 5;
    pub const METHOD: i64 = 6;
    pub const PROPERTY: i64 = 7;
    pub const FIELD: i64 = 8;
    pub const CONSTRUCTOR: i64 = 9;
    pub const ENUM: i64 = 10;
    pub const INTERFACE: i64 = 11;
    pub const FUNCTION: i64 = 12;
    pub const VARIABLE: i64 = 13;
    pub const CONSTANT: i64 = 14;
    pub const STRING: i64 = 15;
    pub const NUMBER: i64 = 16;
    pub const BOOLEAN: i64 = 17;
    pub const ARRAY: i64 = 18;
    pub const OBJECT: i64 = 19;
}

fn symbols_from_response(resp: &Value) -> Vec<Value> {
    resp.get("result").and_then(|r| r.as_array()).cloned().unwrap_or_default()
}

fn symbol_names(symbols: &[Value]) -> Vec<String> {
    symbols
        .iter()
        .filter_map(|s| s.get("name").and_then(|n| n.as_str()).map(String::from))
        .collect()
}

fn find_symbol_by_name<'a>(symbols: &'a [Value], name: &str) -> Option<&'a Value> {
    symbols
        .iter()
        .find(|s| s.get("name").and_then(|n| n.as_str()).map(|n| n == name).unwrap_or(false))
}

// ---------------------------------------------------------------------------
// Document Symbols Gold Corpus Test
// ---------------------------------------------------------------------------

/// Run all document symbols gold fixtures and assert every assertion passes.
///
/// This test will FAIL until `SymbolsGoldFixture`, `SymbolsAssertionKind`,
/// and `load_symbols_gold_fixtures()` exist in `perl-corpus/src/gold.rs`.
#[test]
fn test_symbols_gold_corpus() {
    let root = gold_corpus_root();
    let fixtures: Vec<SymbolsGoldFixture> = match load_symbols_gold_fixtures(&root) {
        Ok(f) => {
            // RED TEST: fixtures MUST exist for a valid scorecard
            // If this assertion fails, it means code-builder has not yet
            // created gold fixtures for symbols testing.
            assert!(
                !f.is_empty(),
                "RED TEST FAILURE: No symbols gold fixtures found in {}. \
                Code-builder must create expected_symbols.json fixtures in test_corpus/gold/<name>/.",
                root.display()
            );
            f
        }
        Err(e) => panic!("Failed to load symbols gold fixtures: {e}"),
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

        let resp = server.get_symbols(&uri);
        let symbols = symbols_from_response(&resp);

        for assertion in &fixture.symbols_assertions {
            total += 1;

            let ok = match &assertion.kind {
                SymbolsAssertionKind::SymbolsNonEmpty => !symbols.is_empty(),

                SymbolsAssertionKind::SymbolsEmpty => symbols.is_empty(),

                SymbolsAssertionKind::SymbolsContains { name } => symbols.iter().any(|s| {
                    s.get("name").and_then(|n| n.as_str()).map(|n| n == name).unwrap_or(false)
                }),

                SymbolsAssertionKind::SymbolsAbsent { name } => !symbols.iter().any(|s| {
                    s.get("name").and_then(|n| n.as_str()).map(|n| n == name).unwrap_or(false)
                }),

                SymbolsAssertionKind::SymbolsKindMatches { name, expected_kind } => {
                    find_symbol_by_name(&symbols, name)
                        .map(|s| s.get("kind").and_then(|k| k.as_i64()) == Some(*expected_kind))
                        .unwrap_or(false)
                }

                SymbolsAssertionKind::SymbolsCountAtLeast { min } => symbols.len() >= *min,

                SymbolsAssertionKind::SymbolsCountAtMost { max } => symbols.len() <= *max,

                SymbolsAssertionKind::SymbolsPackagePresent { name } => {
                    // Package symbols have kind 4 (Package) or 2 (Module)
                    symbols.iter().any(|s| {
                        s.get("name").and_then(|n| n.as_str()).map(|n| n == name).unwrap_or(false)
                            && s.get("kind")
                                .and_then(|k| k.as_i64())
                                .map(|k| k == symbol_kind::PACKAGE || k == symbol_kind::MODULE)
                                .unwrap_or(false)
                    })
                }

                SymbolsAssertionKind::SymbolsSubroutinePresent { name } => {
                    // Subroutine/function symbols have kind 12 (Function)
                    symbols.iter().any(|s| {
                        s.get("name").and_then(|n| n.as_str()).map(|n| n == name).unwrap_or(false)
                            && s.get("kind")
                                .and_then(|k| k.as_i64())
                                .map(|k| k == symbol_kind::FUNCTION)
                                .unwrap_or(false)
                    })
                }

                SymbolsAssertionKind::SymbolsVariablePresent { name } => {
                    // Variable symbols have kind 13 (Variable)
                    symbols.iter().any(|s| {
                        s.get("name").and_then(|n| n.as_str()).map(|n| n == name).unwrap_or(false)
                            && s.get("kind")
                                .and_then(|k| k.as_i64())
                                .map(|k| k == symbol_kind::VARIABLE)
                                .unwrap_or(false)
                    })
                }
            };

            if ok {
                passed += 1;
            } else {
                let names: Vec<String> = symbol_names(&symbols);
                failures.push(format!(
                    "  FAIL [{}] {:?} — got symbols: {:?}",
                    fixture.name,
                    assertion.kind,
                    &names[..names.len().min(20)]
                ));
            }
        }
    }

    println!(
        "\nDocument symbols gold corpus: {}/{} assertions passed ({:.0}%)",
        passed,
        total,
        if total > 0 { passed as f64 / total as f64 * 100.0 } else { 100.0 }
    );
    for f in &failures {
        println!("{f}");
    }

    assert!(
        failures.is_empty(),
        "Document symbols gold corpus: {} assertion(s) failed out of {}:\n{}",
        failures.len(),
        total,
        failures.join("\n")
    );
}
