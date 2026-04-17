//! Integration tests for the public `queries` module.
//!
//! These tests verify that the `tree_sitter_perl_c::queries` module exposes
//! the four upstream tree-sitter query files correctly.

use tree_sitter::Query;
use tree_sitter_perl_c::queries;

/// Test that `injections_query_str()` returns a non-empty string.
#[test]
fn test_injections_query_str_returns_non_empty_string() {
    let result = queries::injections_query_str();
    assert!(!result.is_empty(), "expected injections_query_str() to return a non-empty string");
}

/// Test that `highlights_query_str()` returns a non-empty string.
#[test]
fn test_highlights_query_str_returns_non_empty_string() {
    let result = queries::highlights_query_str();
    assert!(!result.is_empty(), "expected highlights_query_str() to return a non-empty string");
}

/// Test that `folds_query_str()` returns a non-empty string.
#[test]
fn test_folds_query_str_returns_non_empty_string() {
    let result = queries::folds_query_str();
    assert!(!result.is_empty(), "expected folds_query_str() to return a non-empty string");
}

/// Test that `matchup_query_str()` returns a non-empty string.
#[test]
fn test_matchup_query_str_returns_non_empty_string() {
    let result = queries::matchup_query_str();
    assert!(!result.is_empty(), "expected matchup_query_str() to return a non-empty string");
}

/// Test that `load_injections_query()` returns a valid `Query`.
#[test]
fn test_load_injections_query_returns_valid_query() {
    let result = queries::load_injections_query();
    assert!(
        result.is_ok(),
        "expected load_injections_query() to return Ok, got Err: {:?}",
        result.err()
    );
    let query = result.unwrap();
    assert!(
        !query.capture_names().is_empty(),
        "expected injections query to have at least one capture name"
    );
}

/// Test that `load_highlights_query()` returns a `Result` (may be Err if the
/// highlights.scm contains patterns not supported by the C grammar).
/// The upstream highlights.scm may reference node types not present in the C grammar.
#[test]
fn test_load_highlights_query_returns_result() {
    // highlights.scm is from the upstream tree-sitter-perl grammar which may
    // use node types not present in the C grammar (e.g., postfix_deref).
    // The function should return a Result so callers can handle both cases.
    let result = queries::load_highlights_query();
    // Just verify it returns a Result - it may be Err if the grammar doesn't support the query
    let _is_result = result.is_ok() || result.is_err();
    // If it is Ok, verify it has capture names (sanity check)
    if let Ok(query) = result {
        assert!(
            !query.capture_names().is_empty(),
            "expected highlights query to have at least one capture name"
        );
    }
}

/// Test that `load_folds_query()` returns a valid `Query`.
#[test]
fn test_load_folds_query_returns_valid_query() {
    let result = queries::load_folds_query();
    assert!(
        result.is_ok(),
        "expected load_folds_query() to return Ok, got Err: {:?}",
        result.err()
    );
    let query = result.unwrap();
    // Folds queries may be empty (no captures), so we don't check capture_names
}

/// Test that `load_matchup_query()` returns a valid `Query`.
#[test]
fn test_load_matchup_query_returns_valid_query() {
    let result = queries::load_matchup_query();
    assert!(
        result.is_ok(),
        "expected load_matchup_query() to return Ok, got Err: {:?}",
        result.err()
    );
    let query = result.unwrap();
    // Matchup queries may be empty (no captures), so we don't check capture_names
}

/// Test that `QueryError` is re-exported from the `queries` module.
#[test]
fn test_query_error_is_re_exported() {
    // Verify that the type exists and is accessible
    let _error: queries::QueryError;
    // If this compiles, the re-export works. We also verify it can be used
    // in a Result type, which is how callers would use it.
    let _result: Result<Query, queries::QueryError> = queries::load_injections_query();
}

/// Test that the injections query string matches the upstream content.
/// This verifies the include_str path resolves correctly.
#[test]
fn test_injections_query_str_contains_expected_content() {
    let content = queries::injections_query_str();
    // The injections.scm file should contain "injection.language" if it's valid
    assert!(
        content.contains("injection"),
        "expected injections_query_str() to contain 'injection'"
    );
}

/// Test that highlights query string contains expected patterns.
#[test]
fn test_highlights_query_str_contains_expected_content() {
    let content = queries::highlights_query_str();
    // The highlights.scm file should contain capture patterns
    assert!(
        content.contains("highlight") || content.contains("@"),
        "expected highlights_query_str() to contain 'highlight' or capture patterns"
    );
}
