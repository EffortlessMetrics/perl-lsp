use perl_semantic_facts::adapters::{SymbolRefFactIds, symbol_ref_to_occurrence_facts};
use perl_semantic_facts::{AnchorId, EdgeId, EntityId, FileId, OccurrenceId, OccurrenceKind, ScopeId};
use perl_symbol::surface::{SymbolRef, SymbolRefKind};
use perl_symbol::VarKind;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn variable_ref_maps_to_reference_occurrence_without_edge_when_entities_unknown() -> Result<()> {
    let symbol_ref = SymbolRef {
        kind: SymbolRefKind::Variable(VarKind::Scalar),
        name: "count".to_string(),
        qualified_name: "My::Pkg::count".to_string(),
        sigil: Some("$".to_string()),
        package_qualifier: Some("My::Pkg".to_string()),
        full_span: (10, 15),
        anchor_span: Some((10, 15)),
    };

    let facts = symbol_ref_to_occurrence_facts(
        &symbol_ref,
        SymbolRefFactIds { anchor_id: AnchorId(1), occurrence_id: OccurrenceId(2), edge_id: EdgeId(3) },
        FileId(7),
        Some(ScopeId(9)),
        None,
        None,
    );

    assert_eq!(facts.anchor.span_start_byte, 10);
    assert_eq!(facts.anchor.span_end_byte, 15);
    assert_eq!(facts.occurrence.kind, OccurrenceKind::Reference);
    assert_eq!(facts.references_edge, None);
    Ok(())
}

#[test]
fn subroutine_call_maps_to_call_occurrence_and_references_edge() -> Result<()> {
    let symbol_ref = SymbolRef {
        kind: SymbolRefKind::SubroutineCall,
        name: "run".to_string(),
        qualified_name: "My::Pkg::run".to_string(),
        sigil: None,
        package_qualifier: Some("My::Pkg".to_string()),
        full_span: (20, 27),
        anchor_span: Some((20, 27)),
    };

    let facts = symbol_ref_to_occurrence_facts(
        &symbol_ref,
        SymbolRefFactIds { anchor_id: AnchorId(11), occurrence_id: OccurrenceId(12), edge_id: EdgeId(13) },
        FileId(2),
        None,
        Some(EntityId(101)),
        Some(EntityId(202)),
    );

    assert_eq!(facts.occurrence.kind, OccurrenceKind::Call);
    let edge = facts.references_edge.as_ref().ok_or("expected references edge")?;
    assert_eq!(edge.from_entity_id, EntityId(101));
    assert_eq!(edge.to_entity_id, EntityId(202));
    assert_eq!(edge.via_occurrence_id, Some(OccurrenceId(12)));
    Ok(())
}
