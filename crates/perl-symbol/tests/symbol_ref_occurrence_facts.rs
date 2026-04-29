use perl_ast::{Node, NodeKind, SourceLocation};
use perl_semantic_facts::{AnchorId, EdgeId, EntityId, FileId, OccurrenceId, OccurrenceKind, ScopeId};
use perl_symbol::surface::{extract_symbol_refs, symbol_ref_to_occurrence_facts};

fn loc(start: usize, end: usize) -> SourceLocation {
    SourceLocation { start, end }
}

#[test]
fn adapter_emits_anchor_occurrence_and_reference_edge_for_variable_refs() {
    let variable = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "My::Pkg::value".to_string() },
        loc(4, 18),
    );
    let program = Node::new(NodeKind::Program { statements: vec![variable] }, loc(0, 18));
    let refs = extract_symbol_refs(&program);

    assert_eq!(refs.len(), 1);
    let facts = symbol_ref_to_occurrence_facts(
        &refs[0],
        FileId(7),
        Some(ScopeId(3)),
        AnchorId(11),
        OccurrenceId(13),
        Some(EntityId(17)),
        Some(EdgeId(19)),
    );

    assert_eq!(facts.anchor.id, AnchorId(11));
    assert_eq!(facts.anchor.file_id, FileId(7));
    assert_eq!(facts.anchor.span_start_byte, 4);
    assert_eq!(facts.anchor.span_end_byte, 18);
    assert_eq!(facts.occurrence.kind, OccurrenceKind::Reference);
    assert_eq!(facts.occurrence.entity_id, Some(EntityId(17)));

    assert!(facts.reference_edge.is_some());
    if let Some(edge) = facts.reference_edge {
        assert_eq!(edge.id, EdgeId(19));
        assert_eq!(edge.via_occurrence_id, Some(OccurrenceId(13)));
    }
}

#[test]
fn adapter_emits_call_occurrence_without_edge_when_ids_are_missing() {
    let call = Node::new(
        NodeKind::FunctionCall { name: "run_task".to_string(), args: vec![] },
        loc(20, 28),
    );
    let program = Node::new(NodeKind::Program { statements: vec![call] }, loc(20, 28));
    let refs = extract_symbol_refs(&program);

    assert_eq!(refs.len(), 1);
    let facts = symbol_ref_to_occurrence_facts(
        &refs[0],
        FileId(42),
        None,
        AnchorId(1),
        OccurrenceId(2),
        None,
        None,
    );

    assert_eq!(facts.anchor.span_start_byte, 20);
    assert_eq!(facts.anchor.span_end_byte, 28);
    assert_eq!(facts.occurrence.kind, OccurrenceKind::Call);
    assert!(facts.reference_edge.is_none());
}
