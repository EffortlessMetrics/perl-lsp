//! Adapter from phase-1 `SymbolRef` projections to canonical semantic facts.
//!
//! This adapter intentionally mirrors only the currently supported `SymbolRef`
//! families (variables + function calls). It does not attempt to model phase-2
//! families yet (method calls, coderef calls, indirect calls, typeglobs).

use crate::surface::{SymbolRef, SymbolRefKind};
use perl_semantic_facts::{
    AnchorFact, AnchorId, Confidence, EdgeFact, EdgeId, EdgeKind, EntityId, FileId, OccurrenceFact,
    OccurrenceId, OccurrenceKind, Provenance, ScopeId,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolRefOccurrenceFacts {
    pub anchor: AnchorFact,
    pub occurrence: OccurrenceFact,
    pub reference_edge: Option<EdgeFact>,
}

pub fn symbol_ref_to_occurrence_facts(
    symbol_ref: &SymbolRef,
    file_id: FileId,
    scope_id: Option<ScopeId>,
    anchor_id: AnchorId,
    occurrence_id: OccurrenceId,
    entity_id: Option<EntityId>,
    edge_id: Option<EdgeId>,
) -> SymbolRefOccurrenceFacts {
    let (start, end) = symbol_ref.anchor_span.unwrap_or(symbol_ref.full_span);

    let anchor = AnchorFact {
        id: anchor_id,
        file_id,
        span_start_byte: start as u32,
        span_end_byte: end as u32,
        scope_id,
        provenance: Provenance::ExactAst,
        confidence: Confidence::High,
    };

    let occurrence = OccurrenceFact {
        id: occurrence_id,
        kind: occurrence_kind_for_ref(symbol_ref),
        entity_id,
        anchor_id,
        scope_id,
        provenance: Provenance::ExactAst,
        confidence: Confidence::High,
    };

    let reference_edge = match (entity_id, edge_id) {
        (Some(target_id), Some(id)) => Some(EdgeFact {
            id,
            kind: EdgeKind::References,
            from_entity_id: target_id,
            to_entity_id: target_id,
            via_occurrence_id: Some(occurrence_id),
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        }),
        _ => None,
    };

    SymbolRefOccurrenceFacts { anchor, occurrence, reference_edge }
}

fn occurrence_kind_for_ref(symbol_ref: &SymbolRef) -> OccurrenceKind {
    match symbol_ref.kind {
        SymbolRefKind::SubroutineCall => OccurrenceKind::Call,
        SymbolRefKind::Variable(_) => OccurrenceKind::Reference,
    }
}
