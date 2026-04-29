use crate::{
    AnchorFact, AnchorId, Confidence, EdgeFact, EdgeId, EdgeKind, EntityId, FileId, OccurrenceFact,
    OccurrenceId, OccurrenceKind, Provenance, ScopeId,
};
use perl_symbol::surface::{SymbolRef, SymbolRefKind};

/// Deterministic ID assignment for `SymbolRef` -> semantic occurrence facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolRefFactIds {
    pub anchor_id: AnchorId,
    pub occurrence_id: OccurrenceId,
    pub edge_id: EdgeId,
}

/// Canonical semantic facts emitted for one projected `SymbolRef`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolRefOccurrenceFacts {
    pub anchor: AnchorFact,
    pub occurrence: OccurrenceFact,
    pub references_edge: Option<EdgeFact>,
}

/// Adapter for phase-1 `SymbolRef` extraction into canonical semantic occurrence facts.
///
/// Intentional phase-2 exclusions remain the same as `SymbolRef` extraction:
/// static/instance method calls, coderef/indirect call families, and typeglob aliases.
pub fn symbol_ref_to_occurrence_facts(
    symbol_ref: &SymbolRef,
    ids: SymbolRefFactIds,
    file_id: FileId,
    scope_id: Option<ScopeId>,
    from_entity_id: Option<EntityId>,
    to_entity_id: Option<EntityId>,
) -> SymbolRefOccurrenceFacts {
    let anchor = AnchorFact {
        id: ids.anchor_id,
        file_id,
        span_start_byte: symbol_ref.full_span.0 as u32,
        span_end_byte: symbol_ref.full_span.1 as u32,
        scope_id,
        provenance: Provenance::ExactAst,
        confidence: Confidence::High,
    };

    let occurrence = OccurrenceFact {
        id: ids.occurrence_id,
        kind: occurrence_kind_for_symbol_ref_kind(&symbol_ref.kind),
        entity_id: to_entity_id,
        anchor_id: ids.anchor_id,
        scope_id,
        provenance: Provenance::ExactAst,
        confidence: Confidence::High,
    };

    let references_edge = from_entity_id.zip(to_entity_id).map(|(from, to)| EdgeFact {
        id: ids.edge_id,
        kind: EdgeKind::References,
        from_entity_id: from,
        to_entity_id: to,
        via_occurrence_id: Some(ids.occurrence_id),
        provenance: Provenance::ExactAst,
        confidence: Confidence::High,
    });

    SymbolRefOccurrenceFacts { anchor, occurrence, references_edge }
}

fn occurrence_kind_for_symbol_ref_kind(kind: &SymbolRefKind) -> OccurrenceKind {
    match kind {
        SymbolRefKind::Variable(_) => OccurrenceKind::Reference,
        SymbolRefKind::SubroutineCall => OccurrenceKind::Call,
    }
}
