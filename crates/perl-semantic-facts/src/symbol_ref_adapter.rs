//! Adapter from `perl_symbol::surface::SymbolRef` into semantic facts.
//!
//! Phase-1 coverage intentionally mirrors `SymbolRef` extraction and therefore
//! excludes method calls, coderef calls, indirect calls, and typeglob aliases.

use perl_symbol::surface::{SymbolRef, SymbolRefKind};

use crate::{
    AnchorFact, AnchorId, Confidence, EdgeFact, EdgeId, EdgeKind, EntityId, FileId, OccurrenceFact,
    OccurrenceId, OccurrenceKind, Provenance, ScopeId,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolRefFactBatch {
    pub anchors: Vec<AnchorFact>,
    pub occurrences: Vec<OccurrenceFact>,
    pub edges: Vec<EdgeFact>,
}

/// Convert phase-1 `SymbolRef` values into canonical occurrence/reference facts.
///
/// Facts are emitted in input order with deterministic IDs starting at 1.
pub fn adapt_symbol_refs_to_facts<F>(
    refs: &[SymbolRef],
    file_id: FileId,
    scope_id: Option<ScopeId>,
    mut resolve_entity_id: F,
) -> SymbolRefFactBatch
where
    F: FnMut(&SymbolRef) -> Option<EntityId>,
{
    let mut anchors = Vec::with_capacity(refs.len());
    let mut occurrences = Vec::with_capacity(refs.len());
    let mut edges = Vec::new();

    for (index, symbol_ref) in refs.iter().enumerate() {
        let one_indexed = (index as u64) + 1;
        let anchor_id = AnchorId(one_indexed);
        let occurrence_id = OccurrenceId(one_indexed);
        let span = symbol_ref.anchor_span.unwrap_or(symbol_ref.full_span);

        anchors.push(AnchorFact {
            id: anchor_id,
            file_id,
            span_start_byte: span.0 as u32,
            span_end_byte: span.1 as u32,
            scope_id,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        });

        let entity_id = resolve_entity_id(symbol_ref);
        occurrences.push(OccurrenceFact {
            id: occurrence_id,
            kind: occurrence_kind(symbol_ref),
            entity_id,
            anchor_id,
            scope_id,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
        });

        if let Some(to_entity_id) = entity_id {
            edges.push(EdgeFact {
                id: EdgeId(one_indexed),
                kind: EdgeKind::References,
                from_entity_id: to_entity_id,
                to_entity_id,
                via_occurrence_id: Some(occurrence_id),
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
            });
        }
    }

    SymbolRefFactBatch { anchors, occurrences, edges }
}

fn occurrence_kind(symbol_ref: &SymbolRef) -> OccurrenceKind {
    match symbol_ref.kind {
        SymbolRefKind::Variable(_) => OccurrenceKind::Reference,
        SymbolRefKind::SubroutineCall => OccurrenceKind::Call,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_symbol::{VarKind, surface::SymbolRef};

    #[test]
    fn adapts_variable_and_subroutine_refs_deterministically() {
        let refs = vec![
            SymbolRef {
                kind: SymbolRefKind::Variable(VarKind::Scalar),
                name: "x".to_string(),
                qualified_name: "Pkg::x".to_string(),
                sigil: Some("$".to_string()),
                package_qualifier: Some("Pkg".to_string()),
                full_span: (1, 4),
                anchor_span: Some((2, 3)),
            },
            SymbolRef {
                kind: SymbolRefKind::SubroutineCall,
                name: "run".to_string(),
                qualified_name: "Pkg::run".to_string(),
                sigil: None,
                package_qualifier: Some("Pkg".to_string()),
                full_span: (10, 18),
                anchor_span: None,
            },
        ];

        let facts = adapt_symbol_refs_to_facts(&refs, FileId(9), Some(ScopeId(3)), |symbol_ref| {
            if symbol_ref.qualified_name == "Pkg::x" {
                Some(EntityId(100))
            } else {
                None
            }
        });

        assert_eq!(facts.anchors.len(), 2);
        assert_eq!(facts.anchors[0].id, AnchorId(1));
        assert_eq!(facts.anchors[0].span_start_byte, 2);
        assert_eq!(facts.anchors[0].span_end_byte, 3);
        assert_eq!(facts.anchors[1].id, AnchorId(2));
        assert_eq!(facts.anchors[1].span_start_byte, 10);
        assert_eq!(facts.anchors[1].span_end_byte, 18);

        assert_eq!(facts.occurrences.len(), 2);
        assert_eq!(facts.occurrences[0].kind, OccurrenceKind::Reference);
        assert_eq!(facts.occurrences[0].entity_id, Some(EntityId(100)));
        assert_eq!(facts.occurrences[1].kind, OccurrenceKind::Call);
        assert_eq!(facts.occurrences[1].entity_id, None);

        assert_eq!(facts.edges.len(), 1);
        assert_eq!(facts.edges[0].id, EdgeId(1));
        assert_eq!(facts.edges[0].kind, EdgeKind::References);
        assert_eq!(facts.edges[0].from_entity_id, EntityId(100));
        assert_eq!(facts.edges[0].to_entity_id, EntityId(100));
        assert_eq!(facts.edges[0].via_occurrence_id, Some(OccurrenceId(1)));
    }
}
