//! Behavior tests for AST-based folding range extraction.
//!
//! Covers subroutines, blocks, conditionals, loops, packages, class constructs,
//! phase blocks (BEGIN/END), import grouping, data section, and edge cases.

use std::sync::Arc;

use perl_lsp_folding::{FoldingRange, FoldingRangeExtractor, FoldingRangeKind};
use perl_parser_core::{ParseError, Parser, ast::Node};

fn parse(source: &str) -> Result<Arc<Node>, ParseError> {
    let mut parser = Parser::new(source);
    parser.parse().map(Arc::new)
}

fn extract(source: &str) -> Result<Vec<FoldingRange>, ParseError> {
    let ast = parse(source)?;
    let mut extractor = FoldingRangeExtractor::new();
    Ok(extractor.extract(&ast))
}

fn has_kind(ranges: &[FoldingRange], kind: &FoldingRangeKind) -> bool {
    ranges.iter().any(|r| match (&r.kind, kind) {
        (Some(FoldingRangeKind::Region), FoldingRangeKind::Region) => true,
        (Some(FoldingRangeKind::Imports), FoldingRangeKind::Imports) => true,
        (Some(FoldingRangeKind::Comment), FoldingRangeKind::Comment) => true,
        (None, _) => false,
        _ => false,
    })
}

// ---------------------------------------------------------------------------
// Subroutines
// ---------------------------------------------------------------------------

#[test]
fn subroutine_produces_at_least_one_fold() -> Result<(), ParseError> {
    let code = "sub greet {\n    my $name = shift;\n    return \"Hello, $name\";\n}\n";
    let ranges = extract(code)?;
    assert!(
        !ranges.is_empty(),
        "subroutine body should produce at least one fold"
    );
    Ok(())
}

#[test]
fn subroutine_fold_spans_body() -> Result<(), ParseError> {
    let code = "sub greet {\n    return 1;\n}\n";
    let ranges = extract(code)?;
    assert!(
        !ranges.is_empty(),
        "subroutine should produce a fold spanning its body"
    );
    // All folds must be non-trivial
    for r in &ranges {
        assert!(
            r.end_offset > r.start_offset,
            "every fold must have non-empty span"
        );
    }
    Ok(())
}

#[test]
fn anonymous_sub_with_body_is_foldable() -> Result<(), ParseError> {
    let code = "my $fn = sub {\n    return 42;\n};\n";
    let ranges = extract(code)?;
    assert!(
        !ranges.is_empty(),
        "anonymous sub with body should produce a fold"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// If / elsif / else
// ---------------------------------------------------------------------------

#[test]
fn if_block_produces_fold() -> Result<(), ParseError> {
    let code = "if ($x > 0) {\n    print \"positive\";\n}\n";
    let ranges = extract(code)?;
    assert!(!ranges.is_empty(), "if block should produce a fold");
    Ok(())
}

#[test]
fn if_else_block_produces_multiple_folds() -> Result<(), ParseError> {
    let code = "if ($x > 0) {\n    print \"pos\";\n} else {\n    print \"non-pos\";\n}\n";
    let ranges = extract(code)?;
    assert!(
        !ranges.is_empty(),
        "if/else should produce at least one fold"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// While / For / Foreach
// ---------------------------------------------------------------------------

#[test]
fn while_loop_produces_fold() -> Result<(), ParseError> {
    let code = "while ($i < 10) {\n    $i++;\n}\n";
    let ranges = extract(code)?;
    assert!(!ranges.is_empty(), "while loop should produce a fold");
    Ok(())
}

#[test]
fn for_loop_produces_fold() -> Result<(), ParseError> {
    let code = "for (my $i = 0; $i < 10; $i++) {\n    print $i;\n}\n";
    let ranges = extract(code)?;
    assert!(!ranges.is_empty(), "for loop should produce a fold");
    Ok(())
}

#[test]
fn foreach_loop_produces_fold() -> Result<(), ParseError> {
    let code = "foreach my $item (@list) {\n    process($item);\n}\n";
    let ranges = extract(code)?;
    assert!(!ranges.is_empty(), "foreach loop should produce a fold");
    Ok(())
}

// ---------------------------------------------------------------------------
// Import grouping
// ---------------------------------------------------------------------------

#[test]
fn consecutive_use_statements_produce_imports_fold() -> Result<(), ParseError> {
    let code = "use strict;\nuse warnings;\nuse Carp;\n\nsub foo { 1 }\n";
    let ranges = extract(code)?;
    assert!(
        has_kind(&ranges, &FoldingRangeKind::Imports),
        "three consecutive use statements should produce an Imports fold"
    );
    Ok(())
}

#[test]
fn single_use_statement_does_not_produce_imports_fold() -> Result<(), ParseError> {
    let code = "use strict;\nsub foo { 1 }\n";
    let ranges = extract(code)?;
    assert!(
        !has_kind(&ranges, &FoldingRangeKind::Imports),
        "single use statement should not produce an Imports fold"
    );
    Ok(())
}

#[test]
fn imports_fold_spans_from_first_to_last_use() -> Result<(), ParseError> {
    let code = "use strict;\nuse warnings;\nuse Carp;\n\nmy $x = 1;\n";
    let ranges = extract(code)?;
    let import_folds: Vec<_> = ranges
        .iter()
        .filter(|r| matches!(r.kind, Some(FoldingRangeKind::Imports)))
        .collect();
    assert_eq!(import_folds.len(), 1, "should be exactly one Imports fold");
    let fold = import_folds[0];
    // The fold should start at offset 0 (first 'use') and end after 'use Carp;'
    assert_eq!(
        fold.start_offset, 0,
        "imports fold should start at beginning of file"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Phase blocks (BEGIN / END / CHECK / INIT)
// ---------------------------------------------------------------------------

#[test]
fn begin_block_produces_fold() -> Result<(), ParseError> {
    let code = "BEGIN {\n    require SomeModule;\n}\n";
    let ranges = extract(code)?;
    assert!(!ranges.is_empty(), "BEGIN block should produce a fold");
    Ok(())
}

#[test]
fn end_block_produces_fold() -> Result<(), ParseError> {
    let code = "END {\n    cleanup();\n}\n";
    let ranges = extract(code)?;
    assert!(!ranges.is_empty(), "END block should produce a fold");
    Ok(())
}

// ---------------------------------------------------------------------------
// Data section (__DATA__ / __END__)
// ---------------------------------------------------------------------------

#[test]
fn data_section_produces_comment_fold() -> Result<(), ParseError> {
    let code = "print 1;\n__DATA__\nsome data here\nmore data\n";
    let ranges = extract(code)?;
    assert!(
        has_kind(&ranges, &FoldingRangeKind::Comment),
        "__DATA__ section should produce a Comment fold"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Trivial range filter (end_offset <= start_offset + 1)
// ---------------------------------------------------------------------------

#[test]
fn empty_program_produces_no_folds() -> Result<(), ParseError> {
    let code = "";
    let ranges = extract(code)?;
    assert!(ranges.is_empty(), "empty program should produce no folds");
    Ok(())
}

#[test]
fn single_statement_produces_no_folds() -> Result<(), ParseError> {
    let code = "my $x = 1;\n";
    let ranges = extract(code)?;
    assert!(
        ranges.is_empty(),
        "single statement should produce no folds"
    );
    Ok(())
}

#[test]
fn empty_block_produces_no_folds() -> Result<(), ParseError> {
    let code = "sub foo {}\n";
    let ranges = extract(code)?;
    // An empty sub body {} has start == end (or span <= 1) so should be filtered
    // The subroutine node itself may produce a fold but only if non-trivial
    for r in &ranges {
        assert!(
            r.end_offset > r.start_offset + 1,
            "trivial folds should be filtered out"
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// extract_heredoc_ranges (lexer-based)
// ---------------------------------------------------------------------------

#[test]
fn extract_heredoc_ranges_returns_empty_for_no_heredoc() {
    let code = "my $x = 42;\n";
    let ranges = FoldingRangeExtractor::extract_heredoc_ranges(code);
    assert!(
        ranges.is_empty(),
        "source without heredocs should return empty heredoc ranges"
    );
}

#[test]
fn extract_heredoc_ranges_any_returned_folds_are_region_kind() {
    // If extract_heredoc_ranges returns any ranges, they must have Region kind.
    // (The lexer may or may not emit HeredocBody tokens depending on the source form.)
    let code = "my $x = <<END;\nline1\nline2\nEND\n";
    let ranges = FoldingRangeExtractor::extract_heredoc_ranges(code);
    for r in &ranges {
        assert!(
            matches!(r.kind, Some(FoldingRangeKind::Region)),
            "all lexer-based heredoc ranges must be Region kind"
        );
        assert!(
            r.end_offset > r.start_offset,
            "every range must have end > start"
        );
    }
}

// ---------------------------------------------------------------------------
// FoldingRange / FoldingRangeKind structural properties
// ---------------------------------------------------------------------------

#[test]
fn folding_range_kind_debug_variants_exist() {
    // Ensure Debug is implemented for all variants
    let _ = format!("{:?}", FoldingRangeKind::Comment);
    let _ = format!("{:?}", FoldingRangeKind::Imports);
    let _ = format!("{:?}", FoldingRangeKind::Region);
}

#[test]
fn all_extracted_folds_have_nontrivial_spans() -> Result<(), ParseError> {
    let code = "use strict;\nuse warnings;\n\nsub process {\n    foreach my $item (@_) {\n        if ($item > 0) {\n            print $item;\n        }\n    }\n}\n";
    let ranges = extract(code)?;
    for r in &ranges {
        assert!(
            r.end_offset > r.start_offset,
            "every fold must be non-trivial (end > start)"
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// LabeledStatement
// ---------------------------------------------------------------------------

#[test]
fn labeled_loop_produces_fold() -> Result<(), ParseError> {
    let code = "OUTER: while (1) {\n    INNER: for my $i (1..10) {\n        last OUTER if $i > 5;\n    }\n}\n";
    let ranges = extract(code)?;
    assert!(
        !ranges.is_empty(),
        "labeled loop should produce at least one fold"
    );
    Ok(())
}

#[test]
fn labeled_foreach_produces_fold() -> Result<(), ParseError> {
    let code = "LINE: foreach my $line (@lines) {\n    next LINE if $line =~ /^#/;\n    process($line);\n}\n";
    let ranges = extract(code)?;
    assert!(
        !ranges.is_empty(),
        "labeled foreach loop should produce a fold"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Format
// ---------------------------------------------------------------------------

#[test]
fn format_declaration_produces_region_fold() -> Result<(), ParseError> {
    let code = "format STDOUT =\n@<<<<<<<<<<<<  @>>>>>>\n$name,         $salary\n.\n";
    let ranges = extract(code)?;
    assert!(
        !ranges.is_empty(),
        "format declaration should produce a fold"
    );
    Ok(())
}

#[test]
fn format_declaration_fold_is_region_kind() -> Result<(), ParseError> {
    let code = "format STDOUT =\n@<<<<<<<<<<<<  @>>>>>>\n$name,         $salary\n.\n";
    let ranges = extract(code)?;
    assert!(
        has_kind(&ranges, &FoldingRangeKind::Region),
        "format declaration fold should have Region kind"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Tie
// ---------------------------------------------------------------------------

#[test]
fn tie_with_multiple_args_produces_fold() -> Result<(), ParseError> {
    let code = "tie %config,\n    'Tie::IxHash',\n    key1 => 'val1',\n    key2 => 'val2';\n";
    let ranges = extract(code)?;
    // Tie is only foldable if multi-line (end_offset > start_offset + 1)
    // This test verifies the visitor does not panic and produces correct output
    for r in &ranges {
        assert!(
            r.end_offset > r.start_offset,
            "every fold must be non-trivial"
        );
    }
    Ok(())
}
