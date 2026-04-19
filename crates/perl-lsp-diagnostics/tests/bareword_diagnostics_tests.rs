//! Tests for bareword-as-string coercion warnings (issue #2365).
//!
//! Covers:
//! - Bareword under `use strict` is flagged with `unquoted-bareword` code
//! - The diagnostic has a concrete auto-quote suggestion (not None)
//! - The message explains the problem and names the bareword
//! - Severity is Error under strict
//! - Hash-key barewords are NOT flagged (they are auto-quoted by Perl)
//! - Related information includes quoting guidance

use std::sync::Arc;

use perl_lsp_diagnostics::{Diagnostic, DiagnosticSeverity, DiagnosticsProvider};
use perl_parser::Parser;

// ---------------------------------------------------------------------------
// Helper: parse Perl source and return all diagnostics
// ---------------------------------------------------------------------------
fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let output = Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &output.diagnostics, source, None)
}

fn bareword_diags(source: &str) -> Vec<Diagnostic> {
    diagnostics_for(source)
        .into_iter()
        .filter(|d| d.code.as_deref() == Some("PL109"))
        .collect()
}

// ---------------------------------------------------------------------------
// 1. Bareword under use strict is flagged
// ---------------------------------------------------------------------------

#[test]
fn bareword_under_strict_is_flagged() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use strict;\nuse warnings;\nmy $x = FOO;\n";
    let diags = bareword_diags(source);
    assert!(
        !diags.is_empty(),
        "expected at least one unquoted-bareword diagnostic"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 2. Diagnostic code is 'unquoted-bareword'
// ---------------------------------------------------------------------------

#[test]
fn bareword_diagnostic_has_correct_code() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use strict;\nuse warnings;\nmy $x = FOO;\n";
    let diags = bareword_diags(source);
    assert!(
        diags.iter().all(|d| d.code.as_deref() == Some("PL109")),
        "all bareword diagnostics should have stable code PL109"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 3. Severity is Error under strict
// ---------------------------------------------------------------------------

#[test]
fn bareword_under_strict_severity_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use strict;\nuse warnings;\nmy $x = FOO;\n";
    let diags = bareword_diags(source);
    assert!(!diags.is_empty(), "should have a bareword diagnostic");
    assert!(
        diags
            .iter()
            .all(|d| d.severity == DiagnosticSeverity::Error),
        "bareword under strict should have Error severity"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 4. Message mentions the bareword name
// ---------------------------------------------------------------------------

#[test]
fn bareword_diagnostic_message_names_the_bareword() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use strict;\nuse warnings;\nmy $x = FOO;\n";
    let diags = bareword_diags(source);
    assert!(!diags.is_empty(), "should have a bareword diagnostic");
    let msg = &diags[0].message;
    assert!(
        msg.contains("FOO"),
        "message should name the bareword 'FOO', got: {msg}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 5. Message explains strict context
// ---------------------------------------------------------------------------

#[test]
fn bareword_diagnostic_message_explains_strict() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use strict;\nuse warnings;\nmy $x = FOO;\n";
    let diags = bareword_diags(source);
    assert!(!diags.is_empty(), "should have a bareword diagnostic");
    let msg = &diags[0].message;
    assert!(
        msg.to_lowercase().contains("strict") || msg.to_lowercase().contains("bare"),
        "message should mention strict or bareword context, got: {msg}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 6. Diagnostic has a suggestion for quoting the bareword
// ---------------------------------------------------------------------------

#[test]
fn bareword_diagnostic_has_auto_quote_suggestion() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use strict;\nuse warnings;\nmy $x = FOO;\n";
    let diags = bareword_diags(source);
    assert!(!diags.is_empty(), "should have a bareword diagnostic");
    let suggestion = diags[0].suggestion.as_deref();
    assert!(
        suggestion.is_some(),
        "bareword diagnostic should have a suggestion"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 7. Suggestion text mentions quoting with single quotes or double quotes
// ---------------------------------------------------------------------------

#[test]
fn bareword_suggestion_mentions_quoting() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use strict;\nuse warnings;\nmy $x = FOO;\n";
    let diags = bareword_diags(source);
    assert!(!diags.is_empty(), "should have a bareword diagnostic");
    let suggestion = diags[0].suggestion.as_deref().unwrap_or("");
    assert!(
        suggestion.contains('\'')
            || suggestion.contains('"')
            || suggestion.to_lowercase().contains("quot"),
        "suggestion should mention quoting (e.g., 'FOO' or \"FOO\"), got: {suggestion}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 8. Related information includes quoting guidance
// ---------------------------------------------------------------------------

#[test]
fn bareword_diagnostic_has_related_info() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use strict;\nuse warnings;\nmy $x = FOO;\n";
    let diags = bareword_diags(source);
    assert!(!diags.is_empty(), "should have a bareword diagnostic");
    assert!(
        !diags[0].related_information.is_empty(),
        "bareword diagnostic should have related information with quoting guidance"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 9. Hash key barewords are NOT flagged (Perl auto-quotes them)
// ---------------------------------------------------------------------------

#[test]
fn hash_key_bareword_is_not_flagged() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use strict;\nuse warnings;\nmy %h = ();\nmy $v = $h{foo};\n";
    let diags = bareword_diags(source);
    assert!(
        diags.is_empty(),
        "hash key bareword should not be flagged (Perl auto-quotes hash keys), got: {diags:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 10. Multiple barewords in one file each get their own diagnostic
// ---------------------------------------------------------------------------

#[test]
fn multiple_barewords_each_get_diagnostic() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use strict;\nuse warnings;\nmy $x = FOO;\nmy $y = BAR;\n";
    let diags = bareword_diags(source);
    assert!(
        diags.len() >= 2,
        "expected at least 2 bareword diagnostics for FOO and BAR, got: {}",
        diags.len()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 11. Suggestion text includes the bareword name for easy replacement
// ---------------------------------------------------------------------------

#[test]
fn bareword_suggestion_includes_bareword_name() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use strict;\nuse warnings;\nmy $x = MYCONST;\n";
    let diags = bareword_diags(source);
    assert!(!diags.is_empty(), "should have a bareword diagnostic");
    let suggestion = diags[0].suggestion.as_deref().unwrap_or("");
    assert!(
        suggestion.contains("MYCONST"),
        "suggestion should include the bareword name 'MYCONST' for easy copy-paste, got: {suggestion}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 12. No bareword diagnostic when not under strict
// ---------------------------------------------------------------------------

#[test]
fn bareword_without_strict_is_not_flagged() -> Result<(), Box<dyn std::error::Error>> {
    // Without use strict, barewords should not generate unquoted-bareword diagnostics
    let source = "my $x = FOO;\n";
    let diags = bareword_diags(source);
    assert!(
        diags.is_empty(),
        "bareword should not be flagged without use strict, got: {diags:?}"
    );
    Ok(())
}
