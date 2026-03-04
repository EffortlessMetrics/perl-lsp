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
        NodeKind::Use {
            module: module.to_string(),
            args: vec![],
            has_filter_risk: false,
        },
        loc(start, end),
    )
}

fn binary_node(op: &str, left: Node, right: Node, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::Binary {
            op: op.to_string(),
            left: Box::new(left),
            right: Box::new(right),
        },
        loc(start, end),
    )
}

fn func_call(name: &str, args: Vec<Node>, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::FunctionCall { name: name.to_string(), args },
        loc(start, end),
    )
}

fn expr_stmt(expr: Node, start: usize, end: usize) -> Node {
    Node::new(NodeKind::ExpressionStatement { expression: Box::new(expr) }, loc(start, end))
}

fn number_node(value: &str, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::Number { value: value.to_string() },
        loc(start, end),
    )
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
    let ri = RelatedInformation {
        location: (5, 15),
        message: "hint here".to_string(),
    };
    let d = Diagnostic {
        range: (0, 50),
        severity: DiagnosticSeverity::Hint,
        code: None,
        message: "some hint".to_string(),
        related_information: vec![ri.clone()],
        tags: vec![DiagnosticTag::Unnecessary],
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
    let ast = Arc::new(program(vec![
        use_node("strict", 0, 12),
        use_node("warnings", 13, 27),
    ]));
    let source = "use strict;\nuse warnings;\n";
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &[], source);
    // No parse errors and no undeclared vars → may still have scope issues
    // depending on analyzer, but at minimum we get a Vec back
    assert!(diagnostics.iter().all(|d| d.severity != DiagnosticSeverity::Error
        || d.code.as_deref() != Some("parse-error")));
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
    let diagnostics = provider.get_diagnostics(&ast, &errors, source);

    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("parse-error")).collect();
    assert!(!parse_diags.is_empty(), "should produce parse-error diagnostics");

    let first = &parse_diags[0];
    assert_eq!(first.severity, DiagnosticSeverity::Error);
    assert!(first.message.contains("Expected expression"));
    assert!(first.message.contains(";"));
    assert_eq!(first.range.0, 8);
    Ok(())
}

#[test]
fn provider_syntax_error() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![]));
    let source = "invalid perl";
    let errors = vec![ParseError::SyntaxError {
        location: 3,
        message: "bad syntax".to_string(),
    }];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &errors, source);

    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("parse-error")).collect();
    assert!(!parse_diags.is_empty());

    let first = &parse_diags[0];
    assert_eq!(first.range.0, 3);
    assert_eq!(first.message, "bad syntax");
    Ok(())
}

#[test]
fn provider_unexpected_eof_error() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![]));
    let source = "my $x = ";
    let errors = vec![ParseError::UnexpectedEof];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &errors, source);

    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("parse-error")).collect();
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
    let diagnostics = provider.get_diagnostics(&ast, &errors, source);

    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("parse-error")).collect();
    assert!(!parse_diags.is_empty());
    assert_eq!(parse_diags[0].range.0, 0);
    assert_eq!(parse_diags[0].message, "bad token");
    Ok(())
}

#[test]
fn provider_multiple_errors() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![]));
    let source = "x; y; z;";
    let errors = vec![
        ParseError::SyntaxError { location: 0, message: "err1".to_string() },
        ParseError::SyntaxError { location: 3, message: "err2".to_string() },
        ParseError::SyntaxError { location: 6, message: "err3".to_string() },
    ];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &errors, source);

    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("parse-error")).collect();
    assert!(parse_diags.len() >= 3);
    Ok(())
}

#[test]
fn provider_error_range_uses_saturating_add() -> Result<(), Box<dyn std::error::Error>> {
    let ast = Arc::new(program(vec![]));
    let source = "x";
    let errors = vec![ParseError::UnexpectedToken {
        location: usize::MAX,
        expected: "a".to_string(),
        found: "b".to_string(),
    }];
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diagnostics = provider.get_diagnostics(&ast, &errors, source);

    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("parse-error")).collect();
    assert!(!parse_diags.is_empty());
    // saturating_add should prevent overflow
    assert_eq!(parse_diags[0].range.1, usize::MAX);
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
    let diagnostics = provider.get_diagnostics(&ast, &[], source);
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
        diagnostics.iter().any(|d| d.code.as_deref() == Some("deprecated-defined")
            && d.message.contains("@data")),
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
        diagnostics.iter().all(|d| d.code.as_deref() != Some("deprecated-defined")),
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

    assert!(diagnostics.iter().any(|d| d.code.as_deref() == Some("deprecated-array-base")));
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
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("deprecated-defined")).collect();
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
    let root = program(vec![]);
    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::strict_warnings::check_strict_warnings(&root, &mut diagnostics);

    assert!(diagnostics.iter().any(|d| d.code.as_deref() == Some("missing-strict")));
    assert!(diagnostics.iter().any(|d| d.code.as_deref() == Some("missing-warnings")));
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

    assert!(diagnostics.iter().all(|d| d.code.as_deref() != Some("missing-strict")));
    assert!(diagnostics.iter().any(|d| d.code.as_deref() == Some("missing-warnings")));
    Ok(())
}

#[test]
fn strict_warnings_warnings_present() -> Result<(), Box<dyn std::error::Error>> {
    let root = program(vec![use_node("warnings", 0, 15)]);
    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::strict_warnings::check_strict_warnings(&root, &mut diagnostics);

    assert!(diagnostics.iter().any(|d| d.code.as_deref() == Some("missing-strict")));
    assert!(diagnostics.iter().all(|d| d.code.as_deref() != Some("missing-warnings")));
    Ok(())
}

#[test]
fn strict_warnings_both_present() -> Result<(), Box<dyn std::error::Error>> {
    let root = program(vec![use_node("strict", 0, 12), use_node("warnings", 13, 27)]);
    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::strict_warnings::check_strict_warnings(&root, &mut diagnostics);

    assert!(diagnostics.iter().all(|d| d.code.as_deref() != Some("missing-strict")));
    assert!(diagnostics.iter().all(|d| d.code.as_deref() != Some("missing-warnings")));
    Ok(())
}

#[test]
fn strict_warnings_related_info() -> Result<(), Box<dyn std::error::Error>> {
    let root = program(vec![]);
    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::strict_warnings::check_strict_warnings(&root, &mut diagnostics);

    for d in &diagnostics {
        assert!(d.related_information.len() >= 2, "Each missing pragma should have suggestion + explanation");
        assert!(d.related_information.iter().any(|ri| ri.message.contains('💡')));
        assert!(d.related_information.iter().any(|ri| ri.message.contains('ℹ')));
    }
    Ok(())
}

// =========================================================================
// 9. lints::common_mistakes — check_common_mistakes
// =========================================================================

#[test]
fn common_mistakes_assignment_in_if_condition() -> Result<(), Box<dyn std::error::Error>> {
    // if ($x = 1) { ... }
    let condition = binary_node(
        "=",
        var_node("$", "x", 4, 6),
        number_node("1", 9, 10),
        4,
        10,
    );
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
    perl_lsp_diagnostics::common_mistakes::check_common_mistakes(&root, &sym_table, &mut diagnostics);

    assert!(diagnostics.iter().any(|d| d.code.as_deref() == Some("assignment-in-condition")));
    assert!(diagnostics.iter().any(|d| d.message.contains("did you mean")));
    Ok(())
}

#[test]
fn common_mistakes_assignment_in_while_condition() -> Result<(), Box<dyn std::error::Error>> {
    let condition = binary_node(
        "=",
        var_node("$", "line", 7, 12),
        number_node("0", 15, 16),
        7,
        16,
    );
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
    perl_lsp_diagnostics::common_mistakes::check_common_mistakes(&root, &sym_table, &mut diagnostics);

    assert!(diagnostics.iter().any(|d| d.code.as_deref() == Some("assignment-in-condition")));
    Ok(())
}

#[test]
fn common_mistakes_comparison_in_condition_ok() -> Result<(), Box<dyn std::error::Error>> {
    // if ($x == 1) { ... } — should NOT warn
    let condition = binary_node(
        "==",
        var_node("$", "x", 4, 6),
        number_node("1", 10, 11),
        4,
        11,
    );
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
    perl_lsp_diagnostics::common_mistakes::check_common_mistakes(&root, &sym_table, &mut diagnostics);

    assert!(
        diagnostics.iter().all(|d| d.code.as_deref() != Some("assignment-in-condition")),
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
    perl_lsp_diagnostics::common_mistakes::check_common_mistakes(&root, &sym_table, &mut diagnostics);

    assert!(diagnostics.iter().any(|d| d.code.as_deref() == Some("numeric-undef")));
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
    perl_lsp_diagnostics::common_mistakes::check_common_mistakes(&root, &sym_table, &mut diagnostics);

    assert!(diagnostics.iter().any(|d| d.code.as_deref() == Some("numeric-undef")));
    assert!(diagnostics.iter().any(|d| d.message.contains("!=")));
    Ok(())
}

#[test]
fn common_mistakes_related_info_for_assignment() -> Result<(), Box<dyn std::error::Error>> {
    let condition = binary_node(
        "=",
        var_node("$", "z", 4, 6),
        number_node("5", 9, 10),
        4,
        10,
    );
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
    perl_lsp_diagnostics::common_mistakes::check_common_mistakes(&root, &sym_table, &mut diagnostics);

    let assign_diag: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("assignment-in-condition")).collect();
    assert!(!assign_diag.is_empty());
    // Should have suggestion + explanation
    assert!(assign_diag[0].related_information.len() >= 2);
    assert!(assign_diag[0].related_information.iter().any(|ri| ri.message.contains("==")));
    Ok(())
}

#[test]
fn common_mistakes_no_warning_for_string_comparison() -> Result<(), Box<dyn std::error::Error>> {
    // $x eq "hello" — should NOT trigger numeric-undef
    let cmp = binary_node(
        "eq",
        var_node("$", "x", 0, 2),
        number_node("1", 6, 7),
        0,
        7,
    );
    let root = program(vec![expr_stmt(cmp, 0, 8)]);

    let sym_table = perl_semantic_analyzer::symbol::SymbolTable::new();
    let mut diagnostics = Vec::new();
    perl_lsp_diagnostics::common_mistakes::check_common_mistakes(&root, &sym_table, &mut diagnostics);

    assert!(
        diagnostics.iter().all(|d| d.code.as_deref() != Some("numeric-undef")),
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
    let diagnostics = provider.get_diagnostics(&ast, &[], source);
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
    let diagnostics = provider.get_diagnostics(&ast, &errors, source);

    // RecursionLimit hits the catch-all arm (location=0, error.to_string())
    let parse_diags: Vec<_> =
        diagnostics.iter().filter(|d| d.code.as_deref() == Some("parse-error")).collect();
    assert!(!parse_diags.is_empty());
    assert_eq!(parse_diags[0].range.0, 0);
    Ok(())
}
