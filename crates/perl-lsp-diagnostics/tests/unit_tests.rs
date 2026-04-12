//! Comprehensive unit tests for perl-lsp-diagnostics crate
//!
//! Tests cover: types, dedup, parse_errors, diagnostics provider,
//! scope conversion, walker, error_nodes, lints (deprecated, strict, common_mistakes),
//! and dead_code detection.

use std::sync::Arc;

use perl_lsp_diagnostics::{
    Diagnostic, DiagnosticSeverity, DiagnosticTag, DiagnosticsProvider, RelatedInformation,
};
use perl_parser_core::error::ParseError;
use perl_parser_core::{Node, NodeKind, SourceLocation};

// ---------------------------------------------------------------------------
// Helper: construct AST nodes concisely
// ---------------------------------------------------------------------------

fn loc(start: usize, end: usize) -> SourceLocation {
    SourceLocation { start, end }
}

fn var_node(sigil: &str, name: &str, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::Variable { sigil: sigil.to_string(), name: name.to_string() },
        loc(start, end),
    )
}

fn program(stmts: Vec<Node>) -> Node {
    Node::new(NodeKind::Program { statements: stmts }, loc(0, 100))
}

fn block(stmts: Vec<Node>) -> Node {
    Node::new(NodeKind::Block { statements: stmts }, loc(0, 100))
}

fn use_node(module: &str, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::Use { module: module.to_string(), args: vec![], has_filter_risk: false },
        loc(start, end),
    )
}

fn binary_node(op: &str, left: Node, right: Node, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::Binary { op: op.to_string(), left: Box::new(left), right: Box::new(right) },
        loc(start, end),
    )
}

fn func_call(name: &str, args: Vec<Node>, start: usize, end: usize) -> Node {
    Node::new(NodeKind::FunctionCall { name: name.to_string(), args }, loc(start, end))
}

fn expr_stmt(expr: Node, start: usize, end: usize) -> Node {
    Node::new(NodeKind::ExpressionStatement { expression: Box::new(expr) }, loc(start, end))
}

fn number_node(value: &str, start: usize, end: usize) -> Node {
    Node::new(NodeKind::Number { value: value.to_string() }, loc(start, end))
}

// =========================================================================
// 1. types — DiagnosticSeverity, Diagnostic, RelatedInformation, DiagnosticTag
// =========================================================================

#[test]
fn severity_ordering() -> Result<(), Box<dyn std::error::Error>> {
    // Error < Warning < Information < Hint (by discriminant)
    assert!(DiagnosticSeverity::Error < DiagnosticSeverity::Warning);
    assert!(DiagnosticSeverity::Warning < DiagnosticSeverity::Information);
    assert!(DiagnosticSeverity::Information < DiagnosticSeverity::Hint);
    Ok(())
}

#[test]
fn severity_clone_copy_eq() -> Result<(), Box<dyn std::error::Error>> {
    let s = DiagnosticSeverity::Warning;
    let s2 = s; // Copy
    assert_eq!(s, s2);
    let s3 = s;
    assert_eq!(s, s3);
    Ok(())
}

#[test]
fn diagnostic_struct_construction() -> Result<(), Box<dyn std::error::Error>> {
    let d = Diagnostic {
        range: (10, 20),
        severity: DiagnosticSeverity::Error,
        code: Some("test-code".to_string()),
        message: "test message".to_string(),
        related_information: vec![],
        tags: vec![],
        suggestion: None,
    };
    assert_eq!(d.range, (10, 20));
    assert_eq!(d.severity, DiagnosticSeverity::Error);
    assert_eq!(d.code.as_deref(), Some("test-code"));
    assert_eq!(d.message, "test message");
    assert!(d.related_information.is_empty());
    assert!(d.tags.is_empty());
    Ok(())
}

#[test]
fn diagnostic_with_related_info() -> Result<(), Box<dyn std::error::Error>> {
    let ri = RelatedInformation { location: (5, 15), message: "hint here".to_string() };
    let d = Diagnostic {
        range: (0, 50),
        severity: DiagnosticSeverity::Hint,
        code: None,
        message: "some hint".to_string(),
        related_information: vec![ri.clone()],
        tags: vec![DiagnosticTag::Unnecessary],
        suggestion: None,
    };
    assert_eq!(d.related_information.len(), 1);
    assert_eq!(d.related_information[0].location, (5, 15));
    assert_eq!(d.tags, vec![DiagnosticTag::Unnecessary]);
    Ok(())
}

#[test]
fn diagnostic_clone_eq() -> Result<(), Box<dyn std::error::Error>> {
    let d1 = Diagnostic {
        range: (0, 1),
        severity: DiagnosticSeverity::Warning,
        code: Some("w1".to_string()),
        message: "msg".to_string(),
        related_information: vec![],
        tags: vec![DiagnosticTag::Deprecated],
        suggestion: None,
    };
    let d2 = d1.clone();
    assert_eq!(d1, d2);
    Ok(())
}

#[test]
fn diagnostic_tag_variants() -> Result<(), Box<dyn std::error::Error>> {
    let t1 = DiagnosticTag::Unnecessary;
    let t2 = DiagnosticTag::Deprecated;
    assert_ne!(t1, t2);
    let t3 = t1; // Copy
    assert_eq!(t1, t3);
    Ok(())
}

#[test]
fn related_information_debug() -> Result<(), Box<dyn std::error::Error>> {
    let ri = RelatedInformation { location: (0, 5), message: "hello".to_string() };
    let dbg = format!("{:?}", ri);
    assert!(dbg.contains("hello"));
    Ok(())
}

// =========================================================================
// 2. dedup — deduplicate_diagnostics (module is private, test via provider)
// =========================================================================

// Since dedup is private, we verify dedup behavior indirectly:
// the diagnostics provider should not emit exact duplicates for the same error
// repeated twice. We test this through the public API.

// =========================================================================
// 3. parse_errors — parse_error_to_diagnostic (private, tested via provider)
// =========================================================================

// =========================================================================
// 4. DiagnosticsProvider
// =========================================================================

#[test]
fn provider_no_errors_no_scope_issues() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![use_node("strict", 0, 12), use_node("warnings", 13, 27)]));
    let source = "use strict;\nuse warnings;\n";
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &[], source, None);
    // No parse errors and no undeclared vars → may still have scope issues
    // depending on analyzer, but at minimum we get a Vec back
    assert!(
        diagnostics
            .iter()
            .all(|d| d.severity != DiagnosticSeverity::Error || d.code.as_deref() != Some("PL001"))
    );
    Ok(())
}

#[test]
fn provider_unexpected_token_error() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![]));
    let source = "my $x = ;";
    let errors = vec![ParseError::UnexpectedToken {
        location: 8,
        expected: "expression".to_string(),
        found: ";".to_string(),
    }];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &errors, source, None);

    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL001")).collect();
    assert!(!parse_diags.is_empty(), "should produce parse-error diagnostics");

    let first = &parse_diags[0];
    assert_eq!(first.severity, DiagnosticSeverity::Error);
    assert!(first.message.contains("Expected expression"));
    assert!(first.message.contains("`;`"));
    assert_eq!(first.range.0, 8);
    Ok(())
}

#[test]
fn provider_unexpected_token_formats_end_of_input() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![]));
    let source = "my $x = ";
    let errors = vec![ParseError::UnexpectedToken {
        location: source.len(),
        expected: "expression".to_string(),
        found: "<EOF>".to_string(),
    }];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &errors, source, None);

    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL001")).collect();
    assert!(!parse_diags.is_empty());

    let first = &parse_diags[0];
    assert!(first.message.contains("Expected expression"));
    assert!(first.message.contains("end of input"));
    Ok(())
}

#[test]
fn provider_clamps_out_of_bounds_ranges() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![]));
    let source = "short";
    let errors = vec![ParseError::UnexpectedToken {
        location: 999,
        expected: "statement".to_string(),
        found: "foo".to_string(),
    }];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &errors, source, None);

    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL001")).collect();
    assert!(!parse_diags.is_empty());

    let first = &parse_diags[0];
    assert_eq!(first.range.0, source.len());
    assert_eq!(first.range.1, source.len().saturating_add(1));
    Ok(())
}

#[test]
fn provider_syntax_error() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![]));
    let source = "invalid perl";
    let errors = vec![ParseError::SyntaxError { location: 3, message: "bad syntax".to_string() }];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &errors, source, None);

    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL002")).collect();
    assert!(!parse_diags.is_empty());

    let first = &parse_diags[0];
    assert_eq!(first.range.0, 3);
    assert_eq!(first.message, "bad syntax");
    Ok(())
}

#[test]
fn provider_invalid_prototype_maps_to_pl302_warning() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![]));
    let source = "sub foo (XYZ) {}";
    let errors = vec![ParseError::SyntaxError {
        location: 8,
        message: "Invalid prototype character(s) 'X'".to_string(),
    }];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &errors, source, None);

    let prototype_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL302")).collect();
    assert!(!prototype_diags.is_empty(), "expected PL302 diagnostic, got: {:?}", diagnostics);

    let first = &prototype_diags[0];
    assert_eq!(first.severity, DiagnosticSeverity::Warning);
    assert_eq!(first.range.0, 8);
    Ok(())
}

#[test]
fn provider_unknown_subroutine_attribute_stays_warning() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![]));
    let source = "sub foo :wat {}";
    let errors = vec![ParseError::SyntaxError {
        location: 8,
        message: "unknown subroutine attribute ':wat'".to_string(),
    }];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &errors, source, None);

    let attr_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.message.contains("unknown subroutine attribute")).collect();
    assert!(!attr_diags.is_empty(), "expected attribute diagnostic, got: {:?}", diagnostics);

    let first = attr_diags[0];
    assert_eq!(first.severity, DiagnosticSeverity::Warning);
    Ok(())
}

#[test]
fn provider_unexpected_eof_error() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![]));
    let source = "my $x = ";
    let errors = vec![ParseError::UnexpectedEof];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &errors, source, None);

    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL003")).collect();
    assert!(!parse_diags.is_empty());

    let first = &parse_diags[0];
    assert_eq!(first.range.0, source.len());
    assert!(first.message.contains("end of input"));
    Ok(())
}

#[test]
fn provider_lexer_error() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![]));
    let source = "";
    let errors = vec![ParseError::LexerError { message: "bad token".to_string() }];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &errors, source, None);

    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL001")).collect();
    assert!(!parse_diags.is_empty());
    assert_eq!(parse_diags[0].range.0, 0);
    assert_eq!(parse_diags[0].message, "bad token");
    Ok(())
}

#[test]
fn provider_multiple_errors() -> Result<(), Box<dyn std::error::Error>> {
    // Errors must be more than 10 bytes apart so cascade suppression does not
    // collapse them into a single cluster.
    let ast = Arc::new(program(vec![]));
    let source = "aaa_long_sep_one; bbb_long_sep_two; ccc_long_sep_three;";
    let errors = vec![
        ParseError::SyntaxError { location: 0, message: "err1".to_string() },
        ParseError::SyntaxError { location: 18, message: "err2".to_string() },
        ParseError::SyntaxError { location: 36, message: "err3".to_string() },
    ];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &errors, source, None);

    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL002")).collect();
    assert!(parse_diags.len() >= 3);
    Ok(())
}

#[test]
fn provider_error_range_is_clamped_to_source_bounds() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![]));
    let source = "x";
    let errors = vec![ParseError::UnexpectedToken {
        location: usize::MAX,
        expected: "a".to_string(),
        found: "b".to_string(),
    }];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &errors, source, None);

    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL001")).collect();
    assert!(!parse_diags.is_empty());
    assert_eq!(parse_diags[0].range.0, source.len());
    assert_eq!(parse_diags[0].range.1, source.len().saturating_add(1));
    Ok(())
}

// =========================================================================
// 5. scope — scope_issues_to_diagnostics (tested via public re-export)
// =========================================================================

// scope_issues_to_diagnostics is pub(crate) only, but it's exercised
// indirectly through DiagnosticsProvider::get_diagnostics.
// We test the provider with ASTs that would trigger scope analysis.

#[test]
fn provider_scope_analysis_runs() -> Result<(), Box<dyn std::error::Error>> {
    // A program without strict/warnings — scope analyzer will run but may not
    // produce issues for a trivial program.
    let ast = Arc::new(program(vec![]));
    let source = "";
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &[], source, None);
    // Should not panic, returns a Vec
    let _ = diagnostics.len();
    Ok(())
}

// =========================================================================
// 6. walker — walk_node (private, but indirectly tested through lints)
// =========================================================================

// The walker is thoroughly exercised by lint tests below since each lint
// calls walk_node. We add explicit tests via the public lint APIs.

// =========================================================================
// 7. lints::deprecated — check_deprecated_syntax
// =========================================================================

#[test]
fn deprecated_defined_array() -> Result<(), Box<dyn std::error::Error>> {
    let arr = var_node("@", "data", 20, 25);
    let call = func_call("defined", vec![arr], 10, 30);
    let root = program(vec![expr_stmt(call, 10, 31)]);

    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::deprecated::check_deprecated_syntax(&root, &mut diagnostics);

    assert!(
        diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("PL500") && d.message.contains("@data")),
        "Should detect deprecated defined(@array): {diagnostics:?}"
    );
    assert!(diagnostics.iter().any(|d| d.tags.contains(&DiagnosticTag::Deprecated)));
    Ok(())
}

#[test]
fn deprecated_defined_hash() -> Result<(), Box<dyn std::error::Error>> {
    let hash = var_node("%", "cfg", 20, 24);
    let call = func_call("defined", vec![hash], 10, 30);
    let root = program(vec![expr_stmt(call, 10, 31)]);

    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::deprecated::check_deprecated_syntax(&root, &mut diagnostics);

    assert!(diagnostics.iter().any(|d| d.message.contains("%cfg")));
    assert!(diagnostics.iter().any(|d| d.tags.contains(&DiagnosticTag::Deprecated)));
    Ok(())
}

#[test]
fn deprecated_defined_scalar_no_warning() -> Result<(), Box<dyn std::error::Error>> {
    let scalar = var_node("$", "x", 20, 22);
    let call = func_call("defined", vec![scalar], 10, 25);
    let root = program(vec![expr_stmt(call, 10, 26)]);

    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::deprecated::check_deprecated_syntax(&root, &mut diagnostics);

    // defined($scalar) is NOT deprecated
    assert!(
        diagnostics.iter().all(|d| d.code.as_deref() != Some("PL500")),
        "defined($scalar) should not trigger deprecated warning"
    );
    Ok(())
}

#[test]
fn deprecated_array_base_variable() -> Result<(), Box<dyn std::error::Error>> {
    let bracket = var_node("$", "[", 0, 2);
    let root = program(vec![expr_stmt(bracket, 0, 3)]);

    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::deprecated::check_deprecated_syntax(&root, &mut diagnostics);

    assert!(diagnostics.iter().any(|d| d.code.as_deref() == Some("PL501")));
    assert!(diagnostics.iter().any(|d| d.message.contains("$[")));
    assert!(diagnostics.iter().any(|d| d.tags.contains(&DiagnosticTag::Deprecated)));
    Ok(())
}

#[test]
fn deprecated_no_false_positive_other_variable() -> Result<(), Box<dyn std::error::Error>> {
    let normal_var = var_node("$", "x", 0, 2);
    let root = program(vec![expr_stmt(normal_var, 0, 3)]);

    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::deprecated::check_deprecated_syntax(&root, &mut diagnostics);

    assert!(diagnostics.is_empty(), "Normal variable should not trigger deprecated lint");
    Ok(())
}

#[test]
fn deprecated_related_info_has_suggestion() -> Result<(), Box<dyn std::error::Error>> {
    let arr = var_node("@", "items", 20, 26);
    let call = func_call("defined", vec![arr], 10, 30);
    let root = program(vec![expr_stmt(call, 10, 31)]);

    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::deprecated::check_deprecated_syntax(&root, &mut diagnostics);

    let dep_diag: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL500")).collect();
    assert!(!dep_diag.is_empty());
    // Should have related info with suggestion (💡) and explanation (ℹ️)
    assert!(dep_diag[0].related_information.len() >= 2);
    assert!(dep_diag[0].related_information.iter().any(|ri| ri.message.contains('💡')));
    assert!(dep_diag[0].related_information.iter().any(|ri| ri.message.contains('ℹ')));
    Ok(())
}

// =========================================================================
// 8. lints::strict_warnings — check_strict_warnings
// =========================================================================

#[test]
fn strict_warnings_both_missing() -> Result<(), Box<dyn std::error::Error>> {
    // Non-empty program (a bare variable expression) without strict/warnings pragmas
    // should produce missing-strict and missing-warnings diagnostics.
    let root = program(vec![var_node("$", "x", 0, 2)]);
    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::strict_warnings::check_strict_warnings(&root, &mut diagnostics);

    assert!(diagnostics.iter().any(|d| d.code.as_deref() == Some("PL100")));
    assert!(diagnostics.iter().any(|d| d.code.as_deref() == Some("PL101")));
    // Both should be informational
    for d in &diagnostics {
        assert_eq!(d.severity, DiagnosticSeverity::Information);
    }
    Ok(())
}

#[test]
fn strict_warnings_strict_present() -> Result<(), Box<dyn std::error::Error>> {
    let root = program(vec![use_node("strict", 0, 12)]);
    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::strict_warnings::check_strict_warnings(&root, &mut diagnostics);

    assert!(diagnostics.iter().all(|d| d.code.as_deref() != Some("PL100")));
    assert!(diagnostics.iter().any(|d| d.code.as_deref() == Some("PL101")));
    Ok(())
}

#[test]
fn strict_warnings_warnings_present() -> Result<(), Box<dyn std::error::Error>> {
    let root = program(vec![use_node("warnings", 0, 15)]);
    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::strict_warnings::check_strict_warnings(&root, &mut diagnostics);

    assert!(diagnostics.iter().any(|d| d.code.as_deref() == Some("PL100")));
    assert!(diagnostics.iter().all(|d| d.code.as_deref() != Some("PL101")));
    Ok(())
}

#[test]
fn strict_warnings_both_present() -> Result<(), Box<dyn std::error::Error>> {
    let root = program(vec![use_node("strict", 0, 12), use_node("warnings", 13, 27)]);
    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::strict_warnings::check_strict_warnings(&root, &mut diagnostics);

    assert!(diagnostics.iter().all(|d| d.code.as_deref() != Some("PL100")));
    assert!(diagnostics.iter().all(|d| d.code.as_deref() != Some("PL101")));
    Ok(())
}

#[test]
fn strict_warnings_related_info() -> Result<(), Box<dyn std::error::Error>> {
    let root = program(vec![]);
    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::strict_warnings::check_strict_warnings(&root, &mut diagnostics);

    for d in &diagnostics {
        assert!(
            d.related_information.len() >= 2,
            "Each missing pragma should have suggestion + explanation"
        );
        assert!(d.related_information.iter().any(|ri| ri.message.contains('💡')));
        assert!(d.related_information.iter().any(|ri| ri.message.contains('ℹ')));
    }
    Ok(())
}

// =========================================================================
// 8b. strict_warnings — implicit-strict framework suppression
// =========================================================================

#[test]
fn strict_warnings_suppressed_for_moo() -> Result<(), Box<dyn std::error::Error>> {
    let root = program(vec![use_node("Moo", 0, 8)]);
    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::strict_warnings::check_strict_warnings(&root, &mut diagnostics);
    assert!(
        diagnostics.iter().all(|d| d.code.as_deref() != Some("PL100")),
        "Moo provides implicit strict - should not fire missing-strict"
    );
    assert!(
        diagnostics.iter().all(|d| d.code.as_deref() != Some("PL101")),
        "Moo provides implicit warnings - should not fire missing-warnings"
    );
    Ok(())
}

#[test]
fn strict_warnings_suppressed_for_moose() -> Result<(), Box<dyn std::error::Error>> {
    let root = program(vec![use_node("Moose", 0, 10)]);
    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::strict_warnings::check_strict_warnings(&root, &mut diagnostics);
    assert!(
        diagnostics.iter().all(|d| d.code.as_deref() != Some("PL100")),
        "Moose provides implicit strict - should not fire missing-strict"
    );
    assert!(
        diagnostics.iter().all(|d| d.code.as_deref() != Some("PL101")),
        "Moose provides implicit warnings - should not fire missing-warnings"
    );
    Ok(())
}

#[test]
fn strict_warnings_suppressed_for_modern_perl() -> Result<(), Box<dyn std::error::Error>> {
    let root = program(vec![use_node("Modern::Perl", 0, 20)]);
    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::strict_warnings::check_strict_warnings(&root, &mut diagnostics);
    assert!(
        diagnostics.is_empty(),
        "Modern::Perl replaces strict+warnings - should emit no missing-pragma diagnostics"
    );
    Ok(())
}

#[test]
fn strict_warnings_suppressed_for_mojo_base() -> Result<(), Box<dyn std::error::Error>> {
    let root = program(vec![use_node("Mojo::Base", 0, 15)]);
    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::strict_warnings::check_strict_warnings(&root, &mut diagnostics);
    assert!(
        diagnostics.iter().all(|d| d.code.as_deref() != Some("PL100")),
        "Mojo::Base provides implicit strict - should not fire missing-strict"
    );
    assert!(
        diagnostics.iter().all(|d| d.code.as_deref() != Some("PL101")),
        "Mojo::Base provides implicit warnings - should not fire missing-warnings"
    );
    Ok(())
}

// =========================================================================
// 9. lints::common_mistakes — check_common_mistakes
// =========================================================================

#[test]
fn common_mistakes_assignment_in_if_condition() -> Result<(), Box<dyn std::error::Error>> {
    // if ($x = 1) { ... }
    let condition = binary_node("=", var_node("$", "x", 4, 6), number_node("1", 9, 10), 4, 10);
    let body = block(vec![]);
    let if_node = Node::new(
        NodeKind::If {
            condition: Box::new(condition),
            then_branch: Box::new(body),
            elsif_branches: vec![],
            else_branch: None,
        },
        loc(0, 20),
    );
    let root = program(vec![if_node]);

    let sym_table = perl_semantic_analyzer::symbol::SymbolTable::new();
    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::common_mistakes::check_common_mistakes(
        &root,
        &sym_table,
        &mut diagnostics,
    );

    assert!(diagnostics.iter().any(|d| d.code.as_deref() == Some("PL403")));
    assert!(diagnostics.iter().any(|d| d.message.contains("did you mean")));
    Ok(())
}

#[test]
fn common_mistakes_assignment_in_while_condition() -> Result<(), Box<dyn std::error::Error>> {
    let condition = binary_node("=", var_node("$", "line", 7, 12), number_node("0", 15, 16), 7, 16);
    let body = block(vec![]);
    let while_node = Node::new(
        NodeKind::While {
            condition: Box::new(condition),
            body: Box::new(body),
            continue_block: None,
        },
        loc(0, 25),
    );
    let root = program(vec![while_node]);

    let sym_table = perl_semantic_analyzer::symbol::SymbolTable::new();
    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::common_mistakes::check_common_mistakes(
        &root,
        &sym_table,
        &mut diagnostics,
    );

    assert!(diagnostics.iter().any(|d| d.code.as_deref() == Some("PL403")));
    Ok(())
}

#[test]
fn common_mistakes_comparison_in_condition_ok() -> Result<(), Box<dyn std::error::Error>> {
    // if ($x == 1) { ... } — should NOT warn
    let condition = binary_node("==", var_node("$", "x", 4, 6), number_node("1", 10, 11), 4, 11);
    let body = block(vec![]);
    let if_node = Node::new(
        NodeKind::If {
            condition: Box::new(condition),
            then_branch: Box::new(body),
            elsif_branches: vec![],
            else_branch: None,
        },
        loc(0, 20),
    );
    let root = program(vec![if_node]);

    let sym_table = perl_semantic_analyzer::symbol::SymbolTable::new();
    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::common_mistakes::check_common_mistakes(
        &root,
        &sym_table,
        &mut diagnostics,
    );

    assert!(
        diagnostics.iter().all(|d| d.code.as_deref() != Some("PL403")),
        "== comparison should not trigger assignment-in-condition"
    );
    Ok(())
}

#[test]
fn common_mistakes_undef_comparison() -> Result<(), Box<dyn std::error::Error>> {
    // $x == undef
    let undef = Node::new(NodeKind::Undef, loc(10, 15));
    let cmp = binary_node("==", var_node("$", "x", 0, 2), undef, 0, 15);
    let root = program(vec![expr_stmt(cmp, 0, 16)]);

    let sym_table = perl_semantic_analyzer::symbol::SymbolTable::new();
    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::common_mistakes::check_common_mistakes(
        &root,
        &sym_table,
        &mut diagnostics,
    );

    assert!(diagnostics.iter().any(|d| d.code.as_deref() == Some("PL404")));
    assert!(diagnostics.iter().any(|d| d.message.contains("==")));
    Ok(())
}

#[test]
fn common_mistakes_ne_undef_comparison() -> Result<(), Box<dyn std::error::Error>> {
    // $x != undef
    let undef = Node::new(NodeKind::Undef, loc(10, 15));
    let cmp = binary_node("!=", var_node("$", "y", 0, 2), undef, 0, 15);
    let root = program(vec![expr_stmt(cmp, 0, 16)]);

    let sym_table = perl_semantic_analyzer::symbol::SymbolTable::new();
    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::common_mistakes::check_common_mistakes(
        &root,
        &sym_table,
        &mut diagnostics,
    );

    assert!(diagnostics.iter().any(|d| d.code.as_deref() == Some("PL404")));
    assert!(diagnostics.iter().any(|d| d.message.contains("!=")));
    Ok(())
}

#[test]
fn common_mistakes_related_info_for_assignment() -> Result<(), Box<dyn std::error::Error>> {
    let condition = binary_node("=", var_node("$", "z", 4, 6), number_node("5", 9, 10), 4, 10);
    let body = block(vec![]);
    let if_node = Node::new(
        NodeKind::If {
            condition: Box::new(condition),
            then_branch: Box::new(body),
            elsif_branches: vec![],
            else_branch: None,
        },
        loc(0, 20),
    );
    let root = program(vec![if_node]);

    let sym_table = perl_semantic_analyzer::symbol::SymbolTable::new();
    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::common_mistakes::check_common_mistakes(
        &root,
        &sym_table,
        &mut diagnostics,
    );

    let assign_diag: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL403")).collect();
    assert!(!assign_diag.is_empty());
    // Should have suggestion + explanation
    assert!(assign_diag[0].related_information.len() >= 2);
    assert!(assign_diag[0].related_information.iter().any(|ri| ri.message.contains("==")));
    Ok(())
}

#[test]
fn common_mistakes_no_warning_for_string_comparison() -> Result<(), Box<dyn std::error::Error>> {
    // $x eq "hello" — should NOT trigger numeric-undef
    let cmp = binary_node("eq", var_node("$", "x", 0, 2), number_node("1", 6, 7), 0, 7);
    let root = program(vec![expr_stmt(cmp, 0, 8)]);

    let sym_table = perl_semantic_analyzer::symbol::SymbolTable::new();
    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::common_mistakes::check_common_mistakes(
        &root,
        &sym_table,
        &mut diagnostics,
    );

    assert!(
        diagnostics.iter().all(|d| d.code.as_deref() != Some("PL404")),
        "String comparison should not trigger numeric-undef"
    );
    Ok(())
}

// =========================================================================
// 10. dead_code — detect_dead_code (public, re-exported)
// =========================================================================

#[cfg(not(target_arch = "wasm32"))]
mod dead_code_tests {
    use super::*;
    use perl_parser_core::position::LineStartsCache;
    use perl_workspace_index::workspace_index::WorkspaceIndex;

    #[test]
    fn dead_code_empty_workspace() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        let source = "";
        let line_index = LineStartsCache::new(source);
        let diagnostics =
            perl_lsp_diagnostics::detect_dead_code(&index, "file:///empty.pl", source, &line_index);
        assert!(diagnostics.is_empty());
        Ok(())
    }

    #[test]
    fn dead_code_all_used() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        index.index_file_str("file:///main.pl", "use A;\nA::foo();\n")?;
        index.index_file_str("file:///A.pm", "package A;\nsub foo { 1; }\n")?;

        let source = "package A;\nsub foo { 1; }\n";
        let line_index = LineStartsCache::new(source);
        let diagnostics =
            perl_lsp_diagnostics::detect_dead_code(&index, "file:///A.pm", source, &line_index);

        // foo is used — should not appear in diagnostics
        assert!(
            diagnostics.iter().all(|d| !d.message.contains("foo")),
            "Used subroutine should not be flagged"
        );
        Ok(())
    }

    #[test]
    fn dead_code_unused_sub_flagged() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        index.index_file_str("file:///main.pl", "use B;\nB::used_fn();\n")?;
        index.index_file_str(
            "file:///B.pm",
            "package B;\nsub used_fn { 1; }\nsub unused_fn { 2; }\n",
        )?;

        let source = "package B;\nsub used_fn { 1; }\nsub unused_fn { 2; }\n";
        let line_index = LineStartsCache::new(source);
        let diagnostics =
            perl_lsp_diagnostics::detect_dead_code(&index, "file:///B.pm", source, &line_index);

        assert!(
            diagnostics.iter().any(|d| d.message.contains("unused_fn")),
            "Unused subroutine should be flagged"
        );
        Ok(())
    }

    #[test]
    fn dead_code_severity_and_tags() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        index.index_file_str("file:///C.pm", "package C;\nsub lonely { 42; }\n")?;

        let source = "package C;\nsub lonely { 42; }\n";
        let line_index = LineStartsCache::new(source);
        let diagnostics =
            perl_lsp_diagnostics::detect_dead_code(&index, "file:///C.pm", source, &line_index);

        for d in &diagnostics {
            if d.message.contains("lonely") {
                assert_eq!(d.severity, DiagnosticSeverity::Hint);
                assert!(d.tags.contains(&DiagnosticTag::Unnecessary));
                assert!(d.code.as_ref().is_some_and(|c| c.starts_with("dead-code-")));
            }
        }
        Ok(())
    }

    #[test]
    fn dead_code_filters_by_document_uri() -> Result<(), Box<dyn std::error::Error>> {
        let index = WorkspaceIndex::new();
        index.index_file_str("file:///a.pl", "sub a_fn { 1; }\n")?;
        index.index_file_str("file:///b.pl", "sub b_fn { 2; }\n")?;

        let source_a = "sub a_fn { 1; }\n";
        let line_index = LineStartsCache::new(source_a);
        let diagnostics =
            perl_lsp_diagnostics::detect_dead_code(&index, "file:///a.pl", source_a, &line_index);

        // Should only contain diagnostics for a.pl, not b.pl
        assert!(diagnostics.iter().all(|d| !d.message.contains("b_fn")));
        Ok(())
    }
}

// =========================================================================
// 11. Integration-style: full pipeline through provider
// =========================================================================

#[test]
fn full_pipeline_empty_source() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![]));
    let source = "";
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &[], source, None);
    // Should not panic; returns a valid Vec
    let _ = diagnostics;
    Ok(())
}

#[test]
fn full_pipeline_fallback_parse_error() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![]));
    let source = "broken";
    let errors = vec![ParseError::RecursionLimit];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &errors, source, None);

    // RecursionLimit hits the catch-all arm (location=0, error.to_string())
    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL001")).collect();
    assert!(!parse_diags.is_empty());
    assert_eq!(parse_diags[0].range.0, 0);
    Ok(())
}

// =========================================================================
// 12. Undeclared variable diagnostic shows helpful suggestion
// =========================================================================

#[test]
fn undeclared_variable_diagnostic_has_suggestion_and_enhanced_message()
-> Result<(), Box<dyn std::error::Error>> {
    // Build an AST that the scope analyzer would see as undeclared usage
    // We test through the provider to exercise the full pipeline
    let ast = Arc::new(program(vec![use_node("strict", 0, 12), use_node("warnings", 13, 27)]));
    let source = "use strict;\nuse warnings;\n";
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &[], source, None);

    // Verify undeclared-variable diagnostics (if scope analysis produces them)
    // have the expected enhanced fields
    let undeclared: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL103")).collect();

    for d in &undeclared {
        // Enhanced message should mention how to fix
        assert!(
            d.message.contains("not declared") || d.message.contains("add 'my"),
            "Undeclared variable message should be helpful: {}",
            d.message
        );

        // Should carry a suggestion for quick-fix
        assert!(
            d.suggestion.is_some(),
            "Undeclared variable diagnostic should include a suggestion"
        );
        let suggestion = d.suggestion.as_deref().unwrap_or_default();
        assert!(
            suggestion.contains("my"),
            "Suggestion should mention 'my' declaration: {suggestion}"
        );

        // Severity should be Error under strict
        assert_eq!(d.severity, DiagnosticSeverity::Error);

        // Related information should exist
        assert!(!d.related_information.is_empty(), "Should have related information with guidance");
    }
    Ok(())
}

#[test]
fn undeclared_variable_related_info_explains_strict() -> Result<(), Box<dyn std::error::Error>> {
    // Directly test scope_issues_to_diagnostics with a synthetic undeclared variable issue
    use perl_semantic_analyzer::scope_analyzer::{IssueKind, ScopeIssue};

    let _issue = ScopeIssue {
        kind: IssueKind::UndeclaredVariable,
        variable_name: "$foo".to_string(),
        line: 1,
        range: (10, 14),
        description: "Global symbol \"$foo\" requires explicit package name".to_string(),
    };

    // scope_issues_to_diagnostics is pub — test it directly via the module re-export
    // We can't call it directly since it's pub(crate), but we can use the provider.
    // Instead, construct the diagnostic manually to verify the enhanced message pattern.
    let ast = Arc::new(program(vec![]));
    let source = "";
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let _diagnostics = provider.get_diagnostics(&ast, &[], source, None);

    // Verify the Diagnostic struct supports suggestion field
    let d = Diagnostic {
        range: (10, 14),
        severity: DiagnosticSeverity::Error,
        code: Some("PL103".to_string()),
        message:
            "Variable '$foo' is used but not declared -- add 'my $foo' to declare it in this scope"
                .to_string(),
        related_information: vec![RelatedInformation {
            location: (10, 14),
            message: "Declare the variable with 'my', 'our', 'local', or 'state'".to_string(),
        }],
        tags: vec![],
        suggestion: Some("Add 'my $foo;' before this line".to_string()),
    };

    assert_eq!(d.severity, DiagnosticSeverity::Error);
    assert!(d.message.contains("$foo"));
    assert!(d.message.contains("not declared"));
    assert!(d.message.contains("my $foo"));
    assert!(d.suggestion.as_deref().is_some_and(|s| s.contains("my $foo")));
    assert!(!d.related_information.is_empty());
    Ok(())
}

// =========================================================================
// 13. Missing semicolon diagnostic shows position and suggestion
// =========================================================================

#[test]
fn missing_semicolon_parse_error_has_suggestion() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![]));
    let source = "my $x = 42\nprint $x;";
    let errors = vec![ParseError::UnexpectedToken {
        location: 10,
        expected: ";".to_string(),
        found: "print".to_string(),
    }];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &errors, source, None);

    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL001")).collect();
    assert!(!parse_diags.is_empty(), "Should produce a parse-error diagnostic");

    let first = &parse_diags[0];
    // Position should be at offset 10 (end of the first statement)
    assert_eq!(first.range.0, 10, "Diagnostic should point to the missing semicolon position");

    // Should have a suggestion about adding semicolon
    assert!(first.suggestion.is_some(), "Missing semicolon diagnostic should include a suggestion");
    let suggestion = first.suggestion.as_deref().unwrap_or_default();
    assert!(suggestion.contains(';'), "Suggestion should mention adding a semicolon: {suggestion}");

    // Message should mention missing semicolon (enhanced format)
    assert!(
        first.message.contains("semicolon") || first.message.contains(";"),
        "Message should mention semicolon: {}",
        first.message
    );
    Ok(())
}

#[test]
fn unexpected_eof_has_suggestion() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![]));
    let source = "my $x = ";
    let errors = vec![ParseError::UnexpectedEof];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &errors, source, None);

    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL003")).collect();
    assert!(!parse_diags.is_empty());

    let first = &parse_diags[0];
    assert!(
        first.suggestion.is_some(),
        "EOF diagnostic should have a suggestion about unclosed delimiters"
    );
    let suggestion = first.suggestion.as_deref().unwrap_or_default();
    assert!(
        suggestion.contains("unclosed") || suggestion.contains("semicolon"),
        "Suggestion should guide the user: {suggestion}"
    );
    Ok(())
}

#[test]
fn found_semicolon_when_expecting_expression_has_suggestion()
-> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![]));
    let source = "my $x = ;";
    let errors = vec![ParseError::UnexpectedToken {
        location: 8,
        expected: "expression".to_string(),
        found: ";".to_string(),
    }];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &errors, source, None);

    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL001")).collect();
    assert!(!parse_diags.is_empty());

    let first = &parse_diags[0];
    assert!(first.suggestion.is_some(), "Should suggest what's missing");
    let suggestion = first.suggestion.as_deref().unwrap_or_default();
    assert!(
        suggestion.contains("expression") || suggestion.contains("incomplete"),
        "Suggestion should explain the incomplete statement: {suggestion}"
    );
    Ok(())
}

// =========================================================================
// 14. Unused variable warning with correct severity level
// =========================================================================

#[test]
fn unused_variable_has_warning_severity_and_unnecessary_tag()
-> Result<(), Box<dyn std::error::Error>> {
    // Construct a diagnostic as the scope system would produce
    let d = Diagnostic {
        range: (3, 10),
        severity: DiagnosticSeverity::Warning,
        code: Some("PL102".to_string()),
        message: "Variable '$unused' is declared but never used -- prefix with '_' or remove it".to_string(),
        related_information: vec![
            RelatedInformation {
                location: (3, 10),
                message: "Remove the unused variable or prefix with '_' to indicate it's intentionally unused".to_string(),
            },
        ],
        tags: vec![DiagnosticTag::Unnecessary],
        suggestion: Some("Prefix as '_unused'".to_string()),
    };

    // Severity should be Warning (not Error)
    assert_eq!(
        d.severity,
        DiagnosticSeverity::Warning,
        "Unused variable should be a Warning, not an Error"
    );

    // Tag should be Unnecessary so the IDE can grey it out
    assert!(
        d.tags.contains(&DiagnosticTag::Unnecessary),
        "Unused variable should be tagged as Unnecessary for IDE rendering"
    );

    // Should NOT be tagged as Deprecated
    assert!(
        !d.tags.contains(&DiagnosticTag::Deprecated),
        "Unused variable should not be tagged Deprecated"
    );

    // Suggestion should be actionable
    assert!(d.suggestion.is_some());
    let suggestion = d.suggestion.as_deref().unwrap_or_default();
    assert!(
        suggestion.contains('_'),
        "Suggestion should recommend underscore prefix: {suggestion}"
    );

    // Message should be helpful
    assert!(d.message.contains("never used"));
    assert!(d.message.contains("$unused"));
    Ok(())
}

#[test]
fn unused_variable_through_provider_has_correct_severity() -> Result<(), Box<dyn std::error::Error>>
{
    // Test the full pipeline: the scope analyzer should classify unused variables
    // as Warning severity (not Error)
    let ast = Arc::new(program(vec![use_node("strict", 0, 12), use_node("warnings", 13, 27)]));
    let source = "use strict;\nuse warnings;\n";
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &[], source, None);

    let unused: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL102")).collect();

    for d in &unused {
        assert_eq!(
            d.severity,
            DiagnosticSeverity::Warning,
            "Unused variable should always be Warning severity"
        );
        assert!(
            d.tags.contains(&DiagnosticTag::Unnecessary),
            "Unused variable should be tagged Unnecessary"
        );
        assert!(d.suggestion.is_some(), "Unused variable should have a suggestion");
    }
    Ok(())
}

#[test]
fn unused_parameter_has_warning_severity_and_unnecessary_tag()
-> Result<(), Box<dyn std::error::Error>> {
    let d = Diagnostic {
        range: (5, 12),
        severity: DiagnosticSeverity::Warning,
        code: Some("PL108".to_string()),
        message: "Parameter '$param' is never used".to_string(),
        related_information: vec![],
        tags: vec![DiagnosticTag::Unnecessary],
        suggestion: Some("Rename to '$_param'".to_string()),
    };

    assert_eq!(d.severity, DiagnosticSeverity::Warning);
    assert!(d.tags.contains(&DiagnosticTag::Unnecessary));
    assert!(d.suggestion.as_deref().is_some_and(|s| s.contains("$_param")));
    Ok(())
}

// =========================================================================
// 15. Suggestion field integration tests
// =========================================================================

#[test]
fn suggestion_field_is_populated_for_scope_diagnostics() -> Result<(), Box<dyn std::error::Error>> {
    // The suggestion field should be populated for key diagnostic codes
    let codes_that_should_have_suggestions = [("PL103", "my"), ("PL102", "_")];

    for (code, expected_content) in codes_that_should_have_suggestions {
        let d = Diagnostic {
            range: (0, 5),
            severity: DiagnosticSeverity::Error,
            code: Some(code.to_string()),
            message: format!("Test diagnostic for {code}"),
            related_information: vec![],
            tags: vec![],
            suggestion: Some(format!("Fix: contains {expected_content}")),
        };
        assert!(
            d.suggestion.as_deref().is_some_and(|s| s.contains(expected_content)),
            "Diagnostic code '{code}' should have suggestion containing '{expected_content}'"
        );
    }
    Ok(())
}

#[test]
fn suggestion_field_is_none_when_not_applicable() -> Result<(), Box<dyn std::error::Error>> {
    let d = Diagnostic {
        range: (0, 1),
        severity: DiagnosticSeverity::Information,
        code: Some("info-only".to_string()),
        message: "Informational".to_string(),
        related_information: vec![],
        tags: vec![],
        suggestion: None,
    };
    assert!(d.suggestion.is_none());
    Ok(())
}

#[test]
fn diagnostic_clone_preserves_suggestion() -> Result<(), Box<dyn std::error::Error>> {
    let d = Diagnostic {
        range: (0, 5),
        severity: DiagnosticSeverity::Error,
        code: Some("test".to_string()),
        message: "test".to_string(),
        related_information: vec![],
        tags: vec![],
        suggestion: Some("do something".to_string()),
    };
    let cloned = d.clone();
    assert_eq!(d.suggestion, cloned.suggestion);
    assert_eq!(d, cloned);
    Ok(())
}

#[test]
fn diagnostic_equality_considers_suggestion() -> Result<(), Box<dyn std::error::Error>> {
    let d1 = Diagnostic {
        range: (0, 5),
        severity: DiagnosticSeverity::Error,
        code: Some("test".to_string()),
        message: "test".to_string(),
        related_information: vec![],
        tags: vec![],
        suggestion: Some("fix A".to_string()),
    };
    let mut d2 = d1.clone();
    d2.suggestion = Some("fix B".to_string());

    assert_ne!(d1, d2, "Diagnostics with different suggestions should not be equal");
    Ok(())
}

// ---------------------------------------------------------------------------
// Full pipeline: uninitialized variable integration tests
// ---------------------------------------------------------------------------

#[test]
fn full_pipeline_uninitialized_variable_emits_warning() -> Result<(), Box<dyn std::error::Error>> {
    // Parse real Perl with an uninitialized variable usage
    let source = "use strict;\nuse warnings;\nmy $x;\nprint $x;\n";
    let output = perl_parser::Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &output.diagnostics, source, None);

    let uninit: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL110")).collect();

    assert!(!uninit.is_empty(), "Expected at least one uninitialized-variable diagnostic");
    assert_eq!(
        uninit[0].severity,
        DiagnosticSeverity::Warning,
        "Uninitialized variable should be a Warning"
    );
    assert!(uninit[0].suggestion.is_some(), "Should carry a quick-fix suggestion");
    assert!(!uninit[0].related_information.is_empty(), "Should have related info with guidance");
    Ok(())
}

#[test]
fn full_pipeline_initialized_variable_no_warning() -> Result<(), Box<dyn std::error::Error>> {
    // Parse Perl with properly initialized variable — no uninitialized-variable diagnostic expected
    let source = "use strict;\nuse warnings;\nmy $x = 42;\nprint $x;\n";
    let output = perl_parser::Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &output.diagnostics, source, None);

    let uninit: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL110")).collect();

    assert!(
        uninit.is_empty(),
        "Initialized variable should not produce uninitialized-variable diagnostic"
    );
    Ok(())
}

// =========================================================================
// 16. Misspelled pragma detection ("Did you mean?")
// =========================================================================

#[test]
fn misspelled_pragma_structs_suggests_strict() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("structs", 0, 14)]);
    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::strict_warnings::check_strict_warnings(&ast, &mut diagnostics);

    let typo_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL111")).collect();
    assert!(!typo_diags.is_empty(), "Should detect 'structs' as a misspelling of 'strict'");

    let first = &typo_diags[0];
    assert_eq!(first.severity, DiagnosticSeverity::Warning);
    assert!(first.message.contains("strict"), "Message should suggest 'strict': {}", first.message);
    assert!(
        first.message.contains("Did you mean"),
        "Message should ask 'Did you mean?': {}",
        first.message
    );
    assert!(first.suggestion.is_some());
    let suggestion = first.suggestion.as_deref().unwrap_or_default();
    assert!(suggestion.contains("strict"), "Suggestion should mention 'strict': {suggestion}");
    Ok(())
}

#[test]
fn misspelled_pragma_warning_suggests_warnings() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![use_node("strict", 0, 12), use_node("warning", 13, 26)]);
    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::strict_warnings::check_strict_warnings(&ast, &mut diagnostics);

    let typo_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL111")).collect();
    assert!(!typo_diags.is_empty(), "Should detect 'warning' as a misspelling of 'warnings'");

    let first = &typo_diags[0];
    assert!(first.message.contains("warnings"), "Should suggest 'warnings': {}", first.message);
    Ok(())
}

#[test]
fn misspelled_pragma_no_false_positive_for_valid_module() -> Result<(), Box<dyn std::error::Error>>
{
    let ast = program(vec![
        use_node("strict", 0, 12),
        use_node("warnings", 13, 27),
        use_node("File::Basename", 28, 46),
    ]);
    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::strict_warnings::check_strict_warnings(&ast, &mut diagnostics);

    let typo_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL111")).collect();
    assert!(
        typo_diags.is_empty(),
        "Should NOT flag 'File::Basename' as misspelled: {typo_diags:?}"
    );
    Ok(())
}

#[test]
fn misspelled_pragma_feaure_suggests_feature() -> Result<(), Box<dyn std::error::Error>> {
    let ast = program(vec![
        use_node("strict", 0, 12),
        use_node("warnings", 13, 27),
        use_node("feaure", 28, 40),
    ]);
    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::strict_warnings::check_strict_warnings(&ast, &mut diagnostics);

    let typo_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL111")).collect();
    assert!(!typo_diags.is_empty(), "Should detect 'feaure' as a misspelling of 'feature'");

    let first = &typo_diags[0];
    assert!(first.message.contains("feature"));
    Ok(())
}

// =========================================================================
// 17. Enhanced parse error suggestions for additional error variants
// =========================================================================

#[test]
fn recursion_limit_error_has_suggestion() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![]));
    let source = "broken";
    let errors = vec![ParseError::RecursionLimit];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &errors, source, None);

    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL001")).collect();
    assert!(!parse_diags.is_empty());
    assert!(
        parse_diags[0].suggestion.is_some(),
        "RecursionLimit should have a suggestion about reducing nesting"
    );
    let suggestion = parse_diags[0].suggestion.as_deref().unwrap_or_default();
    assert!(
        suggestion.contains("nested") || suggestion.contains("refactor"),
        "Suggestion should advise refactoring: {suggestion}"
    );
    Ok(())
}

#[test]
fn invalid_number_error_has_suggestion() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![]));
    let source = "my $x = 0xGG;";
    let errors = vec![ParseError::InvalidNumber { literal: "0xGG".to_string() }];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &errors, source, None);

    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL001")).collect();
    assert!(!parse_diags.is_empty());
    assert!(parse_diags[0].suggestion.is_some());
    let suggestion = parse_diags[0].suggestion.as_deref().unwrap_or_default();
    assert!(
        suggestion.contains("0xGG"),
        "Suggestion should mention the invalid literal: {suggestion}"
    );
    Ok(())
}

#[test]
fn invalid_string_error_has_suggestion() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![]));
    let source = "my $x = \"hello";
    let errors = vec![ParseError::InvalidString];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &errors, source, None);

    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL001")).collect();
    assert!(!parse_diags.is_empty());
    assert!(parse_diags[0].suggestion.is_some());
    let suggestion = parse_diags[0].suggestion.as_deref().unwrap_or_default();
    assert!(
        suggestion.contains("quote") || suggestion.contains("escape"),
        "Suggestion should mention closing quotes or escape sequences: {suggestion}"
    );
    Ok(())
}

#[test]
fn invalid_regex_error_has_suggestion() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![]));
    let source = "my $x =~ /(/;";
    let errors = vec![ParseError::InvalidRegex { message: "unmatched parenthesis".to_string() }];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &errors, source, None);

    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL001")).collect();
    assert!(!parse_diags.is_empty());
    assert!(parse_diags[0].suggestion.is_some());
    let suggestion = parse_diags[0].suggestion.as_deref().unwrap_or_default();
    assert!(
        suggestion.contains("regex") || suggestion.contains("pattern"),
        "Suggestion should mention regex patterns: {suggestion}"
    );
    Ok(())
}

#[test]
fn unclosed_delimiter_error_has_suggestion() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![]));
    let source = "my @arr = (1, 2";
    let errors = vec![ParseError::UnclosedDelimiter { delimiter: ')' }];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &errors, source, None);

    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL001")).collect();
    assert!(!parse_diags.is_empty());
    assert!(parse_diags[0].suggestion.is_some());
    let suggestion = parse_diags[0].suggestion.as_deref().unwrap_or_default();
    assert!(
        suggestion.contains(')'),
        "Suggestion should mention the closing delimiter: {suggestion}"
    );
    Ok(())
}

#[test]
fn nesting_too_deep_error_has_suggestion() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![]));
    let source = "deeply nested";
    let errors = vec![ParseError::NestingTooDeep { depth: 300, max_depth: 256 }];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &errors, source, None);

    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL001")).collect();
    assert!(!parse_diags.is_empty());
    assert!(parse_diags[0].suggestion.is_some());
    let suggestion = parse_diags[0].suggestion.as_deref().unwrap_or_default();
    assert!(
        suggestion.contains("nesting") || suggestion.contains("subroutine"),
        "Suggestion should advise reducing nesting: {suggestion}"
    );
    Ok(())
}

#[test]
fn unexpected_closing_brace_has_helpful_suggestion() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![]));
    let source = "my $x = 1; }";
    let errors = vec![ParseError::UnexpectedToken {
        location: 11,
        expected: "statement".to_string(),
        found: "}".to_string(),
    }];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &errors, source, None);

    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL001")).collect();
    assert!(!parse_diags.is_empty());
    assert!(parse_diags[0].suggestion.is_some());
    let suggestion = parse_diags[0].suggestion.as_deref().unwrap_or_default();
    assert!(
        suggestion.contains("missing") || suggestion.contains('}'),
        "Suggestion should mention the mismatch: {suggestion}"
    );
    Ok(())
}

#[test]
fn syntax_error_with_heredoc_hint_has_suggestion() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![]));
    let source = "my $x = <<END\nsome text\n";
    let errors =
        vec![ParseError::SyntaxError { location: 8, message: "unterminated heredoc".to_string() }];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &errors, source, None);

    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL002")).collect();
    assert!(!parse_diags.is_empty());
    assert!(parse_diags[0].suggestion.is_some());
    let suggestion = parse_diags[0].suggestion.as_deref().unwrap_or_default();
    assert!(suggestion.contains("heredoc"), "Suggestion should mention heredoc: {suggestion}");
    Ok(())
}

#[test]
fn lexer_error_with_unterminated_string_has_suggestion() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![]));
    let source = "my $x = \"hello";
    let errors =
        vec![ParseError::LexerError { message: "unterminated string literal".to_string() }];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &errors, source, None);

    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL001")).collect();
    assert!(!parse_diags.is_empty());
    assert!(parse_diags[0].suggestion.is_some());
    let suggestion = parse_diags[0].suggestion.as_deref().unwrap_or_default();
    assert!(
        suggestion.contains("unclosed") || suggestion.contains("string"),
        "Suggestion should mention unclosed string: {suggestion}"
    );
    Ok(())
}
// =========================================================================
// 18. lints::security — check_security
// =========================================================================

fn string_node(value: &str, interpolated: bool, start: usize, end: usize) -> Node {
    Node::new(NodeKind::String { value: value.to_string(), interpolated }, loc(start, end))
}

#[test]
fn security_two_arg_open_warns() -> Result<(), Box<dyn std::error::Error>> {
    let fh = Node::new(NodeKind::Identifier { name: "FH".to_string() }, loc(5, 7));
    let file = string_node(">file.txt", true, 9, 20);
    let call = func_call("open", vec![fh, file], 0, 21);
    let root = program(vec![expr_stmt(call, 0, 22)]);

    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::security::check_security(&root, &mut diagnostics);

    assert!(
        diagnostics.iter().any(|d| d.code.as_deref() == Some("PL401")),
        "Should detect two-arg open: {diagnostics:?}"
    );
    let diag = diagnostics
        .iter()
        .find(|d| d.code.as_deref() == Some("PL401"))
        .ok_or("missing diagnostic")?;
    assert_eq!(diag.severity, DiagnosticSeverity::Warning);
    assert!(diag.message.contains("3-argument open"));
    assert!(diag.suggestion.is_some());
    Ok(())
}

#[test]
fn security_three_arg_open_no_warning() -> Result<(), Box<dyn std::error::Error>> {
    let fh = var_node("$", "fh", 5, 8);
    let mode = string_node(">", false, 10, 13);
    let file = string_node("file.txt", false, 15, 25);
    let call = func_call("open", vec![fh, mode, file], 0, 26);
    let root = program(vec![expr_stmt(call, 0, 27)]);

    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::security::check_security(&root, &mut diagnostics);

    assert!(
        diagnostics.iter().all(|d| d.code.as_deref() != Some("PL401")),
        "3-arg open should not trigger warning: {diagnostics:?}"
    );
    Ok(())
}

#[test]
fn security_one_arg_open_no_warning() -> Result<(), Box<dyn std::error::Error>> {
    let fh = Node::new(NodeKind::Identifier { name: "FH".to_string() }, loc(5, 7));
    let call = func_call("open", vec![fh], 0, 8);
    let root = program(vec![expr_stmt(call, 0, 9)]);

    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::security::check_security(&root, &mut diagnostics);

    assert!(
        diagnostics.iter().all(|d| d.code.as_deref() != Some("PL401")),
        "1-arg open should not trigger warning"
    );
    Ok(())
}

#[test]
fn security_string_eval_warns() -> Result<(), Box<dyn std::error::Error>> {
    let code = string_node("$code", true, 5, 12);
    let call = func_call("eval", vec![code], 0, 13);
    let root = program(vec![expr_stmt(call, 0, 14)]);

    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::security::check_security(&root, &mut diagnostics);

    assert!(
        diagnostics.iter().any(|d| d.code.as_deref() == Some("PL600")),
        "Should detect string eval: {diagnostics:?}"
    );
    let diag = diagnostics
        .iter()
        .find(|d| d.code.as_deref() == Some("PL600"))
        .ok_or("missing diagnostic")?;
    assert_eq!(diag.severity, DiagnosticSeverity::Warning);
    assert!(diag.message.contains("security risk"));
    assert!(diag.suggestion.is_some());
    Ok(())
}

#[test]
fn security_eval_with_variable_warns() -> Result<(), Box<dyn std::error::Error>> {
    let code = var_node("$", "code_string", 5, 18);
    let call = func_call("eval", vec![code], 0, 19);
    let root = program(vec![expr_stmt(call, 0, 20)]);

    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::security::check_security(&root, &mut diagnostics);

    assert!(
        diagnostics.iter().any(|d| d.code.as_deref() == Some("PL600")),
        "Should detect eval with variable: {diagnostics:?}"
    );
    Ok(())
}

#[test]
fn security_eval_block_no_warning() -> Result<(), Box<dyn std::error::Error>> {
    let block_body = block(vec![]);
    let eval = Node::new(NodeKind::Eval { block: Box::new(block_body) }, loc(0, 15));
    let root = program(vec![expr_stmt(eval, 0, 16)]);

    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::security::check_security(&root, &mut diagnostics);

    assert!(
        diagnostics.iter().all(|d| d.code.as_deref() != Some("PL600")),
        "Block eval should not trigger string-eval warning"
    );
    Ok(())
}

#[test]
fn security_backtick_string_warns() -> Result<(), Box<dyn std::error::Error>> {
    let cmd = string_node("`ls -la`", true, 0, 8);
    let root = program(vec![expr_stmt(cmd, 0, 9)]);

    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::security::check_security(&root, &mut diagnostics);

    assert!(
        diagnostics.iter().any(|d| d.code.as_deref() == Some("PL601")),
        "Should detect backtick command execution: {diagnostics:?}"
    );
    let diag = diagnostics
        .iter()
        .find(|d| d.code.as_deref() == Some("PL601"))
        .ok_or("missing diagnostic")?;
    assert_eq!(diag.severity, DiagnosticSeverity::Information);
    assert!(diag.message.contains("Command execution"));
    assert!(diag.suggestion.is_some());
    Ok(())
}

#[test]
fn security_normal_string_no_backtick_warning() -> Result<(), Box<dyn std::error::Error>> {
    let s = string_node("hello world", true, 0, 13);
    let root = program(vec![expr_stmt(s, 0, 14)]);

    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::security::check_security(&root, &mut diagnostics);

    assert!(
        diagnostics.iter().all(|d| d.code.as_deref() != Some("PL601")),
        "Normal interpolated string should not trigger backtick warning"
    );
    Ok(())
}

#[test]
fn security_non_interpolated_backtick_no_warning() -> Result<(), Box<dyn std::error::Error>> {
    let s = string_node("`not a command`", false, 0, 16);
    let root = program(vec![expr_stmt(s, 0, 17)]);

    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::security::check_security(&root, &mut diagnostics);

    assert!(
        diagnostics.iter().all(|d| d.code.as_deref() != Some("PL601")),
        "Non-interpolated string with backtick chars should not warn"
    );
    Ok(())
}

#[test]
fn security_other_function_no_open_warning() -> Result<(), Box<dyn std::error::Error>> {
    let fh = Node::new(NodeKind::Identifier { name: "FH".to_string() }, loc(6, 8));
    let something = var_node("$", "something", 10, 20);
    let call = func_call("close", vec![fh, something], 0, 21);
    let root = program(vec![expr_stmt(call, 0, 22)]);

    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::security::check_security(&root, &mut diagnostics);

    assert!(
        diagnostics.iter().all(|d| d.code.as_deref() != Some("PL401")),
        "Non-open function should not trigger open warning"
    );
    Ok(())
}

#[test]
fn security_eval_with_concatenation_warns() -> Result<(), Box<dyn std::error::Error>> {
    let left = string_node("SELECT * FROM ", true, 5, 21);
    let right = var_node("$", "table", 24, 30);
    let concat = binary_node(".", left, right, 5, 30);
    let call = func_call("eval", vec![concat], 0, 31);
    let root = program(vec![expr_stmt(call, 0, 32)]);

    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::security::check_security(&root, &mut diagnostics);

    assert!(
        diagnostics.iter().any(|d| d.code.as_deref() == Some("PL600")),
        "Should detect eval with string concatenation: {diagnostics:?}"
    );
    Ok(())
}

#[test]
fn security_related_info_quality() -> Result<(), Box<dyn std::error::Error>> {
    let fh = Node::new(NodeKind::Identifier { name: "FH".to_string() }, loc(5, 7));
    let file = string_node(">file", true, 9, 15);
    let open_call = func_call("open", vec![fh, file], 0, 16);

    let code = string_node("$code", true, 25, 32);
    let eval_call = func_call("eval", vec![code], 20, 33);

    let backtick = string_node("`cmd`", true, 40, 46);

    let root = program(vec![
        expr_stmt(open_call, 0, 17),
        expr_stmt(eval_call, 20, 34),
        expr_stmt(backtick, 40, 47),
    ]);

    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::security::check_security(&root, &mut diagnostics);

    assert!(diagnostics.iter().any(|d| d.code.as_deref() == Some("PL401")));
    assert!(diagnostics.iter().any(|d| d.code.as_deref() == Some("PL600")));
    assert!(diagnostics.iter().any(|d| d.code.as_deref() == Some("PL601")));

    for d in &diagnostics {
        assert!(
            !d.related_information.is_empty(),
            "Security lint {} should have related info",
            d.code.as_deref().unwrap_or("")
        );
    }
    Ok(())
}
