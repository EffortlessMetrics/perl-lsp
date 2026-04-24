#![cfg(feature = "incremental")]

use perl_parser::incremental_v2::IncrementalParserV2;
use perl_parser_core::{
    ast::Node, edit::Edit, error::ParseResult, parser::Parser, position::Position,
};

fn fresh_parse(source: &str) -> ParseResult<Node> {
    Parser::new(source).parse()
}

#[test]
fn batch_non_overlapping_identifier_and_value_edits_preserve_ast_and_reuse() -> ParseResult<()> {
    let mut parser = IncrementalParserV2::new();
    let source1 = "my $foo = 10;\nmy $bar = 20;\nmy $baz = 30;";
    parser.parse(source1)?;

    // 10 -> 100
    parser.edit(Edit::new(
        10,
        12,
        13,
        Position::new(10, 1, 11),
        Position::new(12, 1, 13),
        Position::new(13, 1, 14),
    ));
    // baz -> buzz (adjusted by previous +1 shift)
    parser.edit(Edit::new(
        33,
        36,
        37,
        Position::new(33, 3, 5),
        Position::new(36, 3, 8),
        Position::new(37, 3, 9),
    ));

    let source2 = "my $foo = 100;\nmy $bar = 20;\nmy $buzz = 30;";
    let incremental_tree = parser.parse(source2)?;
    let full_tree = fresh_parse(source2)?;

    assert_eq!(incremental_tree, full_tree);
    assert!(parser.reused_nodes >= 6);
    assert!(parser.used_advanced_reuse());
    assert!(parser
        .get_last_reuse_analysis()
        .is_some_and(|analysis| analysis.analysis_stats.position_adjustments > 0));

    Ok(())
}

#[test]
fn batch_whitespace_and_comment_edits_in_separate_regions_preserve_ast() -> ParseResult<()> {
    let mut parser = IncrementalParserV2::new();
    let source1 = "my $x = 1;\nmy $y = 2;";
    parser.parse(source1)?;

    // Expand spacing in first statement.
    parser.edit(Edit::new(
        5,
        6,
        8,
        Position::new(5, 1, 6),
        Position::new(6, 1, 7),
        Position::new(8, 1, 9),
    ));
    // Append trailing comment in second statement (adjusted by +2 shift).
    parser.edit(Edit::new(
        23,
        23,
        30,
        Position::new(23, 2, 12),
        Position::new(23, 2, 12),
        Position::new(30, 2, 19),
    ));

    let source2 = "my $x   = 1;\nmy $y = 2; # note";
    let incremental_tree = parser.parse(source2)?;
    let full_tree = fresh_parse(source2)?;

    assert_eq!(incremental_tree, full_tree);
    assert!(parser.reused_nodes > 0);
    assert!(parser.used_advanced_reuse() || parser.reused_nodes >= parser.reparsed_nodes);

    Ok(())
}
