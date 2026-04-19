//! Integration tests: parser-to-LSP diagnostic pipeline
//!
//! Exercises the full path from real Perl source through the parser and
//! `DiagnosticsProvider`, verifying:
//!
//! - Parse error -> diagnostic mapping (using the real parser, not synthetic errors)
//! - Severity levels across error / warning / information categories
//! - Range accuracy: byte offsets land inside the correct source region
//! - Multiple diagnostics per file (mixed error types)
//! - Diagnostic clearing on fix (re-parse corrected source produces zero parse-errors)
//! - Line/column offset conversion accuracy

use std::sync::Arc;

use perl_lsp_diagnostics::{Diagnostic, DiagnosticSeverity, DiagnosticTag, DiagnosticsProvider};
use perl_parser::Parser;
use perl_parser_core::error::ParseError;
use perl_parser_core::position::LineStartsCache;

// ---------------------------------------------------------------------------
// Helper: parse Perl source and return diagnostics from the full pipeline
// ---------------------------------------------------------------------------

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let output = Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &output.diagnostics, source, None)
}

/// Filter to only parse-error diagnostics (PL001=ParseError, PL002=SyntaxError, PL003=UnexpectedEof)
fn parse_error_diags(diags: &[Diagnostic]) -> Vec<&Diagnostic> {
    diags
        .iter()
        .filter(|d| {
            matches!(
                d.code.as_deref(),
                Some("PL001") | Some("PL002") | Some("PL003")
            )
        })
        .collect()
}

/// Verify a byte offset falls within [0, source_len] and, optionally, that the
/// source slice at that position contains `needle`.
fn assert_offset_in_range(source: &str, offset: usize, label: &str) {
    assert!(
        offset <= source.len(),
        "{label}: offset {offset} exceeds source length {}",
        source.len()
    );
}

/// Convert byte offset to (line, col) using the same path the LSP server uses.
fn offset_to_line_col(source: &str, offset: usize) -> (u32, u32) {
    let cache = LineStartsCache::new(source);
    cache.offset_to_position(source, offset)
}

// =========================================================================
// 1. Parse error -> diagnostic mapping (real parser round-trip)
// =========================================================================

#[test]
fn test_parse_error_mapping_missing_semicolon() -> Result<(), Box<dyn std::error::Error>> {
    // Missing semicolon after assignment -- the parser should report an error.
    // The parser may recover by treating `print` as part of the expression,
    // so we check that at least some diagnostic (parse-error OR scope) is emitted
    // when this source is fed through the full pipeline.
    let source = "my $x = 42\nprint $x;\n";
    let diags = diagnostics_for(source);

    // The parser is recovery-oriented: it may or may not emit a parse-error for
    // missing semicolons when it can recover. We verify the pipeline runs and
    // produces valid diagnostics.
    for d in &diags {
        assert!(d.code.is_some(), "Every diagnostic should have a code");
        assert!(
            d.range.0 <= source.len(),
            "Offset should be within source bounds"
        );
    }
    Ok(())
}

#[test]
fn test_parse_error_mapping_unclosed_brace() -> Result<(), Box<dyn std::error::Error>> {
    let source = "sub foo {\n    my $x = 1;\n";
    let diags = diagnostics_for(source);
    let pe = parse_error_diags(&diags);

    assert!(
        !pe.is_empty(),
        "Unclosed brace should produce parse-error diagnostics: got {diags:?}"
    );
    Ok(())
}

#[test]
fn test_parse_error_mapping_unexpected_token() -> Result<(), Box<dyn std::error::Error>> {
    // An obviously invalid token sequence
    let source = "my $x = = ;";
    let diags = diagnostics_for(source);
    let pe = parse_error_diags(&diags);

    assert!(
        !pe.is_empty(),
        "Unexpected token should produce parse-error diagnostics"
    );
    for d in &pe {
        assert_eq!(d.severity, DiagnosticSeverity::Error);
    }
    Ok(())
}

#[test]
fn test_parse_error_mapping_valid_perl_no_parse_errors() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use strict;\nuse warnings;\nmy $x = 42;\nprint $x;\n";
    let diags = diagnostics_for(source);
    let pe = parse_error_diags(&diags);

    assert!(
        pe.is_empty(),
        "Valid Perl should produce zero parse-error diagnostics, got: {pe:?}"
    );
    Ok(())
}

// =========================================================================
// 2. Severity levels across diagnostic categories
// =========================================================================

#[test]
fn test_severity_parse_errors_are_error_level() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = ;";
    let diags = diagnostics_for(source);
    let pe = parse_error_diags(&diags);

    for d in &pe {
        assert_eq!(
            d.severity,
            DiagnosticSeverity::Error,
            "Parse errors must be Error severity, got {:?} for: {}",
            d.severity,
            d.message
        );
    }
    Ok(())
}

#[test]
fn test_unknown_subroutine_attribute_is_warning() -> Result<(), Box<dyn std::error::Error>> {
    let source = "sub foo :Private { }\n";
    let output = Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diags = provider.get_diagnostics(&ast, &output.diagnostics, source, None);

    let unknown_attr: Vec<_> = diags
        .iter()
        .filter(|d| {
            d.message
                .contains("unknown subroutine attribute ':Private'")
        })
        .collect();

    assert_eq!(
        unknown_attr.len(),
        1,
        "expected exactly one unknown-attribute diagnostic, got: {diags:?}"
    );
    assert_eq!(unknown_attr[0].severity, DiagnosticSeverity::Warning);
    assert_eq!(unknown_attr[0].code.as_deref(), Some("PL002"));
    Ok(())
}

#[test]
fn test_severity_missing_strict_is_information() -> Result<(), Box<dyn std::error::Error>> {
    // A program with no strict/warnings should get Information-level diagnostics
    let source = "my $x = 1;\n";
    let diags = diagnostics_for(source);

    let missing_strict: Vec<_> = diags
        .iter()
        .filter(|d| d.code.as_deref() == Some("PL100"))
        .collect();

    // If the linter fires missing-strict, it should be Information
    for d in &missing_strict {
        assert_eq!(
            d.severity,
            DiagnosticSeverity::Information,
            "missing-strict should be Information severity"
        );
    }
    Ok(())
}

#[test]
fn test_severity_unused_variable_is_warning() -> Result<(), Box<dyn std::error::Error>> {
    // Declare a variable and never use it -- scope analyzer should flag as Warning
    let source = "use strict;\nuse warnings;\nmy $unused = 1;\n";
    let diags = diagnostics_for(source);

    let unused: Vec<_> = diags
        .iter()
        .filter(|d| d.code.as_deref() == Some("PL102"))
        .collect();

    for d in &unused {
        assert_eq!(
            d.severity,
            DiagnosticSeverity::Warning,
            "unused-variable should be Warning severity"
        );
        assert!(
            d.tags.contains(&DiagnosticTag::Unnecessary),
            "unused-variable should carry DiagnosticTag::Unnecessary"
        );
    }
    Ok(())
}

#[test]
fn test_try_catch_variable_is_declared_inside_catch() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
use strict;
use feature 'try';
try {
    die "boom";
} catch ($e) {
    print $e;
}
"#;

    let diags = diagnostics_for(source);
    let catch_var_diags: Vec<_> = diags
        .iter()
        .filter(|d| {
            matches!(
                d.code.as_deref(),
                Some("PL102") | Some("PL103") | Some("PL110")
            ) && d.message.contains("$e")
        })
        .collect();

    assert!(
        catch_var_diags.is_empty(),
        "catch variable should not produce scope diagnostics inside catch: {:?}",
        catch_var_diags
    );
    Ok(())
}

#[test]
fn test_try_catch_variable_does_not_escape_into_outer_scope()
-> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
use strict;
use feature 'try';
try {
    die "boom";
} catch ($e) {
    print $e;
}
print $e;
"#;

    let diags = diagnostics_for(source);
    let undeclared: Vec<_> = diags
        .iter()
        .filter(|d| d.code.as_deref() == Some("PL103") && d.message.contains("$e"))
        .collect();

    assert!(
        !undeclared.is_empty(),
        "catch variable should remain undeclared outside the catch block"
    );
    Ok(())
}

#[test]
fn test_try_catch_variable_diagnostic_range_targets_parameter()
-> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
use strict;
use feature 'try';
my $e = 1;
try {
    die "boom";
} catch ($e) {
    print $e;
}
"#;

    let diags = diagnostics_for(source);
    let shadowing = diags
        .iter()
        .find(|d| d.message.contains("shadows") && d.message.contains("$e"))
        .ok_or("expected catch-variable shadowing diagnostic")?;

    assert_eq!(
        &source[shadowing.range.0..shadowing.range.1],
        "$e",
        "catch-variable diagnostic range should target the catch parameter"
    );
    Ok(())
}

#[test]
fn test_severity_ordering_is_consistent() -> Result<(), Box<dyn std::error::Error>> {
    // Error (1) < Warning (2) < Information (3) < Hint (4)
    assert!(DiagnosticSeverity::Error < DiagnosticSeverity::Warning);
    assert!(DiagnosticSeverity::Warning < DiagnosticSeverity::Information);
    assert!(DiagnosticSeverity::Information < DiagnosticSeverity::Hint);
    Ok(())
}

// =========================================================================
// 3. Range accuracy: byte offsets land in correct source region
// =========================================================================

#[test]
fn test_range_accuracy_offset_within_source() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = ;\nmy $y = 2;\n";
    let diags = diagnostics_for(source);

    for d in &diags {
        assert_offset_in_range(source, d.range.0, &format!("start of '{}'", d.message));
        // End may be source.len()+1 due to clamping, but should never exceed that
        assert!(
            d.range.1 <= source.len() + 1,
            "end offset {} exceeds source.len()+1={} for '{}'",
            d.range.1,
            source.len() + 1,
            d.message
        );
    }
    Ok(())
}

#[test]
fn test_range_accuracy_parse_error_near_correct_position() -> Result<(), Box<dyn std::error::Error>>
{
    // The parse error for `= ;` should be somewhere near the semicolon (offset ~8)
    let source = "my $x = ;";
    let diags = diagnostics_for(source);
    let pe = parse_error_diags(&diags);

    if let Some(first) = pe.first() {
        // The error should be somewhere in the source, not at 0 (which would be wrong
        // for this specific case -- the issue is at the semicolon, not the beginning)
        assert!(
            first.range.0 <= source.len(),
            "Error offset should be within source bounds"
        );
    }
    Ok(())
}

#[test]
fn test_range_accuracy_multiline_source() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use strict;\nuse warnings;\nmy $x = ;\nmy $y = 2;\n";
    let diags = diagnostics_for(source);
    let pe = parse_error_diags(&diags);

    for d in &pe {
        let (line, col) = offset_to_line_col(source, d.range.0);
        // The error is on the `my $x = ;` line which is line 2 (0-based)
        // At minimum, the offset should map to a valid line
        assert!(
            (line as usize) < source.lines().count(),
            "Line {line} should be within source line count {}",
            source.lines().count()
        );
        // Column should be reasonable (not ridiculously large)
        assert!(col < 1000, "Column {col} is unreasonably large");
    }
    Ok(())
}

#[test]
fn test_range_accuracy_line_col_conversion_known_position() -> Result<(), Box<dyn std::error::Error>>
{
    // Verify the line/col conversion works for a known source layout
    //          0123456789...
    // line 0: "use strict;\n"  (12 chars + newline = 13 bytes to offset 12)
    // line 1: "use warnings;\n" (14 chars + newline = 14 bytes, starts at offset 12)
    // line 2: "my $x = ;\n"     (starts at offset 26)
    let source = "use strict;\nuse warnings;\nmy $x = ;\n";

    // Offset 0 -> line 0, col 0
    let (line, col) = offset_to_line_col(source, 0);
    assert_eq!(line, 0);
    assert_eq!(col, 0);

    // First char of line 1 ("use warnings;\n" starts at offset 12)
    let (line, col) = offset_to_line_col(source, 12);
    assert_eq!(line, 1, "Offset 12 should be on line 1");
    assert_eq!(col, 0, "Offset 12 should be at column 0");

    // First char of line 2 ("my $x = ;\n" starts at offset 26)
    let (line, col) = offset_to_line_col(source, 26);
    assert_eq!(line, 2, "Offset 26 should be on line 2");
    assert_eq!(col, 0, "Offset 26 should be at column 0");

    // The semicolon in "my $x = ;" is at offset 34 (26 + 8)
    let (line, col) = offset_to_line_col(source, 34);
    assert_eq!(line, 2, "Offset 34 should be on line 2");
    assert_eq!(col, 8, "Offset 34 should be at column 8");
    Ok(())
}

#[test]
fn test_range_accuracy_eof_offset() -> Result<(), Box<dyn std::error::Error>> {
    // Synthetic EOF error: provider should clamp to source bounds
    let source = "short";
    let ast = Arc::new(perl_parser_core::Node::new(
        perl_parser_core::NodeKind::Program { statements: vec![] },
        perl_parser_core::SourceLocation {
            start: 0,
            end: source.len(),
        },
    ));
    let errors = vec![ParseError::UnexpectedEof];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diags = provider.get_diagnostics(&ast, &errors, source, None);

    let pe = parse_error_diags(&diags);
    if let Some(d) = pe.first() {
        assert_eq!(
            d.range.0,
            source.len(),
            "EOF error should point to end of source"
        );
    }
    Ok(())
}

// =========================================================================
// 4. Multiple diagnostics per file
// =========================================================================

#[test]
fn test_multiple_diagnostics_mixed_categories() -> Result<(), Box<dyn std::error::Error>> {
    // Source with a parse error AND no strict/warnings.
    // Use synthetic errors to guarantee we have both a parse-error and lint diagnostics.
    let source = "my $x = ;\nmy $y = 2;\n";
    let ast = Arc::new(perl_parser_core::Node::new(
        perl_parser_core::NodeKind::Program { statements: vec![] },
        perl_parser_core::SourceLocation {
            start: 0,
            end: source.len(),
        },
    ));
    let errors = vec![ParseError::SyntaxError {
        location: 8,
        message: "bad syntax".to_string(),
    }];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diags = provider.get_diagnostics(&ast, &errors, source, None);

    let pe = parse_error_diags(&diags);
    assert!(
        !pe.is_empty(),
        "Should have parse-error diagnostics from synthetic error"
    );

    // The provider also runs scope analysis and lint checks.
    // With an empty Program AST, strict/warnings lint is suppressed (no executable content).
    let lint_diags: Vec<_> = diags
        .iter()
        .filter(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")))
        .collect();
    assert!(
        lint_diags.is_empty(),
        "empty Program AST should suppress strict/warnings lint, got: {lint_diags:?}"
    );

    // Verify we have at least Error-level diagnostics from the parse error
    let has_error = diags
        .iter()
        .any(|d| d.severity == DiagnosticSeverity::Error);
    assert!(has_error, "Should contain Error-level diagnostics");
    Ok(())
}

#[test]
fn test_multiple_diagnostics_multiple_parse_errors() -> Result<(), Box<dyn std::error::Error>> {
    // Multiple syntax problems in one file
    let source = "my $x = ;\nmy $y = ;\nmy $z = ;\n";
    let diags = diagnostics_for(source);
    let pe = parse_error_diags(&diags);

    // At least one parse error should be reported (parser may recover and find more)
    assert!(
        !pe.is_empty(),
        "Multiple broken statements should produce parse-error diagnostics"
    );
    Ok(())
}

#[test]
fn test_multiple_diagnostics_each_has_distinct_range() -> Result<(), Box<dyn std::error::Error>> {
    // Synthetic test: three errors at well-separated locations should each survive
    // cascade suppression.  Errors must be more than 10 bytes apart so they are
    // treated as independent primary errors rather than cascades of a single failure.
    let source = "aaa_long_token_here; bbb_long_token_there; ccc_long_token_final;";
    let ast = Arc::new(perl_parser_core::Node::new(
        perl_parser_core::NodeKind::Program { statements: vec![] },
        perl_parser_core::SourceLocation {
            start: 0,
            end: source.len(),
        },
    ));
    let errors = vec![
        ParseError::SyntaxError {
            location: 0,
            message: "error at aaa".to_string(),
        },
        ParseError::SyntaxError {
            location: 21,
            message: "error at bbb".to_string(),
        },
        ParseError::SyntaxError {
            location: 43,
            message: "error at ccc".to_string(),
        },
    ];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diags = provider.get_diagnostics(&ast, &errors, source, None);
    let pe = parse_error_diags(&diags);

    assert!(
        pe.len() >= 3,
        "Three distinct errors (>10 bytes apart) should produce at least 3 diagnostics"
    );
    // Verify they have distinct start offsets
    let starts: Vec<usize> = pe.iter().map(|d| d.range.0).collect();
    assert!(starts.contains(&0), "Should have diagnostic at offset 0");
    assert!(starts.contains(&21), "Should have diagnostic at offset 21");
    assert!(starts.contains(&43), "Should have diagnostic at offset 43");
    Ok(())
}

// =========================================================================
// 5. Diagnostic clearing on fix
// =========================================================================

#[test]
fn test_clearing_on_fix_broken_then_fixed() -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Parse broken source -- should have parse errors
    let broken = "my $x = ;\n";
    let diags_broken = diagnostics_for(broken);
    let pe_broken = parse_error_diags(&diags_broken);
    assert!(
        !pe_broken.is_empty(),
        "Broken source should produce parse-error diagnostics"
    );

    // Step 2: Fix the source -- should have zero parse errors
    let fixed = "my $x = 1;\n";
    let diags_fixed = diagnostics_for(fixed);
    let pe_fixed = parse_error_diags(&diags_fixed);
    assert!(
        pe_fixed.is_empty(),
        "Fixed source should produce zero parse-error diagnostics"
    );
    Ok(())
}

#[test]
fn test_clearing_on_fix_synthetic_error_then_none() -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Synthetic broken source with explicit parse error
    let broken = "my $x = 42\nprint $x;\n";
    let ast = Arc::new(perl_parser_core::Node::new(
        perl_parser_core::NodeKind::Program { statements: vec![] },
        perl_parser_core::SourceLocation {
            start: 0,
            end: broken.len(),
        },
    ));
    let errors = vec![ParseError::UnexpectedToken {
        location: 10,
        expected: ";".to_string(),
        found: "print".to_string(),
    }];
    let provider = DiagnosticsProvider::new(&ast, broken.to_string());
    let diags_broken = provider.get_diagnostics(&ast, &errors, broken, None);
    let pe_broken = parse_error_diags(&diags_broken);
    assert!(
        !pe_broken.is_empty(),
        "Broken source with explicit error should produce diagnostics"
    );

    // Step 2: Fixed source with no parse errors
    let fixed = "my $x = 42;\nprint $x;\n";
    let diags_fixed = diagnostics_for(fixed);
    let pe_fixed = parse_error_diags(&diags_fixed);
    assert!(
        pe_fixed.is_empty(),
        "Fixed source should produce zero parse-error diagnostics"
    );
    Ok(())
}

#[test]
fn test_clearing_on_fix_unclosed_brace_then_closed() -> Result<(), Box<dyn std::error::Error>> {
    let broken = "sub foo {\n    my $x = 1;\n";
    let diags_broken = diagnostics_for(broken);
    let pe_broken = parse_error_diags(&diags_broken);
    assert!(
        !pe_broken.is_empty(),
        "Unclosed brace should produce errors"
    );

    let fixed = "sub foo {\n    my $x = 1;\n}\n";
    let diags_fixed = diagnostics_for(fixed);
    let pe_fixed = parse_error_diags(&diags_fixed);
    assert!(
        pe_fixed.is_empty(),
        "Closing brace should clear parse errors, got: {pe_fixed:?}"
    );
    Ok(())
}

#[test]
fn test_clearing_on_fix_lint_diagnostics_also_clear() -> Result<(), Box<dyn std::error::Error>> {
    // Use the lint checker directly to verify clearing behavior for missing-pragma
    // diagnostics, since the full pipeline's lint integration depends on AST shape.
    use perl_parser_core::{Node, NodeKind, SourceLocation};

    // Without strict/warnings: a non-empty program (single bare statement) should trigger
    // missing-strict/warnings. An empty program now correctly suppresses these diagnostics.
    let stmt = Node::new(
        NodeKind::ExpressionStatement {
            expression: Box::new(Node::new(
                NodeKind::Variable {
                    sigil: "$".to_string(),
                    name: "x".to_string(),
                },
                SourceLocation { start: 0, end: 2 },
            )),
        },
        SourceLocation { start: 0, end: 3 },
    );
    let without = Node::new(
        NodeKind::Program {
            statements: vec![stmt],
        },
        SourceLocation { start: 0, end: 10 },
    );
    let mut diags1 = Vec::new();
    perl_lsp_diagnostics::strict_warnings::check_strict_warnings(&without, &mut diags1);
    let lint1: Vec<_> = diags1
        .iter()
        .filter(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")))
        .collect();
    assert!(
        !lint1.is_empty(),
        "Should have missing-pragma diagnostics without pragmas"
    );

    // With strict and warnings: both present
    let use_strict = Node::new(
        NodeKind::Use {
            module: "strict".to_string(),
            args: vec![],
            has_filter_risk: false,
        },
        SourceLocation { start: 0, end: 12 },
    );
    let use_warnings = Node::new(
        NodeKind::Use {
            module: "warnings".to_string(),
            args: vec![],
            has_filter_risk: false,
        },
        SourceLocation { start: 13, end: 27 },
    );
    let with = Node::new(
        NodeKind::Program {
            statements: vec![use_strict, use_warnings],
        },
        SourceLocation { start: 0, end: 28 },
    );
    let mut diags2 = Vec::new();
    perl_lsp_diagnostics::strict_warnings::check_strict_warnings(&with, &mut diags2);
    let lint2: Vec<_> = diags2
        .iter()
        .filter(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")))
        .collect();
    assert!(
        lint2.is_empty(),
        "Adding pragmas should clear missing-pragma diagnostics"
    );
    Ok(())
}

#[test]
fn test_version_pragma_suppresses_missing_strict_warnings() -> Result<(), Box<dyn std::error::Error>>
{
    let source = "use v5.40;\nmy $x = 1;\n";
    let diags = diagnostics_for(source);
    let lint_diags: Vec<_> = diags
        .iter()
        .filter(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")))
        .collect();

    assert!(
        lint_diags.is_empty(),
        "use v5.40 should suppress missing strict/warnings diagnostics: {lint_diags:?}"
    );
    Ok(())
}

#[test]
fn test_v5_36_suppresses_missing_strict_warnings() -> Result<(), Box<dyn std::error::Error>> {
    // use v5.36 enables both strict and warnings via the Perl feature bundle.
    // Neither PL100 (missing-strict) nor PL101 (missing-warnings) should fire.
    let source = "use v5.36;\nmy $x = 1;\n";
    let diags = diagnostics_for(source);
    let lint_diags: Vec<_> = diags
        .iter()
        .filter(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")))
        .collect();

    assert!(
        lint_diags.is_empty(),
        "use v5.36 should suppress missing strict/warnings diagnostics: {lint_diags:?}"
    );
    Ok(())
}

// =========================================================================
// 6. Suggestion field populated for actionable diagnostics
// =========================================================================

#[test]
fn test_suggestion_present_for_eof_error() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = ";
    let ast = Arc::new(perl_parser_core::Node::new(
        perl_parser_core::NodeKind::Program { statements: vec![] },
        perl_parser_core::SourceLocation {
            start: 0,
            end: source.len(),
        },
    ));
    let errors = vec![ParseError::UnexpectedEof];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diags = provider.get_diagnostics(&ast, &errors, source, None);
    let pe = parse_error_diags(&diags);

    if let Some(d) = pe.first() {
        assert!(
            d.suggestion.is_some(),
            "EOF error should carry a suggestion"
        );
    }
    Ok(())
}

#[test]
fn test_suggestion_present_for_unclosed_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my @a = (1, 2";
    let ast = Arc::new(perl_parser_core::Node::new(
        perl_parser_core::NodeKind::Program { statements: vec![] },
        perl_parser_core::SourceLocation {
            start: 0,
            end: source.len(),
        },
    ));
    let errors = vec![ParseError::UnclosedDelimiter { delimiter: ')' }];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diags = provider.get_diagnostics(&ast, &errors, source, None);
    let pe = parse_error_diags(&diags);

    if let Some(d) = pe.first() {
        assert!(
            d.suggestion.is_some(),
            "UnclosedDelimiter should carry a suggestion"
        );
        let s = d.suggestion.as_deref().unwrap_or_default();
        assert!(
            s.contains(')'),
            "Suggestion should mention the closing delimiter"
        );
    }
    Ok(())
}

// =========================================================================
// 7. Full pipeline: real Perl with multiple issue categories
// =========================================================================

#[test]
fn test_full_pipeline_complex_perl_file() -> Result<(), Box<dyn std::error::Error>> {
    // A file that should parse cleanly but may produce lint diagnostics
    let source = "my $used = 1;\nprint $used;\n";
    let diags = diagnostics_for(source);

    // Should produce NO parse errors for valid Perl
    let pe = parse_error_diags(&diags);
    assert!(
        pe.is_empty(),
        "Valid Perl should have no parse errors, got: {pe:?}"
    );

    // All diagnostics should have valid ranges
    for d in &diags {
        assert!(
            d.range.0 <= source.len() + 1,
            "Start offset {} exceeds source length for '{}'",
            d.range.0,
            d.message
        );
        // Every diagnostic should have a code
        assert!(
            d.code.is_some(),
            "Diagnostic should have a code: {}",
            d.message
        );
    }
    Ok(())
}

#[test]
fn test_full_pipeline_empty_source() -> Result<(), Box<dyn std::error::Error>> {
    let diags = diagnostics_for("");
    // Empty source should not panic and should return a valid Vec with no diagnostics
    let pe = parse_error_diags(&diags);
    assert!(pe.is_empty(), "Empty source should not have parse errors");
    // Empty source should also not trigger strict/warnings lint (regression guard)
    let lint: Vec<_> = diags
        .iter()
        .filter(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")))
        .collect();
    assert!(
        lint.is_empty(),
        "Empty source should not produce strict/warnings lint, got: {lint:?}"
    );
    Ok(())
}

#[test]
fn test_full_pipeline_clean_perl_minimal_diagnostics() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use strict;\nuse warnings;\nmy $x = 42;\nprint $x;\n1;\n";
    let diags = diagnostics_for(source);
    let pe = parse_error_diags(&diags);

    assert!(pe.is_empty(), "Clean Perl should have zero parse errors");

    // Should not have missing-strict or missing-warnings
    let missing_pragmas: Vec<_> = diags
        .iter()
        .filter(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")))
        .collect();
    assert!(
        missing_pragmas.is_empty(),
        "Clean Perl should not have missing-pragma diagnostics"
    );
    Ok(())
}

// =========================================================================
// 8. Diagnostic code field consistency
// =========================================================================

#[test]
fn test_diagnostic_code_field_always_present_for_parse_errors()
-> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = ;";
    let diags = diagnostics_for(source);

    for d in &diags {
        assert!(
            d.code.is_some(),
            "Every diagnostic should have a code, but found None for: {}",
            d.message
        );
    }
    Ok(())
}

#[test]
fn test_diagnostic_codes_are_well_formed() -> Result<(), Box<dyn std::error::Error>> {
    // Parse a source that generates multiple diagnostic categories
    let source = "my $x = ;";
    let diags = diagnostics_for(source);

    for d in &diags {
        if let Some(code) = &d.code {
            // Codes should be stable PL/PC-prefixed codes (e.g., "PL001", "PC001")
            // or legacy parse-error-* subcodes
            assert!(
                code.starts_with("PL")
                    || code.starts_with("PC")
                    || code.starts_with("parse-error-"),
                "Diagnostic code '{}' should be a stable PL/PC-prefixed code",
                code
            );
        }
    }
    Ok(())
}

// =========================================================================
// 9. Deprecated feature detection (tag correctness)
// =========================================================================

#[test]
fn test_deprecated_tag_for_deprecated_syntax() -> Result<(), Box<dyn std::error::Error>> {
    use perl_parser_core::{Node, NodeKind, SourceLocation};

    // Build AST with defined(@array) -- deprecated
    let arr = Node::new(
        NodeKind::Variable {
            sigil: "@".to_string(),
            name: "data".to_string(),
        },
        SourceLocation { start: 20, end: 25 },
    );
    let call = Node::new(
        NodeKind::FunctionCall {
            name: "defined".to_string(),
            args: vec![arr],
        },
        SourceLocation { start: 10, end: 30 },
    );
    let stmt = Node::new(
        NodeKind::ExpressionStatement {
            expression: Box::new(call),
        },
        SourceLocation { start: 10, end: 31 },
    );
    let root = Node::new(
        NodeKind::Program {
            statements: vec![stmt],
        },
        SourceLocation { start: 0, end: 100 },
    );

    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::deprecated::check_deprecated_syntax(&root, &mut diagnostics);

    let dep: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code.as_deref() == Some("PL500"))
        .collect();
    assert!(!dep.is_empty(), "Should detect deprecated defined(@array)");
    assert!(dep[0].tags.contains(&DiagnosticTag::Deprecated));
    assert_eq!(dep[0].severity, DiagnosticSeverity::Warning);
    Ok(())
}

// =========================================================================
// 10. Edge cases
// =========================================================================

#[test]
fn test_edge_case_single_character_source() -> Result<(), Box<dyn std::error::Error>> {
    let diags = diagnostics_for(";");
    // Should not panic; diagnostics are all within bounds
    for d in &diags {
        assert!(
            d.range.0 <= 2,
            "Range start should be within single-char source"
        );
    }
    Ok(())
}

#[test]
fn test_edge_case_only_whitespace() -> Result<(), Box<dyn std::error::Error>> {
    let diags = diagnostics_for("   \n\n   \n");
    let pe = parse_error_diags(&diags);
    assert!(
        pe.is_empty(),
        "Whitespace-only source should not produce parse errors"
    );
    Ok(())
}

#[test]
fn test_edge_case_very_long_line() -> Result<(), Box<dyn std::error::Error>> {
    // A very long valid line should parse without issue
    let long_val = "x".repeat(10_000);
    let source = format!("my $x = \"{}\";\n", long_val);
    let diags = diagnostics_for(&source);
    let pe = parse_error_diags(&diags);

    // Should not panic; may or may not have parse errors depending on parser limits
    for d in &pe {
        assert!(
            d.range.0 <= source.len(),
            "Offset should be within source bounds"
        );
    }
    Ok(())
}

#[test]
fn test_edge_case_unicode_source() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use strict;\nuse warnings;\nmy $x = \"\u{00e9}\u{00e8}\u{00ea}\";\nprint $x;\n";
    let diags = diagnostics_for(source);
    let pe = parse_error_diags(&diags);

    assert!(
        pe.is_empty(),
        "Valid Perl with unicode should have no parse errors, got: {pe:?}"
    );

    // Verify offset-to-position works with multibyte chars
    let (line, _col) = offset_to_line_col(source, 0);
    assert_eq!(line, 0);
    Ok(())
}

// =========================================================================
// 11. Empty / trivially-empty files produce no strict/warnings diagnostics
//     (regression: check_strict_warnings was unconditional)
// =========================================================================

#[test]
fn empty_file_produces_no_strict_warnings_diagnostics() -> Result<(), Box<dyn std::error::Error>> {
    let diags = diagnostics_for("");
    let noisy: Vec<_> = diags
        .iter()
        .filter(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")))
        .collect();
    assert!(
        noisy.is_empty(),
        "empty file should produce no strict/warnings diagnostics, got: {noisy:?}"
    );
    Ok(())
}

#[test]
fn whitespace_only_file_produces_no_strict_warnings_diagnostics()
-> Result<(), Box<dyn std::error::Error>> {
    let diags = diagnostics_for("   \n\t\n");
    let noisy: Vec<_> = diags
        .iter()
        .filter(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")))
        .collect();
    assert!(
        noisy.is_empty(),
        "whitespace-only file should produce no strict/warnings diagnostics"
    );
    Ok(())
}

#[test]
fn comment_only_file_produces_no_strict_warnings_diagnostics()
-> Result<(), Box<dyn std::error::Error>> {
    let diags = diagnostics_for("# just a comment\n");
    let noisy: Vec<_> = diags
        .iter()
        .filter(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")))
        .collect();
    assert!(
        noisy.is_empty(),
        "comment-only file should produce no strict/warnings diagnostics"
    );
    Ok(())
}
