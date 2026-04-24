#![cfg(feature = "incremental")]

use perl_parser::incremental_advanced_reuse::{AdvancedReuseAnalyzer, ReuseConfig, ReuseType};
use perl_parser_core::{
    ast::{Node, NodeKind},
    edit::EditSet,
    SourceLocation,
};

#[test]
fn shifted_identifier_reuse_prefers_safe_shifted_candidate() {
    let mut analyzer = AdvancedReuseAnalyzer::new();
    let old_tree = Node::new(
        NodeKind::Identifier { name: "same".to_string() },
        SourceLocation { start: 100, end: 104 },
    );
    let new_tree = Node::new(
        NodeKind::Identifier { name: "same".to_string() },
        SourceLocation { start: 112, end: 116 },
    );
    let edits = EditSet::new();
    let config = ReuseConfig { max_position_shift: 200, ..ReuseConfig::default() };

    let result = analyzer.analyze_reuse_opportunities(&old_tree, &new_tree, &edits, &config);

    let reuse = result.reuse_map.get(&100);
    assert!(reuse.is_some());
    if let Some(reuse) = reuse {
        assert_eq!(reuse.reuse_type, ReuseType::PositionShift);
        assert_eq!(reuse.target_position, 112);
        assert!(reuse.confidence_score >= config.min_confidence);
    }
}

#[test]
fn unsafe_leaf_structural_edit_does_not_reuse_when_content_changes() {
    let mut analyzer = AdvancedReuseAnalyzer::new();
    let old_tree = Node::new(
        NodeKind::Number { value: "1".to_string() },
        SourceLocation { start: 10, end: 11 },
    );
    let new_tree = Node::new(
        NodeKind::Number { value: "2".to_string() },
        SourceLocation { start: 10, end: 11 },
    );
    let edits = EditSet::new();
    let config = ReuseConfig {
        enable_content_reuse: false,
        aggressive_structural_matching: true,
        min_confidence: 0.75,
        ..ReuseConfig::default()
    };

    let result = analyzer.analyze_reuse_opportunities(&old_tree, &new_tree, &edits, &config);

    assert!(result.reuse_map.is_empty());
}

#[test]
fn container_reuse_rejects_large_shift_for_structural_equivalent_matches() {
    let mut analyzer = AdvancedReuseAnalyzer::new();
    let old_tree = Node::new(
        NodeKind::Binary {
            op: "+".to_string(),
            left: Box::new(Node::new(
                NodeKind::Identifier { name: "a".to_string() },
                SourceLocation { start: 1, end: 2 },
            )),
            right: Box::new(Node::new(
                NodeKind::Identifier { name: "b".to_string() },
                SourceLocation { start: 5, end: 6 },
            )),
        },
        SourceLocation { start: 0, end: 6 },
    );
    let new_tree = Node::new(
        NodeKind::Binary {
            op: "+".to_string(),
            left: Box::new(Node::new(
                NodeKind::Identifier { name: "x".to_string() },
                SourceLocation { start: 401, end: 402 },
            )),
            right: Box::new(Node::new(
                NodeKind::Identifier { name: "y".to_string() },
                SourceLocation { start: 405, end: 406 },
            )),
        },
        SourceLocation { start: 400, end: 406 },
    );
    let edits = EditSet::new();
    let config = ReuseConfig {
        max_position_shift: 100,
        enable_content_reuse: false,
        ..ReuseConfig::default()
    };

    let result = analyzer.analyze_reuse_opportunities(&old_tree, &new_tree, &edits, &config);

    assert!(result.reuse_map.is_empty());
}
