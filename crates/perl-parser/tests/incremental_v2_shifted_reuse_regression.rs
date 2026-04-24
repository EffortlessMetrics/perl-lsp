#![cfg(feature = "incremental")]
use perl_parser::Parser;
use perl_parser::incremental::incremental_advanced_reuse::{AdvancedReuseAnalyzer, ReuseConfig};
use perl_parser::incremental::incremental_v2::IncrementalParserV2;
use perl_parser_core::{
    edit::{Edit, EditSet},
    error::ParseResult,
    position::Position,
};

#[test]
fn multi_edit_shifted_reuse_preserves_fresh_parse_equivalence() -> ParseResult<()> {
    let mut parser = IncrementalParserV2::new();
    let source1 = "my $x = 1;\nmy $y = 22;\nmy $z = 333;\n";
    parser.parse(source1)?;

    parser.edit(Edit::new(
        8,
        9,
        12,
        Position::new(8, 1, 9),
        Position::new(9, 1, 10),
        Position::new(12, 1, 13),
    ));
    parser.edit(Edit::new(
        34,
        37,
        35,
        Position::new(34, 3, 9),
        Position::new(37, 3, 12),
        Position::new(35, 3, 10),
    ));

    let source2 = "my $x = 1000;\nmy $y = 22;\nmy $z = 9;\n";
    let incremental_tree = parser.parse(source2)?;
    let fresh_tree = Parser::new(source2).parse()?;

    assert_eq!(incremental_tree, fresh_tree);
    assert!(parser.reused_nodes > parser.reparsed_nodes);

    let mut no_shift_config = ReuseConfig::default();
    no_shift_config.max_position_shift = 0;

    let old_tree = Parser::new(source1).parse()?;
    let new_tree = Parser::new(source2).parse()?;
    let mut edits = EditSet::new();
    edits.add(Edit::new(
        8,
        9,
        12,
        Position::new(8, 1, 9),
        Position::new(9, 1, 10),
        Position::new(12, 1, 13),
    ));
    edits.add(Edit::new(
        34,
        37,
        35,
        Position::new(34, 3, 9),
        Position::new(37, 3, 12),
        Position::new(35, 3, 10),
    ));

    let mut conservative = AdvancedReuseAnalyzer::new();
    let conservative_result =
        conservative.analyze_reuse_opportunities(&old_tree, &new_tree, &edits, &no_shift_config);

    let mut shifted = AdvancedReuseAnalyzer::new();
    let shifted_result =
        shifted.analyze_reuse_opportunities(&old_tree, &new_tree, &edits, &ReuseConfig::default());
    assert!(
        shifted_result.reused_nodes >= conservative_result.reused_nodes,
        "expected shifted matching to perform at least as well as no-shift matching"
    );

    Ok(())
}

#[test]
fn separated_whitespace_and_comment_edits_improve_shifted_reuse_metrics() -> ParseResult<()> {
    let source1 = "my $a = 1;\nmy $b = 2;\nmy $c = 3;\n";
    let mut edits = EditSet::new();
    edits.add(Edit::new(
        5,
        5,
        7,
        Position::new(5, 1, 6),
        Position::new(5, 1, 6),
        Position::new(7, 1, 8),
    ));
    edits.add(Edit::new(
        22,
        22,
        29,
        Position::new(22, 2, 12),
        Position::new(22, 2, 12),
        Position::new(29, 2, 19),
    ));

    let source2 = "my $a   = 1;\nmy $b = 2; # note\nmy $c = 3;\n";
    let mut parser = IncrementalParserV2::new();
    parser.parse(source1)?;
    for edit in edits.edits().iter().cloned() {
        parser.edit(edit);
    }
    let incremental_tree = parser.parse(source2)?;
    let fresh_tree = Parser::new(source2).parse()?;

    assert_eq!(incremental_tree, fresh_tree);
    let old_tree = Parser::new(source1).parse()?;
    let new_tree = Parser::new(source2).parse()?;

    let mut no_shift_config = ReuseConfig::default();
    no_shift_config.max_position_shift = 0;

    let mut conservative = AdvancedReuseAnalyzer::new();
    let conservative_result =
        conservative.analyze_reuse_opportunities(&old_tree, &new_tree, &edits, &no_shift_config);

    let mut shifted = AdvancedReuseAnalyzer::new();
    let shifted_result =
        shifted.analyze_reuse_opportunities(&old_tree, &new_tree, &edits, &ReuseConfig::default());

    assert!(
        shifted_result.reused_nodes >= conservative_result.reused_nodes,
        "expected shifted matching to improve or match conservative reuse"
    );
    assert!(shifted_result.reused_nodes >= 6);

    Ok(())
}
