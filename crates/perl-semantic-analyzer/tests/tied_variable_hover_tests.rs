//! Tests for tied variable magic method builtin documentation.
//!
//! Verifies that `get_builtin_documentation` returns entries for all
//! standard Perl tie magic methods (TIESCALAR, TIEARRAY, TIEHASH,
//! TIEHANDLE, FETCH, STORE, FIRSTKEY, NEXTKEY, DESTROY).

use perl_semantic_analyzer::analysis::semantic::get_builtin_documentation;

/// All tie magic methods must have documentation entries.
#[test]
fn test_tie_magic_methods_have_docs() {
    let magic_methods = [
        "TIESCALAR",
        "TIEARRAY",
        "TIEHASH",
        "TIEHANDLE",
        "FETCH",
        "STORE",
        "FIRSTKEY",
        "NEXTKEY",
        "DESTROY",
    ];
    for method in &magic_methods {
        assert!(
            get_builtin_documentation(method).is_some(),
            "Missing builtin docs for tie magic method: {method}"
        );
    }
}

/// Each documented magic method must have a non-empty signature and description.
#[test]
fn test_tie_magic_method_docs_are_non_empty() -> Result<(), Box<dyn std::error::Error>> {
    let magic_methods = [
        "TIESCALAR",
        "TIEARRAY",
        "TIEHASH",
        "TIEHANDLE",
        "FETCH",
        "STORE",
        "FIRSTKEY",
        "NEXTKEY",
        "DESTROY",
    ];
    for method in &magic_methods {
        let doc = get_builtin_documentation(method)
            .ok_or_else(|| format!("Missing docs for {method}"))?;
        assert!(!doc.signature.is_empty(), "signature should be non-empty for {method}");
        assert!(!doc.description.is_empty(), "description should be non-empty for {method}");
    }
    Ok(())
}
