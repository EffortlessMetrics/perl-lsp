#![cfg(feature = "incremental")]

use perl_parser::incremental_advanced_reuse::{AdvancedReuseAnalyzer, ReuseConfig, ReuseType};
use perl_parser_core::{
    SourceLocation,
    ast::{Node, NodeKind},
    edit::{Edit, EditSet},
    position::Position,
};
use std::collections::HashSet;

#[test]
fn shifted_identifier_reuse_is_preferred_for_small_prefix_insert() {
    let mut analyzer = AdvancedReuseAnalyzer::new();
    let config = ReuseConfig { min_confidence: 0.7, ..ReuseConfig::default() };

    let old_tree = Node::new(
        NodeKind::Program {
            statements: vec![Node::new(
                NodeKind::Identifier { name: "value".to_string() },
                SourceLocation { start: 10, end: 15 },
            )],
        },
        SourceLocation { start: 0, end: 15 },
    );

    let new_tree = Node::new(
        NodeKind::Program {
            statements: vec![Node::new(
                NodeKind::Identifier { name: "value".to_string() },
                SourceLocation { start: 14, end: 19 },
            )],
        },
        SourceLocation { start: 0, end: 19 },
    );

    let mut edits = EditSet::new();
    edits.add(Edit::new(
        0,
        0,
        4,
        Position::new(0, 0, 0),
        Position::new(0, 0, 0),
        Position::new(4, 0, 4),
    ));

    let result = analyzer.analyze_reuse_opportunities(&old_tree, &new_tree, &edits, &config);

    assert!(result.reuse_map.contains_key(&10));
    if let Some(identifier_reuse) = result.reuse_map.get(&10) {
        assert_eq!(identifier_reuse.target_position, 14);
        assert_ne!(identifier_reuse.reuse_type, ReuseType::StructuralEquivalent);
        assert!(identifier_reuse.confidence_score >= config.min_confidence);
    }
}

#[test]
fn container_reuse_not_selected_for_statement_count_change() {
    let mut analyzer = AdvancedReuseAnalyzer::new();
    let config = ReuseConfig::default();

    let old_tree = Node::new(
        NodeKind::Program {
            statements: vec![
                Node::new(
                    NodeKind::Number { value: "1".to_string() },
                    SourceLocation { start: 0, end: 1 },
                ),
                Node::new(
                    NodeKind::Number { value: "2".to_string() },
                    SourceLocation { start: 2, end: 3 },
                ),
            ],
        },
        SourceLocation { start: 0, end: 3 },
    );

    let new_tree = Node::new(
        NodeKind::Program {
            statements: vec![Node::new(
                NodeKind::Number { value: "1".to_string() },
                SourceLocation { start: 0, end: 1 },
            )],
        },
        SourceLocation { start: 0, end: 1 },
    );

    let mut edits = EditSet::new();
    edits.add(Edit::new(
        2,
        3,
        0,
        Position::new(2, 0, 2),
        Position::new(3, 0, 3),
        Position::new(2, 0, 2),
    ));

    let result = analyzer.analyze_reuse_opportunities(&old_tree, &new_tree, &edits, &config);
    let root = result.reuse_map.get(&0);
    assert!(
        root.is_none()
            || root.is_some_and(|strategy| strategy.reuse_type != ReuseType::StructuralEquivalent)
    );
}

#[test]
fn candidate_filtering_keeps_target_positions_unique() {
    let mut analyzer = AdvancedReuseAnalyzer::new();
    let config = ReuseConfig::default();

    let old_tree = Node::new(
        NodeKind::Program {
            statements: vec![
                Node::new(
                    NodeKind::Number { value: "10".to_string() },
                    SourceLocation { start: 10, end: 12 },
                ),
                Node::new(
                    NodeKind::Number { value: "10".to_string() },
                    SourceLocation { start: 20, end: 22 },
                ),
            ],
        },
        SourceLocation { start: 0, end: 22 },
    );
    let new_tree = Node::new(
        NodeKind::Program {
            statements: vec![
                Node::new(
                    NodeKind::Number { value: "10".to_string() },
                    SourceLocation { start: 100, end: 102 },
                ),
                Node::new(
                    NodeKind::Number { value: "10".to_string() },
                    SourceLocation { start: 110, end: 112 },
                ),
            ],
        },
        SourceLocation { start: 0, end: 112 },
    );

    let result =
        analyzer.analyze_reuse_opportunities(&old_tree, &new_tree, &EditSet::new(), &config);

    let target_positions: HashSet<usize> =
        result.reuse_map.values().map(|strategy| strategy.target_position).collect();
    assert_eq!(target_positions.len(), result.reuse_map.len());
}
