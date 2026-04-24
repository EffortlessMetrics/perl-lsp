#![cfg(feature = "incremental")]

use perl_parser::{edit::Edit, incremental_v2::IncrementalParserV2, position::Position, Parser};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn batch_non_overlapping_number_edits_improve_reuse() -> TestResult {
    let mut parser = IncrementalParserV2::new();
    let source1 = "my $x = 10;\nmy $y = 20;\nmy $z = 30;\n";
    parser.parse(source1)?;

    parser.edit(Edit::new(
        8,
        10,
        11,
        Position::new(8, 1, 9),
        Position::new(10, 1, 11),
        Position::new(11, 1, 12),
    ));
    parser.edit(Edit::new(
        22,
        24,
        25,
        Position::new(22, 2, 9),
        Position::new(24, 2, 11),
        Position::new(25, 2, 12),
    ));

    let source2 = "my $x = 100;\nmy $y = 200;\nmy $z = 30;\n";
    let incremental_tree = parser.parse(source2)?;

    let mut fresh_parser = Parser::new(source2);
    let fresh_tree = fresh_parser.parse()?;

    assert_eq!(incremental_tree, fresh_tree, "incremental tree must match full parse");
    assert!(
        parser.reused_nodes >= 6,
        "expected at least 6 reused nodes, got {}",
        parser.reused_nodes
    );

    Ok(())
}

#[test]
fn multi_region_whitespace_comment_edits_reuse_shifted_nodes() -> TestResult {
    let mut parser = IncrementalParserV2::new();
    let source1 = "my $x = 42;\nmy $y = 7;\n";
    parser.parse(source1)?;

    parser.edit(Edit::new(
        10,
        10,
        18,
        Position::new(10, 1, 11),
        Position::new(10, 1, 11),
        Position::new(18, 1, 19),
    ));
    parser.edit(Edit::new(
        23,
        23,
        25,
        Position::new(23, 2, 1),
        Position::new(23, 2, 1),
        Position::new(25, 2, 3),
    ));

    let source2 = "my $x = 42; # one\n\nmy $y = 7;\n";
    let incremental_tree = parser.parse(source2)?;

    let mut fresh_parser = Parser::new(source2);
    let fresh_tree = fresh_parser.parse()?;

    assert_eq!(incremental_tree, fresh_tree, "incremental tree must match full parse");
    assert!(
        parser.reused_nodes >= 4,
        "expected at least 4 reused nodes, got {}",
        parser.reused_nodes
    );

    Ok(())
}

#[test]
fn identifier_and_value_batch_edits_preserve_equivalence() -> TestResult {
    let mut parser = IncrementalParserV2::new();
    let source1 = "my $foo = 1;\nmy $bar = $foo;\n";
    parser.parse(source1)?;

    parser.edit(Edit::new(
        4,
        7,
        11,
        Position::new(4, 1, 5),
        Position::new(7, 1, 8),
        Position::new(11, 1, 12),
    ));
    parser.edit(Edit::new(
        22,
        25,
        29,
        Position::new(22, 2, 11),
        Position::new(25, 2, 14),
        Position::new(29, 2, 18),
    ));
    parser.edit(Edit::new(
        15,
        16,
        18,
        Position::new(15, 2, 4),
        Position::new(16, 2, 5),
        Position::new(18, 2, 7),
    ));

    let source2 = "my $foobar = 11;\nmy $bar = $foobar;\n";
    let incremental_tree = parser.parse(source2)?;

    let mut fresh_parser = Parser::new(source2);
    let fresh_tree = fresh_parser.parse()?;

    assert_eq!(incremental_tree, fresh_tree, "incremental tree must match full parse");
    assert!(
        parser.reused_nodes >= 3,
        "expected at least 3 reused nodes, got {}",
        parser.reused_nodes
    );

    Ok(())
}
