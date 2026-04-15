//! Hover documentation tests for POD embedded *inside* subroutine bodies.
//!
//! Perl permits POD blocks anywhere a statement is valid, including inside a
//! `sub { ... }` body. Historically the semantic analyzer only looked backwards
//! from the subroutine's start position to collect docs — so inline POD was
//! dropped on the floor. Issue #3407 tracks this gap.
//!
//! POD in Perl must start at column 0 (the `=<directive>` line has no leading
//! whitespace); the lexer in `perl-lexer` enforces that rule. These tests use
//! column-0 POD directives accordingly and verify that the analyzer falls back
//! to scanning the subroutine body for inline POD when no preceding doc block
//! is present.

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::semantic::SemanticAnalyzer;
use perl_tdd_support::{must, must_some};

fn sub_hover_doc(code: &str, sub_name: &str) -> Option<String> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    let symbols = analyzer.symbol_table().find_symbol(
        sub_name,
        0,
        perl_semantic_analyzer::analysis::symbol::SymbolKind::Subroutine,
    );
    let symbol = symbols.first()?;
    let hover = analyzer.hover_at(symbol.location)?;
    hover.documentation.clone()
}

#[test]
fn pod_inline_in_sub_body_surfaces_in_hover() {
    let code = "sub process_data {
=pod

Internal documentation for this sub

=cut
    my $data = shift;
    return $data;
}
";
    let doc = must_some(sub_hover_doc(code, "process_data"));
    assert!(
        doc.contains("Internal documentation for this sub"),
        "hover doc should include inline POD content; got: {doc:?}"
    );
}

#[test]
fn pod_head1_inside_sub_body_surfaces_in_hover() {
    let code = "sub compute {
=head1 DESCRIPTION

Computes a value from the arguments.

=cut
    return 42;
}
";
    let doc = must_some(sub_hover_doc(code, "compute"));
    assert!(
        doc.contains("Computes a value from the arguments"),
        "hover doc should include inline =head1 POD content; got: {doc:?}"
    );
}

#[test]
fn preceding_comment_wins_over_inline_pod() {
    // Preceding documentation remains the canonical source when both are
    // present — the inline POD is only a fallback.
    let code = "# Adds two numbers together
sub add {
=pod

Internal details nobody should read.

=cut
    my ($x, $y) = @_;
    return $x + $y;
}
";
    let doc = must_some(sub_hover_doc(code, "add"));
    assert!(
        doc.contains("Adds two numbers together"),
        "preceding comment should win; got: {doc:?}"
    );
    assert!(
        !doc.contains("Internal details nobody should read"),
        "preceding comment should suppress inline POD fallback; got: {doc:?}"
    );
}

#[test]
fn preceding_pod_wins_over_inline_pod() {
    let code = "=head1 NAME

pick - returns the first element

=cut
sub pick {
=pod

fallback text

=cut
    return shift;
}
";
    let doc = must_some(sub_hover_doc(code, "pick"));
    assert!(
        doc.contains("pick - returns the first element"),
        "preceding POD should win over inline POD; got: {doc:?}"
    );
}

#[test]
fn sub_without_any_pod_has_no_doc() {
    // Regression guard: subs with no documentation in or before the body
    // should continue to report `None`, not an empty string or a stray match.
    let code = "sub plain {
    my $x = shift;
    return $x;
}
";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    let symbols = analyzer.symbol_table().find_symbol(
        "plain",
        0,
        perl_semantic_analyzer::analysis::symbol::SymbolKind::Subroutine,
    );
    let symbol = must_some(symbols.first());
    let hover = must_some(analyzer.hover_at(symbol.location));
    assert!(
        hover.documentation.is_none(),
        "sub with no documentation should report no hover doc; got: {:?}",
        hover.documentation
    );
}

#[test]
fn pod_in_body_does_not_bleed_to_later_sub() {
    // When sub A has inline POD, sub B should not accidentally inherit it.
    let code = "sub first {
=pod

Documents first only.

=cut
    return 1;
}

sub second {
    return 2;
}
";
    let first_doc = must_some(sub_hover_doc(code, "first"));
    assert!(
        first_doc.contains("Documents first only"),
        "first's hover should include its inline POD; got: {first_doc:?}"
    );

    // second has no POD of its own. Its hover entry exists, but its
    // documentation should not contain first's inline POD.
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);
    let symbols = analyzer.symbol_table().find_symbol(
        "second",
        0,
        perl_semantic_analyzer::analysis::symbol::SymbolKind::Subroutine,
    );
    let symbol = must_some(symbols.first());
    let hover = must_some(analyzer.hover_at(symbol.location));
    assert!(
        hover.documentation.as_deref().is_none_or(|d| !d.contains("Documents first only")),
        "second should not inherit first's inline POD; got: {:?}",
        hover.documentation
    );
}
