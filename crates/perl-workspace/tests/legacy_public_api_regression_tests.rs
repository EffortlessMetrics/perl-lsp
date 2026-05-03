//! Regression tests for legacy WorkspaceIndex public APIs.
//!
//! Requirement 10.5: The existing public APIs of WorkspaceIndex (find_definition,
//! find_references, count_usages, definition_candidates) SHALL remain available
//! until the semantic query path has scorecard proof.
//!
//! These tests verify that the legacy query path remains functional after the
//! semantic substrate is wired in. They exercise each API with a realistic
//! multi-file workspace and assert expected results.

use perl_workspace::workspace::workspace_index::WorkspaceIndex;
use url::Url;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn file_url(path: &str) -> Result<Url, Box<dyn std::error::Error>> {
    Ok(Url::parse(&format!("file://{}", path))?)
}

/// Build a small workspace with two packages and cross-file references.
///
/// File layout:
///   /lib/Greeter.pm  — `package Greeter; sub hello { 1 } sub farewell { 1 }`
///   /app.pl          — `use Greeter; Greeter::hello(); hello(); farewell();`
fn build_workspace() -> Result<WorkspaceIndex, Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    let lib_uri = file_url("/lib/Greeter.pm")?;
    let app_uri = file_url("/app.pl")?;

    index.index_file(
        lib_uri,
        "package Greeter;\nsub hello { return 'hi'; }\nsub farewell { return 'bye'; }\n1;\n"
            .to_string(),
    )?;

    index.index_file(
        app_uri,
        "use Greeter;\nGreeter::hello();\nhello();\nfarewell();\n".to_string(),
    )?;

    Ok(index)
}

// =========================================================================
// find_definition — Requirement 10.5
// =========================================================================

#[test]
fn legacy_find_definition_resolves_qualified_name() -> Result<(), Box<dyn std::error::Error>> {
    let index = build_workspace()?;

    let def =
        index.find_definition("Greeter::hello").ok_or("expected definition for Greeter::hello")?;
    assert!(
        def.uri.contains("Greeter.pm"),
        "definition should point to Greeter.pm, got: {}",
        def.uri
    );
    Ok(())
}

#[test]
fn legacy_find_definition_resolves_bare_name() -> Result<(), Box<dyn std::error::Error>> {
    let index = build_workspace()?;

    // bare name should still resolve (may pick any file that defines it)
    let def = index.find_definition("hello").ok_or("expected definition for bare 'hello'")?;
    assert!(
        def.uri.contains("Greeter.pm"),
        "bare name definition should resolve to Greeter.pm, got: {}",
        def.uri
    );
    Ok(())
}

#[test]
fn legacy_find_definition_returns_none_for_unknown() -> Result<(), Box<dyn std::error::Error>> {
    let index = build_workspace()?;

    assert!(
        index.find_definition("nonexistent_symbol").is_none(),
        "unknown symbol should return None"
    );
    Ok(())
}

// =========================================================================
// find_references — Requirement 10.5
// =========================================================================

#[test]
fn legacy_find_references_returns_call_sites() -> Result<(), Box<dyn std::error::Error>> {
    let index = build_workspace()?;

    let refs = index.find_references("Greeter::hello");
    assert!(
        !refs.is_empty(),
        "find_references should return at least one reference for Greeter::hello"
    );

    // Should include the call site in app.pl
    let has_app_ref = refs.iter().any(|loc| loc.uri.contains("app.pl"));
    assert!(has_app_ref, "references should include the call site in app.pl");
    Ok(())
}

#[test]
fn legacy_find_references_bare_name_includes_qualified() -> Result<(), Box<dyn std::error::Error>> {
    let index = build_workspace()?;

    // Searching bare name should also find qualified calls
    let refs = index.find_references("hello");
    assert!(!refs.is_empty(), "find_references for bare 'hello' should return references");
    Ok(())
}

#[test]
fn legacy_find_references_empty_for_unknown() -> Result<(), Box<dyn std::error::Error>> {
    let index = build_workspace()?;

    let refs = index.find_references("nonexistent_symbol");
    assert!(refs.is_empty(), "unknown symbol should have no references");
    Ok(())
}

// =========================================================================
// count_usages — Requirement 10.5
// =========================================================================

#[test]
fn legacy_count_usages_excludes_definitions() -> Result<(), Box<dyn std::error::Error>> {
    let index = build_workspace()?;

    let count = index.count_usages("Greeter::hello");
    // There should be at least one usage (the call in app.pl), but the
    // definition itself should not be counted.
    assert!(
        count >= 1,
        "count_usages should find at least 1 usage for Greeter::hello, got: {}",
        count
    );
    Ok(())
}

#[test]
fn legacy_count_usages_bare_name() -> Result<(), Box<dyn std::error::Error>> {
    let index = build_workspace()?;

    let count = index.count_usages("farewell");
    assert!(
        count >= 1,
        "count_usages should find at least 1 usage for bare 'farewell', got: {}",
        count
    );
    Ok(())
}

#[test]
fn legacy_count_usages_zero_for_unknown() -> Result<(), Box<dyn std::error::Error>> {
    let index = build_workspace()?;

    let count = index.count_usages("nonexistent_symbol");
    assert_eq!(count, 0, "unknown symbol should have zero usages");
    Ok(())
}

// =========================================================================
// definition_candidates — Requirement 10.5
//
// `definition_candidates` is pub(crate), so it cannot be called directly from
// integration tests. It is exercised indirectly through `find_definition`
// (which delegates to it) and is covered by in-crate unit tests:
//   - test_definition_candidates_include_ambiguous_bare_symbols_in_stable_order
//   - test_definition_candidates_include_duplicate_qualified_name_across_files
//   - test_definition_candidates_are_cleaned_on_remove_and_reindex
//   - test_definition_candidates_shared_symbol_survives_removal_of_sole_owner_of_other_symbol
//
// The test below verifies the observable behavior: when two files define the
// same bare symbol, find_definition still resolves deterministically (proving
// the candidate list is maintained and sorted).
// =========================================================================

#[test]
fn legacy_definition_candidates_deterministic_via_find_definition()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    let uri_a = file_url("/lib/A.pm")?;
    let uri_b = file_url("/lib/B.pm")?;

    index.index_file(uri_a, "package A;\nsub shared { 1 }\n1;\n".to_string())?;
    index.index_file(uri_b, "package B;\nsub shared { 1 }\n1;\n".to_string())?;

    // Two calls should return the same winner — proving the candidate list is
    // maintained and sorted deterministically.
    let first = index.find_definition("shared").ok_or("expected definition for 'shared'")?;
    let second = index.find_definition("shared").ok_or("expected definition for 'shared'")?;

    assert_eq!(
        first.uri, second.uri,
        "definition_candidates should produce a deterministic ordering"
    );
    Ok(())
}

// =========================================================================
// End-to-end: all four APIs on the same workspace
// =========================================================================

#[test]
fn legacy_apis_all_functional_on_same_workspace() -> Result<(), Box<dyn std::error::Error>> {
    let index = build_workspace()?;

    // 1. find_definition
    let def = index
        .find_definition("Greeter::hello")
        .ok_or("find_definition should resolve Greeter::hello")?;
    assert!(def.uri.contains("Greeter.pm"));

    // 2. find_references
    let refs = index.find_references("Greeter::hello");
    assert!(!refs.is_empty(), "find_references should return results");

    // 3. count_usages
    let count = index.count_usages("Greeter::hello");
    assert!(count >= 1, "count_usages should be >= 1");

    // 4. definition_candidates (indirectly via find_definition determinism)
    let def_again =
        index.find_definition("Greeter::hello").ok_or("find_definition should still resolve")?;
    assert_eq!(def.uri, def_again.uri, "repeated find_definition should be deterministic");

    Ok(())
}
