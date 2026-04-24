#![cfg(feature = "incremental")]

use perl_parser::{
    edit::Edit, error::ParseError, incremental_v2::IncrementalParserV2, position::Position, Parser,
};

fn edit(source: &str, old: &str, new: &str) -> Result<(usize, usize, usize), ParseError> {
    let start = source.find(old).ok_or(ParseError::UnexpectedEof)?;
    let old_end = start + old.len();
    let new_end = start + new.len();
    Ok((start, old_end, new_end))
}

#[test]
fn batch_non_overlapping_edits_preserve_ast_and_improve_shifted_reuse() -> Result<(), ParseError> {
    let mut parser = IncrementalParserV2::new();
    let source1 = "my $alpha = 10;\nmy $beta = 20;\nmy $delta = 30;\n";
    parser.parse(source1)?;

    let (start1, old_end1, new_end1) = edit(source1, "10", "100")?;
    parser.edit(Edit::new(
        start1,
        old_end1,
        new_end1,
        Position::new(start1, 0, 0),
        Position::new(old_end1, 0, 0),
        Position::new(new_end1, 0, 0),
    ));

    let source_after_first = source1.replacen("10", "100", 1);
    let (start2, old_end2, new_end2) = edit(&source_after_first, "$delta", "$omega")?;
    parser.edit(Edit::new(
        start2,
        old_end2,
        new_end2,
        Position::new(start2, 0, 0),
        Position::new(old_end2, 0, 0),
        Position::new(new_end2, 0, 0),
    ));

    let source2 = "my $alpha = 100;\nmy $beta = 20;\nmy $omega = 30;\n";
    let incremental_tree = parser.parse(source2)?;
    let mut fresh_parser = Parser::new(source2);
    let fresh_tree = fresh_parser.parse()?;

    assert_eq!(incremental_tree, fresh_tree);
    assert!(parser.reused_nodes >= 6);
    assert!(parser.last_reuse_analysis.is_some());

    Ok(())
}

#[test]
fn multi_region_comment_and_whitespace_edits_keep_equivalence_and_reuse() -> Result<(), ParseError>
{
    let mut parser = IncrementalParserV2::new();
    let source1 = "my $x = 42;\nmy $y = 7;\nmy $z = 9;\n";
    parser.parse(source1)?;

    let (start1, old_end1, new_end1) = edit(source1, ";\nmy $y", "; # local comment\nmy $y")?;
    parser.edit(Edit::new(
        start1,
        old_end1,
        new_end1,
        Position::new(start1, 0, 0),
        Position::new(old_end1, 0, 0),
        Position::new(new_end1, 0, 0),
    ));

    let source_after_first = source1.replacen(";\nmy $y", "; # local comment\nmy $y", 1);
    let (start2, old_end2, new_end2) = edit(&source_after_first, "$z = 9", "$z   = 9")?;
    parser.edit(Edit::new(
        start2,
        old_end2,
        new_end2,
        Position::new(start2, 0, 0),
        Position::new(old_end2, 0, 0),
        Position::new(new_end2, 0, 0),
    ));

    let source2 = "my $x = 42; # local comment\nmy $y = 7;\nmy $z   = 9;\n";
    let incremental_tree = parser.parse(source2)?;
    let mut fresh_parser = Parser::new(source2);
    let fresh_tree = fresh_parser.parse()?;

    assert_eq!(incremental_tree, fresh_tree);
    let total_nodes = parser.reused_nodes + parser.reparsed_nodes;
    let reuse_ratio =
        if total_nodes > 0 { parser.reused_nodes as f64 / total_nodes as f64 } else { 0.0 };
    assert!(reuse_ratio >= 0.6);
    assert!(parser.last_reuse_analysis.is_some());

    Ok(())
}
