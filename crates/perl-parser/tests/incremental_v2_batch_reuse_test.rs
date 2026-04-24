#![cfg(feature = "incremental")]

use perl_parser::incremental_v2::IncrementalParserV2;
use perl_parser_core::{edit::Edit, error::ParseResult, parser::Parser, position::Position};

fn assert_incremental_matches_full(
    parser: &mut IncrementalParserV2,
    source: &str,
) -> ParseResult<()> {
    let incremental_tree = parser.parse(source)?;
    let mut full_parser = Parser::new(source);
    let full_tree = full_parser.parse()?;
    assert_eq!(format!("{incremental_tree:?}"), format!("{full_tree:?}"));
    Ok(())
}

#[test]
fn batch_non_overlapping_literal_edits_reuse_shifted_nodes() -> ParseResult<()> {
    let mut parser = IncrementalParserV2::new();
    let source1 = "my $x = 10;\nmy $y = 20;\nmy $z = 30;";
    parser.parse(source1)?;

    parser.edit(Edit::new(
        8,
        10,
        12,
        Position::new(8, 1, 9),
        Position::new(10, 1, 11),
        Position::new(12, 1, 13),
    ));
    parser.edit(Edit::new(
        24,
        26,
        27,
        Position::new(24, 2, 9),
        Position::new(26, 2, 11),
        Position::new(27, 2, 12),
    ));

    let source2 = "my $x = 1000;\nmy $y = 7;\nmy $z = 30;";
    assert_incremental_matches_full(&mut parser, source2)?;
    assert!(parser.reused_nodes > 0);
    Ok(())
}

#[test]
fn batch_whitespace_and_identifier_edit_reuses_unaffected_regions() -> ParseResult<()> {
    let mut parser = IncrementalParserV2::new();
    let source1 = "my $first = 1;\nmy $second = 2;";
    parser.parse(source1)?;

    parser.edit(Edit::new(
        3,
        4,
        5,
        Position::new(3, 1, 4),
        Position::new(4, 1, 5),
        Position::new(5, 1, 6),
    ));
    parser.edit(Edit::new(
        21,
        27,
        22,
        Position::new(21, 2, 5),
        Position::new(27, 2, 11),
        Position::new(22, 2, 6),
    ));

    let source2 = "my  $first = 1;\nmy $id = 2;";
    assert_incremental_matches_full(&mut parser, source2)?;
    assert!(parser.reused_nodes > 0);
    Ok(())
}

#[test]
fn batch_multibyte_and_identifier_edits_keep_safe_shifted_reuse() -> ParseResult<()> {
    let mut parser = IncrementalParserV2::new();
    let source1 = "my $x = 1;\nmy $name = 22;";
    parser.parse(source1)?;

    parser.edit(Edit::new(
        8,
        9,
        13,
        Position::new(8, 1, 9),
        Position::new(9, 1, 10),
        Position::new(13, 1, 14),
    ));
    parser.edit(Edit::new(
        17,
        22,
        21,
        Position::new(17, 2, 5),
        Position::new(22, 2, 10),
        Position::new(21, 2, 9),
    ));

    let source2 = "my $x = \"é\";\nmy $id = 22;";
    assert_incremental_matches_full(&mut parser, source2)?;
    assert!(parser.reused_nodes > 0);
    Ok(())
}
