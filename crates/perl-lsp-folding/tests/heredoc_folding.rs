//! Tests for heredoc folding range extraction.

use std::sync::Arc;

use perl_lsp_folding::{FoldingRangeExtractor, FoldingRangeKind};
use perl_parser_core::{ParseError, Parser, ast::Node};

fn parse(source: &str) -> Result<Arc<Node>, ParseError> {
    let mut parser = Parser::new(source);
    parser.parse().map(Arc::new)
}

#[test]
fn heredoc_produces_region_fold_via_ast() -> Result<(), ParseError> {
    let code = "my $x = <<END;\nline one\nline two\nEND\n";
    let ast = parse(code)?;
    let mut extractor = FoldingRangeExtractor::new();
    let ranges = extractor.extract(&ast);

    let region_folds: Vec<_> = ranges
        .iter()
        .filter(|r| matches!(r.kind, Some(FoldingRangeKind::Region)))
        .collect();

    assert!(
        !region_folds.is_empty(),
        "heredoc should produce at least one Region fold"
    );

    // The fold should cover a meaningful span
    for fold in &region_folds {
        assert!(
            fold.end_offset > fold.start_offset,
            "fold range must be non-empty"
        );
    }
    Ok(())
}

#[test]
fn indented_heredoc_produces_region_fold() -> Result<(), ParseError> {
    let code = "my $x = <<~END;\n    line one\n    line two\nEND\n";
    let ast = parse(code)?;
    let mut extractor = FoldingRangeExtractor::new();
    let ranges = extractor.extract(&ast);

    let region_folds: Vec<_> = ranges
        .iter()
        .filter(|r| matches!(r.kind, Some(FoldingRangeKind::Region)))
        .collect();

    assert!(
        !region_folds.is_empty(),
        "indented heredoc should produce a Region fold"
    );
    Ok(())
}

#[test]
fn single_quoted_heredoc_produces_region_fold() -> Result<(), ParseError> {
    let code = "my $x = <<'END';\nno $interpolation\nEND\n";
    let ast = parse(code)?;
    let mut extractor = FoldingRangeExtractor::new();
    let ranges = extractor.extract(&ast);

    let region_folds: Vec<_> = ranges
        .iter()
        .filter(|r| matches!(r.kind, Some(FoldingRangeKind::Region)))
        .collect();

    assert!(
        !region_folds.is_empty(),
        "single-quoted heredoc should produce a Region fold"
    );
    Ok(())
}

#[test]
fn heredoc_fold_has_region_kind_not_none() -> Result<(), ParseError> {
    let code = "my $msg = <<EOF;\nHello\nWorld\nEOF\n";
    let ast = parse(code)?;
    let mut extractor = FoldingRangeExtractor::new();
    let ranges = extractor.extract(&ast);

    // Every Region fold must actually have kind = Region (not None)
    let region_folds: Vec<_> = ranges
        .iter()
        .filter(|r| matches!(r.kind, Some(FoldingRangeKind::Region)))
        .collect();

    assert!(
        !region_folds.is_empty(),
        "heredoc should produce at least one Region fold"
    );

    // Verify that no fold has kind = None (the old bug)
    let none_folds: Vec<_> = ranges.iter().filter(|r| r.kind.is_none()).collect();
    assert!(
        none_folds.is_empty(),
        "heredoc folds should not have kind = None; found {} with None kind",
        none_folds.len()
    );
    Ok(())
}

#[test]
fn no_heredoc_no_region_fold() -> Result<(), ParseError> {
    let code = "my $x = 42;\nprint $x;\n";
    let ast = parse(code)?;
    let mut extractor = FoldingRangeExtractor::new();
    let ranges = extractor.extract(&ast);

    let region_folds: Vec<_> = ranges
        .iter()
        .filter(|r| matches!(r.kind, Some(FoldingRangeKind::Region)))
        .collect();

    assert!(
        region_folds.is_empty(),
        "code without heredocs should not produce Region folds"
    );
    Ok(())
}

#[test]
fn lexer_based_heredoc_ranges_also_region() {
    let code = "my $x = <<END;\nline1\nline2\nEND\n";
    let ranges = FoldingRangeExtractor::extract_heredoc_ranges(code);

    for r in &ranges {
        assert!(r.end_offset > r.start_offset);
        assert!(
            matches!(r.kind, Some(FoldingRangeKind::Region)),
            "lexer-based heredoc fold should be Region"
        );
    }
}
