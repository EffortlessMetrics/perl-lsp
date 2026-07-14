//! Regression tests for document-highlight caret-at-trailing-edge boundary.
//!
//! Defect: `find_node_at_offset` (and its `extract_symbol_at_offset` fallback)
//! used a half-open containment check (`offset >= node.location.end`), so a
//! caret resting at the trailing edge of an identifier — the common "caret just
//! after the word" position produced by double-click-select in VS Code — yielded
//! zero highlights. The sibling references provider
//! (`navigation/references.rs`) already treats the trailing edge as on-token
//! (`offset > node.location.end`); this pins document-highlight to the same
//! inclusive-end boundary.

use perl_lsp_rs_core::providers::document_highlight::DocumentHighlightProvider;

/// Parse source into an AST node using the same parser the live LSP path uses.
fn parse(source: &str) -> perl_ast::Node {
    let mut parser = perl_parser_core::Parser::new(source);
    match parser.parse() {
        Ok(ast) => ast,
        Err(e) => unreachable!("fixture must parse: {e:?}"),
    }
}

/// Fixture: `$total` spans bytes 3..9 (`my $total = 0;\n$total += 5;\n`).
/// It appears twice (declaration + compound assignment), so a caret on the
/// symbol must yield exactly two highlights.
const FIXTURE: &str = "my $total = 0;\n$total += 5;\n";

#[test]
fn highlight_at_trailing_edge_returns_occurrences() -> Result<(), Box<dyn std::error::Error>> {
    let ast = parse(FIXTURE);
    let provider = DocumentHighlightProvider::new();

    // Byte 9 is immediately after the 'l' of the first `$total` — i.e. exactly
    // at `node.location.end`. This is the position a double-click-select caret
    // lands on. Pre-fix this returned 0 (the half-open `offset >= end` check
    // rejected 9 >= 9); post-fix it must return both occurrences.
    let highlights = provider.find_highlights(&ast, FIXTURE, 9);
    assert_eq!(
        highlights.len(),
        2,
        "caret at trailing edge (byte==node.end) must highlight both $total occurrences, got {highlights:?}"
    );

    Ok(())
}

#[test]
fn highlight_inside_token_still_returns_occurrences() -> Result<(), Box<dyn std::error::Error>> {
    // Sanity anchor: a caret in the middle of the token was always correct and
    // must remain so after the boundary change.
    let ast = parse(FIXTURE);
    let provider = DocumentHighlightProvider::new();

    let highlights = provider.find_highlights(&ast, FIXTURE, 6);
    assert_eq!(
        highlights.len(),
        2,
        "caret inside $total must highlight both occurrences, got {highlights:?}"
    );

    Ok(())
}

#[test]
fn highlight_past_token_returns_nothing() -> Result<(), Box<dyn std::error::Error>> {
    // Guard the other side of the boundary: byte 10 is past `$total` (on the
    // space) and must not highlight the variable. This ensures the fix widens
    // the inclusive end by exactly one byte, not further.
    let ast = parse(FIXTURE);
    let provider = DocumentHighlightProvider::new();

    let highlights = provider.find_highlights(&ast, FIXTURE, 10);
    assert!(
        highlights.is_empty(),
        "caret past $total (byte 10, on the space) must not highlight it, got {highlights:?}"
    );

    Ok(())
}

/// Follow-up to the trailing-edge fix above: `find_node_at_offset` and
/// `extract_symbol_at_offset` became inclusive, but a caret at the exact same
/// trailing-edge offset also has to survive `find_subscript_parent` (the `[]`/
/// `{}` container-normalization check) and the Try/catch synthetic fallback,
/// both of which had their own half-open bounds left unfixed. A caret at the
/// trailing edge of `$arr` in `$arr[0]` must still promote the sigil to `@`
/// (matching the sibling `@arr` declaration) rather than silently keeping the
/// stale `$` sigil and missing it.
const SUBSCRIPT_FIXTURE: &str = "my @arr = (1, 2, 3);\n$arr[0] = 5;\n";

#[test]
fn highlight_subscript_container_at_trailing_edge_normalizes_sigil()
-> Result<(), Box<dyn std::error::Error>> {
    let ast = parse(SUBSCRIPT_FIXTURE);
    let provider = DocumentHighlightProvider::new();

    let usage = SUBSCRIPT_FIXTURE.find("$arr[0]").ok_or("fixture must contain $arr[0]")?;
    // Trailing edge of `$arr`: right after the second `r`, before `[`.
    let trailing_edge = usage + "$arr".len();

    let highlights = provider.find_highlights(&ast, SUBSCRIPT_FIXTURE, trailing_edge);

    // Pre-fix: `find_subscript_parent`'s half-open bound rejected this offset
    // as "inside the left child", so the sigil stayed `$` and cross-sigil
    // matching against the `@arr` declaration (`is_cross_sigil_match`) never
    // fired -- only the `$arr[0]` occurrence itself matched (1 highlight).
    // Post-fix: the sigil normalizes to `@`, so both the `@arr` declaration
    // and the `$arr[0]` usage (via cross-sigil match) highlight.
    assert_eq!(
        highlights.len(),
        2,
        "caret at trailing edge of `$arr` in `$arr[0]` must normalize to `@arr` and highlight \
         both the declaration and the subscript usage, got {highlights:?}"
    );

    let decl_start = SUBSCRIPT_FIXTURE.find("@arr").ok_or("fixture must contain @arr decl")?;
    assert!(
        highlights.iter().any(|h| h.location.start == decl_start),
        "the `@arr` declaration must be among the highlights, got {highlights:?}"
    );

    Ok(())
}

/// Sibling boundary for the Try/catch synthetic-symbol fallback in
/// `extract_symbol_at_offset`: the outer containment gate is inclusive, but
/// the inner `match_indices` search for the catch parameter string was still
/// half-open, so a caret at the trailing edge of the catch parameter itself
/// (e.g. `$err` in `catch ($err)`) fell through to zero highlights.
const TRY_CATCH_FIXTURE: &str = "try {\n1;\n} catch ($err) {\nprint $err;\n}\n";

#[test]
fn highlight_catch_parameter_at_trailing_edge_returns_occurrence()
-> Result<(), Box<dyn std::error::Error>> {
    let ast = parse(TRY_CATCH_FIXTURE);
    let provider = DocumentHighlightProvider::new();

    let param = TRY_CATCH_FIXTURE.find("$err)").ok_or("fixture must contain catch ($err)")?;
    // Trailing edge of the catch parameter `$err`: right after the second
    // `r`, before the closing `)`. The catch parameter itself is stored as a
    // bare string on the `Try` node (not a `Variable` AST node), so
    // `find_node_at_offset` never finds it here -- this exercises the
    // synthetic-symbol fallback in `extract_symbol_at_offset` exclusively.
    let trailing_edge = param + "$err".len();

    let highlights = provider.find_highlights(&ast, TRY_CATCH_FIXTURE, trailing_edge);

    // Pre-fix: the half-open `relative_offset < pos + var_str.len()` check
    // rejected the trailing-edge offset, so no symbol was recovered at all
    // and `find_highlights` returned zero results (neither the synthetic
    // catch-parameter highlight nor the `print $err;` usage). Post-fix: the
    // symbol resolves and both the catch-parameter binding (synthesized by
    // `collect_highlights_with_parent`'s separate Try-catch handling) and the
    // real usage highlight.
    assert_eq!(
        highlights.len(),
        2,
        "caret at trailing edge of catch parameter `$err` must still resolve the symbol \
         and highlight both the parameter binding and its use in the catch body, \
         got {highlights:?}"
    );

    Ok(())
}
