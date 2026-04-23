//! Tests for inline POD hover documentation on subroutines and methods.
//!
//! POD blocks may appear inside a subroutine body (e.g. `=pod ... =cut`
//! between statements) and should be associated with the enclosing
//! subroutine for hover documentation. The default `extract_documentation`
//! only scans backwards from a position, so inline POD inside a sub body
//! is missed without a body-aware fallback.
//!
//! Per perlpod, POD directives must start at column 0 — the parser's
//! tokenizer recognises `=pod`/`=head1`/`=cut` as a trivia block only
//! when they begin a line. The fixtures below therefore place POD
//! directives at column 0 inside sub bodies, which is the form real
//! Perl source uses.
//!
//! See: <https://github.com/EffortlessMetrics/perl-lsp/issues/3407>

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::semantic::SemanticAnalyzer;
use perl_semantic_analyzer::symbol::SymbolKind;
use perl_tdd_support::must;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// `=pod ... =cut` immediately inside a sub body should be exposed as the
/// sub's hover documentation.
#[test]
fn inline_pod_pod_cut_is_associated_with_subroutine_hover() -> TestResult {
    let code = "sub process_data {\n\
                =pod\n\
                Internal documentation for this sub\n\
                =cut\n\
                    my $data = shift;\n\
                    return $data;\n\
                }\n";

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    let symbols = analyzer.symbol_table().find_symbol("process_data", 0, SymbolKind::Subroutine);
    let symbol = symbols.first().ok_or("process_data should be in the symbol table")?;
    let hover = analyzer.hover_at(symbol.location).ok_or("expected hover info for process_data")?;
    let doc = hover.documentation.as_deref().unwrap_or("");
    assert!(
        doc.contains("Internal documentation for this sub"),
        "expected inline POD to be surfaced as hover documentation, got: {doc:?}"
    );
    Ok(())
}

/// `=head1` style POD inside a sub body should also be picked up.
#[test]
fn inline_pod_head1_inside_sub_body_is_surfaced_in_hover() -> TestResult {
    let code = "sub greet {\n\
                =head1 DESCRIPTION\n\
                \n\
                Says hello in a friendly way.\n\
                \n\
                =cut\n\
                    return \"hi\";\n\
                }\n";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    let symbols = analyzer.symbol_table().find_symbol("greet", 0, SymbolKind::Subroutine);
    let symbol = symbols.first().ok_or("greet should be in the symbol table")?;
    let hover = analyzer.hover_at(symbol.location).ok_or("expected hover for greet")?;
    let doc = hover.documentation.as_deref().unwrap_or("");
    assert!(
        doc.contains("Says hello in a friendly way"),
        "expected =head1 inline POD to be surfaced as hover documentation, got: {doc:?}"
    );
    Ok(())
}

/// Preceding comment documentation should always win over inline POD when
/// both are present — preserves the existing leading-doc precedence and
/// matches what users author when they intentionally annotate a sub.
#[test]
fn leading_comment_doc_wins_over_inline_pod() -> TestResult {
    let code = "# Preferred leading documentation\n\
                sub do_thing {\n\
                =pod\n\
                Inline body documentation\n\
                =cut\n\
                    return 1;\n\
                }\n";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    let symbols = analyzer.symbol_table().find_symbol("do_thing", 0, SymbolKind::Subroutine);
    let symbol = symbols.first().ok_or("do_thing should be in the symbol table")?;
    let hover = analyzer.hover_at(symbol.location).ok_or("expected hover for do_thing")?;
    let doc = hover.documentation.as_deref().unwrap_or("");
    assert!(
        doc.contains("Preferred leading documentation"),
        "leading comment should still take precedence, got: {doc:?}"
    );
    assert!(
        !doc.contains("Inline body documentation"),
        "inline POD should not override the explicit leading comment, got: {doc:?}"
    );
    Ok(())
}

/// A sub with neither leading docs nor inline POD must yield no hover docs
/// (no false positives from unrelated nearby POD blocks).
#[test]
fn sub_without_any_pod_or_comments_has_no_doc() -> TestResult {
    let code = "sub plain_sub {\n    return 42;\n}\n";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    let symbols = analyzer.symbol_table().find_symbol("plain_sub", 0, SymbolKind::Subroutine);
    let symbol = symbols.first().ok_or("plain_sub should be in the symbol table")?;
    let hover = analyzer.hover_at(symbol.location).ok_or("expected hover for plain_sub")?;
    assert!(
        hover.documentation.is_none(),
        "expected no documentation, got: {:?}",
        hover.documentation
    );
    Ok(())
}

/// Each sub must surface its own inline POD — adjacent subs that each
/// document themselves with inline POD should each get their own docs,
/// without cross-contamination through the body-aware fallback.
///
/// This also guards against the pre-existing bleed where leading-doc
/// extraction matched POD from earlier in the source — fixed by
/// anchoring the POD/comment regex to the absolute end of `before`.
#[test]
fn inline_pod_attaches_to_each_subs_own_body() -> TestResult {
    let code = "sub outer {\n\
                =pod\n\
                Outer body documentation\n\
                =cut\n\
                    return 1;\n\
                }\n\
                \n\
                # Leading docs for inner\n\
                sub inner {\n\
                =pod\n\
                Inner body documentation\n\
                =cut\n\
                    return 2;\n\
                }\n";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    let outer = analyzer.symbol_table().find_symbol("outer", 0, SymbolKind::Subroutine);
    let inner = analyzer.symbol_table().find_symbol("inner", 0, SymbolKind::Subroutine);
    let outer_sym = outer.first().ok_or("outer should be in the symbol table")?;
    let inner_sym = inner.first().ok_or("inner should be in the symbol table")?;

    let outer_hover = analyzer.hover_at(outer_sym.location).ok_or("expected hover for outer")?;
    let inner_hover = analyzer.hover_at(inner_sym.location).ok_or("expected hover for inner")?;

    let outer_doc = outer_hover.documentation.as_deref().unwrap_or("");
    let inner_doc = inner_hover.documentation.as_deref().unwrap_or("");

    assert!(
        outer_doc.contains("Outer body documentation"),
        "expected outer to surface its inline POD, got: {outer_doc:?}"
    );
    // `inner` has explicit leading docs, so they win — the body-aware
    // fallback must not override that, and outer's body POD must not bleed.
    assert!(
        inner_doc.contains("Leading docs for inner"),
        "expected inner's leading comment to win, got: {inner_doc:?}"
    );
    assert!(
        !inner_doc.contains("Outer body documentation"),
        "outer's inline POD must not bleed into inner's hover, got: {inner_doc:?}"
    );
    assert!(
        !inner_doc.contains("Inner body documentation"),
        "inner's leading docs should win over its inline POD, got: {inner_doc:?}"
    );
    Ok(())
}

/// Regression guard for the pre-existing leading-doc bleed: an earlier
/// POD block followed by unrelated code must not be returned as docs for
/// a later sub. Before the `\z` anchor fix, multiline `$` matched
/// end-of-any-line and pulled in earlier POD blocks.
#[test]
fn earlier_pod_does_not_bleed_into_later_undocumented_sub() -> TestResult {
    let code = "=pod\n\
                Module-level POD\n\
                =cut\n\
                \n\
                my $x = 1;\n\
                my $y = 2;\n\
                \n\
                sub later {\n\
                    return $x + $y;\n\
                }\n";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    let symbols = analyzer.symbol_table().find_symbol("later", 0, SymbolKind::Subroutine);
    let symbol = symbols.first().ok_or("later should be in the symbol table")?;
    let hover = analyzer.hover_at(symbol.location).ok_or("expected hover for later")?;
    assert!(
        hover.documentation.is_none(),
        "earlier POD must not attach to a later, unrelated sub: {:?}",
        hover.documentation
    );
    Ok(())
}

/// The first inline POD block inside a sub body wins when multiple are
/// present — matches the convention of putting the descriptive POD first.
#[test]
fn first_inline_pod_block_wins_when_multiple_present() -> TestResult {
    let code = "sub multi_pod {\n\
                =pod\n\
                First body doc\n\
                =cut\n\
                    my $x = 1;\n\
                =pod\n\
                Second body doc\n\
                =cut\n\
                    return $x;\n\
                }\n";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    let symbols = analyzer.symbol_table().find_symbol("multi_pod", 0, SymbolKind::Subroutine);
    let symbol = symbols.first().ok_or("multi_pod should be in the symbol table")?;
    let hover = analyzer.hover_at(symbol.location).ok_or("expected hover for multi_pod")?;
    let doc = hover.documentation.as_deref().unwrap_or("");
    assert!(doc.contains("First body doc"), "expected first inline POD to win, got: {doc:?}");
    assert!(!doc.contains("Second body doc"), "expected only the first inline POD, got: {doc:?}");
    Ok(())
}

/// Inline POD inside an anonymous sub should also be surfaced — anonymous
/// subs commonly carry inline docs in callback-heavy code.
#[test]
fn inline_pod_in_anonymous_sub_is_surfaced() -> TestResult {
    let code = "my $cb = sub {\n\
                =pod\n\
                Anonymous callback doc\n\
                =cut\n\
                    return 1;\n\
                };\n";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    // The anonymous sub's hover is keyed on its node location. Walk all
    // hovers and assert at least one carries the inline POD.
    let found = analyzer
        .all_hover_entries()
        .any(|h| h.documentation.as_deref().is_some_and(|d| d.contains("Anonymous callback doc")));
    assert!(found, "expected anonymous sub hover to carry inline POD");
    Ok(())
}

/// Windows-style CRLF line endings should not prevent inline POD detection.
#[test]
fn inline_pod_with_crlf_line_endings_is_surfaced() -> TestResult {
    let code = "sub windows_doc {\r\n\
=pod\r\n\
CRLF inline pod docs\r\n\
=cut\r\n\
    return 1;\r\n\
}\r\n";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    let symbols = analyzer.symbol_table().find_symbol("windows_doc", 0, SymbolKind::Subroutine);
    let symbol = symbols.first().ok_or("windows_doc should be in the symbol table")?;
    let hover = analyzer.hover_at(symbol.location).ok_or("expected hover for windows_doc")?;
    let doc = hover.documentation.as_deref().unwrap_or("");
    assert!(
        doc.contains("CRLF inline pod docs"),
        "expected inline POD docs with CRLF endings, got: {doc:?}"
    );
    Ok(())
}
